use super::*;

struct Fixture {
    _root: tempfile::TempDir,
    root: PathBuf,
    stage: tempfile::TempDir,
    exe: PathBuf,
}
impl Fixture {
    fn new(process: bool) -> Self {
        let root = tempfile::Builder::new()
            .prefix("Spotiamp test espaces é ")
            .tempdir()
            .unwrap();
        let canonical = root.path().canonicalize().unwrap();
        let stage = tempfile::Builder::new()
            .prefix(STAGE_PREFIX)
            .tempdir_in(&canonical)
            .unwrap();
        fs::create_dir(stage.path().join("package")).unwrap();
        let exe = canonical.join(BINARY);
        if process {
            fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        } else {
            fs::write(&exe, b"old executable").unwrap();
        }
        Self {
            _root: root,
            root: canonical,
            stage,
            exe,
        }
    }
    fn payload(&self, name: &str, data: &[u8]) {
        let path = self.stage.path().join("package").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, data).unwrap();
    }
    fn manifest(&self, files: &[&str]) -> Manifest {
        let manifest = prepare_manifest(
            self.stage.path(),
            &self.exe,
            files.iter().map(PathBuf::from).collect(),
            true,
        )
        .unwrap();
        write_new(
            self.stage.path(),
            "manifest.json",
            &serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        manifest
    }
}

#[test]
fn package_replacement_preserves_unlisted_files_and_rolls_back_completely() {
    let fixture = Fixture::new(false);
    fs::write(fixture.root.join("README.md"), b"old readme").unwrap();
    fs::write(fixture.root.join("my playlist.txt"), b"personal").unwrap();
    fixture.payload("README.md", b"new readme");
    fixture.payload("licenses/new.txt", b"new license");
    fixture.payload(BINARY, b"new executable");
    let manifest = fixture.manifest(&[BINARY, "README.md", "licenses/new.txt"]);
    assert_eq!(manifest.files.last().unwrap().source, Path::new(BINARY));
    let mut transaction = Transaction::new(fixture.stage.path(), &fixture.root);
    transaction.replace(&manifest.files).unwrap();
    assert_eq!(fs::read(&fixture.exe).unwrap(), b"new executable");
    assert_eq!(
        fs::read(fixture.root.join("README.md")).unwrap(),
        b"new readme"
    );
    assert_eq!(
        fs::read(fixture.root.join("my playlist.txt")).unwrap(),
        b"personal"
    );
    transaction.rollback().unwrap();
    assert_eq!(fs::read(&fixture.exe).unwrap(), b"old executable");
    assert_eq!(
        fs::read(fixture.root.join("README.md")).unwrap(),
        b"old readme"
    );
    assert!(!fixture.root.join("licenses").exists());
    assert_eq!(
        fs::read(fixture.root.join("my playlist.txt")).unwrap(),
        b"personal"
    );
}

#[test]
fn mid_transaction_failure_restores_already_moved_files() {
    let fixture = Fixture::new(false);
    fs::write(fixture.root.join("README.md"), b"old readme").unwrap();
    fixture.payload("README.md", b"new readme");
    fixture.payload(BINARY, b"new executable");
    let manifest = fixture.manifest(&["README.md", BINARY]);
    // Simulate a payload disappearing after preflight, after an earlier file can move.
    fs::remove_file(fixture.stage.path().join("package").join(BINARY)).unwrap();
    let mut transaction = Transaction::new(fixture.stage.path(), &fixture.root);
    assert!(transaction.replace(&manifest.files).is_err());
    assert_eq!(
        fs::read(fixture.root.join("README.md")).unwrap(),
        b"new readme"
    );
    transaction.rollback().unwrap();
    assert_eq!(
        fs::read(fixture.root.join("README.md")).unwrap(),
        b"old readme"
    );
    assert_eq!(fs::read(&fixture.exe).unwrap(), b"old executable");
}

#[test]
fn validation_rejects_escape_conflicts_and_arbitrary_cleanup() {
    for path in [
        "../file",
        "/tmp/file",
        "",
        "a/b/../../c",
        "C:evil",
        ".spotiamp-update-other/file",
    ] {
        assert!(validate_relative(Path::new(path)).is_err(), "{path}");
    }
    // This validator receives native Paths; raw ZIP names have their own validator.
    assert!(validate_relative(&Path::new("a").join("b")).is_ok());
    #[cfg(windows)]
    {
        assert!(validate_relative(Path::new(r"a\b")).is_ok());
        assert!(validate_relative(Path::new(r"a\..\..\outside")).is_err());
    }
    #[cfg(unix)]
    assert!(validate_relative(Path::new(r"a\b")).is_err());
    let fixture = Fixture::new(false);
    fixture.payload(BINARY, b"new executable");
    fixture.payload("README.md", b"readme");
    let mut manifest = fixture.manifest(&[BINARY, "README.md"]);
    manifest.files.push(Entry {
        source: "README.md".into(),
        target: "README.md".into(),
        hash: String::new(),
    });
    assert!(validate_manifest(fixture.stage.path(), &manifest).is_err());
    assert!(read_manifest(fixture.stage.path(), "wrong-token").is_err());
    assert!(cleanup(&fixture.root, &manifest.nonce, &fixture.exe, false).is_err());
    assert!(fixture.exe.exists());
    assert!(matches!(
        dispatch_internal(&["spotiamp".into(), "--cleanup-update".into()]),
        Dispatch::Exit(Err(_))
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_target_or_stage_is_refused() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new(false);
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("kept.txt"), b"untouched").unwrap();
    symlink(outside.path(), fixture.root.join("licenses")).unwrap();
    assert!(check_path(&fixture.root, Path::new("licenses/kept.txt"), true).is_err());
    let fake_stage = fixture.root.join(format!("{STAGE_PREFIX}fake"));
    symlink(outside.path(), &fake_stage).unwrap();
    assert!(check_stage(&fake_stage, &fixture.root).is_err());
    assert_eq!(
        fs::read(outside.path().join("kept.txt")).unwrap(),
        b"untouched"
    );
}

fn fixture_command(exe: &Path, stage: &Path, manifest: &Manifest, failed: bool) -> Command {
    let mut command = quiet_command(exe);
    command
        .args([
            "--exact",
            "updater::install::tests::startup_process_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("SPOTIAMP_INSTALL_FIXTURE_STAGE", stage)
        .env("SPOTIAMP_INSTALL_FIXTURE_NONCE", &manifest.nonce)
        .env(
            "SPOTIAMP_INSTALL_FIXTURE_FAILED",
            if failed { "1" } else { "0" },
        );
    command
}

// The same Rust test executable is copied into an isolated temporary install.
// No shell/interpreter and no real Spotiamp executable are modified.
#[test]
#[ignore = "internal subprocess fixture, launched by installer tests"]
fn startup_process_fixture() {
    let Some(stage) = std::env::var_os("SPOTIAMP_INSTALL_FIXTURE_STAGE") else {
        return;
    };
    let stage = PathBuf::from(stage);
    let nonce = std::env::var("SPOTIAMP_INSTALL_FIXTURE_NONCE").unwrap();
    let failed = std::env::var("SPOTIAMP_INSTALL_FIXTURE_FAILED").unwrap() == "1";
    let manifest = read_manifest(&stage, &nonce).unwrap();
    write_new(
        &stage,
        "fixture-pid",
        std::process::id().to_string().as_bytes(),
    )
    .unwrap();
    finish_startup(
        &stage,
        &manifest,
        &std::env::current_exe().unwrap().canonicalize().unwrap(),
        failed,
    )
    .unwrap();
    if failed {
        assert!(
            read_small(&stage.join("update-error.txt"), 4096)
                .unwrap()
                .contains("restauree")
        );
    }
}

fn reap_fixture(stage: &Path) {
    let pid: u32 = read_small(&stage.join("fixture-pid"), 32)
        .unwrap()
        .parse()
        .unwrap();
    #[cfg(unix)]
    {
        unsafe extern "C" {
            fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
        }
        let deadline = Instant::now();
        loop {
            let mut status = 0;
            let result = unsafe { waitpid(pid as i32, &mut status, 1) };
            if result == pid as i32 {
                assert_eq!(status, 0, "fixture failed");
                break;
            }
            assert!(
                deadline.elapsed() < Duration::from_secs(5),
                "fixture did not exit"
            );
            thread::sleep(POLL);
        }
    }
    #[cfg(windows)]
    if let Ok(process) = ProcessWait::open(pid) {
        process.wait(Duration::from_secs(5)).unwrap();
    }
}

#[test]
fn copied_native_process_confirms_restart_then_stage_cleanup() {
    let fixture = Fixture::new(true);
    fs::copy(
        &fixture.exe,
        fixture.stage.path().join("package").join(BINARY),
    )
    .unwrap();
    fixture.payload("README.md", b"new package");
    fs::write(fixture.root.join("personal.txt"), b"preserved").unwrap();
    let manifest = fixture.manifest(&[BINARY, "README.md"]);
    install_and_restart(fixture.stage.path(), &manifest, fixture_command).unwrap();
    assert!(marker_matches(
        fixture.stage.path(),
        "new-commit",
        &manifest.nonce
    ));
    reap_fixture(fixture.stage.path());
    // The now-exited fixture PID also exercises the cleanup wait path.
    let pid = read_small(&fixture.stage.path().join("fixture-pid"), 32).unwrap();
    write_new(fixture.stage.path(), "helper-pid", pid.as_bytes()).unwrap();
    cleanup(fixture.stage.path(), &manifest.nonce, &fixture.exe, false).unwrap();
    assert!(!fixture.stage.path().exists());
    assert_eq!(
        fs::read(fixture.root.join("README.md")).unwrap(),
        b"new package"
    );
    assert_eq!(
        fs::read(fixture.root.join("personal.txt")).unwrap(),
        b"preserved"
    );
}

#[test]
fn invalid_new_executable_rolls_back_and_restarts_old_native_process() {
    let fixture = Fixture::new(true);
    let old_hash = hash_file(&fixture.exe).unwrap();
    fs::write(fixture.root.join("README.md"), b"old package").unwrap();
    fixture.payload(BINARY, b"this is not an executable");
    fixture.payload("README.md", b"new package");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            fixture.stage.path().join("package").join(BINARY),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    let manifest = fixture.manifest(&[BINARY, "README.md"]);
    install_and_restart(fixture.stage.path(), &manifest, fixture_command).unwrap();
    assert_eq!(hash_file(&fixture.exe).unwrap(), old_hash);
    assert_eq!(
        fs::read(fixture.root.join("README.md")).unwrap(),
        b"old package"
    );
    assert!(marker_matches(
        fixture.stage.path(),
        "old-commit",
        &manifest.nonce
    ));
    assert!(!fixture.stage.path().join("new-commit").exists());
    reap_fixture(fixture.stage.path());
}

#[test]
fn exclusive_lock_and_changed_installed_binary_prevent_handoff() {
    let fixture = Fixture::new(false);
    let first = install_lock(&fixture.root).unwrap();
    assert!(install_lock(&fixture.root).is_err());
    drop(first);
    assert!(install_lock(&fixture.root).is_ok());
    assert!(fixture.root.join(".spotiamp-update.lock").is_file());
    fixture.payload(BINARY, b"new executable");
    let manifest = fixture.manifest(&[BINARY]);
    fs::write(&fixture.exe, b"changed by another installation").unwrap();
    assert!(validate_payload(fixture.stage.path(), &fixture.root, &manifest).is_err());
    assert!(!fixture.stage.path().join("ready").exists());
}

#[test]
#[ignore = "internal subprocess fixture, launched by installer tests"]
fn waiting_process_fixture() {
    let Some(stage) = std::env::var_os("SPOTIAMP_INSTALL_WAIT_STAGE") else {
        return;
    };
    let stage = PathBuf::from(stage);
    write_marker(&stage, "fixture-alive", "alive").unwrap();
    wait_marker(&stage, "fixture-exit", "exit", Duration::from_secs(10)).unwrap();
}

#[test]
fn native_process_wait_blocks_replacement_until_running_executable_exits() {
    let fixture = Fixture::new(true);
    let mut command = quiet_command(&fixture.exe);
    command
        .args([
            "--exact",
            "updater::install::tests::waiting_process_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("SPOTIAMP_INSTALL_WAIT_STAGE", fixture.stage.path());
    let mut child = command.spawn().unwrap();
    wait_child_marker(
        fixture.stage.path(),
        "fixture-alive",
        "alive",
        &mut child,
        Duration::from_secs(5),
    )
    .unwrap();
    let process = ProcessWait::open(child.id()).unwrap();
    assert!(process.wait(Duration::from_millis(30)).is_err());
    #[cfg(windows)]
    assert!(
        OpenOptions::new().write(true).open(&fixture.exe).is_err(),
        "Windows must deny writes to the running executable"
    );
    write_marker(fixture.stage.path(), "fixture-exit", "exit").unwrap();
    // Reap concurrently: on Unix a dead but unreaped child still has a PID.
    let reaper = thread::spawn(move || child.wait().unwrap());
    process.wait(Duration::from_secs(5)).unwrap();
    assert!(reaper.join().unwrap().success());
    assert!(OpenOptions::new().write(true).open(&fixture.exe).is_ok());
}

#[cfg(windows)]
#[test]
fn windows_stage_uses_the_canonical_executable_parent_including_long_path_prefix() {
    let root = tempfile::Builder::new()
        .prefix("Spotiamp paths é ")
        .tempdir()
        .unwrap();
    let ordinary_exe = root.path().join(BINARY);
    fs::write(&ordinary_exe, b"test executable").unwrap();
    let canonical_exe = ordinary_exe.canonicalize().unwrap();
    let parent = canonical_exe.parent().unwrap();
    // This is the contract of package::prepare: canonicalize before tempdir_in.
    let stage = tempfile::Builder::new()
        .prefix(STAGE_PREFIX)
        .tempdir_in(parent)
        .unwrap();
    check_stage(stage.path(), parent).unwrap();
    assert_eq!(stage.path().canonicalize().unwrap(), stage.path());
    assert_eq!(stage.path().parent(), canonical_exe.parent());
}

#[cfg(windows)]
#[test]
fn windows_transient_sharing_lock_is_retried_then_replacement_succeeds() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new(false);
    let source = fixture.root.join("source.txt");
    let target = fixture.root.join("target.txt");
    fs::write(&source, b"replacement").unwrap();
    // Deny FILE_SHARE_DELETE while another process/scanner could hold this file.
    let held = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&source)
        .unwrap();
    let release = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        drop(held);
    });
    rename_file(&source, &target).unwrap();
    release.join().unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"replacement");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn exited_but_unreaped_parent_does_not_block_installation() {
    let fixture = Fixture::new(true);
    let mut command = quiet_command(&fixture.exe);
    command
        .args([
            "--exact",
            "updater::install::tests::waiting_process_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("SPOTIAMP_INSTALL_WAIT_STAGE", fixture.stage.path());
    let mut child = command.spawn().unwrap();
    wait_child_marker(
        fixture.stage.path(),
        "fixture-alive",
        "alive",
        &mut child,
        Duration::from_secs(5),
    )
    .unwrap();
    let process = ProcessWait::open(child.id()).unwrap();
    write_marker(fixture.stage.path(), "fixture-exit", "exit").unwrap();
    // Deliberately do not wait()/try_wait(): the terminated child remains a zombie.
    let result = process.wait(Duration::from_secs(5));
    #[cfg(target_os = "linux")]
    let was_zombie = linux_process_state(child.id()).is_some_and(|(state, _)| state == b'Z');
    #[cfg(target_os = "macos")]
    let was_zombie = mac_process_state(child.id())
        .unwrap()
        .is_some_and(|(state, _)| state == libc::SZOMB);
    let status = child.wait().unwrap();
    assert!(result.is_ok());
    assert!(
        was_zombie,
        "fixture must remain a zombie before its parent reaps it"
    );
    assert!(status.success());
}
