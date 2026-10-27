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
    ReorderQueue {
        request_id: u64,
        expected_queue: Vec<String>,
        indices: Vec<usize>,
        insert_before: usize,
    },
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
    Search(String),
    /// Rows are captured from the visible selection; later browsing cannot retarget it.
    QueueTracks {
        tracks: Vec<TrackRow>,
        origin: Option<PlaylistId>,
        /// Optional destination identity captured at drop time.
        expected_queue: Option<Vec<String>>,
        replace: bool,
        play: bool,
        insert_at: Option<usize>,
    },
    AddToPlaylist {
        id: PlaylistId,
        tracks: Vec<TrackRow>,
    },
    RemoveFromPlaylist {
        id: PlaylistId,
        expected_tracks: std::sync::Arc<Vec<TrackRow>>,
        expected_revision: std::sync::Arc<[u8]>,
        indices: Vec<usize>,
    },
    ReorderPlaylist {
        id: PlaylistId,
        expected_tracks: std::sync::Arc<Vec<TrackRow>>,
        expected_revision: std::sync::Arc<[u8]>,
        indices: Vec<usize>,
        insert_before: usize,
    },
    CreatePlaylist {
        name: String,
        tracks: Vec<TrackRow>,
    },
    QueueSearch {
        revision: u64,
        index: usize,
        play: bool,
    },
    Queue {
        id: PlaylistId,
        replace: bool,
        play: bool,
        start: usize,
    },
}

/// Cheap to copy: catalogue, selected playlist, and bounded search results are
/// shared. Audio snapshots never contain this library.
#[derive(Clone, Debug, Default)]
pub struct LibrarySnapshot {
    pub revision: u64,
    pub entries: std::sync::Arc<Vec<PlaylistEntry>>,
    pub selected_id: Option<PlaylistId>,
    pub tracks: std::sync::Arc<Vec<TrackRow>>,
    /// Only a complete, owned playlist with a known Spotify revision is editable.
    pub selected_owned: bool,
    pub selected_playlist_revision: std::sync::Arc<[u8]>,
    pub loading: bool,
    pub busy: bool,
    pub status: LocalizedText,
    pub connected: bool,
    pub queue_len: usize,
    pub playlist_write_busy: bool,
    pub playlist_write_status: LocalizedText,
    pub playlist_remove_revision: u64,
    pub playlist_reorder_result: Option<std::sync::Arc<PlaylistReorderResult>>,
    pub search: SearchSnapshot,
}

/// A confirmed move identifies occurrences by their old positions, including
/// repeated tracks. The view waits for this revision's refreshed contents.
#[derive(Clone, Debug)]
pub struct PlaylistReorderResult {
    pub id: PlaylistId,
    pub previous_revision: std::sync::Arc<[u8]>,
    pub revision: std::sync::Arc<[u8]>,
    pub order: Vec<usize>,
}

/// Move a sorted selection to a gap in the original list. Returns the new
/// sequence of original indices; an invalid request or unchanged order is None.
pub fn reorder_indices(len: usize, indices: &[usize], insert_before: usize) -> Option<Vec<usize>> {
    if indices.is_empty()
        || insert_before > len
        || indices.last().is_some_and(|&index| index >= len)
        || indices.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return None;
    }
    let mut selected = indices.iter().copied().peekable();
    let mut order = Vec::with_capacity(len);
    for index in 0..=len {
        if index == insert_before {
            order.extend_from_slice(indices);
        }
        if selected.peek() == Some(&index) {
            selected.next();
        } else if index < len {
            order.push(index);
        }
    }
    order
        .iter()
        .enumerate()
        .any(|(i, &old)| i != old)
        .then_some(order)
}

pub const SEARCH_QUERY_MAX_CHARS: usize = 256;
pub const SEARCH_QUERY_MAX_BYTES: usize = 1024;

pub const PLAYLIST_NAME_MAX_CHARS: usize = 100;
pub const PLAYLIST_NAME_MAX_BYTES: usize = 400;

/// One rule for the name editor and the worker boundary. Whitespace surrounding
/// a name is harmless, but embedded control characters must not reach Spotify.
pub fn normalized_playlist_name(name: &str) -> Option<&str> {
    let name = name.trim();
    (!name.is_empty()
        && name.len() <= PLAYLIST_NAME_MAX_BYTES
        && name.chars().count() <= PLAYLIST_NAME_MAX_CHARS
        && !name.chars().any(char::is_control))
    .then_some(name)
}

/// Search rows are independent of the selected playlist and absent until an
/// explicit search. Revision identifies one request, including its metadata.
#[derive(Clone, Debug, Default)]
pub struct SearchSnapshot {
    pub revision: u64,
    pub query: std::sync::Arc<str>,
    pub tracks: std::sync::Arc<Vec<TrackRow>>,
    pub loading: bool,
    pub status: LocalizedText,
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
    /// Acknowledges an accepted local move without disturbing playback events.
    pub queue_reorder_request: u64,
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
            queue_reorder_request: 0,
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
    fn reordering_uses_original_gaps_and_preserves_selected_occurrences() {
        assert_eq!(reorder_indices(6, &[1, 3], 6), Some(vec![0, 2, 4, 5, 1, 3]));
        assert_eq!(reorder_indices(6, &[1, 3], 0), Some(vec![1, 3, 0, 2, 4, 5]));
        assert_eq!(reorder_indices(6, &[1, 3], 3), Some(vec![0, 2, 1, 3, 4, 5]));
        for (indices, gap) in [
            (vec![], 0),
            (vec![1, 1], 0),
            (vec![3, 1], 0),
            (vec![6], 0),
            (vec![1], 7),
            (vec![1, 2], 1),
            (vec![1, 2], 2),
            (vec![1, 2], 3),
            ((0..6).collect(), 4),
        ] {
            assert_eq!(reorder_indices(6, &indices, gap), None);
        }
    }

    #[test]
    fn every_small_selection_and_gap_produces_the_requested_stable_partition() {
        for len in 1..9 {
            for mask in 1..1usize << len {
                let selected = (0..len)
                    .filter(|i| mask & (1 << i) != 0)
                    .collect::<Vec<_>>();
                for gap in 0..=len {
                    let mut expected = (0..gap)
                        .filter(|i| mask & (1 << i) == 0)
                        .collect::<Vec<_>>();
                    expected.extend(&selected);
                    expected.extend((gap..len).filter(|i| mask & (1 << i) == 0));
                    let actual = reorder_indices(len, &selected, gap);
                    assert_eq!(
                        actual,
                        (expected != (0..len).collect::<Vec<_>>()).then_some(expected)
                    );
                }
            }
        }
    }
    #[test]
    fn playlist_names_preserve_unicode_and_obey_editor_and_worker_limits() {
        assert_eq!(
            normalized_playlist_name("  Été 日本 🎵  "),
            Some("Été 日本 🎵")
        );
        assert!(normalized_playlist_name(&"🎵".repeat(100)).is_some());
        for invalid in [
            String::new(),
            " \t\n ".into(),
            "a\nb".into(),
            "a\0b".into(),
            "é".repeat(101),
        ] {
            assert_eq!(normalized_playlist_name(&invalid), None);
        }
    }
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
