#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub enum AudioQuality {
    Kbps96,
    Kbps160,
    #[default]
    Kbps320,
}

impl AudioQuality {
    pub fn kbps(self) -> u16 {
        match self {
            Self::Kbps96 => 96,
            Self::Kbps160 => 160,
            Self::Kbps320 => 320,
        }
    }
}

#[cfg_attr(not(feature = "spotify"), allow(dead_code))]
#[derive(Clone, Debug)]
pub enum Command {
    Connect,
    Logout,
    Add(String),
    Play(usize),
    PlayIndex(usize),
    Toggle,
    Stop,
    Next,
    Previous,
    Seek(u32),
    Volume(u16),
    SetQuality(AudioQuality),
    Remove(usize),
    Clear,
    Library(LibraryCommand),
    Shutdown,
}

#[cfg_attr(not(feature = "spotify"), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlaylistId {
    Spotify(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistEntry {
    pub id: PlaylistId,
    pub name: String,
    pub track_count: Option<usize>,
    pub duration_ms: Option<u64>,
    pub folder: String,
    pub available: bool,
}

#[cfg_attr(not(feature = "spotify"), allow(dead_code))]
#[derive(Clone, Debug)]
pub enum LibraryCommand {
    Open,
    Close,
    Refresh,
    Select(PlaylistId),
    Queue {
        id: PlaylistId,
        replace: bool,
        play: bool,
        start: usize,
    },
}

/// Cheap to copy: library contents are shared and only the selected playlist
/// has track rows in memory. Audio snapshots never contain this library.
#[derive(Clone, Debug, Default)]
pub struct LibrarySnapshot {
    pub revision: u64,
    pub entries: std::sync::Arc<Vec<PlaylistEntry>>,
    pub selected_id: Option<PlaylistId>,
    pub tracks: std::sync::Arc<Vec<TrackRow>>,
    pub loading: bool,
    pub busy: bool,
    pub status: String,
    pub connected: bool,
    pub queue_len: usize,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct TrackRow {
    pub uri: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Snapshot {
    pub connected: bool,
    pub busy: bool,
    pub playing: bool,
    pub status: String,
    pub queue: Vec<TrackRow>,
    pub current: Option<usize>,
    pub position_ms: u32,
    pub duration_ms: u32,
    pub volume: u16,
    pub quality: AudioQuality,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            connected: false,
            busy: false,
            playing: false,
            status: "Connectez Spotify pour commencer.".into(),
            queue: Vec::new(),
            current: None,
            position_ms: 0,
            duration_ms: 0,
            volume: 32768,
            quality: AudioQuality::default(),
        }
    }
}
