//! A bounded appearance preference, independent of audio and authentication.
use crate::appearance::Skin;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_BYTES: u64 = 16;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.filter(|path| path.is_absolute())
        .map(|path| path.join("Spotiamp").join("skin"))
}

pub fn load() -> Skin {
    settings_path()
        .map(|path| load_from(&path))
        .unwrap_or_default()
}

fn load_from(path: &Path) -> Skin {
    let Ok(file) = File::open(path) else {
        return Skin::default();
    };
    let mut value = String::new();
    // Read one extra byte so a valid prefix followed by an oversized suffix
    // cannot be accepted after trimming. Never read the whole untrusted file.
    if file.take(MAX_BYTES + 1).read_to_string(&mut value).is_err()
        || value.len() as u64 > MAX_BYTES
    {
        return Skin::default();
    }
    Skin::parse(&value).unwrap_or_default()
}

pub fn save(skin: Skin) -> Result<(), &'static str> {
    let path = settings_path().ok_or("Skin applique mais non memorise : dossier indisponible")?;
    save_to(&path, skin).map_err(|_| "Skin applique mais non memorise : ecriture impossible")
}

fn save_to(path: &Path, skin: Skin) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing settings directory"))?;
    fs::create_dir_all(parent)?;
    let (temporary, mut file) = temporary_file(parent)?;
    let result = (|| {
        file.write_all(skin.as_str().as_bytes())?;
        file.write_all(b"\n")?;
        // Close before rename, including on Windows. The previous preference
        // remains intact if writing or replacement fails; only a complete
        // value becomes visible at the final path.
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn temporary_file(parent: &Path) -> io::Result<(PathBuf, File)> {
    for _ in 0..64 {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".skin-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Cannot reserve a temporary preference file",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            loop {
                let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("spotiamp-skin-test-{}-{id}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Cannot create isolated preference fixture: {error}"),
                }
            }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn both_skins_round_trip_without_touching_audio_or_other_files() {
        let dir = TestDirectory::new();
        let folder = dir.0.join("Spotiamp");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("skin");
        let audio = folder.join("audio-quality");
        let unrelated = folder.join("test-session-sentinel");
        fs::write(&audio, b"160\n").unwrap();
        fs::write(&unrelated, b"synthetic sentinel only").unwrap();
        assert_eq!(load_from(&path), Skin::Classic);
        for skin in [Skin::Modern, Skin::Classic, Skin::Modern] {
            save_to(&path, skin).unwrap();
            assert_eq!(load_from(&path), skin);
            assert_eq!(
                fs::read(&path).unwrap(),
                format!("{}\n", skin.as_str()).as_bytes()
            );
            assert_eq!(fs::read(&audio).unwrap(), b"160\n");
            assert_eq!(fs::read(&unrelated).unwrap(), b"synthetic sentinel only");
            assert_eq!(fs::read_dir(&folder).unwrap().count(), 3);
        }
    }

    #[test]
    fn missing_corrupt_truncated_non_utf8_and_oversized_values_use_classic() {
        let dir = TestDirectory::new();
        let path = dir.0.join("skin");
        assert_eq!(load_from(&path), Skin::Classic);
        for invalid in [
            b"".as_slice(),
            b"unknown",
            b"moder",
            b"classi",
            b"modern\0",
            &[0xff],
            b"modern          \n",
            b"modern\nmodern\nmodern\n",
        ] {
            fs::write(&path, invalid).unwrap();
            assert_eq!(
                load_from(&path),
                Skin::Classic,
                "Invalid bytes: {invalid:?}"
            );
        }
        fs::write(&path, b"modern\r\n").unwrap();
        assert_eq!(load_from(&path), Skin::Modern);
    }

    #[test]
    fn creates_the_preference_directory_and_cleans_up_a_failed_replacement() {
        let dir = TestDirectory::new();
        let path = dir.0.join("nested/Spotiamp/skin");
        save_to(&path, Skin::Modern).unwrap();
        assert_eq!(load_from(&path), Skin::Modern);
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let sentinel = path.join("keep");
        fs::write(&sentinel, b"existing directory").unwrap();
        assert!(save_to(&path, Skin::Classic).is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"existing directory");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        assert_eq!(load_from(&path), Skin::Classic);
    }
}
