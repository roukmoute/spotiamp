//! Bounded local playback state, independent of Spotify credentials. Only a
//! complete replacement becomes visible; a failed checkpoint keeps the old one.
use crate::model::{PlaylistId, Snapshot, TrackRow, TrackTitleKind};
use librespot_core::SpotifyId;
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const SCHEMA: u64 = 1;
const MAX_TRACKS: usize = 500;
const MAX_BYTES: usize = 1024 * 1024;
const MAX_TEXT_BYTES: usize = 4096;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackSession {
    pub queue: Vec<TrackRow>,
    pub current: Option<usize>,
    pub position_ms: u32,
    pub playlist: Option<PlaylistId>,
    pub volume: u16,
    pub shuffle: bool,
    pub repeat: bool,
}

pub fn from_snapshot(snapshot: &Snapshot, playlist: Option<PlaylistId>) -> PlaybackSession {
    PlaybackSession {
        queue: snapshot.queue.clone(),
        current: snapshot.current,
        position_ms: bounded_position(&snapshot.queue, snapshot.current, snapshot.position_ms),
        playlist,
        volume: snapshot.volume,
        shuffle: snapshot.shuffle,
        repeat: snapshot.repeat,
    }
}

fn bounded_position(queue: &[TrackRow], current: Option<usize>, position: u32) -> u32 {
    current
        .and_then(|index| queue.get(index))
        .map_or(0, |track| position.min(track.duration_ms.saturating_sub(1)))
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
        .map(|path| path.join("Spotiamp").join("playback-session.json"))
}

pub fn load() -> Option<PlaybackSession> {
    settings_path().and_then(|path| load_from(&path))
}

fn load_from(path: &Path) -> Option<PlaybackSession> {
    let file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    // The extra byte distinguishes an oversized file from a valid JSON prefix.
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > MAX_BYTES {
        return None;
    }
    parse(&bytes)
}

fn parse(bytes: &[u8]) -> Option<PlaybackSession> {
    if bytes.len() > MAX_BYTES {
        return None;
    }
    let root: Value = serde_json::from_slice(bytes).ok()?;
    let root = root.as_object()?;
    if root.get("schema")?.as_u64()? != SCHEMA {
        return None;
    }
    let rows = root.get("queue")?.as_array()?;
    if rows.len() > MAX_TRACKS {
        return None;
    }
    let mut queue = Vec::with_capacity(rows.len());
    for row in rows {
        let row = row.as_object()?;
        let uri = row.get("uri")?.as_str()?;
        if !valid_uri(uri, "spotify:track:") {
            return None;
        }
        let title = bounded_text(row.get("title")?)?;
        let artist = bounded_text(row.get("artist")?)?;
        let duration_ms = u32::try_from(row.get("duration_ms")?.as_u64()?).ok()?;
        let title_kind = match row.get("title_kind") {
            None => TrackTitleKind::Metadata,
            Some(value) => match value.as_str()? {
                "metadata" => TrackTitleKind::Metadata,
                "loading" => TrackTitleKind::Loading,
                "unavailable" => TrackTitleKind::Unavailable,
                _ => return None,
            },
        };
        queue.push(TrackRow {
            uri: uri.to_owned(),
            title: title.to_owned(),
            artist: artist.to_owned(),
            duration_ms,
            title_kind,
        });
    }
    let current = match root.get("current")? {
        Value::Null => None,
        value => Some(usize::try_from(value.as_u64()?).ok()?),
    };
    if current.is_some_and(|index| index >= queue.len()) {
        return None;
    }
    let position_ms = u32::try_from(root.get("position_ms")?.as_u64()?).ok()?;
    let playlist = match root.get("playlist")? {
        Value::Null => None,
        value => {
            let uri = value.as_str()?;
            if !valid_uri(uri, "spotify:playlist:") {
                return None;
            }
            Some(PlaylistId::Spotify(uri.to_owned()))
        }
    };
    Some(PlaybackSession {
        current,
        position_ms: bounded_position(&queue, current, position_ms),
        queue,
        playlist,
        volume: u16::try_from(root.get("volume")?.as_u64()?).ok()?,
        shuffle: root.get("shuffle")?.as_bool()?,
        repeat: root.get("repeat")?.as_bool()?,
    })
}

fn bounded_text(value: &Value) -> Option<&str> {
    let text = value.as_str()?;
    (text.len() <= MAX_TEXT_BYTES).then_some(text)
}

fn valid_uri(uri: &str, prefix: &str) -> bool {
    uri.strip_prefix(prefix).is_some_and(|id| {
        id.len() == 22
            && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
            && SpotifyId::from_base62(id).is_ok()
    })
}

fn valid_session(session: &PlaybackSession) -> bool {
    session.queue.len() <= MAX_TRACKS
        && session
            .current
            .is_none_or(|index| index < session.queue.len())
        && session.queue.iter().all(|track| {
            valid_uri(&track.uri, "spotify:track:")
                && track.title.len() <= MAX_TEXT_BYTES
                && track.artist.len() <= MAX_TEXT_BYTES
        })
        && session.playlist.as_ref().is_none_or(|playlist| {
            let PlaylistId::Spotify(uri) = playlist;
            valid_uri(uri, "spotify:playlist:")
        })
}

pub fn save(session: &PlaybackSession) -> io::Result<()> {
    let path = settings_path().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Preferences directory unavailable")
    })?;
    save_to(&path, session)
}

fn save_to(path: &Path, session: &PlaybackSession) -> io::Result<()> {
    if !valid_session(session) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid playback session",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing settings directory"))?;
    fs::create_dir_all(parent)?;
    let (temporary, file) = temporary_file(parent)?;
    let result = (|| {
        let mut writer = BoundedWriter {
            inner: BufWriter::new(file),
            remaining: MAX_BYTES,
        };
        write_session(&mut writer, session)?;
        writer.flush()?;
        // Close before atomic replacement, including on Windows. No partial
        // checkpoint can replace the previous complete session.
        drop(writer);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

struct BoundedWriter<W> {
    inner: W,
    remaining: usize,
}

impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Playback session exceeds the size limit",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn write_session(writer: &mut impl Write, session: &PlaybackSession) -> io::Result<()> {
    // Stream directly into an 8 KiB file buffer instead of allocating a JSON
    // tree and another complete copy of every title at each checkpoint.
    write!(writer, "{{\"schema\":{SCHEMA},\"queue\":[")?;
    for (index, track) in session.queue.iter().enumerate() {
        if index != 0 {
            writer.write_all(b",")?;
        }
        writer.write_all(b"{\"uri\":")?;
        json_string(writer, &track.uri)?;
        writer.write_all(b",\"title\":")?;
        json_string(writer, &track.title)?;
        writer.write_all(b",\"artist\":")?;
        json_string(writer, &track.artist)?;
        write!(
            writer,
            ",\"duration_ms\":{},\"title_kind\":",
            track.duration_ms
        )?;
        json_string(
            writer,
            match track.title_kind {
                TrackTitleKind::Metadata => "metadata",
                TrackTitleKind::Loading => "loading",
                TrackTitleKind::Unavailable => "unavailable",
            },
        )?;
        writer.write_all(b"}")?;
    }
    writer.write_all(b"],\"current\":")?;
    if let Some(current) = session.current {
        write!(writer, "{current}")?;
    } else {
        writer.write_all(b"null")?;
    }
    write!(
        writer,
        ",\"position_ms\":{},\"playlist\":",
        bounded_position(&session.queue, session.current, session.position_ms)
    )?;
    if let Some(PlaylistId::Spotify(uri)) = &session.playlist {
        json_string(writer, uri)?;
    } else {
        writer.write_all(b"null")?;
    }
    writeln!(
        writer,
        ",\"volume\":{},\"shuffle\":{},\"repeat\":{}}}",
        session.volume, session.shuffle, session.repeat
    )
}

fn json_string(writer: &mut impl Write, value: &str) -> io::Result<()> {
    serde_json::to_writer(writer, value).map_err(io::Error::other)
}

fn temporary_file(parent: &Path) -> io::Result<(PathBuf, File)> {
    for _ in 0..64 {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".playback-session-{}-{id}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Cannot reserve a temporary playback session file",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            loop {
                let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "spotiamp-playback-test-{}-{id}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Cannot create isolated playback fixture: {error}"),
                }
            }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample() -> PlaybackSession {
        PlaybackSession {
            queue: [
                TrackTitleKind::Metadata,
                TrackTitleKind::Loading,
                TrackTitleKind::Unavailable,
            ]
            .into_iter()
            .map(|title_kind| TrackRow {
                // Duplicates deliberately retain their own metadata and index.
                uri: "spotify:track:0000000000000000000001".into(),
                title: format!("Titre \"étoile\" / 🎵 {title_kind:?}"),
                artist: "Artiste\\Groupe\nFeaturing".into(),
                duration_ms: 180_000,
                title_kind,
            })
            .collect(),
            current: Some(1),
            position_ms: 42_000,
            playlist: Some(PlaylistId::Spotify(
                "spotify:playlist:0000000000000000000002".into(),
            )),
            volume: 43_210,
            shuffle: true,
            repeat: true,
        }
    }

    fn encode(session: &PlaybackSession) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_session(&mut bytes, session).unwrap();
        bytes
    }

    #[test]
    fn ordered_duplicate_tracks_metadata_modes_and_position_survive_replacements() {
        let dir = TestDirectory::new();
        let folder = dir.0.join("Spotiamp");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("playback-session.json");
        let sentinels = [
            ("skin", b"modern\n".as_slice()),
            ("language", b"en\n"),
            ("test-session", b"synthetic only"),
        ];
        for (name, bytes) in sentinels {
            fs::write(folder.join(name), bytes).unwrap();
        }
        let mut session = sample();
        for current in [Some(1), Some(2), Some(0), None] {
            session.current = current;
            session.position_ms = if current.is_some() { 42_000 } else { 0 };
            save_to(&path, &session).unwrap();
            assert_eq!(load_from(&path), Some(session.clone()));
            for (name, bytes) in sentinels {
                assert_eq!(fs::read(folder.join(name)).unwrap(), bytes);
            }
            assert_eq!(fs::read_dir(&folder).unwrap().count(), sentinels.len() + 1);
        }
    }

    #[test]
    fn position_is_bounded_and_cleared_queue_replaces_previous_playback() {
        let dir = TestDirectory::new();
        let path = dir.0.join("nested/playback-session.json");
        let mut session = sample();
        session.position_ms = u32::MAX;
        save_to(&path, &session).unwrap();
        assert_eq!(load_from(&path).unwrap().position_ms, 179_999);
        session.queue[1].duration_ms = 0;
        save_to(&path, &session).unwrap();
        assert_eq!(load_from(&path).unwrap().position_ms, 0);
        session.queue.clear();
        session.current = None;
        session.playlist = None;
        save_to(&path, &session).unwrap();
        let restored = load_from(&path).unwrap();
        assert!(restored.queue.is_empty());
        assert_eq!(restored.current, None);
        assert_eq!(restored.playlist, None);
        assert_eq!(restored.position_ms, 0);
    }

    #[test]
    fn corrupt_unsupported_oversized_and_invalid_fields_fail_closed() {
        let dir = TestDirectory::new();
        let path = dir.0.join("playback-session.json");
        assert!(load_from(&path).is_none());
        let valid: Value = serde_json::from_slice(&encode(&sample())).unwrap();
        for invalid in [b"".as_slice(), b"{", &[0xff], b"null", b"[]"] {
            fs::write(&path, invalid).unwrap();
            assert!(load_from(&path).is_none());
        }
        let mut oversized = encode(&sample());
        oversized.resize(MAX_BYTES + 1, b' ');
        fs::write(&path, oversized).unwrap();
        assert!(load_from(&path).is_none());
        for (key, value) in [
            ("schema", json!(2)),
            ("schema", json!(1.5)),
            ("queue", json!([])),
            ("queue", json!("invalid")),
            ("current", json!(3)),
            ("current", json!(-1)),
            ("current", json!(0.5)),
            ("position_ms", json!(u64::MAX)),
            ("position_ms", json!(-1)),
            ("volume", json!(65536)),
            ("volume", json!("10")),
            ("shuffle", json!(1)),
            ("repeat", json!(null)),
            ("playlist", json!("spotify:track:0000000000000000000002")),
            ("playlist", json!("spotify:playlist:zzzzzzzzzzzzzzzzzzzzzz")),
        ] {
            let mut changed = valid.clone();
            changed[key] = value;
            assert!(
                parse(&serde_json::to_vec(&changed).unwrap()).is_none(),
                "Invalid field {key}"
            );
        }
        for (key, value) in [
            ("uri", json!("spotify:track:short")),
            ("uri", json!("spotify:album:0000000000000000000001")),
            ("uri", json!("spotify:track:zzzzzzzzzzzzzzzzzzzzzz")),
            ("uri", json!("spotify:track:0000000000000000000001:extra")),
            ("title", json!("x".repeat(MAX_TEXT_BYTES + 1))),
            ("artist", json!(false)),
            ("duration_ms", json!(-1)),
            ("title_kind", json!("other")),
        ] {
            let mut changed = valid.clone();
            changed["queue"][0][key] = value;
            assert!(
                parse(&serde_json::to_vec(&changed).unwrap()).is_none(),
                "Invalid row field {key}"
            );
        }
    }

    #[test]
    fn invalid_or_oversized_save_keeps_previous_file_and_removes_temporary_files() {
        let dir = TestDirectory::new();
        let path = dir.0.join("playback-session.json");
        let valid = sample();
        save_to(&path, &valid).unwrap();
        let previous = fs::read(&path).unwrap();
        let mut too_many = valid.clone();
        too_many
            .queue
            .resize(MAX_TRACKS + 1, valid.queue[0].clone());
        assert!(save_to(&path, &too_many).is_err());
        assert_eq!(fs::read(&path).unwrap(), previous);
        let mut too_large = valid.clone();
        too_large.queue.resize(MAX_TRACKS, valid.queue[0].clone());
        for row in &mut too_large.queue {
            row.title = "\u{0001}".repeat(MAX_TEXT_BYTES);
        }
        assert!(save_to(&path, &too_large).is_err());
        assert_eq!(fs::read(&path).unwrap(), previous);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), b"existing directory").unwrap();
        assert!(save_to(&path, &valid).is_err());
        assert_eq!(fs::read(path.join("keep")).unwrap(), b"existing directory");
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }

    #[test]
    fn exact_queue_limit_and_legacy_metadata_kind_are_supported() {
        let mut session = sample();
        session.queue.resize(MAX_TRACKS, session.queue[0].clone());
        session.current = Some(MAX_TRACKS - 1);
        assert_eq!(parse(&encode(&session)), Some(session.clone()));
        let mut value: Value = serde_json::from_slice(&encode(&session)).unwrap();
        value["queue"][0]
            .as_object_mut()
            .unwrap()
            .remove("title_kind");
        assert_eq!(parse(&serde_json::to_vec(&value).unwrap()), Some(session));
    }

    #[test]
    fn snapshot_conversion_keeps_only_playback_preferences_and_clamps_position() {
        let session = sample();
        let snapshot = Snapshot {
            queue: session.queue.clone(),
            current: session.current,
            position_ms: u32::MAX,
            volume: session.volume,
            shuffle: session.shuffle,
            repeat: session.repeat,
            connected: true,
            playing: true,
            busy: true,
            status: "temporary".into(),
            ..Default::default()
        };
        let mut expected = session.clone();
        expected.position_ms = 179_999;
        assert_eq!(from_snapshot(&snapshot, session.playlist), expected);
    }
}
