//! A small, versioned window layout, independent of skin, audio and credentials.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedLayout {
    pub player: (isize, isize),
    pub library: (isize, isize),
    pub attached: bool,
    pub library_open: bool,
    pub scale: u8,
}

const MAX_BYTES: u64 = 256;
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
        .map(|path| path.join("Spotiamp").join("window-layout"))
}

pub fn load() -> Option<SavedLayout> {
    settings_path().and_then(|path| load_from(&path))
}

fn load_from(path: &Path) -> Option<SavedLayout> {
    let file = File::open(path).ok()?;
    let mut value = String::new();
    // One extra byte distinguishes an oversized file from a valid short prefix.
    file.take(MAX_BYTES + 1).read_to_string(&mut value).ok()?;
    if value.len() as u64 > MAX_BYTES {
        return None;
    }
    parse(&value)
}

fn parse(value: &str) -> Option<SavedLayout> {
    // v1 player_x player_y library_x library_y attached library_open scale
    let mut fields = value.split_ascii_whitespace();
    if fields.next()? != "v1" {
        return None;
    }
    let player = (coordinate(fields.next()?)?, coordinate(fields.next()?)?);
    let library = (coordinate(fields.next()?)?, coordinate(fields.next()?)?);
    let attached = bit(fields.next()?)?;
    let library_open = bit(fields.next()?)?;
    let scale = match fields.next()? {
        "1" => 1,
        "2" => 2,
        _ => return None,
    };
    if fields.next().is_some() {
        return None;
    }
    Some(SavedLayout {
        player,
        library,
        attached,
        library_open,
        scale,
    })
}

fn coordinate(value: &str) -> Option<isize> {
    value.parse::<i32>().ok().map(|value| value as isize)
}

fn bit(value: &str) -> Option<bool> {
    match value {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

pub fn save(layout: SavedLayout) -> Result<(), &'static str> {
    let path = settings_path().ok_or("Dossier de preferences indisponible")?;
    save_to(&path, layout).map_err(|_| "Disposition non memorisee : ecriture impossible")
}

fn save_to(path: &Path, layout: SavedLayout) -> io::Result<()> {
    if !matches!(layout.scale, 1 | 2)
        || [
            layout.player.0,
            layout.player.1,
            layout.library.0,
            layout.library.1,
        ]
        .iter()
        .any(|&value| i32::try_from(value).is_err())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid window layout",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing settings directory"))?;
    fs::create_dir_all(parent)?;
    let (temporary, mut file) = temporary_file(parent)?;
    let result = (|| {
        writeln!(
            file,
            "v1 {} {} {} {} {} {} {}",
            layout.player.0,
            layout.player.1,
            layout.library.0,
            layout.library.1,
            u8::from(layout.attached),
            u8::from(layout.library_open),
            layout.scale
        )?;
        // Close before replacement, including on Windows. Only a complete
        // record becomes visible; failed writes keep the previous preference.
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
        let path = parent.join(format!(".window-layout-{}-{id}.tmp", std::process::id()));
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
                let path = std::env::temp_dir().join(format!(
                    "spotiamp-window-layout-test-{}-{id}",
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

    fn sample() -> SavedLayout {
        SavedLayout {
            player: (100, 80),
            library: (660, 80),
            attached: true,
            library_open: true,
            scale: 1,
        }
    }

    #[test]
    fn all_edges_free_closed_and_negative_positions_round_trip_without_touching_other_preferences()
    {
        let dir = TestDirectory::new();
        let folder = dir.0.join("Spotiamp");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("window-layout");
        let sentinels = [
            ("audio-quality", b"160\n".as_slice()),
            ("skin", b"modern\n"),
            ("synthetic-session-sentinel", b"test data only"),
        ];
        for (name, value) in sentinels {
            fs::write(folder.join(name), value).unwrap();
        }
        for scale in [1, 2] {
            for (library, attached, library_open) in [
                ((660, 80), true, true),
                ((-660, 80), true, true),
                ((100, -390), true, true),
                ((100, 550), true, true),
                ((-1250, -700), false, true),
                ((-1250, -700), false, false),
                ((660, 80), true, false),
            ] {
                let layout = SavedLayout {
                    library,
                    attached,
                    library_open,
                    scale,
                    ..sample()
                };
                save_to(&path, layout).unwrap();
                assert_eq!(load_from(&path), Some(layout));
                let encoded = fs::read_to_string(&path).unwrap();
                assert_eq!(encoded.split_ascii_whitespace().count(), 8);
                assert!(encoded.len() as u64 <= MAX_BYTES);
                for (name, value) in sentinels {
                    assert_eq!(fs::read(folder.join(name)).unwrap(), value);
                }
                assert_eq!(fs::read_dir(&folder).unwrap().count(), 4);
            }
        }
    }

    #[test]
    fn missing_truncated_corrupt_unknown_and_oversized_files_are_ignored() {
        let dir = TestDirectory::new();
        let path = dir.0.join("window-layout");
        assert_eq!(load_from(&path), None);
        for invalid in [
            b"".as_slice(),
            b"v1",
            b"v1 100 80 660 80 1 1",
            b"v1 100 80 660 80 1 1 1 extra",
            b"v2 100 80 660 80 1 1 1",
            b"v1 100 80 660 80 true 1 1",
            b"v1 100 80 660 80 1 false 1",
            b"v1 100 80 660 80 01 1 1",
            b"v1 100 80 660 80 1 2 1",
            b"v1 100 80 660 80 1 1 0",
            b"v1 100 80 660 80 1 1 3",
            b"v1 100 80 660 80 1 1 01",
            b"v1 2147483648 80 660 80 1 1 1",
            b"v1 100 -2147483649 660 80 1 1 1",
            b"v1 100 80 1.5 80 1 1 1",
            b"v1 100 80 660 NaN 1 1 1",
            b"v1 100 80 660 80 1 1 1\0",
            b"v1 100 80 660 80 1 1 1\xff",
        ] {
            fs::write(&path, invalid).unwrap();
            assert_eq!(load_from(&path), None, "Invalid bytes: {invalid:?}");
        }
        let mut padded = b"v1 100 80 660 80 1 1 1\n".to_vec();
        padded.resize(MAX_BYTES as usize, b' ');
        fs::write(&path, &padded).unwrap();
        assert_eq!(load_from(&path), Some(sample()));
        padded.push(b' ');
        fs::write(&path, padded).unwrap();
        assert_eq!(load_from(&path), None);
        fs::write(&path, b"v1 100 80 660 80 1 1 1\r\n").unwrap();
        assert_eq!(load_from(&path), Some(sample()));
    }

    #[test]
    fn native_coordinate_limits_are_preserved() {
        let dir = TestDirectory::new();
        let path = dir.0.join("window-layout");
        let layout = SavedLayout {
            player: (i32::MIN as isize, i32::MAX as isize),
            library: (i32::MAX as isize, i32::MIN as isize),
            ..sample()
        };
        save_to(&path, layout).unwrap();
        assert_eq!(load_from(&path), Some(layout));
    }

    #[test]
    fn invalid_save_keeps_the_previous_complete_layout() {
        let dir = TestDirectory::new();
        let path = dir.0.join("window-layout");
        save_to(&path, sample()).unwrap();
        let before = fs::read(&path).unwrap();
        for scale in [0, 3, u8::MAX] {
            assert!(save_to(&path, SavedLayout { scale, ..sample() }).is_err());
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        #[cfg(target_pointer_width = "64")]
        for player in [(isize::MIN, 0), (0, isize::MAX)] {
            assert!(save_to(&path, SavedLayout { player, ..sample() }).is_err());
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }

    #[test]
    fn creates_directory_and_cleans_failed_replacement_without_removing_existing_files() {
        let dir = TestDirectory::new();
        let path = dir.0.join("nested/Spotiamp/window-layout");
        save_to(&path, sample()).unwrap();
        assert_eq!(load_from(&path), Some(sample()));
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let sentinel = path.join("keep");
        fs::write(&sentinel, b"existing directory").unwrap();
        assert!(save_to(&path, sample()).is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"existing directory");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        assert_eq!(load_from(&path), None);
    }
}
