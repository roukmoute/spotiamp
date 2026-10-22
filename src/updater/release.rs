//! Public GitHub releases and bounded, verified archive downloads.
use crate::localized_text::LocalizedText;

use reqwest::{Url, blocking::Client, header, redirect};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
    time::Duration,
};

const RELEASES_URL: &str = "https://api.github.com/repos/roukmoute/spotiamp/releases?per_page=100";
const DOWNLOAD_BASE: &str = "https://github.com/roukmoute/spotiamp/releases/download";
const MAX_JSON_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub tag: String,
    pub asset_url: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Platform {
    pub asset_name: &'static str,
    pub binary_name: &'static str,
    pub targets: &'static [&'static str],
}

pub fn platform() -> Option<Platform> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some(Platform {
            asset_name: "Spotiamp-Windows-x64.zip",
            binary_name: "spotiamp.exe",
            targets: &["x86_64-pc-windows-gnu", "x86_64-pc-windows-msvc"],
        }),
        ("linux", "x86_64") => Some(Platform {
            asset_name: "Spotiamp-Linux-x64.zip",
            binary_name: "spotiamp",
            targets: &["x86_64-unknown-linux-gnu"],
        }),
        ("macos", "aarch64") => Some(Platform {
            asset_name: "Spotiamp-macOS-arm64.zip",
            binary_name: "spotiamp",
            targets: &["aarch64-apple-darwin"],
        }),
        _ => None,
    }
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

fn allowed_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "api.github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}

fn client() -> Result<Client, LocalizedText> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "X-GitHub-Api-Version",
        header::HeaderValue::from_static("2022-11-28"),
    );
    Client::builder()
        .user_agent(concat!("Spotiamp/", env!("CARGO_PKG_VERSION")))
        .default_headers(headers)
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(REQUEST_TIMEOUT)
        .redirect(redirect::Policy::custom(|attempt| {
            if !allowed_url(attempt.url()) {
                attempt.error("Unauthorized redirect URL")
            } else {
                redirect::Policy::limited(5).redirect(attempt)
            }
        }))
        .build()
        .map_err(|error| {
            LocalizedText::new(
                format!("Connexion GitHub indisponible : {error}"),
                format!("GitHub connection unavailable: {error}"),
            )
        })
}

fn digest_valid(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_tag(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()
}

fn asset_url(tag: &str, platform: Platform) -> String {
    format!("{DOWNLOAD_BASE}/{tag}/{}", platform.asset_name)
}

fn select_release(
    releases: &[ApiRelease],
    current: &Version,
    platform: Platform,
) -> Result<Option<Release>, LocalizedText> {
    let mut best: Option<(Version, &ApiRelease, &ApiAsset)> = None;
    for release in releases.iter().take(100) {
        let Some(version) = parse_tag(&release.tag_name) else {
            continue;
        };
        if release.draft
            || !version.cmp_precedence(current).is_gt()
            || (current.pre.is_empty() && (release.prerelease || !version.pre.is_empty()))
            || best
                .as_ref()
                .is_some_and(|(previous, _, _)| !version.cmp_precedence(previous).is_gt())
        {
            continue;
        }
        let Some(asset) = release
            .assets
            .iter()
            .find(|asset| asset.name == platform.asset_name)
        else {
            continue;
        };
        best = Some((version, release, asset));
    }
    let Some((version, release, asset)) = best else {
        return Ok(None);
    };
    let digest = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|d| digest_valid(d))
        .ok_or(LocalizedText::new(
            "Une mise à jour existe, mais son empreinte SHA-256 est absente ou invalide",
            "An update is available, but its SHA-256 checksum is missing or invalid",
        ))?;
    if !(1..=MAX_ARCHIVE_BYTES).contains(&asset.size) {
        return Err(LocalizedText::new(
            "Une mise à jour existe, mais sa taille est invalide",
            "An update is available, but its size is invalid",
        ));
    }
    if asset.browser_download_url != asset_url(&release.tag_name, platform) {
        return Err(LocalizedText::new(
            "Une mise à jour existe, mais son adresse est invalide",
            "An update is available, but its URL is invalid",
        ));
    }
    Ok(Some(Release {
        version: version.to_string(),
        tag: release.tag_name.clone(),
        asset_url: asset.browser_download_url.clone(),
        bytes: asset.size,
        sha256: digest.to_ascii_lowercase(),
    }))
}

fn read_json(mut reader: impl Read) -> Result<Vec<u8>, LocalizedText> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(MAX_JSON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            LocalizedText::new(
                format!("Lecture des versions impossible : {error}"),
                format!("Could not read releases: {error}"),
            )
        })?;
    if bytes.len() as u64 > MAX_JSON_BYTES {
        return Err(LocalizedText::new(
            "La réponse GitHub dépasse la taille autorisée",
            "The GitHub response exceeds the allowed size",
        ));
    }
    Ok(bytes)
}

pub fn check() -> Result<Option<Release>, LocalizedText> {
    let platform = platform().ok_or(LocalizedText::new(
        "Mise à jour indisponible pour cette plateforme",
        "Updates are unavailable for this platform",
    ))?;
    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| {
        LocalizedText::new(
            "Version actuelle non reconnue",
            "Current version not recognized",
        )
    })?;
    let response = client()?
        .get(RELEASES_URL)
        .header(header::ACCEPT, "application/vnd.github+json")
        // A per-request timeout also bounds the asynchronous body behind blocking Read.
        .timeout(REQUEST_TIMEOUT)
        .send()
        .map_err(|error| {
            LocalizedText::new(
                format!("Recherche des versions impossible : {error}"),
                format!("Could not check releases: {error}"),
            )
        })?;
    if !response.status().is_success() {
        return Err(LocalizedText::new(
            format!("GitHub a répondu HTTP {}", response.status()),
            format!("GitHub returned HTTP {}", response.status()),
        ));
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_JSON_BYTES)
    {
        return Err(LocalizedText::new(
            "La réponse GitHub dépasse la taille autorisée",
            "The GitHub response exceeds the allowed size",
        ));
    }
    let releases: Vec<ApiRelease> =
        serde_json::from_slice(&read_json(response)?).map_err(|_| {
            LocalizedText::new(
                "La liste des versions GitHub est invalide",
                "The GitHub release list is invalid",
            )
        })?;
    select_release(&releases, &current, platform)
}

fn receive_archive(
    mut reader: impl Read,
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
    mut progress: impl FnMut(u8),
) -> Result<(), LocalizedText> {
    if !(1..=MAX_ARCHIVE_BYTES).contains(&expected_bytes) || !digest_valid(expected_sha256) {
        return Err(LocalizedText::new(
            "Métadonnées de téléchargement invalides",
            "Invalid download metadata",
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            LocalizedText::new(
                format!("Création du fichier de mise à jour impossible : {error}"),
                format!("Could not create the update file: {error}"),
            )
        })?;
    let result = (|| {
        let mut buffer = [0_u8; 32 * 1024];
        let mut sha256 = Sha256::new();
        let mut total = 0_u64;
        let mut last_progress = 0;
        progress(0);
        loop {
            // Read at most one byte beyond the advertised size to detect excess data.
            let capacity = (expected_bytes - total + 1).min(buffer.len() as u64) as usize;
            let count = match reader.read(&mut buffer[..capacity]) {
                Ok(count) => count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    return Err(LocalizedText::new(
                        format!("Téléchargement interrompu : {error}"),
                        format!("Download interrupted: {error}"),
                    ));
                }
            };
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > expected_bytes {
                return Err(LocalizedText::new(
                    "L'archive dépasse la taille annoncée",
                    "The archive exceeds its advertised size",
                ));
            }
            file.write_all(&buffer[..count]).map_err(|error| {
                LocalizedText::new(
                    format!("Écriture de la mise à jour impossible : {error}"),
                    format!("Could not write the update: {error}"),
                )
            })?;
            sha256.update(&buffer[..count]);
            let percent = ((total * 100 / expected_bytes).min(99)) as u8;
            if percent > last_progress {
                progress(percent);
                last_progress = percent;
            }
        }
        if total != expected_bytes {
            return Err(LocalizedText::new(
                "Le téléchargement est incomplet",
                "The download is incomplete",
            ));
        }
        if !format!("{:x}", sha256.finalize()).eq_ignore_ascii_case(expected_sha256) {
            return Err(LocalizedText::new(
                "L'empreinte SHA-256 de l'archive est incorrecte",
                "The archive SHA-256 checksum is incorrect",
            ));
        }
        file.sync_all().map_err(|error| {
            LocalizedText::new(
                format!("Enregistrement de la mise à jour impossible : {error}"),
                format!("Could not save the update: {error}"),
            )
        })?;
        progress(100);
        Ok(())
    })();
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}

pub fn download(
    release: &Release,
    path: &Path,
    progress: impl FnMut(u8),
) -> Result<(), LocalizedText> {
    let platform = platform().ok_or(LocalizedText::new(
        "Mise à jour indisponible pour cette plateforme",
        "Updates are unavailable for this platform",
    ))?;
    if !(1..=MAX_ARCHIVE_BYTES).contains(&release.bytes)
        || !digest_valid(&release.sha256)
        || release.asset_url != asset_url(&release.tag, platform)
        || parse_tag(&release.tag).is_none_or(|version| version.to_string() != release.version)
    {
        return Err(LocalizedText::new(
            "Métadonnées de téléchargement invalides",
            "Invalid download metadata",
        ));
    }
    let response = client()?
        .get(&release.asset_url)
        .header(header::ACCEPT, "application/octet-stream")
        .timeout(REQUEST_TIMEOUT)
        .send()
        .map_err(|error| {
            LocalizedText::new(
                format!("Téléchargement impossible : {error}"),
                format!("Could not download the update: {error}"),
            )
        })?;
    if !response.status().is_success() {
        return Err(LocalizedText::new(
            format!("GitHub a répondu HTTP {}", response.status()),
            format!("GitHub returned HTTP {}", response.status()),
        ));
    }
    if response
        .content_length()
        .is_some_and(|size| size != release.bytes)
    {
        return Err(LocalizedText::new(
            "La taille de l'archive ne correspond pas à celle annoncée",
            "The archive size differs from its advertised size",
        ));
    }
    receive_archive(response, path, release.bytes, &release.sha256, progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    const WINDOWS: Platform = Platform {
        asset_name: "Spotiamp-Windows-x64.zip",
        binary_name: "spotiamp.exe",
        targets: &["x86_64-pc-windows-gnu", "x86_64-pc-windows-msvc"],
    };

    fn release(tag: &str) -> serde_json::Value {
        json!({
            "tag_name": tag,
            "draft": false,
            "prerelease": tag.contains('-'),
            "assets": [{
                "name": WINDOWS.asset_name,
                "browser_download_url": asset_url(tag, WINDOWS),
                "size": 100,
                "digest": format!("sha256:{}", "a".repeat(64)),
            }],
        })
    }

    fn selection(
        releases: Vec<serde_json::Value>,
        current: &str,
    ) -> Result<Option<Release>, LocalizedText> {
        let releases: Vec<ApiRelease> =
            serde_json::from_value(serde_json::Value::Array(releases)).unwrap();
        select_release(&releases, &Version::parse(current).unwrap(), WINDOWS)
    }

    fn select(releases: Vec<serde_json::Value>, current: &str) -> Option<Release> {
        selection(releases, current).unwrap()
    }

    #[test]
    fn alpha_versions_follow_semver_precedence_in_any_response_order() {
        let chosen = select(
            vec![release("v0.1.0-alpha.10"), release("v0.1.0-alpha.9")],
            "0.1.0-alpha.8",
        )
        .unwrap();
        assert_eq!(chosen.version, "0.1.0-alpha.10");
        assert_eq!(chosen.tag, "v0.1.0-alpha.10");
    }

    #[test]
    fn stable_builds_exclude_prereleases_and_alphas_can_receive_stable() {
        assert!(select(vec![release("v0.2.0-alpha.1")], "0.1.0").is_none());
        let mut marked_preview = release("v0.2.0");
        marked_preview["prerelease"] = json!(true);
        assert!(select(vec![marked_preview], "0.1.0").is_none());
        assert_eq!(
            select(vec![release("v0.1.0")], "0.1.0-alpha.9")
                .unwrap()
                .version,
            "0.1.0"
        );
    }

    #[test]
    fn drafts_invalid_tags_other_platforms_and_old_versions_are_ignored() {
        let mut draft = release("v0.3.0");
        draft["draft"] = json!(true);
        let mut other = release("v0.4.0");
        other["assets"][0]["name"] = json!("Spotiamp-macOS-arm64.zip");
        assert!(
            select(
                vec![
                    draft,
                    other,
                    release("latest"),
                    release("v0.0.9"),
                    release("v0.1.0"),
                    release("v0.1.0+other-build")
                ],
                "0.1.0"
            )
            .is_none()
        );
    }

    #[test]
    fn missing_malformed_hashes_and_invalid_sizes_are_reported() {
        for digest in [
            serde_json::Value::Null,
            json!("sha256:abc"),
            json!(format!("sha256:{}", "g".repeat(64))),
            json!(format!("sha512:{}", "a".repeat(64))),
        ] {
            let mut bad = release("v0.2.0");
            bad["assets"][0]["digest"] = digest;
            assert!(selection(vec![bad], "0.1.0").is_err());
        }
        for size in [0, MAX_ARCHIVE_BYTES + 1] {
            let mut bad = release("v0.2.0");
            bad["assets"][0]["size"] = json!(size);
            assert!(selection(vec![bad], "0.1.0").is_err());
        }
        let mut valid = release("v0.2.0");
        valid["assets"][0]["size"] = json!(MAX_ARCHIVE_BYTES);
        assert!(select(vec![valid], "0.1.0").is_some());
    }

    #[test]
    fn asset_urls_must_match_the_repository_tag_and_platform_exactly() {
        for url in [
            "http://github.com/roukmoute/spotiamp/releases/download/v0.2.0/Spotiamp-Windows-x64.zip",
            "https://example.com/update.zip",
            "https://github.com/another/project/releases/download/v0.2.0/Spotiamp-Windows-x64.zip",
            "https://github.com/roukmoute/spotiamp/releases/download/v0.2.0/Spotiamp-Windows-x64.zip?other=1",
        ] {
            let mut bad = release("v0.2.0");
            bad["assets"][0]["browser_download_url"] = json!(url);
            assert!(selection(vec![bad], "0.1.0").is_err());
        }
    }

    #[test]
    fn invalid_newest_asset_is_not_hidden_by_an_older_valid_update() {
        let mut bad = release("v0.3.0");
        bad["assets"][0]["digest"] = serde_json::Value::Null;
        assert!(selection(vec![release("v0.2.0"), bad], "0.1.0").is_err());
    }

    #[test]
    fn redirects_only_allow_https_github_hosts_on_the_standard_port() {
        for url in [
            "https://github.com/a",
            "https://api.github.com/a",
            "https://release-assets.githubusercontent.com/a?signature=example",
            "https://objects.githubusercontent.com/a",
        ] {
            assert!(allowed_url(&Url::parse(url).unwrap()));
        }
        for url in [
            "http://github.com/a",
            "https://github.com.evil.invalid/a",
            "https://evil.invalid/a",
            "https://github.com:444/a",
            "https://user@github.com/a",
            "https://127.0.0.1/a",
        ] {
            assert!(!allowed_url(&Url::parse(url).unwrap()));
        }
    }

    #[test]
    fn json_read_has_a_hard_limit_even_without_content_length() {
        assert_eq!(read_json(Cursor::new(b"[]")).unwrap(), b"[]");
        assert!(read_json(std::io::repeat(b' ').take(MAX_JSON_BYTES + 1)).is_err());
    }

    #[test]
    fn streamed_archive_checks_hash_size_and_reports_completion_after_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.zip");
        let data = vec![42_u8; 70_000];
        let sha = format!("{:x}", Sha256::digest(&data));
        let mut progress = Vec::new();
        receive_archive(Cursor::new(&data), &path, data.len() as u64, &sha, |p| {
            progress.push(p)
        })
        .unwrap();
        assert_eq!(fs::read(path).unwrap(), data);
        assert_eq!(progress.first(), Some(&0));
        assert_eq!(progress.last(), Some(&100));
        assert!(progress.windows(2).all(|p| p[0] < p[1]));
    }

    #[test]
    fn bad_hash_truncated_or_excess_data_are_removed_and_never_complete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.zip");
        let data = b"archive fixture";
        let good_sha = format!("{:x}", Sha256::digest(data));
        for (size, hash) in [
            (data.len() as u64, "0".repeat(64)),
            (data.len() as u64 - 1, good_sha.clone()),
            (data.len() as u64 + 1, good_sha),
        ] {
            let mut progress = Vec::new();
            assert!(
                receive_archive(Cursor::new(data), &path, size, &hash, |p| progress.push(p))
                    .is_err()
            );
            assert!(!path.exists());
            assert!(!progress.contains(&100));
        }
    }

    #[test]
    fn read_errors_remove_partial_archives_and_existing_files_are_preserved() {
        struct BrokenReader;
        impl Read for BrokenReader {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("fixture read failed"))
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update.zip");
        let sha = format!("{:x}", Sha256::digest(b"a"));
        assert!(receive_archive(BrokenReader, &path, 1, &sha, |_| {}).is_err());
        assert!(!path.exists());
        fs::write(&path, b"keep").unwrap();
        assert!(receive_archive(Cursor::new(b"a"), &path, 1, &sha, |_| {}).is_err());
        assert_eq!(fs::read(path).unwrap(), b"keep");
    }
}
