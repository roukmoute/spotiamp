//! Bounded extraction into a private sibling directory; never into the live app.
use super::{
    install::{Prepared, STAGE_PREFIX},
    release::{self, Platform, Release},
};
use crate::localized_text::LocalizedText;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

const MAX_FILES: usize = 10_000;
const MAX_UNPACKED: u64 = 128 * 1024 * 1024;
const MAX_FILE: u64 = 64 * 1024 * 1024;

pub fn prepare(release: &Release, progress: impl FnMut(u8)) -> Result<Prepared, LocalizedText> {
    let platform = release::platform().ok_or(LocalizedText::new(
        "Mise a jour indisponible pour ce systeme.",
        "Updates are unavailable for this system.",
    ))?;
    let exe = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|_| {
            LocalizedText::new(
                "Emplacement du lecteur introuvable.",
                "Could not locate the player.",
            )
        })?;
    let parent = exe.parent().ok_or(LocalizedText::new(
        "Dossier du lecteur introuvable.",
        "Could not locate the player directory.",
    ))?;
    let stage = tempfile::Builder::new()
        .prefix(STAGE_PREFIX)
        .tempdir_in(parent)
        .map_err(|_| {
            LocalizedText::new(
                "Dossier non modifiable. Deplacez Spotiamp dans votre dossier utilisateur.",
                "This directory is not writable. Move Spotiamp to your user directory.",
            )
        })?;
    let archive = stage.path().join("download.zip");
    release::download(release, &archive, progress)?;
    let files = extract(
        &archive,
        &stage.path().join("package"),
        platform,
        &release.version,
    )?;
    fs::remove_file(&archive).map_err(|_| {
        LocalizedText::new(
            "Impossible de finaliser le telechargement.",
            "Could not finish the download.",
        )
    })?;
    Ok(Prepared { stage, files })
}

fn safe_path(name: &str, platform: Platform) -> Option<PathBuf> {
    if name.is_empty() || name.contains(['\\', ':', '\0']) {
        return None;
    }
    let path = Path::new(name);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    // Reject dot components even when std::path normalizes them away.
    if name
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' ']))
    {
        return None;
    }
    let first = path.components().next()?.as_os_str().to_str()?;
    let allowed = if path.components().count() == 1 {
        first == platform.binary_name
            || first.ends_with(".md")
            || first.starts_with("LICENSE")
            || [
                "BUILD-INFO.json",
                "THIRD-PARTY.json",
                "THIRD-PARTY.txt",
                "license-origins.json",
            ]
            .contains(&first)
    } else {
        ["licenses", "mpl-source", "scripts"].contains(&first)
    };
    allowed.then(|| path.to_path_buf())
}

pub(super) fn extract(
    archive: &Path,
    destination: &Path,
    platform: Platform,
    version: &str,
) -> Result<Vec<PathBuf>, LocalizedText> {
    let file = File::open(archive).map_err(|_| {
        LocalizedText::new(
            "Archive telechargee inaccessible.",
            "The downloaded archive is inaccessible.",
        )
    })?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| {
        LocalizedText::new(
            "Archive de mise a jour invalide.",
            "Invalid update archive.",
        )
    })?;
    if zip.is_empty() || zip.len() > MAX_FILES {
        return Err(LocalizedText::new(
            "Nombre de fichiers de mise a jour invalide.",
            "Invalid number of update files.",
        ));
    }
    fs::create_dir(destination).map_err(|_| {
        LocalizedText::new(
            "Impossible de preparer la mise a jour.",
            "Could not prepare the update.",
        )
    })?;
    let mut seen = HashSet::new();
    let mut total = 0_u64;
    let mut files = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|_| {
            LocalizedText::new(
                "Archive incomplete ou chiffree.",
                "The archive is incomplete or encrypted.",
            )
        })?;
        // Our packages contain regular files only; links and explicit directory entries are unnecessary.
        if !entry.is_file()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        {
            return Err(LocalizedText::new(
                "L'archive contient un lien ou un fichier non autorise.",
                "The archive contains an unauthorized link or file.",
            ));
        }
        let name = entry
            .name()
            .map_err(|_| {
                LocalizedText::new(
                    "Nom de fichier invalide dans la mise a jour.",
                    "The update contains an invalid filename.",
                )
            })?
            .into_owned();
        let relative = safe_path(&name, platform).ok_or(LocalizedText::new(
            "Chemin non autorise dans la mise a jour.",
            "The update contains an unauthorized path.",
        ))?;
        // Case folding also protects extraction on case-insensitive Windows/macOS filesystems.
        if !seen.insert(name.to_lowercase()) {
            return Err(LocalizedText::new(
                "Fichier duplique dans la mise a jour.",
                "The update contains a duplicate file.",
            ));
        }
        let size = entry.size();
        total = total.checked_add(size).ok_or(LocalizedText::new(
            "Archive trop volumineuse.",
            "The archive is too large.",
        ))?;
        if size > MAX_FILE || total > MAX_UNPACKED {
            return Err(LocalizedText::new(
                "Archive trop volumineuse.",
                "The archive is too large.",
            ));
        }
        let output = destination.join(&relative);
        fs::create_dir_all(output.parent().unwrap()).map_err(|_| {
            LocalizedText::new(
                "Preparation du dossier impossible.",
                "Could not prepare the directory.",
            )
        })?;
        let mut target = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .map_err(|_| {
                LocalizedText::new(
                    "Ecriture de la mise a jour impossible.",
                    "Could not write the update.",
                )
            })?;
        let actual = io::copy(&mut (&mut entry).take(size + 1), &mut target).map_err(|_| {
            LocalizedText::new(
                "Contenu de l'archive corrompu.",
                "The archive contents are corrupted.",
            )
        })?;
        if actual != size {
            return Err(LocalizedText::new(
                "Taille de fichier incorrecte dans l'archive.",
                "A file in the archive has an incorrect size.",
            ));
        }
        target.sync_all().map_err(|_| {
            LocalizedText::new(
                "Enregistrement de la mise a jour impossible.",
                "Could not save the update.",
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                &output,
                fs::Permissions::from_mode(if relative == Path::new(platform.binary_name) {
                    0o755
                } else {
                    0o644
                }),
            )
            .map_err(|_| {
                LocalizedText::new(
                    "Permissions de la mise a jour impossibles.",
                    "Could not set update file permissions.",
                )
            })?;
        }
        files.push(relative);
    }
    validate_build(destination, platform, version)?;
    Ok(files)
}

#[derive(Deserialize)]
struct BuildInfo {
    app_version: String,
    git_commit: String,
    binary_name: String,
    target: String,
    binary_sha256: String,
    binary_bytes: u64,
}

fn validate_build(
    directory: &Path,
    platform: Platform,
    version: &str,
) -> Result<(), LocalizedText> {
    let mut metadata = Vec::new();
    File::open(directory.join("BUILD-INFO.json"))
        .map_err(|_| {
            LocalizedText::new(
                "Informations de version manquantes.",
                "Version information is missing.",
            )
        })?
        .take(64 * 1024 + 1)
        .read_to_end(&mut metadata)
        .map_err(|_| {
            LocalizedText::new(
                "Informations de version illisibles.",
                "Version information is unreadable.",
            )
        })?;
    if metadata.len() > 64 * 1024 {
        return Err(LocalizedText::new(
            "Informations de version trop volumineuses.",
            "Version information is too large.",
        ));
    }
    let build: BuildInfo = serde_json::from_slice(&metadata).map_err(|_| {
        LocalizedText::new(
            "Informations de version invalides.",
            "Version information is invalid.",
        )
    })?;
    if build.app_version != version
        || build.binary_name != platform.binary_name
        || !platform.targets.contains(&build.target.as_str())
        || build.git_commit.len() != 40
        || !build.git_commit.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(LocalizedText::new(
            "Version ou architecture du paquet incompatible.",
            "Incompatible package version or architecture.",
        ));
    }
    let mut file = File::open(directory.join(platform.binary_name)).map_err(|_| {
        LocalizedText::new(
            "Executable absent de la mise a jour.",
            "The update executable is missing.",
        )
    })?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 32 * 1024];
    let mut size = 0_u64;
    loop {
        let count = file.read(&mut buffer).map_err(|_| {
            LocalizedText::new("Executable illisible.", "The executable is unreadable.")
        })?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
        size += count as u64;
    }
    if size == 0
        || size != build.binary_bytes
        || format!("{:x}", hash.finalize()) != build.binary_sha256.to_lowercase()
    {
        return Err(LocalizedText::new(
            "Verification de l'executable echouee.",
            "Executable verification failed.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
