//! Bounded extraction into a private sibling directory; never into the live app.
use super::{
    install::{Prepared, STAGE_PREFIX},
    release::{self, Platform, Release},
};
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

pub fn prepare(release: &Release, progress: impl FnMut(u8)) -> Result<Prepared, String> {
    let platform = release::platform().ok_or("Mise a jour indisponible pour ce systeme.")?;
    let exe = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|_| "Emplacement du lecteur introuvable.")?;
    let parent = exe.parent().ok_or("Dossier du lecteur introuvable.")?;
    let stage = tempfile::Builder::new()
        .prefix(STAGE_PREFIX)
        .tempdir_in(parent)
        .map_err(|_| "Dossier non modifiable. Deplacez Spotiamp dans votre dossier utilisateur.")?;
    let archive = stage.path().join("download.zip");
    release::download(release, &archive, progress)?;
    let files = extract(
        &archive,
        &stage.path().join("package"),
        platform,
        &release.version,
    )?;
    fs::remove_file(&archive).map_err(|_| "Impossible de finaliser le telechargement.")?;
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
) -> Result<Vec<PathBuf>, String> {
    let file = File::open(archive).map_err(|_| "Archive telechargee inaccessible.")?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| "Archive de mise a jour invalide.")?;
    if zip.is_empty() || zip.len() > MAX_FILES {
        return Err("Nombre de fichiers de mise a jour invalide.".into());
    }
    fs::create_dir(destination).map_err(|_| "Impossible de preparer la mise a jour.")?;
    let mut seen = HashSet::new();
    let mut total = 0_u64;
    let mut files = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|_| "Archive incomplete ou chiffree.")?;
        // Our packages contain regular files only; links and explicit directory entries are unnecessary.
        if !entry.is_file()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        {
            return Err("L'archive contient un lien ou un fichier non autorise.".into());
        }
        let name = entry
            .name()
            .map_err(|_| "Nom de fichier invalide dans la mise a jour.")?
            .into_owned();
        let relative =
            safe_path(&name, platform).ok_or("Chemin non autorise dans la mise a jour.")?;
        // Case folding also protects extraction on case-insensitive Windows/macOS filesystems.
        if !seen.insert(name.to_lowercase()) {
            return Err("Fichier duplique dans la mise a jour.".into());
        }
        let size = entry.size();
        total = total.checked_add(size).ok_or("Archive trop volumineuse.")?;
        if size > MAX_FILE || total > MAX_UNPACKED {
            return Err("Archive trop volumineuse.".into());
        }
        let output = destination.join(&relative);
        fs::create_dir_all(output.parent().unwrap())
            .map_err(|_| "Preparation du dossier impossible.")?;
        let mut target = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .map_err(|_| "Ecriture de la mise a jour impossible.")?;
        let actual = io::copy(&mut (&mut entry).take(size + 1), &mut target)
            .map_err(|_| "Contenu de l'archive corrompu.")?;
        if actual != size {
            return Err("Taille de fichier incorrecte dans l'archive.".into());
        }
        target
            .sync_all()
            .map_err(|_| "Enregistrement de la mise a jour impossible.")?;
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
            .map_err(|_| "Permissions de la mise a jour impossibles.")?;
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

fn validate_build(directory: &Path, platform: Platform, version: &str) -> Result<(), String> {
    let mut metadata = Vec::new();
    File::open(directory.join("BUILD-INFO.json"))
        .map_err(|_| "Informations de version manquantes.")?
        .take(64 * 1024 + 1)
        .read_to_end(&mut metadata)
        .map_err(|_| "Informations de version illisibles.")?;
    if metadata.len() > 64 * 1024 {
        return Err("Informations de version trop volumineuses.".into());
    }
    let build: BuildInfo =
        serde_json::from_slice(&metadata).map_err(|_| "Informations de version invalides.")?;
    if build.app_version != version
        || build.binary_name != platform.binary_name
        || !platform.targets.contains(&build.target.as_str())
        || build.git_commit.len() != 40
        || !build.git_commit.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Version ou architecture du paquet incompatible.".into());
    }
    let mut file = File::open(directory.join(platform.binary_name))
        .map_err(|_| "Executable absent de la mise a jour.")?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 32 * 1024];
    let mut size = 0_u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| "Executable illisible.")?;
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
        return Err("Verification de l'executable echouee.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
