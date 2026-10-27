//! Native, bounded handoff: the running application never overwrites itself.
//! Only the files in a verified package are replaced; other files are untouched.
use super::{error, in_languages};
use crate::locale::Language;
use crate::localized_text::LocalizedText;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const STAGE_PREFIX: &str = ".spotiamp-update-";
#[cfg(not(test))]
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
// Native installer fixtures hash a copied ~261 MB debug test executable before
// signalling startup. Concurrent hashes on a constrained CI runner can exceed
// the production deadline, whose release executable is only about 9 MB.
#[cfg(test)]
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);
const EXIT_TIMEOUT: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(25);
const MAX_MANIFEST: u64 = 4 * 1024 * 1024;
const BINARY: &str = if cfg!(windows) {
    "spotiamp.exe"
} else {
    "spotiamp"
};
const HELPER: &str = if cfg!(windows) {
    "helper.exe"
} else {
    "helper"
};

pub struct Prepared {
    pub stage: tempfile::TempDir,
    pub files: Vec<PathBuf>,
}

pub enum Dispatch {
    NotInternal,
    Exit(Result<(), LocalizedText>),
    Continue(Result<(), LocalizedText>),
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    schema: u32,
    nonce: String,
    created: u64,
    parent_pid: u32,
    target_name: PathBuf,
    old_hash: String,
    new_hash: String,
    scale2: bool,
    files: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    source: PathBuf,
    target: PathBuf,
    hash: String,
}

/// Called on the update worker. Success means the helper is ready and the UI
/// must now close normally. The helper still waits for this process to exit.
pub fn launch(prepared: Prepared, scale2: bool) -> Result<(), LocalizedText> {
    let exe = std::env::current_exe()
        .map_err(|e| error("Executable introuvable", "Could not find the executable", e))?;
    let exe = exe.canonicalize().map_err(|e| {
        error(
            "Executable inaccessible",
            "The executable is inaccessible",
            e,
        )
    })?;
    let stage = prepared.stage.path();
    check_stage(
        stage,
        exe.parent().ok_or(LocalizedText::new(
            "Dossier du lecteur introuvable.",
            "Could not locate the player directory.",
        ))?,
    )?;
    let manifest = prepare_manifest(stage, &exe, prepared.files, scale2)?;
    fs::copy(&exe, stage.join(HELPER)).map_err(|e| {
        error(
            "Copie de l'assistant impossible",
            "Could not copy the helper",
            e,
        )
    })?;
    write_new(
        stage,
        "manifest.json",
        &serde_json::to_vec(&manifest).map_err(|e| e.to_string())?,
    )?;
    let mut command = quiet_command(&stage.join(HELPER));
    command
        .arg("--apply-update")
        .arg(stage)
        .arg(&manifest.nonce);
    let mut helper = command.spawn().map_err(|e| {
        error(
            "Lancement de l'assistant impossible",
            "Could not start the helper",
            e,
        )
    })?;
    let ready = wait_child_marker(
        stage,
        "ready",
        &manifest.nonce,
        &mut helper,
        HANDSHAKE_TIMEOUT,
    );
    if let Err(failure) = ready {
        stop_child(&mut helper);
        return Err(read_small(&stage.join("helper-error.txt"), 4096)
            .map(helper_failure)
            .unwrap_or(failure));
    }
    // Keep the stage before granting permission: TempDir must never race the helper.
    let stage = prepared.stage.keep();
    if let Err(failure) = write_marker(&stage, "go", &manifest.nonce) {
        stop_child(&mut helper);
        let _ = fs::remove_dir_all(&stage);
        return Err(failure);
    }
    Ok(())
}

/// Pass the complete args_os list, including argv[0], before starting UI/audio.
pub fn dispatch_internal(args: &[OsString]) -> Dispatch {
    let Some(mode) = args.get(1).and_then(|s| s.to_str()) else {
        return Dispatch::NotInternal;
    };
    if !["--apply-update", "--cleanup-update", "--update-failed"].contains(&mode) {
        return Dispatch::NotInternal;
    }
    let mut startup_committed = false;
    let parsed = (|| {
        let stage = args.get(2).map(PathBuf::from).ok_or(LocalizedText::new(
            "Dossier de mise a jour absent.",
            "The update directory is missing.",
        ))?;
        let nonce = args
            .get(3)
            .and_then(|s| s.to_str())
            .ok_or(LocalizedText::new(
                "Confirmation de mise a jour absente.",
                "Update confirmation is missing.",
            ))?;
        let exe = std::env::current_exe()
            .map_err(|e| error("Executable introuvable", "Could not find the executable", e))?
            .canonicalize()
            .map_err(|e| {
                error(
                    "Executable inaccessible",
                    "The executable is inaccessible",
                    e,
                )
            })?;
        let manifest = read_manifest(&stage, nonce)?;
        if mode == "--apply-update" {
            check_stage(
                &stage,
                stage.parent().ok_or(LocalizedText::new(
                    "Dossier invalide.",
                    "Invalid directory.",
                ))?,
            )?;
            if exe != stage.join(HELPER) || hash_file(&exe)? != manifest.old_hash {
                return Err(LocalizedText::new(
                    "Assistant de mise a jour invalide.",
                    "Invalid update helper.",
                ));
            }
            let result = apply(&stage, &manifest);
            if let Err(failure) = &result {
                let _ = write_notice(&stage, "helper-error.txt", failure);
            }
            result
        } else {
            let failed = mode == "--update-failed";
            finish_startup(&stage, &manifest, &exe, failed)?;
            startup_committed = true;
            let notice = if failed {
                Err(read_small(&stage.join("update-error.txt"), 4096)
                    .map(helper_failure)
                    .unwrap_or_else(|_| {
                        LocalizedText::new(
                            "La mise a jour a echoue ; la version precedente a ete restauree.",
                            "The update failed; the previous version was restored.",
                        )
                    }))
            } else {
                Ok(())
            };
            let cleanup_stage = stage.clone();
            let cleanup_nonce = manifest.nonce.clone();
            let _ = thread::Builder::new()
                .name("spotiamp-update-cleanup".into())
                .spawn(move || {
                    let _ = cleanup(&cleanup_stage, &cleanup_nonce, &exe, failed);
                });
            notice
        }
    })();
    if mode == "--apply-update" {
        Dispatch::Exit(parsed)
    } else {
        // A failed validation/handshake must not start the UI while rollback is
        // potentially replacing this executable. Only a committed old/new start continues.
        if startup_committed {
            Dispatch::Continue(parsed)
        } else {
            Dispatch::Exit(parsed)
        }
    }
}

fn prepare_manifest(
    stage: &Path,
    exe: &Path,
    files: Vec<PathBuf>,
    scale2: bool,
) -> Result<Manifest, LocalizedText> {
    if files.is_empty() || files.len() > 10_000 {
        return Err(LocalizedText::new(
            "Liste de fichiers invalide.",
            "Invalid file list.",
        ));
    }
    let target_name = PathBuf::from(exe.file_name().ok_or(LocalizedText::new(
        "Nom du lecteur introuvable.",
        "Could not find the player filename.",
    ))?);
    validate_relative(&target_name)?;
    let token = tempfile::Builder::new()
        .prefix("nonce-")
        .rand_bytes(32)
        .tempfile_in(stage)
        .map_err(|e| {
            error(
                "Preparation de la confirmation impossible",
                "Could not prepare confirmation",
                e,
            )
        })?;
    let nonce = token
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    drop(token);
    let mut entries = Vec::with_capacity(files.len());
    for source in files {
        validate_relative(&source)?;
        let target = if source == Path::new(BINARY) {
            target_name.clone()
        } else {
            source.clone()
        };
        check_path(&stage.join("package"), &source, false)?;
        entries.push(Entry {
            hash: hash_file(&stage.join("package").join(&source))?,
            source,
            target,
        });
    }
    let new_hash = entries
        .iter()
        .find(|e| e.source == Path::new(BINARY))
        .ok_or(LocalizedText::new(
            "Executable absent du paquet.",
            "The package executable is missing.",
        ))?
        .hash
        .clone();
    // Replace the executable last, minimizing the time it is absent.
    entries.sort_by_key(|e| e.source == Path::new(BINARY));
    let manifest = Manifest {
        schema: 1,
        nonce,
        created: now(),
        parent_pid: std::process::id(),
        target_name,
        old_hash: hash_file(exe)?,
        new_hash,
        scale2,
        files: entries,
    };
    validate_manifest(stage, &manifest)?;
    Ok(manifest)
}

fn read_manifest(stage: &Path, nonce: &str) -> Result<Manifest, LocalizedText> {
    check_stage(
        stage,
        stage.parent().ok_or(LocalizedText::new(
            "Dossier invalide.",
            "Invalid directory.",
        ))?,
    )?;
    check_regular(&stage.join("manifest.json"))?;
    let text = read_small(&stage.join("manifest.json"), MAX_MANIFEST)?;
    let manifest: Manifest = serde_json::from_str(&text).map_err(|_| {
        LocalizedText::new(
            "Manifeste de mise a jour invalide.",
            "Invalid update manifest.",
        )
    })?;
    if nonce != manifest.nonce
        || nonce.len() < 32
        || nonce.len() > 128
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(LocalizedText::new(
            "Confirmation de mise a jour invalide.",
            "Invalid update confirmation.",
        ));
    }
    validate_manifest(stage, &manifest)?;
    Ok(manifest)
}

fn validate_manifest(stage: &Path, manifest: &Manifest) -> Result<(), LocalizedText> {
    if manifest.schema != 1
        || manifest.parent_pid == 0
        || manifest.created > now() + 60
        || now().saturating_sub(manifest.created) > 86_400
        || manifest.files.is_empty()
        || manifest.files.len() > 10_000
    {
        return Err(LocalizedText::new(
            "Manifeste de mise a jour expire ou invalide.",
            "Expired or invalid update manifest.",
        ));
    }
    validate_relative(&manifest.target_name)?;
    if manifest.target_name.components().count() != 1 {
        return Err(LocalizedText::new(
            "Nom de lecteur invalide.",
            "Invalid player filename.",
        ));
    }
    let mut seen = HashSet::new();
    for entry in &manifest.files {
        validate_relative(&entry.source)?;
        validate_relative(&entry.target)?;
        let expected = if entry.source == Path::new(BINARY) {
            &manifest.target_name
        } else {
            &entry.source
        };
        if &entry.target != expected || !seen.insert(entry.target.to_string_lossy().to_lowercase())
        {
            return Err(LocalizedText::new(
                "Destination dupliquee ou invalide.",
                "Duplicate or invalid destination.",
            ));
        }
    }
    for entry in &manifest.files {
        for ancestor in entry
            .target
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
        {
            if seen.contains(&ancestor.to_string_lossy().to_lowercase()) {
                return Err(LocalizedText::new(
                    "Conflit de destinations.",
                    "Conflicting destinations.",
                ));
            }
        }
    }
    if !manifest
        .files
        .iter()
        .any(|entry| entry.source == Path::new(BINARY) && entry.hash == manifest.new_hash)
    {
        return Err(LocalizedText::new(
            "Executable absent du manifeste.",
            "The executable is missing from the manifest.",
        ));
    }
    check_stage(
        stage,
        stage.parent().ok_or(LocalizedText::new(
            "Dossier invalide.",
            "Invalid directory.",
        ))?,
    )
}

fn apply(stage: &Path, manifest: &Manifest) -> Result<(), LocalizedText> {
    let target = stage.parent().ok_or(LocalizedText::new(
        "Dossier du lecteur introuvable.",
        "Could not locate the player directory.",
    ))?;
    let _lock = install_lock(target)?;
    // Preflight happens while the old UI is alive; permission/path failures keep it alive.
    validate_payload(stage, target, manifest)?;
    let parent = ProcessWait::open(manifest.parent_pid)?;
    write_new(
        stage,
        "helper-pid",
        std::process::id().to_string().as_bytes(),
    )?;
    write_marker(stage, "ready", &manifest.nonce)?;
    wait_marker(stage, "go", &manifest.nonce, HANDSHAKE_TIMEOUT)?;
    parent.wait(EXIT_TIMEOUT)?;
    install_and_restart(stage, manifest, normal_restart)
}

type CommandBuilder = fn(&Path, &Path, &Manifest, bool) -> Command;

fn install_and_restart(
    stage: &Path,
    manifest: &Manifest,
    command: CommandBuilder,
) -> Result<(), LocalizedText> {
    let target = stage.parent().ok_or(LocalizedText::new(
        "Dossier du lecteur introuvable.",
        "Could not locate the player directory.",
    ))?;
    let mut transaction = Transaction::new(stage, target);
    let result = validate_payload(stage, target, manifest)
        .and_then(|()| transaction.replace(&manifest.files))
        .and_then(|()| restart(stage, manifest, false, command));
    match result {
        Ok(()) => Ok(()),
        Err(failure) => {
            if let Err(rollback) = transaction.rollback() {
                let message = in_languages(|language| {
                    format!(
                        "{} {}\n{} {}\n{} {}",
                        language.text("Mise a jour echouee :", "Update failed:"),
                        failure.text(language),
                        language.text("Restauration incomplete :", "Rollback incomplete:"),
                        rollback.text(language),
                        language.text("Sauvegardes conservees dans", "Backups retained in"),
                        stage.display()
                    )
                });
                let _ = write_notice(stage, "update-error.txt", &message);
                return Err(message);
            }
            let message = in_languages(|language| {
                format!(
                    "{} {}",
                    language.text(
                        "Mise a jour annulee ; version precedente restauree.",
                        "Update cancelled; previous version restored.",
                    ),
                    failure.text(language)
                )
            });
            write_notice(stage, "update-error.txt", &message)?;
            restart(stage, manifest, true, command).map_err(|e| {
                in_languages(|language| {
                    format!(
                        "{}\n{} {}",
                        message.text(language),
                        language.text("Relancement impossible :", "Could not restart:"),
                        e.text(language)
                    )
                })
            })
        }
    }
}

fn validate_payload(stage: &Path, target: &Path, manifest: &Manifest) -> Result<(), LocalizedText> {
    if hash_file(&target.join(&manifest.target_name))? != manifest.old_hash {
        return Err(LocalizedText::new(
            "La version installee a change ; recommencez la mise a jour.",
            "The installed version has changed; start the update again.",
        ));
    }
    for entry in &manifest.files {
        check_path(&stage.join("package"), &entry.source, false)?;
        if hash_file(&stage.join("package").join(&entry.source))? != entry.hash {
            return Err(LocalizedText::new(
                "Le paquet prepare a change.",
                "The prepared package has changed.",
            ));
        }
        check_path(target, &entry.target, true)?;
    }
    Ok(())
}

struct Replacement {
    source: PathBuf,
    target: PathBuf,
    backup: Option<PathBuf>,
    installed: bool,
}
struct Transaction<'a> {
    stage: &'a Path,
    target: &'a Path,
    changes: Vec<Replacement>,
    created_dirs: Vec<PathBuf>,
}
impl<'a> Transaction<'a> {
    fn new(stage: &'a Path, target: &'a Path) -> Self {
        Self {
            stage,
            target,
            changes: Vec::new(),
            created_dirs: Vec::new(),
        }
    }
    fn replace(&mut self, entries: &[Entry]) -> Result<(), LocalizedText> {
        fs::create_dir(self.stage.join("backup")).map_err(|e| {
            error(
                "Creation des sauvegardes impossible",
                "Could not create backups",
                e,
            )
        })?;
        for entry in entries {
            check_path(self.target, &entry.target, true)?;
            let source = self.stage.join("package").join(&entry.source);
            check_regular(&source)?;
            let target = self.target.join(&entry.target);
            create_parents(self.target, &entry.target, &mut self.created_dirs)?;
            let backup = if target.exists() {
                let backup = self.stage.join("backup").join(&entry.target);
                fs::create_dir_all(backup.parent().unwrap()).map_err(|e| {
                    error(
                        "Creation des sauvegardes impossible",
                        "Could not create backups",
                        e,
                    )
                })?;
                rename_file(&target, &backup).map_err(|e| {
                    error(
                        "Sauvegarde du fichier impossible",
                        "Could not back up the file",
                        e,
                    )
                })?;
                Some(backup)
            } else {
                None
            };
            self.changes.push(Replacement {
                source: source.clone(),
                target: target.clone(),
                backup,
                installed: false,
            });
            rename_file(&source, &target).map_err(|e| {
                error(
                    "Remplacement du fichier impossible",
                    "Could not replace the file",
                    e,
                )
            })?;
            self.changes.last_mut().unwrap().installed = true;
        }
        Ok(())
    }
    fn rollback(&mut self) -> Result<(), LocalizedText> {
        let mut errors = Vec::new();
        for change in self.changes.iter().rev() {
            if change.installed
                && let Err(e) = rename_file(&change.target, &change.source)
            {
                errors.push(error(
                    "Retrait du nouveau fichier impossible",
                    "Could not remove the new file",
                    e,
                ));
                continue;
            }
            if let Some(backup) = &change.backup
                && let Err(e) = rename_file(backup, &change.target)
            {
                errors.push(error(
                    "Restauration du fichier impossible",
                    "Could not restore the file",
                    e,
                ));
            }
        }
        for directory in self.created_dirs.iter().rev() {
            let _ = fs::remove_dir(directory);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(in_languages(|language| {
                errors
                    .iter()
                    .map(|e| e.text(language))
                    .collect::<Vec<_>>()
                    .join(if language == Language::French {
                        " ; "
                    } else {
                        "; "
                    })
            }))
        }
    }
}

fn normal_restart(exe: &Path, stage: &Path, manifest: &Manifest, failed: bool) -> Command {
    let mut command = quiet_command(exe);
    command
        .arg(if failed {
            "--update-failed"
        } else {
            "--cleanup-update"
        })
        .arg(stage)
        .arg(&manifest.nonce);
    if manifest.scale2 {
        command.args(["--scale", "2"]);
    }
    command
}

fn restart(
    stage: &Path,
    manifest: &Manifest,
    failed: bool,
    command: impl FnOnce(&Path, &Path, &Manifest, bool) -> Command,
) -> Result<(), LocalizedText> {
    let exe = stage.parent().unwrap().join(&manifest.target_name);
    check_regular(&exe)?;
    let mut child = command(&exe, stage, manifest, failed)
        .spawn()
        .map_err(|e| {
            error(
                "Relancement du lecteur impossible",
                "Could not restart the player",
                e,
            )
        })?;
    let prefix = if failed { "old" } else { "new" };
    let result = wait_child_marker(
        stage,
        &format!("{prefix}-started"),
        &manifest.nonce,
        &mut child,
        HANDSHAKE_TIMEOUT,
    )
    .and_then(|()| write_marker(stage, &format!("{prefix}-commit"), &manifest.nonce));
    if result.is_err() {
        stop_child(&mut child);
    }
    result
}

fn finish_startup(
    stage: &Path,
    manifest: &Manifest,
    exe: &Path,
    failed: bool,
) -> Result<(), LocalizedText> {
    let target = exe.parent().ok_or(LocalizedText::new(
        "Dossier du lecteur introuvable.",
        "Could not locate the player directory.",
    ))?;
    check_stage(stage, target)?;
    if exe != target.join(&manifest.target_name)
        || &hash_file(exe)?
            != if failed {
                &manifest.old_hash
            } else {
                &manifest.new_hash
            }
    {
        return Err(LocalizedText::new(
            "Executable de redemarrage invalide.",
            "Invalid restart executable.",
        ));
    }
    let prefix = if failed { "old" } else { "new" };
    write_marker(stage, &format!("{prefix}-started"), &manifest.nonce)?;
    wait_marker(
        stage,
        &format!("{prefix}-commit"),
        &manifest.nonce,
        HANDSHAKE_TIMEOUT,
    )
}

fn cleanup(stage: &Path, nonce: &str, exe: &Path, failed: bool) -> Result<(), LocalizedText> {
    let manifest = read_manifest(stage, nonce)?;
    check_stage(
        stage,
        exe.parent().ok_or(LocalizedText::new(
            "Dossier invalide.",
            "Invalid directory.",
        ))?,
    )?;
    if exe != stage.parent().unwrap().join(&manifest.target_name)
        || &hash_file(exe)?
            != if failed {
                &manifest.old_hash
            } else {
                &manifest.new_hash
            }
        || !marker_matches(
            stage,
            if failed { "old-commit" } else { "new-commit" },
            nonce,
        )
    {
        return Err(LocalizedText::new(
            "Nettoyage de mise a jour refuse.",
            "Update cleanup refused.",
        ));
    }
    #[cfg(windows)]
    {
        let pid: u32 = read_small(&stage.join("helper-pid"), 32)?
            .parse()
            .map_err(|_| LocalizedText::new("Assistant invalide.", "Invalid helper."))?;
        if let Ok(helper) = ProcessWait::open(pid) {
            helper.wait(EXIT_TIMEOUT)?;
        }
    }
    // On Unix an executing file can be removed. After COMMIT the helper no
    // longer reads the stage; waiting for kill(pid, 0) would retain zombie PIDs.
    // Refuse links/reparse points throughout the dedicated stage before removal.
    check_tree(stage)?;
    fs::remove_dir_all(stage).map_err(|e| {
        error(
            "Nettoyage de la mise a jour impossible",
            "Could not clean up the update",
            e,
        )
    })
}

fn install_lock(target: &Path) -> Result<File, LocalizedText> {
    let path = target.join(".spotiamp-update.lock");
    match fs::symlink_metadata(&path) {
        Ok(_) => check_regular(&path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.to_string().into()),
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| {
            error(
                "Verrou de mise a jour inaccessible",
                "The update lock is inaccessible",
                e,
            )
        })?;
    file.try_lock().map_err(|_| {
        LocalizedText::new(
            "Une autre mise a jour est deja en cours dans ce dossier.",
            "Another update is already running in this directory.",
        )
    })?;
    // Never unlink the lock: another process may already be waiting on its inode.
    Ok(file)
}

fn check_stage(stage: &Path, target: &Path) -> Result<(), LocalizedText> {
    if !stage.is_absolute()
        || stage.parent() != Some(target)
        || !stage
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with(STAGE_PREFIX) && s.len() > STAGE_PREFIX.len())
    {
        return Err(LocalizedText::new(
            "Dossier de preparation invalide.",
            "Invalid staging directory.",
        ));
    }
    check_directory(target)?;
    check_directory(stage)?;
    if stage
        .canonicalize()
        .map_err(|e| error("Dossier inaccessible", "The directory is inaccessible", e))?
        != stage
    {
        return Err(LocalizedText::new(
            "Lien de preparation refuse.",
            "Staging link refused.",
        ));
    }
    Ok(())
}

fn validate_relative(path: &Path) -> Result<(), LocalizedText> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path.components().any(|c| {
            c.as_os_str().to_str().is_none_or(|s| {
                s.contains([':', '\\']) || s.ends_with(['.', ' ']) || s.starts_with(STAGE_PREFIX)
            })
        })
    {
        return Err(LocalizedText::new(
            "Chemin de mise a jour invalide.",
            "Invalid update path.",
        ));
    }
    Ok(())
}

fn metadata(path: &Path) -> Result<fs::Metadata, LocalizedText> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| error("Fichier inaccessible", "The file is inaccessible", e))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(LocalizedText::new(
                "Point de reanalyse refuse.",
                "Reparse point refused.",
            ));
        }
    }
    if metadata.file_type().is_symlink() {
        return Err(LocalizedText::new(
            "Lien symbolique refuse.",
            "Symbolic link refused.",
        ));
    }
    Ok(metadata)
}
fn check_regular(path: &Path) -> Result<(), LocalizedText> {
    if metadata(path)?.is_file() {
        Ok(())
    } else {
        Err(LocalizedText::new(
            "Fichier ordinaire attendu.",
            "Expected a regular file.",
        ))
    }
}
fn check_directory(path: &Path) -> Result<(), LocalizedText> {
    if metadata(path)?.is_dir() {
        Ok(())
    } else {
        Err(LocalizedText::new(
            "Dossier ordinaire attendu.",
            "Expected a regular directory.",
        ))
    }
}
fn check_path(root: &Path, relative: &Path, allow_missing: bool) -> Result<(), LocalizedText> {
    check_directory(root)?;
    let mut path = root.to_path_buf();
    for (index, component) in relative.components().enumerate() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && allow_missing => return Ok(()),
            Err(e) => {
                return Err(error(
                    "Destination inaccessible",
                    "The destination is inaccessible",
                    e,
                ));
            }
            Ok(_) => {
                if index + 1 == relative.components().count() {
                    check_regular(&path)?;
                } else {
                    check_directory(&path)?;
                }
            }
        }
    }
    Ok(())
}
fn create_parents(
    root: &Path,
    relative: &Path,
    created: &mut Vec<PathBuf>,
) -> Result<(), LocalizedText> {
    let mut path = root.to_path_buf();
    for part in relative.parent().unwrap().components() {
        path.push(part);
        match fs::create_dir(&path) {
            Ok(()) => created.push(path.clone()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => check_directory(&path)?,
            Err(e) => {
                return Err(error(
                    "Creation du dossier impossible",
                    "Could not create the directory",
                    e,
                ));
            }
        }
    }
    Ok(())
}
fn check_tree(path: &Path) -> Result<(), LocalizedText> {
    let meta = metadata(path)?;
    if meta.is_dir() {
        for child in fs::read_dir(path)
            .map_err(|e| error("Dossier inaccessible", "The directory is inaccessible", e))?
        {
            check_tree(
                &child
                    .map_err(|e| error("Dossier inaccessible", "The directory is inaccessible", e))?
                    .path(),
            )?;
        }
    } else if !meta.is_file() {
        return Err(LocalizedText::new(
            "Type de fichier inattendu.",
            "Unexpected file type.",
        ));
    }
    Ok(())
}
fn hash_file(path: &Path) -> Result<String, LocalizedText> {
    check_regular(path)?;
    let mut file =
        File::open(path).map_err(|e| error("Fichier illisible", "The file is unreadable", e))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| error("Fichier illisible", "The file is unreadable", e))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn read_small(path: &Path, limit: u64) -> Result<String, LocalizedText> {
    check_regular(path)?;
    let mut text = String::new();
    File::open(path)
        .map_err(|e| error("Fichier illisible", "The file is unreadable", e))?
        .take(limit + 1)
        .read_to_string(&mut text)
        .map_err(|e| error("Fichier illisible", "The file is unreadable", e))?;
    if text.len() as u64 > limit {
        Err(LocalizedText::new(
            "Fichier de confirmation trop volumineux.",
            "The confirmation file is too large.",
        ))
    } else {
        Ok(text)
    }
}
fn write_new(stage: &Path, name: &str, data: &[u8]) -> Result<(), LocalizedText> {
    let temporary = stage.join(format!("{name}.tmp"));
    let destination = stage.join(name);
    if destination.exists() {
        return Err(LocalizedText::new(
            "Confirmation deja presente.",
            "Confirmation already exists.",
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| {
            error(
                "Ecriture de confirmation impossible",
                "Could not write confirmation",
                e,
            )
        })?;
    file.write_all(data)
        .and_then(|()| file.sync_all())
        .map_err(|e| {
            error(
                "Enregistrement de confirmation impossible",
                "Could not save confirmation",
                e,
            )
        })?;
    drop(file);
    rename_file(&temporary, &destination).map_err(|e| {
        error(
            "Validation de confirmation impossible",
            "Could not validate confirmation",
            e,
        )
    })
}
fn write_notice(stage: &Path, name: &str, message: &LocalizedText) -> Result<(), LocalizedText> {
    // At most 4096 UTF-8 bytes; truncation never splits a character.
    // Keep the helper protocol plain text and its diagnostics in English.
    // The UI adds localized context when loading a notice after restart.
    let bounded = message
        .text(Language::English)
        .chars()
        .take(1024)
        .collect::<String>();
    write_new(stage, name, bounded.as_bytes())
}
fn write_marker(stage: &Path, name: &str, nonce: &str) -> Result<(), LocalizedText> {
    write_new(stage, name, nonce.as_bytes())
}
fn marker_matches(stage: &Path, name: &str, nonce: &str) -> bool {
    read_small(&stage.join(name), 128).is_ok_and(|text| text == nonce)
}
fn wait_marker(
    stage: &Path,
    name: &str,
    nonce: &str,
    timeout: Duration,
) -> Result<(), LocalizedText> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if marker_matches(stage, name, nonce) {
            return Ok(());
        }
        thread::sleep(POLL);
    }
    Err(LocalizedText::new(
        "Delai de confirmation de la mise a jour depasse.",
        "Update confirmation timed out.",
    ))
}
fn wait_child_marker(
    stage: &Path,
    name: &str,
    nonce: &str,
    child: &mut Child,
    timeout: Duration,
) -> Result<(), LocalizedText> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if child
            .try_wait()
            .map_err(|e| {
                error(
                    "Etat du processus inaccessible",
                    "Could not inspect the process",
                    e,
                )
            })?
            .is_some()
        {
            return Err(LocalizedText::new(
                "Le processus de mise a jour s'est arrete.",
                "The update process stopped.",
            ));
        }
        if marker_matches(stage, name, nonce) {
            return Ok(());
        }
        thread::sleep(POLL);
    }
    Err(LocalizedText::new(
        "Le lecteur n'a pas confirme son demarrage.",
        "The player did not confirm startup.",
    ))
}
fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if !matches!(child.try_wait(), Ok(None)) {
            break;
        }
        thread::sleep(POLL);
    }
}
fn rename_file(source: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let started = Instant::now();
        loop {
            match fs::rename(source, target) {
                Err(error)
                    if matches!(error.raw_os_error(), Some(5 | 32 | 33))
                        && started.elapsed() < Duration::from_secs(2) =>
                {
                    thread::sleep(POLL)
                }
                result => return result,
            }
        }
    }
    #[cfg(not(windows))]
    fs::rename(source, target)
}

fn quiet_command(exe: &Path) -> Command {
    let mut command = Command::new(exe);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn helper_failure(diagnostic: String) -> LocalizedText {
    error(
        "La mise a jour a echoue. Diagnostic",
        "The update failed. Diagnostic",
        diagnostic,
    )
}

#[cfg(unix)]
struct ProcessWait {
    pid: u32,
    #[cfg(target_os = "linux")]
    start_time: Option<u64>,
    #[cfg(target_os = "macos")]
    start_time: Option<(u64, u64)>,
}
#[cfg(target_os = "linux")]
fn linux_process_state(pid: u32) -> Option<(u8, u64)> {
    // The command name is parenthesized and can contain spaces or ')'.
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields = stat
        .get(stat.rfind(')')? + 1..)?
        .split_whitespace()
        .collect::<Vec<_>>();
    Some((
        *fields.first()?.as_bytes().first()?,
        fields.get(19)?.parse().ok()?,
    ))
}
#[cfg(target_os = "macos")]
type MacProcessStatus = (u32, (u64, u64));
#[cfg(target_os = "macos")]
fn mac_process_state(pid: u32) -> std::io::Result<Option<MacProcessStatus>> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
    // arg=1 explicitly includes exited processes awaiting collection. libc
    // supplies the platform ABI; only read the struct after a full-sized result.
    let count = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            1,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if count == size {
        let info = unsafe { info.assume_init() };
        Ok(Some((
            info.pbi_status,
            (info.pbi_start_tvsec, info.pbi_start_tvusec),
        )))
    } else {
        let error = std::io::Error::last_os_error();
        if count == 0 && error.raw_os_error() == Some(libc::ESRCH) {
            Ok(None)
        } else if count == 0 {
            Err(error)
        } else {
            Err(std::io::Error::other("Incomplete process information"))
        }
    }
}
#[cfg(unix)]
impl ProcessWait {
    fn open(pid: u32) -> Result<Self, LocalizedText> {
        if pid == 0 || pid > i32::MAX as u32 {
            Err(LocalizedText::new(
                "Processus invalide.",
                "Invalid process.",
            ))
        } else {
            Ok(Self {
                pid,
                #[cfg(target_os = "linux")]
                start_time: linux_process_state(pid).map(|(_, started)| started),
                #[cfg(target_os = "macos")]
                start_time: mac_process_state(pid)
                    .map_err(|e| {
                        error(
                            "Etat du processus inaccessible",
                            "Could not inspect the process",
                            e,
                        )
                    })?
                    .map(|(_, started)| started),
            })
        }
    }
    fn wait(&self, timeout: Duration) -> Result<(), LocalizedText> {
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
        }
        let start = Instant::now();
        while start.elapsed() < timeout {
            #[cfg(target_os = "linux")]
            if let Some((state, started)) = linux_process_state(self.pid)
                && (matches!(state, b'Z' | b'X')
                    || self.start_time.is_some_and(|original| original != started))
            {
                return Ok(());
            }
            #[cfg(target_os = "macos")]
            match mac_process_state(self.pid).map_err(|e| {
                error(
                    "Etat du processus inaccessible",
                    "Could not inspect the process",
                    e,
                )
            })? {
                None => return Ok(()),
                Some((state, started))
                    if state == libc::SZOMB
                        || self.start_time.is_some_and(|original| original != started) =>
                {
                    return Ok(());
                }
                Some(_) => (),
            }
            // Signal zero checks existence without sending a signal.
            if unsafe { kill(self.pid as i32, 0) } != 0
                && std::io::Error::last_os_error().raw_os_error() == Some(3)
            {
                return Ok(());
            }
            thread::sleep(POLL);
        }
        Err(LocalizedText::new(
            "Le lecteur ne s'est pas ferme a temps.",
            "The player did not close in time.",
        ))
    }
}
#[cfg(windows)]
struct ProcessWait(*mut std::ffi::c_void);
#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
    fn WaitForSingleObject(handle: *mut std::ffi::c_void, timeout: u32) -> u32;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
}
#[cfg(windows)]
impl ProcessWait {
    fn open(pid: u32) -> Result<Self, LocalizedText> {
        let handle = unsafe { OpenProcess(0x00100000, 0, pid) };
        if handle.is_null() {
            Err(error(
                "Ouverture du processus impossible",
                "Could not open the process",
                std::io::Error::last_os_error(),
            ))
        } else {
            Ok(Self(handle))
        }
    }
    fn wait(&self, timeout: Duration) -> Result<(), LocalizedText> {
        if unsafe { WaitForSingleObject(self.0, timeout.as_millis().min(u32::MAX as u128) as u32) }
            == 0
        {
            Ok(())
        } else {
            Err(LocalizedText::new(
                "Le lecteur ne s'est pas ferme a temps.",
                "The player did not close in time.",
            ))
        }
    }
}
#[cfg(windows)]
impl Drop for ProcessWait {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests;
