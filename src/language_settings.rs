//! One bounded, atomically replaced language preference. No runtime file watch,
//! locale package, background work or access to authentication data is needed.
use crate::locale::LanguagePreference;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_BYTES: u64 = 16;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveError {
    MissingDirectory,
    WriteFailed,
}

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
        .map(|path| path.join("Spotiamp").join("language"))
}

pub fn load() -> LanguagePreference {
    settings_path()
        .map(|path| load_from(&path))
        .unwrap_or_default()
}

fn load_from(path: &Path) -> LanguagePreference {
    let Ok(file) = File::open(path) else {
        return LanguagePreference::Auto;
    };
    let mut value = String::new();
    // One extra byte rejects oversized suffixes even after a valid prefix.
    if file.take(MAX_BYTES + 1).read_to_string(&mut value).is_err()
        || value.len() as u64 > MAX_BYTES
    {
        return LanguagePreference::Auto;
    }
    LanguagePreference::parse(&value).unwrap_or_default()
}

pub fn save(preference: LanguagePreference) -> Result<(), SaveError> {
    let path = settings_path().ok_or(SaveError::MissingDirectory)?;
    save_to(&path, preference).map_err(|_| SaveError::WriteFailed)
}

fn save_to(path: &Path, preference: LanguagePreference) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing settings directory"))?;
    fs::create_dir_all(parent)?;
    let (temporary, mut file) = temporary_file(parent)?;
    let result = (|| {
        writeln!(file, "{}", preference.as_str())?;
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
        let path = parent.join(format!(".language-{}-{id}.tmp", std::process::id()));
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
    use crate::locale::Language;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            loop {
                let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "spotiamp-language-test-{}-{id}",
                    std::process::id()
                ));
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
    fn manual_choices_and_auto_survive_restart_without_touching_other_preferences() {
        let dir = TestDirectory::new();
        let folder = dir.0.join("Spotiamp");
        let path = folder.join("language");
        fs::create_dir(&folder).unwrap();
        let sentinels = [
            ("audio-quality", b"160\n".as_slice()),
            ("skin", b"modern\n"),
            ("window-layout", b"synthetic layout sentinel"),
            ("test-session-sentinel", b"synthetic data only"),
        ];
        for (name, bytes) in sentinels {
            fs::write(folder.join(name), bytes).unwrap();
        }
        assert_eq!(load_from(&path), LanguagePreference::Auto);
        for preference in [
            LanguagePreference::Specific(Language::English),
            LanguagePreference::Specific(Language::French),
            LanguagePreference::Auto,
            LanguagePreference::Specific(Language::English),
        ] {
            save_to(&path, preference).unwrap();
            assert_eq!(load_from(&path), preference);
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                format!("{}\n", preference.as_str())
            );
            for (name, bytes) in sentinels {
                assert_eq!(fs::read(folder.join(name)).unwrap(), bytes);
            }
            assert_eq!(fs::read_dir(&folder).unwrap().count(), sentinels.len() + 1);
        }
    }

    #[test]
    fn missing_corrupt_non_utf8_and_oversized_files_fall_back_to_auto() {
        let dir = TestDirectory::new();
        let path = dir.0.join("language");
        assert_eq!(load_from(&path), LanguagePreference::Auto);
        for invalid in [
            b"".as_slice(),
            b"unknown",
            b"f",
            b"a",
            b"fr\0",
            &[0xff],
            b"fr               \n",
            b"en\nen\nen\nen\nen\nen\n",
        ] {
            fs::write(&path, invalid).unwrap();
            assert_eq!(
                load_from(&path),
                LanguagePreference::Auto,
                "Bytes: {invalid:?}"
            );
        }
        fs::write(&path, b"fr\r\n").unwrap();
        assert_eq!(
            load_from(&path),
            LanguagePreference::Specific(Language::French)
        );
    }

    #[test]
    fn nested_directory_creation_and_failed_replacement_cleanup() {
        let dir = TestDirectory::new();
        let path = dir.0.join("nested/Spotiamp/language");
        save_to(&path, LanguagePreference::Specific(Language::French)).unwrap();
        assert_eq!(
            load_from(&path),
            LanguagePreference::Specific(Language::French)
        );
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let sentinel = path.join("keep");
        fs::write(&sentinel, b"existing directory").unwrap();
        assert!(save_to(&path, LanguagePreference::Auto).is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"existing directory");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        assert_eq!(load_from(&path), LanguagePreference::Auto);
    }
}
