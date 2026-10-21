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
    Shutdown,
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
