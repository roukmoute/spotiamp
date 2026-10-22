use crate::{locale::Language, localized_text::LocalizedText};

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
    CancelConnect,
    Logout,
    Add(String),
    Play(usize),
    PlayIndex(usize),
    Toggle,
    ToggleShuffle,
    ToggleRepeat,
    Stop,
    Next,
    Previous,
    Seek(u32),
    Volume(u16),
    SetQuality(AudioQuality),
    SetLanguage(Language),
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
    pub status: LocalizedText,
    pub connected: bool,
    pub queue_len: usize,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct TrackRow {
    pub title_kind: TrackTitleKind,
    pub uri: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: u32,
}

/// Identifies only application placeholders; real Spotify names are never translated.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub enum TrackTitleKind {
    #[default]
    Metadata,
    Loading,
    Unavailable,
}

impl TrackRow {
    pub fn display_title(&self, language: Language) -> &str {
        match self.title_kind {
            TrackTitleKind::Metadata => &self.title,
            TrackTitleKind::Loading => language.text("Chargement du titre...", "Loading track..."),
            TrackTitleKind::Unavailable => language.text(
                "Titre Spotify (metadonnees indisponibles)",
                "Spotify track (metadata unavailable)",
            ),
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub enum PlaybackOrigin {
    #[default]
    Explicit,
    Automatic,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Snapshot {
    pub connected: bool,
    pub busy: bool,
    pub playing: bool,
    pub status: LocalizedText,
    pub queue: Vec<TrackRow>,
    pub current: Option<usize>,
    /// Changes once per accepted load or explicit restart, including the same
    /// queue index, so a view can reveal it without following progress ticks.
    pub playback_revision: u64,
    /// Describes the request that produced playback_revision, not progress.
    pub playback_origin: PlaybackOrigin,
    pub position_ms: u32,
    pub duration_ms: u32,
    pub volume: u16,
    pub quality: AudioQuality,
    pub shuffle: bool,
    pub repeat: bool,
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
            playback_revision: 0,
            playback_origin: PlaybackOrigin::default(),
            position_ms: 0,
            duration_ms: 0,
            volume: 32768,
            quality: AudioQuality::default(),
            shuffle: false,
            repeat: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholders_translate_but_identical_real_titles_do_not() {
        let mut row = TrackRow {
            title: "Chargement du titre...".into(),
            artist: "French-named artist".into(),
            ..TrackRow::default()
        };
        assert_eq!(row.display_title(Language::English), row.title);
        row.title_kind = TrackTitleKind::Loading;
        assert_eq!(row.display_title(Language::English), "Loading track...");
        row.title_kind = TrackTitleKind::Unavailable;
        assert_eq!(
            row.display_title(Language::English),
            "Spotify track (metadata unavailable)"
        );
        assert_eq!(row.artist, "French-named artist");
    }
}
