//! One small preference file; authentication remains in the OS credential store.
use crate::model::AudioQuality;
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

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
        .map(|path| path.join("Spotiamp").join("audio-quality"))
}

pub fn load() -> AudioQuality {
    settings_path()
        .map(|path| load_from(&path))
        .unwrap_or_default()
}

fn load_from(path: &Path) -> AudioQuality {
    // Never read an unbounded file when only three short values are accepted.
    let mut value = String::new();
    let Ok(file) = File::open(path) else {
        return AudioQuality::default();
    };
    if file.take(32).read_to_string(&mut value).is_err() {
        return AudioQuality::default();
    }
    match value.trim() {
        "96" => AudioQuality::Kbps96,
        "160" => AudioQuality::Kbps160,
        _ => AudioQuality::Kbps320,
    }
}

pub fn save(quality: AudioQuality) -> Result<(), &'static str> {
    let path = settings_path().ok_or("Dossier de preferences indisponible")?;
    save_to(&path, quality)
        .map_err(|_| "Qualite appliquee mais non memorisee : ecriture impossible")
}

fn save_to(path: &Path, quality: AudioQuality) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // An interrupted write safely falls back to 320 on the next startup.
    fs::write(path, format!("{}\n", quality.kbps()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_survives_restart_and_invalid_preferences_use_the_best_default() {
        let dir = std::env::temp_dir().join(format!(
            "spotiamp-quality-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("preferences/audio-quality");
        assert_eq!(load_from(&path), AudioQuality::Kbps320);
        for quality in [
            AudioQuality::Kbps96,
            AudioQuality::Kbps160,
            AudioQuality::Kbps320,
        ] {
            save_to(&path, quality).unwrap();
            assert_eq!(load_from(&path), quality);
        }
        for invalid in [b"".as_slice(), b"unknown", b"1600", &[0xff]] {
            fs::write(&path, invalid).unwrap();
            assert_eq!(load_from(&path), AudioQuality::Kbps320);
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
