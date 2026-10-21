use super::*;
use std::io::Write;
use zip::write::SimpleFileOptions;

fn platform() -> Platform {
    Platform {
        asset_name: "Spotiamp-Linux-x64.zip",
        binary_name: "spotiamp",
        targets: &["x86_64-unknown-linux-gnu"],
    }
}
fn metadata(version: &str, binary: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"app_version":version,"git_commit":"a".repeat(40),"binary_name":"spotiamp","target":"x86_64-unknown-linux-gnu","binary_bytes":binary.len(),"binary_sha256":format!("{:x}",Sha256::digest(binary))})).unwrap()
}
fn make_zip(path: &Path, entries: &[(&str, Vec<u8>)]) {
    let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
    for (name, data) in entries {
        zip.start_file(
            *name,
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn valid_package_is_staged_with_exact_bytes_and_unix_executable_mode() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("valid.zip");
    let bin = b"test binary";
    make_zip(
        &archive,
        &[
            ("spotiamp", bin.to_vec()),
            ("BUILD-INFO.json", metadata("0.1.0-alpha.9", bin)),
            ("licenses/example/LICENSE", b"MIT".to_vec()),
        ],
    );
    let target = temp.path().join("package");
    let files = extract(&archive, &target, platform(), "0.1.0-alpha.9").unwrap();
    assert_eq!(files.len(), 3);
    assert_eq!(fs::read(target.join("spotiamp")).unwrap(), bin);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(target.join("spotiamp"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
}

#[test]
fn rejects_escape_paths_symlinks_and_case_collisions() {
    for name in [
        "../escape",
        "/escape",
        "licenses/../../escape",
        "licenses/./bad",
        "licenses/a\\b",
        "licenses/c:d",
        "licenses/trailing.",
        "licenses//bad",
        ".spotiamp-update-plan.json",
    ] {
        assert!(safe_path(name, platform()).is_none(), "{name}");
    }
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("duplicate.zip");
    make_zip(&archive, &[("README.md", vec![]), ("readme.md", vec![])]);
    assert!(
        extract(
            &archive,
            &temp.path().join("duplicate"),
            platform(),
            "1.0.0"
        )
        .unwrap_err()
        .contains("duplique")
    );
    let archive = temp.path().join("link.zip");
    let mut zip = zip::ZipWriter::new(File::create(&archive).unwrap());
    zip.add_symlink(
        "licenses/link",
        "../../escape",
        SimpleFileOptions::default(),
    )
    .unwrap();
    zip.finish().unwrap();
    assert!(extract(&archive, &temp.path().join("link"), platform(), "1.0.0").is_err());
    assert!(!temp.path().join("escape").exists());
}

#[test]
fn rejects_wrong_version_target_checksum_and_missing_binary() {
    for case in 0..5 {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("bad.zip");
        let bin = b"expected bytes";
        let mut meta: serde_json::Value = serde_json::from_slice(&metadata("1.0.0", bin)).unwrap();
        match case {
            0 => meta["app_version"] = "2.0.0".into(),
            1 => meta["target"] = "aarch64-apple-darwin".into(),
            2 => meta["binary_sha256"] = "0".repeat(64).into(),
            3 => meta["git_commit"] = serde_json::Value::Null,
            _ => {}
        }
        let mut entries = vec![("BUILD-INFO.json", serde_json::to_vec(&meta).unwrap())];
        if case != 4 {
            entries.push(("spotiamp", bin.to_vec()));
        }
        make_zip(&archive, &entries);
        assert!(extract(&archive, &temp.path().join("package"), platform(), "1.0.0").is_err());
    }
}

#[test]
fn rejects_oversized_entry_before_creating_output_file() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("oversized.zip");
    make_zip(&archive, &[("README.md", b"small".to_vec())]);
    // Tamper the declared uncompressed size in the central directory only.
    let mut bytes = fs::read(&archive).unwrap();
    let start = bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    bytes[start + 24..start + 28].copy_from_slice(&((MAX_FILE + 1) as u32).to_le_bytes());
    fs::write(&archive, bytes).unwrap();
    let package = temp.path().join("package");
    assert!(extract(&archive, &package, platform(), "1.0.0").is_err());
    assert!(!package.join("README.md").exists());
}

#[test]
#[ignore = "requires SPOTIAMP_RELEASE_ARCHIVE pointing to the native release ZIP"]
fn release_archive_passes_native_update_validation() {
    let archive = std::env::var_os("SPOTIAMP_RELEASE_ARCHIVE")
        .map(PathBuf::from)
        .expect("Set SPOTIAMP_RELEASE_ARCHIVE to the packaged release ZIP for this platform");
    let native = release::platform().expect("This platform has no release archive contract");
    let temporary = tempfile::tempdir().unwrap();
    let destination = temporary.path().join("package");
    let files = extract(&archive, &destination, native, env!("CARGO_PKG_VERSION")).expect(
        "The real release archive must pass the updater's native extraction and validation",
    );
    assert!(files.contains(&PathBuf::from(native.binary_name)));
    assert!(files.contains(&PathBuf::from("BUILD-INFO.json")));
    let metadata: BuildInfo =
        serde_json::from_slice(&fs::read(destination.join("BUILD-INFO.json")).unwrap()).unwrap();
    assert_eq!(metadata.app_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(metadata.binary_name, native.binary_name);
    assert!(native.targets.contains(&metadata.target.as_str()));
    assert_eq!(metadata.git_commit.len(), 40);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(destination.join(native.binary_name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
    println!(
        "Validated {} files for {} version {} from commit {} (binary SHA256 {}).",
        files.len(),
        metadata.target,
        metadata.app_version,
        metadata.git_commit,
        metadata.binary_sha256,
    );
    // The temporary directory is dropped here; the executable is never run or installed.
}
