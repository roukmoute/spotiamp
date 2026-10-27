//! A native bitmap library. The framebuffer exists only while its window is open.
use crate::{
    appearance::Skin,
    locale::Language,
    localized_text::LocalizedText,
    model::{
        LibraryCommand, LibrarySnapshot, PlaylistEntry, PlaylistId, SEARCH_QUERY_MAX_BYTES,
        SEARCH_QUERY_MAX_CHARS, TrackRow, normalized_playlist_name,
    },
};
mod modern;
mod picker;
mod selection;
mod send_menu;
mod text_input;
use font8x8::UnicodeFonts;
use minifb::Key;
use picker::PlaylistPicker;
use selection::TrackSelection;
use send_menu::SendMenu;
use std::{
    fmt::Write,
    sync::Arc,
    time::{Duration, Instant},
};
use text_input::TextInput;

pub const WIDTH: usize = 760;
pub const HEIGHT: usize = 470;
pub const TITLEBAR_DRAG_REGION: (usize, usize, usize, usize) = (6, 2, 690, 16);
const LEFT_ROWS: usize = 11;
const LEFT_TOP: usize = 119;
const SEARCH_INPUT: Rect = (324, 29, 250, 23);
const SEARCH_COLUMNS: usize = 29;
const RIGHT_ROWS: usize = 11;
const BG: u32 = 0x29283d;
const BLACK: u32 = 0x050805;
const GREEN: u32 = 0x5bff35;
const WHITE: u32 = 0xc8c9d4;
const GOLD: u32 = 0xb3a978;
const DIM: u32 = 0x778273;
type Rect = (usize, usize, usize, usize);

#[derive(Debug)]
pub enum Action {
    Command(LibraryCommand),
    Minimize,
    Close,
    Paste,
    Copy(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Button {
    Minimize,
    Close,
    Refresh,
    Search,
    Clear,
    Play,
    Append,
    SendTo,
    PickerConfirm,
    PickerNew,
    PickerCreate,
    PickerCancel,
}
impl Button {
    fn rect(self) -> Rect {
        match self {
            Self::Minimize => (704, 3, 20, 14),
            Self::Close => (728, 3, 20, 14),
            Self::Refresh => (12, 29, 164, 23),
            Self::Search => (580, 29, 76, 23),
            Self::Clear => (662, 29, 86, 23),
            Self::Play => (246, 413, 106, 24),
            Self::Append => (360, 413, 128, 24),
            Self::SendTo => (246, 413, 168, 24),
            Self::PickerNew => (192, 348, 128, 24),
            Self::PickerConfirm | Self::PickerCreate => (328, 348, 110, 24),
            Self::PickerCancel => (450, 348, 110, 24),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Button(Button),
    Search,
    SearchInput,
    Playlist(usize),
    Track(usize),
    Destination(usize),
    PlaylistName,
    MenuItem(usize),
    MenuOutside,
}

struct PendingRemoval {
    id: PlaylistId,
    tracks: Arc<Vec<TrackRow>>,
    playlist_revision: Arc<[u8]>,
    revision: u64,
}

pub struct View {
    pub pixels: Vec<u32>,
    skin: Skin,
    language: Language,
    selected_id: Option<PlaylistId>,
    pending_selection: Option<PlaylistId>,
    observed_id: Option<PlaylistId>,
    observed_playlist_revision: Arc<[u8]>,
    selected_track: Option<usize>,
    playlist_choices: TrackSelection,
    search_choices: TrackSelection,
    picker: Option<PlaylistPicker>,
    send_menu: Option<SendMenu>,
    submitted_picker: Option<PlaylistPicker>,
    pending_removal: Option<PendingRemoval>,
    press_collapse: bool,
    press_modified: bool,
    selected_uri: Option<String>,
    left_focus: bool,
    search_mode: bool,
    input_focus: bool,
    query: String,
    cursor: usize,
    anchor: usize,
    input_offset: usize,
    search_track: Option<usize>,
    search_offset: usize,
    search_remainder: f64,
    search_revision: u64,
    search_pending: Option<(u64, String)>,
    search_click: Option<(u64, usize, Instant)>,
    left_offset: usize,
    right_offset: usize,
    left_remainder: f64,
    right_remainder: f64,
    focused: bool,
    pressed: Option<Target>,
    hovered: Option<Target>,
    press_revision: u64,
    pressed_playlist: Option<PlaylistId>,
    last_click: Option<(PlaylistId, Target, Instant)>,
    notice: Option<(LocalizedText, Instant)>,
    ui_revision: u64,
    snapshot_revision: Option<u64>,
    rendered: Option<(u64, u64)>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            pixels: vec![BG; WIDTH * HEIGHT],
            skin: Skin::Classic,
            language: Language::French,
            selected_id: None,
            pending_selection: None,
            observed_id: None,
            observed_playlist_revision: Arc::from([]),
            selected_track: None,
            playlist_choices: TrackSelection::default(),
            search_choices: TrackSelection::default(),
            picker: None,
            send_menu: None,
            submitted_picker: None,
            pending_removal: None,
            press_collapse: false,
            press_modified: false,
            selected_uri: None,
            left_focus: true,
            search_mode: false,
            input_focus: false,
            query: String::new(),
            cursor: 0,
            anchor: 0,
            input_offset: 0,
            search_track: None,
            search_offset: 0,
            search_remainder: 0.0,
            search_revision: 0,
            search_pending: None,
            search_click: None,
            left_offset: 0,
            right_offset: 0,
            left_remainder: 0.0,
            right_remainder: 0.0,
            focused: true,
            pressed: None,
            hovered: None,
            press_revision: 0,
            pressed_playlist: None,
            last_click: None,
            notice: None,
            ui_revision: 0,
            snapshot_revision: None,
            rendered: None,
        }
    }
}
impl View {
    fn dirty(&mut self) {
        self.ui_revision = self.ui_revision.wrapping_add(1);
    }
    /// Changes only presentation: keep selection, scroll positions and buffer.
    pub fn set_skin(&mut self, skin: Skin) {
        if self.skin == skin {
            return;
        }
        self.skin = skin;
        self.cancel_pointer();
        self.last_click = None;
        self.search_click = None;
        self.dirty();
    }
    pub fn set_language(&mut self, language: Language) {
        if self.language != language {
            self.language = language;
            self.cancel_pointer();
            self.last_click = None;
            self.search_click = None;
            self.dirty();
        }
    }
    pub fn notice(&mut self, text: impl Into<LocalizedText>) {
        self.notice = Some((text.into(), Instant::now()));
        self.dirty();
    }
    pub fn set_focus(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.dirty();
        }
        if !focused {
            self.close_send_menu();
            self.cancel_pointer();
            self.last_click = None;
            self.search_click = None;
        }
    }
    pub fn cancel_pointer(&mut self) {
        // Win32 also reports capture loss after a normal button release.
        // Only an interrupted press should break the double-click sequence.
        if self.pressed.is_some() {
            self.last_click = None;
            self.search_click = None;
        }
        self.clear_press();
    }
    fn clear_press(&mut self) {
        if self.pressed.take().is_some() {
            self.dirty();
        }
        self.pressed_playlist = None;
        self.hovered = None;
    }
    fn selected<'a>(&self, s: &'a LibrarySnapshot) -> Option<&'a PlaylistEntry> {
        self.selected_id
            .as_ref()
            .and_then(|id| s.entries.iter().find(|e| &e.id == id))
    }
    fn tracks_ready(&self, s: &LibrarySnapshot) -> bool {
        !s.loading && self.selected_id.is_some() && self.selected_id == s.selected_id
    }
    fn active_tracks<'a>(&self, s: &'a LibrarySnapshot) -> &'a [TrackRow] {
        if self.search_mode {
            &s.search.tracks
        } else {
            &s.tracks
        }
    }
    fn active_ready(&self, s: &LibrarySnapshot) -> bool {
        if self.search_mode {
            !s.search.loading && self.search_pending.is_none()
        } else {
            self.tracks_ready(s)
        }
    }
    fn active_offset(&self) -> usize {
        if self.search_mode {
            self.search_offset
        } else {
            self.right_offset
        }
    }
    fn active_selection(&self) -> Option<usize> {
        if self.search_mode {
            self.search_track
        } else {
            self.selected_track
        }
    }
    fn open_search(&mut self, input: bool) {
        self.search_mode = true;
        self.input_focus = input;
        self.left_focus = !input;
        self.search_click = None;
        self.last_click = None;
        self.dirty();
    }
    pub fn wants_text_input(&self) -> bool {
        self.focused
            && self.send_menu.is_none()
            && self
                .picker
                .as_ref()
                .map_or(self.search_mode && self.input_focus, |picker| {
                    picker.creating
                })
    }
    #[cfg(test)]
    fn selection(&self) -> std::ops::Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }
    fn query_input(&mut self) -> TextInput<'_> {
        TextInput {
            text: &mut self.query,
            cursor: &mut self.cursor,
            anchor: &mut self.anchor,
            offset: &mut self.input_offset,
            columns: SEARCH_COLUMNS,
        }
    }
    fn reveal_cursor(&mut self) {
        self.query_input().reveal();
    }
    pub fn type_char(&mut self, ch: char) {
        if !self.wants_text_input() {
            return;
        }
        let changed = if let Some(picker) = &mut self.picker {
            picker.type_char(ch)
        } else {
            self.query_input()
                .insert(ch, SEARCH_QUERY_MAX_CHARS, SEARCH_QUERY_MAX_BYTES)
        };
        if changed {
            self.dirty();
        }
    }
    pub fn paste(&mut self, text: &str) {
        if !self.wants_text_input() {
            return;
        }
        for ch in text.chars().take(SEARCH_QUERY_MAX_CHARS) {
            self.type_char(if ch.is_whitespace() { ' ' } else { ch });
        }
    }
    /// Retire recovery only once this creation has actually reached the engine.
    pub fn creation_dispatched(&mut self) {
        self.submitted_picker = None;
    }
    /// Input processing batches actions before dispatch. Recover only the exact
    /// rejected command, never an unrelated command from the same input batch.
    pub fn command_rejected(&mut self, command: &LibraryCommand) {
        match command {
            LibraryCommand::RemoveFromPlaylist {
                id,
                expected_tracks,
                expected_revision,
                ..
            } if self.pending_removal.as_ref().is_some_and(|pending| {
                &pending.id == id
                    && Arc::ptr_eq(&pending.tracks, expected_tracks)
                    && &pending.playlist_revision == expected_revision
            }) =>
            {
                self.pending_removal = None;
                self.dirty();
            }
            LibraryCommand::CreatePlaylist { name, tracks }
                if self.submitted_picker.as_ref().is_some_and(|picker| {
                    normalized_playlist_name(&picker.name) == Some(name.as_str())
                        && &picker.tracks == tracks
                }) =>
            {
                self.picker = self.submitted_picker.take();
                self.dirty();
            }
            LibraryCommand::Search(query)
                if self
                    .search_pending
                    .as_ref()
                    .is_some_and(|(_, pending)| pending == query) =>
            {
                self.search_pending = None;
                self.dirty();
            }
            _ => (),
        }
    }
    fn submit_search(&mut self, s: &LibrarySnapshot) -> Option<Action> {
        if !s.connected {
            return None;
        }
        let query = self.query.trim();
        if self
            .search_pending
            .as_ref()
            .is_some_and(|(_, pending)| pending == query)
            || (s.search.loading && s.search.query.as_ref() == query)
        {
            return None;
        }
        self.search_click = None;
        self.search_pending = Some((s.search.revision, query.to_owned()));
        self.search_track = None;
        self.search_choices.clear();
        self.search_offset = 0;
        self.search_remainder = 0.0;
        self.dirty();
        Some(Action::Command(LibraryCommand::Search(
            self.query.trim().to_owned(),
        )))
    }
    fn edit_key(&mut self, key: Key, control: bool, shift: bool) -> Option<Action> {
        let action = self.query_input().key(key, control, shift);
        self.dirty();
        action
    }
    fn sync(&mut self, s: &LibrarySnapshot) {
        if self.snapshot_revision == Some(s.revision) {
            return;
        }
        let initial = self.snapshot_revision.is_none();
        self.snapshot_revision = Some(s.revision);
        if self
            .send_menu
            .as_ref()
            .is_some_and(|menu| !menu.source_matches(self, s))
        {
            self.close_send_menu();
        }
        if self
            .pending_removal
            .as_ref()
            .is_some_and(|pending| !s.connected || pending.revision != s.playlist_remove_revision)
        {
            let pending = self.pending_removal.take().unwrap();
            // A removed occurrence must not jump to another equal URI after
            // reconciliation. Metadata-only updates preserve the selection.
            if self.selected_id.as_ref() == Some(&pending.id)
                && s.selected_id.as_ref() == Some(&pending.id)
                && (pending.tracks.len() != s.tracks.len()
                    || pending
                        .tracks
                        .iter()
                        .zip(s.tracks.iter())
                        .any(|(a, b)| a.uri != b.uri))
            {
                self.playlist_choices.clear();
                self.selected_track = None;
                self.selected_uri = None;
            }
        }
        if self.search_revision != s.search.revision {
            self.search_revision = s.search.revision;
            self.search_track = None;
            self.search_choices.clear();
            self.search_offset = 0;
            self.search_remainder = 0.0;
            self.search_click = None;
        }
        // Starting a search first clears old results, then publishes its query
        // under the same request revision. Acknowledge either publication order.
        if self
            .search_pending
            .as_ref()
            .is_some_and(|(revision, query)| {
                *revision != s.search.revision && query.as_str() == s.search.query.as_ref()
            })
        {
            self.search_pending = None;
        }
        if !s.connected {
            self.search_pending = None;
        }
        if initial && !s.search.query.is_empty() {
            self.query = s.search.query.to_string();
            self.cursor = self.query.len();
            self.anchor = self.cursor;
            self.reveal_cursor();
        }
        self.search_offset = self
            .search_offset
            .min(s.search.tracks.len().saturating_sub(RIGHT_ROWS));
        if self
            .search_track
            .is_some_and(|i| i >= s.search.tracks.len())
        {
            self.search_track = None;
        }

        if !self.search_choices.valid(&s.search.tracks) {
            self.search_track = self
                .search_track
                .and_then(|index| self.search_choices.remapped_focus(index, &s.search.tracks));
            self.search_choices.reconcile(&s.search.tracks);
        }
        if self.observed_playlist_revision != s.selected_playlist_revision {
            // Spotify revisions distinguish identical track occurrences. A
            // selection made against an older revision must be chosen again.
            if self.observed_id == s.selected_id {
                self.playlist_choices.clear();
                self.selected_track = None;
                self.selected_uri = None;
            }
            self.observed_playlist_revision = s.selected_playlist_revision.clone();
        }
        self.playlist_choices.reconcile(&s.tracks);
        if !s.connected {
            self.picker = None;
            self.submitted_picker = None;
        }

        // Loading and metadata updates can arrive between the two clicks.
        // Keep a playlist click only while the same available ID occupies its row.
        if !self.last_click.as_ref().is_some_and(|(id, target, _)| {
            matches!(target, Target::Playlist(i) if s.connected
                && s.entries.get(*i).is_some_and(|e| e.available && &e.id == id))
        }) {
            self.last_click = None;
        }
        if initial || self.observed_id != s.selected_id {
            if self.pending_selection.is_none() || self.pending_selection == s.selected_id {
                let changed = self.selected_id != s.selected_id;
                self.selected_id = s.selected_id.clone();
                self.pending_selection = None;
                if changed {
                    self.selected_track = None;
                    self.playlist_choices.clear();
                    self.selected_uri = None;
                    self.right_offset = 0;
                    if let Some(index) = s
                        .entries
                        .iter()
                        .position(|entry| Some(&entry.id) == self.selected_id.as_ref())
                    {
                        reveal(index, &mut self.left_offset, LEFT_ROWS);
                    }
                }
            }
            self.observed_id = s.selected_id.clone();
        }
        // A failed selection keeps the backend's previous collection. It must
        // never display that collection's tracks under a different heading.
        if self.pending_selection.is_some()
            && !s.loading
            && !s.busy
            && self.pending_selection != s.selected_id
        {
            self.pending_selection = None;
            self.selected_id = s.selected_id.clone();
            self.selected_track = None;
            self.playlist_choices.clear();
            self.selected_uri = None;
            self.right_offset = 0;
        }
        if self.selected_id.is_some() && self.selected(s).is_none() {
            self.selected_id = None;
            self.pending_selection = None;
            self.selected_track = None;
            self.playlist_choices.clear();
            self.selected_uri = None;
            self.right_offset = 0;
        }
        self.left_offset = self
            .left_offset
            .min(s.entries.len().saturating_sub(LEFT_ROWS));
        if self.tracks_ready(s) {
            if let Some(uri) = &self.selected_uri
                && self
                    .selected_track
                    .is_none_or(|i| s.tracks.get(i).is_none_or(|row| &row.uri != uri))
            {
                self.selected_track = s.tracks.iter().position(|row| &row.uri == uri);
                if self.selected_track.is_none() {
                    self.selected_uri = None;
                }
            }
            self.right_offset = self
                .right_offset
                .min(s.tracks.len().saturating_sub(RIGHT_ROWS));
        } else {
            self.selected_track = None;
            self.playlist_choices.clear();
            self.selected_uri = None;
            self.right_offset = 0;
        }
    }
    fn enabled(&self, b: Button, s: &LibrarySnapshot) -> bool {
        match b {
            Button::Minimize | Button::Close | Button::PickerCancel => true,
            Button::SendTo => {
                s.connected
                    && self.active_ready(s)
                    && if !self.search_mode && self.left_focus {
                        self.selected(s).is_some_and(|entry| entry.available)
                            && !s.tracks.is_empty()
                    } else {
                        self.active_choices().len() > 0
                    }
            }
            Button::PickerNew => self.picker.is_some() && s.connected && !s.playlist_write_busy,
            Button::PickerCreate => self.picker.as_ref().is_some_and(|picker| {
                picker.creating
                    && s.connected
                    && !s.playlist_write_busy
                    && normalized_playlist_name(&picker.name).is_some()
            }),
            Button::PickerConfirm => self.picker.as_ref().is_some_and(|picker| {
                !picker.creating
                    && s.connected
                    && !s.playlist_write_busy
                    && picker.destination.as_ref().is_some_and(|id| {
                        s.entries
                            .iter()
                            .any(|entry| &entry.id == id && entry.available)
                    })
            }),
            Button::Refresh => s.connected && !s.busy && !s.loading,
            Button::Search => s.connected && !self.query.trim().is_empty(),
            Button::Clear => !self.query.is_empty() || !s.search.query.is_empty(),
            Button::Play | Button::Append => {
                s.connected
                    && !s.busy
                    && self.active_ready(s)
                    && !self.active_tracks(s).is_empty()
                    && if self.search_mode {
                        self.active_choices().len() > 0
                    } else {
                        self.selected(s).is_some_and(|e| e.available)
                            && (self.left_focus || self.active_choices().len() > 0)
                    }
            }
        }
    }
    fn target(&self, x: usize, y: usize) -> Option<Target> {
        if let Some(menu) = &self.send_menu {
            return Some(menu.target(x, y));
        }
        if let Some(picker) = &self.picker {
            return picker.target(x, y);
        }
        if self.search_mode {
            for b in [Button::Search, Button::Clear] {
                if inside(b.rect(), x, y) {
                    return Some(Target::Button(b));
                }
            }
            if inside(SEARCH_INPUT, x, y) {
                return Some(Target::SearchInput);
            }
        }
        if inside((18, 65, 202, 24), x, y) {
            return Some(Target::Search);
        }
        for b in [
            Button::Minimize,
            Button::Close,
            Button::Refresh,
            Button::SendTo,
        ] {
            if inside(b.rect(), x, y) {
                return Some(Target::Button(b));
            }
        }
        if inside((18, LEFT_TOP, 202, LEFT_ROWS * 28), x, y) {
            return Some(Target::Playlist(self.left_offset + (y - LEFT_TOP) / 28));
        }
        if inside((252, 120, 480, RIGHT_ROWS * 24), x, y) {
            return Some(Target::Track(self.active_offset() + (y - 120) / 24));
        }
        None
    }
    pub fn pointer_down(&mut self, x: usize, y: usize, s: &LibrarySnapshot) -> Option<Action> {
        self.pointer_down_with_modifiers(x, y, false, false, s)
    }
    pub fn pointer_down_with_modifiers(
        &mut self,
        x: usize,
        y: usize,
        control: bool,
        shift: bool,
        s: &LibrarySnapshot,
    ) -> Option<Action> {
        let menu_was_open = self.send_menu.is_some();
        self.sync(s);
        self.clear_press();
        if menu_was_open && self.send_menu.is_none() {
            return None;
        }
        if self.send_menu.is_some() && self.target(x, y) == Some(Target::MenuOutside) {
            self.close_send_menu();
            return None;
        }
        self.press_collapse = false;
        self.press_modified = control || shift;
        if !self.focused {
            return None;
        }
        let target = self.target(x, y);
        let allowed = match target {
            Some(Target::Button(b)) => self.enabled(b, s),
            Some(Target::MenuItem(index)) => self.menu_item_enabled(index, s),
            Some(Target::MenuOutside) => false,
            Some(Target::Playlist(i)) => s.entries.get(i).is_some_and(|e| e.available),
            Some(Target::Track(i)) => self.active_ready(s) && i < self.active_tracks(s).len(),
            Some(Target::Search | Target::SearchInput | Target::PlaylistName) => true,
            Some(Target::Destination(i)) => self
                .picker
                .as_ref()
                .is_some_and(|picker| i < picker.destinations.len()),
            None => false,
        };
        if allowed {
            if let Some(Target::Track(index)) = target {
                self.left_focus = false;
                self.input_focus = false;
                if !control && !shift && self.active_choices().contains(index) {
                    self.press_collapse = true;
                } else {
                    self.choose_track(index, control, shift, s);
                }
            }
            if !matches!(target, Some(Target::Track(_))) {
                self.search_click = None;
            }
            if self.input_focus
                && !matches!(
                    target,
                    Some(Target::SearchInput | Target::Button(Button::SendTo))
                )
            {
                self.input_focus = false;
                self.dirty();
            }
            self.pressed = target;
            self.hovered = target;
            self.press_revision = s.revision;
            self.pressed_playlist = match target {
                Some(Target::Playlist(i)) => Some(s.entries[i].id.clone()),
                _ => None,
            };
            self.dirty();
        } else {
            self.last_click = None;
            self.search_click = None;
        }
        None
    }
    pub fn pointer_move(&mut self, position: Option<(usize, usize)>) {
        let target = position.and_then(|(x, y)| self.target(x, y));
        if let Some(menu) = &mut self.send_menu {
            // Native polling also reports an unchanged pointer after keyboard input.
            // Keep this separate from pressed/hovered, which cancel_pointer clears.
            let moved = menu.pointer_position != position;
            menu.pointer_position = position;
            if moved
                && let Some(Target::MenuItem(index)) = target
                && menu.focus != index
            {
                menu.focus = index;
                self.dirty();
            }
        }
        if self.hovered != target {
            self.hovered = target;
            if self.pressed.is_some() {
                self.dirty();
            }
        }
    }
    pub fn pointer_up(
        &mut self,
        position: Option<(usize, usize)>,
        s: &LibrarySnapshot,
    ) -> Option<Action> {
        self.sync(s);
        let pressed = self.pressed.take();
        let playlist = self.pressed_playlist.take();
        self.pointer_move(position);
        if pressed.is_some() {
            self.dirty();
        }
        if !self.focused || pressed != self.hovered {
            self.last_click = None;
            self.search_click = None;
            return None;
        }
        let target = pressed?;
        if self.press_revision != s.revision
            && !matches!(
                target,
                Target::Playlist(_)
                    | Target::Search
                    | Target::SearchInput
                    | Target::PlaylistName
                    | Target::MenuItem(_)
                    | Target::Destination(_)
                    | Target::Button(Button::Close | Button::Minimize)
            )
        {
            self.last_click = None;
            self.search_click = None;
            return None;
        }
        match target {
            Target::MenuItem(index) => self.activate_menu_item(index, s),
            Target::PlaylistName => {
                if let Some(picker) = &mut self.picker {
                    picker.position_name(position.map_or(0, |(x, _)| x));
                    self.dirty();
                }
                None
            }
            Target::Destination(index) => {
                if let Some(picker) = &mut self.picker {
                    picker.choose(index);
                    self.dirty();
                }
                None
            }
            Target::Button(b) if self.enabled(b, s) => self.button(b, s),
            Target::Search => {
                self.open_search(true);
                None
            }
            Target::SearchInput => {
                self.open_search(true);
                let column = position.map_or(0, |(x, _)| x.saturating_sub(SEARCH_INPUT.0 + 5) / 8)
                    + self.input_offset;
                self.cursor = self
                    .query
                    .char_indices()
                    .nth(column)
                    .map_or(self.query.len(), |(i, _)| i);
                self.anchor = self.cursor;
                self.reveal_cursor();
                None
            }
            Target::Playlist(index) => {
                let Some(entry) = s
                    .entries
                    .get(index)
                    .filter(|e| e.available && playlist.as_ref() == Some(&e.id))
                else {
                    self.last_click = None;
                    return None;
                };
                self.left_focus = true;
                let double = self.is_double_click(&entry.id, target);
                let selection = self.select_playlist(index, s);
                self.last_click = (!double).then(|| (entry.id.clone(), target, Instant::now()));
                if double && s.connected {
                    // The engine also handles playlists that are still loading.
                    self.selected_track = None;
                    self.playlist_choices.clear();
                    self.selected_uri = None;
                    self.right_offset = 0;
                    Some(Action::Command(LibraryCommand::Queue {
                        id: entry.id.clone(),
                        replace: true,
                        play: true,
                        start: 0,
                    }))
                } else {
                    selection
                }
            }
            Target::Track(index) if self.active_ready(s) && index < self.active_tracks(s).len() => {
                self.left_focus = false;
                self.input_focus = false;
                if self.press_collapse {
                    self.choose_track(index, false, false, s);
                }
                if self.press_modified {
                    self.search_click = None;
                    self.last_click = None;
                    return None;
                }
                if self.search_mode {
                    let double = self.search_click.is_some_and(|(revision, old, at)| {
                        revision == s.search.revision
                            && old == index
                            && at.elapsed() < Duration::from_millis(450)
                    });
                    self.search_click =
                        (!double).then(|| (s.search.revision, index, Instant::now()));
                    return if double { self.queue(s, true) } else { None };
                }
                let id = self.selected_id.clone()?;
                let double = self.is_double_click(&id, target);
                self.last_click = (!double).then(|| (id, target, Instant::now()));
                if double && self.enabled(Button::Play, s) {
                    self.queue(s, true)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn is_double_click(&self, id: &PlaylistId, target: Target) -> bool {
        self.last_click
            .as_ref()
            .is_some_and(|(old, old_target, at)| {
                old == id && *old_target == target && at.elapsed() < Duration::from_millis(450)
            })
    }
    fn select_playlist(&mut self, index: usize, s: &LibrarySnapshot) -> Option<Action> {
        let entry = s.entries.get(index)?;
        if !entry.available {
            return None;
        }
        self.dirty();
        self.search_mode = false;
        self.input_focus = false;
        self.search_click = None;
        self.last_click = None;
        if self.selected_id.as_ref() == Some(&entry.id) {
            return None;
        }
        self.selected_id = Some(entry.id.clone());
        self.pending_selection = Some(entry.id.clone());
        self.selected_track = None;
        self.playlist_choices.clear();
        self.selected_uri = None;
        self.right_offset = 0;
        self.right_remainder = 0.0;
        reveal(index, &mut self.left_offset, LEFT_ROWS);
        Some(Action::Command(LibraryCommand::Select(entry.id.clone())))
    }
    fn focus_track(&mut self, index: usize, s: &LibrarySnapshot) {
        if self.search_mode {
            if index < s.search.tracks.len() {
                self.search_track = Some(index);
                reveal(index, &mut self.search_offset, RIGHT_ROWS);
                self.dirty();
            }
            return;
        }
        if let Some(row) = s.tracks.get(index) {
            self.selected_track = Some(index);
            self.selected_uri = Some(row.uri.clone());
            reveal(index, &mut self.right_offset, RIGHT_ROWS);
            self.dirty();
        }
    }
    fn queue(&self, s: &LibrarySnapshot, play: bool) -> Option<Action> {
        if !self.enabled(if play { Button::Play } else { Button::Append }, s) {
            return None;
        }
        if !self.search_mode && self.left_focus {
            return Some(Action::Command(LibraryCommand::Queue {
                id: self.selected_id.clone()?,
                replace: play,
                play,
                start: 0,
            }));
        }
        let tracks = self.selected_tracks(s);
        if tracks.is_empty() {
            return None;
        }
        Some(Action::Command(LibraryCommand::QueueTracks {
            tracks,
            origin: self.selection_origin(),
            expected_queue: None,
            replace: play,
            play,
            insert_at: None,
        }))
    }

    fn button(&mut self, button: Button, s: &LibrarySnapshot) -> Option<Action> {
        self.last_click = None;
        match button {
            Button::Minimize => Some(Action::Minimize),
            Button::Close => Some(Action::Close),
            Button::Refresh => Some(Action::Command(LibraryCommand::Refresh)),
            Button::Search => {
                self.input_focus = true;
                self.left_focus = false;
                self.submit_search(s)
            }
            Button::Clear => {
                self.query.clear();
                self.cursor = 0;
                self.anchor = 0;
                self.input_offset = 0;
                self.search_click = None;
                self.search_track = None;
                self.search_choices.clear();
                self.search_offset = 0;
                self.search_pending = Some((s.search.revision, String::new()));
                self.open_search(true);
                Some(Action::Command(LibraryCommand::Search(String::new())))
            }
            Button::SendTo => {
                self.open_send_menu(s);
                None
            }
            Button::Play => self.queue(s, true),
            Button::Append => self.queue(s, false),
            Button::PickerNew => {
                self.new_playlist();
                None
            }
            Button::PickerCreate => self.create_playlist(s),
            Button::PickerCancel => {
                self.cancel_picker();
                None
            }
            Button::PickerConfirm => self.confirm_picker(s),
        }
    }
    fn can_remove_selected(&self, s: &LibrarySnapshot) -> bool {
        !self.search_mode
            && !self.left_focus
            && !self.input_focus
            && self.picker.is_none()
            && self.submitted_picker.is_none()
            && self.pending_removal.is_none()
            && s.connected
            && s.selected_owned
            && !s.selected_playlist_revision.is_empty()
            && !s.busy
            && !s.playlist_write_busy
            && self.tracks_ready(s)
            && self.selected(s).is_some_and(|entry| entry.available)
    }

    fn remove_selected(&mut self, s: &LibrarySnapshot) -> Option<Action> {
        if !self.can_remove_selected(s) {
            return None;
        }
        self.remove_captured(
            s,
            self.selected_id.clone()?,
            s.tracks.clone(),
            s.selected_playlist_revision.clone(),
            self.playlist_choices.indices(&s.tracks),
        )
    }

    fn remove_captured(
        &mut self,
        s: &LibrarySnapshot,
        id: PlaylistId,
        tracks: Arc<Vec<TrackRow>>,
        revision: Arc<[u8]>,
        indices: Vec<usize>,
    ) -> Option<Action> {
        if !self.can_remove_selected(s)
            || indices.is_empty()
            || Some(&id) != self.selected_id.as_ref()
            || revision != s.selected_playlist_revision
            || tracks.len() != s.tracks.len()
            || tracks
                .iter()
                .zip(s.tracks.iter())
                .any(|(a, b)| a.uri != b.uri)
        {
            return None;
        }
        self.pending_removal = Some(PendingRemoval {
            id: id.clone(),
            tracks: tracks.clone(),
            playlist_revision: revision.clone(),
            revision: s.playlist_remove_revision,
        });
        self.dirty();
        Some(Action::Command(LibraryCommand::RemoveFromPlaylist {
            id,
            expected_tracks: tracks,
            expected_revision: revision,
            indices,
        }))
    }

    pub fn key(
        &mut self,
        key: Key,
        control: bool,
        shift: bool,
        s: &LibrarySnapshot,
    ) -> Option<Action> {
        let menu_was_open = self.send_menu.is_some();
        self.sync(s);
        self.cancel_pointer();
        self.last_click = None;
        self.search_click = None;
        if !self.focused {
            return None;
        }
        if menu_was_open {
            return self.send_menu_key(key, s);
        }
        if self.picker.is_some() {
            return self.picker_key(key, control, shift, s);
        }
        if (key == Key::Menu || (key == Key::F10 && shift)) && !self.wants_text_input() {
            self.open_send_menu(s);
            return None;
        }
        if control && key == Key::F {
            self.open_search(true);
            self.anchor = 0;
            self.cursor = self.query.len();
            self.reveal_cursor();
            return None;
        }
        if self.wants_text_input() {
            match key {
                Key::Enter => return self.submit_search(s),
                Key::Escape => {
                    self.input_focus = false;
                    self.left_focus = true;
                    self.dirty();
                    return None;
                }
                Key::Tab => {
                    self.input_focus = false;
                    self.left_focus = shift;
                    self.dirty();
                    return None;
                }
                Key::Down => {
                    self.input_focus = false;
                    self.left_focus = false;
                    self.dirty();
                }
                _ => return self.edit_key(key, control, shift),
            }
        }
        match key {
            Key::Delete if !control && !shift => return self.remove_selected(s),
            Key::Escape => return Some(Action::Close),
            Key::F5 if self.enabled(Button::Refresh, s) => return self.button(Button::Refresh, s),
            Key::Tab => {
                if self.search_mode && ((self.left_focus && !shift) || (!self.left_focus && shift))
                {
                    self.input_focus = true;
                    self.left_focus = false;
                } else {
                    self.left_focus = !self.left_focus;
                }
                self.dirty();
                return None;
            }
            Key::Left => {
                self.left_focus = true;
                self.dirty();
                return None;
            }
            Key::Right => {
                self.left_focus = false;
                self.dirty();
                return None;
            }
            _ => (),
        }
        if control && key == Key::A && !self.left_focus && self.active_ready(s) {
            if self.search_mode {
                self.search_choices.all(&s.search.tracks);
            } else {
                self.playlist_choices.all(&s.tracks);
            }
            if self.active_selection().is_none() && !self.active_tracks(s).is_empty() {
                self.focus_track(0, s);
            }
            self.dirty();
            return None;
        }
        if matches!(
            key,
            Key::Up | Key::Down | Key::Home | Key::End | Key::PageUp | Key::PageDown
        ) {
            if self.left_focus && key == Key::Home {
                self.open_search(false);
                return None;
            }
            let length = if self.left_focus {
                s.entries.len()
            } else if self.active_ready(s) {
                self.active_tracks(s).len()
            } else {
                0
            };
            if length == 0 {
                return None;
            }
            let old = if self.left_focus {
                s.entries
                    .iter()
                    .position(|e| Some(&e.id) == self.selected_id.as_ref())
            } else {
                self.active_selection()
            };
            let page = if self.left_focus {
                LEFT_ROWS
            } else {
                RIGHT_ROWS
            };
            if self.left_focus
                && (key == Key::Home || (key == Key::Up && (self.search_mode || old == Some(0))))
            {
                self.open_search(false);
                return None;
            }
            let old = if self.left_focus && self.search_mode {
                None
            } else {
                old
            };
            let mut index = match key {
                Key::Home => 0,
                Key::End => length - 1,
                Key::Up => old.unwrap_or(1).saturating_sub(1),
                Key::Down => old.map_or(0, |i| (i + 1).min(length - 1)),
                Key::PageUp => old.unwrap_or(0).saturating_sub(page),
                _ => (old.unwrap_or(0) + page).min(length - 1),
            };
            self.last_click = None;
            if self.left_focus {
                let backwards = matches!(key, Key::Up | Key::PageUp | Key::End);
                while !s.entries[index].available {
                    index = if backwards {
                        index.checked_sub(1)?
                    } else {
                        index.checked_add(1).filter(|&i| i < length)?
                    };
                }
                return self.select_playlist(index, s);
            }
            if control && !shift {
                self.focus_track(index, s);
            } else {
                self.choose_track(index, control, shift, s);
            }
            return None;
        }
        if key == Key::Enter {
            if self.search_mode && self.left_focus {
                self.open_search(true);
                return None;
            }
            return self.queue(s, !(control || shift));
        }
        None
    }
    pub fn scroll(&mut self, y: f32, position: Option<(usize, usize)>, s: &LibrarySnapshot) {
        self.sync(s);
        if !y.is_finite() || self.send_menu.is_some() {
            return;
        }
        if self.picker.is_some() {
            self.picker_scroll(y, position);
            return;
        }
        let left = match position {
            Some((x, y)) if (12..230).contains(&x) && (LEFT_TOP..427).contains(&y) => true,
            Some((x, y)) if (246..746).contains(&x) && (119..384).contains(&y) => false,
            _ => return,
        };
        let len = if left {
            s.entries.len()
        } else if self.active_ready(s) {
            self.active_tracks(s).len()
        } else {
            0
        };
        let (offset, remainder, rows) = if left {
            (&mut self.left_offset, &mut self.left_remainder, LEFT_ROWS)
        } else if self.search_mode {
            (
                &mut self.search_offset,
                &mut self.search_remainder,
                RIGHT_ROWS,
            )
        } else {
            (
                &mut self.right_offset,
                &mut self.right_remainder,
                RIGHT_ROWS,
            )
        };
        let max = len.saturating_sub(rows);
        let old = *offset;
        *remainder += f64::from(y).clamp(-1000.0, 1000.0);
        let whole = (remainder.abs() + 1e-6).floor().copysign(*remainder) as i64;
        *remainder -= whole as f64;
        *offset = (*offset as i64 - whole).clamp(0, max as i64) as usize;
        if (*offset == 0 && *remainder > 0.0) || (*offset == max && *remainder < 0.0) {
            *remainder = 0.0;
        }
        if *offset != old {
            self.last_click = None;
            self.search_click = None;
            self.cancel_pointer();
            self.dirty();
        }
    }

    /// Returns false without drawing or scanning track contents when nothing changed.
    pub fn render(&mut self, s: &LibrarySnapshot) -> bool {
        self.sync(s);
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() >= Duration::from_secs(8))
        {
            self.notice = None;
            self.dirty();
        }
        let fingerprint = (s.revision, self.ui_revision);
        if self.rendered == Some(fingerprint) {
            return false;
        }
        self.rendered = Some(fingerprint);
        let selected = self.selected(s);
        let ready = self.active_ready(s);
        let tracks = self.active_tracks(s);
        let offset = self.active_offset();
        let choices = if self.search_mode {
            &self.search_choices
        } else {
            &self.playlist_choices
        };
        let search_enabled = self.enabled(Button::Search, s);
        let clear_enabled = self.enabled(Button::Clear, s);
        let send_enabled = self.enabled(Button::SendTo, s);
        let menu_enabled = std::array::from_fn(|index| self.menu_item_enabled(index, s));
        let picker_confirm_enabled = self.enabled(Button::PickerConfirm, s);
        let picker_new_enabled = self.enabled(Button::PickerNew, s);
        let picker_create_enabled = self.enabled(Button::PickerCreate, s);
        let refresh_enabled = self.enabled(Button::Refresh, s);
        let pressed = self.pressed.filter(|p| Some(*p) == self.hovered);
        let palette = modern::Palette::new(self.skin, self.focused);
        let language = self.language;
        let mut c = Canvas {
            pixels: &mut self.pixels,
            focused: self.focused,
            skin: self.skin,
        };
        if self.skin == Skin::Modern {
            c.modern_chrome(palette.title_line);
        } else {
            c.pixels.fill(palette.background);
            c.bevel((0, 0, WIDTH, HEIGHT), palette.background, false);
            c.bevel((3, 3, WIDTH - 6, HEIGHT - 6), palette.background, true);
            c.bevel((6, 4, 688, 14), palette.header_alt, true);
            for y in [7, 10, 13] {
                c.rect((12, y, 208, 1), palette.title_line);
                c.rect((480, y, 208, 1), palette.title_line);
            }
        }
        c.text(
            232,
            7,
            language.text("SPOTIAMP  /  BIBLIOTHÈQUE", "SPOTIAMP  /  LIBRARY"),
            palette.title_text,
            30,
        );
        c.button(
            Button::Minimize,
            "_",
            true,
            pressed == Some(Target::Button(Button::Minimize)),
        );
        c.button(
            Button::Close,
            "×",
            true,
            pressed == Some(Target::Button(Button::Close)),
        );
        c.button(
            Button::Refresh,
            language.text("ACTUALISER", "REFRESH"),
            refresh_enabled,
            pressed == Some(Target::Button(Button::Refresh)),
        );
        if self.search_mode {
            c.text(
                246,
                37,
                language.text("RECHERCHE", "SEARCH"),
                palette.chrome_text,
                9,
            );
            c.bevel(SEARCH_INPUT, palette.well, true);
            let selected_text = self.cursor.min(self.anchor)..self.cursor.max(self.anchor);
            if self.input_focus && self.focused && !selected_text.is_empty() {
                let begin = self.query[..selected_text.start]
                    .chars()
                    .count()
                    .saturating_sub(self.input_offset)
                    .min(SEARCH_COLUMNS);
                let end = self.query[..selected_text.end]
                    .chars()
                    .count()
                    .saturating_sub(self.input_offset)
                    .min(SEARCH_COLUMNS);
                c.rect(
                    (329 + begin * 8, 34, (end - begin) * 8, 12),
                    palette.selection,
                );
            }
            c.text_skip(
                329,
                37,
                &self.query,
                palette.primary,
                self.input_offset,
                SEARCH_COLUMNS,
            );
            if self.input_focus && self.focused {
                let caret = self.query[..self.cursor]
                    .chars()
                    .count()
                    .saturating_sub(self.input_offset)
                    .min(SEARCH_COLUMNS);
                c.rect((329 + caret * 8, 34, 1, 13), palette.accent);
            }
            c.button(
                Button::Search,
                language.text("CHERCHER", "SEARCH"),
                search_enabled,
                pressed == Some(Target::Button(Button::Search)),
            );
            c.button(
                Button::Clear,
                language.text("EFFACER", "CLEAR"),
                clear_enabled,
                pressed == Some(Target::Button(Button::Clear)),
            );
        } else {
            c.text(
                192,
                37,
                language.text("VOS PLAYLISTS SPOTIFY", "YOUR SPOTIFY PLAYLISTS"),
                palette.chrome_text,
                36,
            );
            c.text(
                574,
                37,
                if s.connected {
                    "SPOTIFY"
                } else {
                    language.text("HORS CONNEXION", "OFFLINE")
                },
                palette.chrome_muted,
                21,
            );
        }
        c.bevel((12, 60, 220, 377), palette.well, true);
        if self.search_mode {
            c.rect((18, 65, 202, 24), palette.selection);
        }
        c.text(24, 73, ">", palette.accent, 1);
        c.text(
            40,
            73,
            language.text("Recherche", "Search"),
            if self.search_mode {
                palette.bright
            } else {
                palette.primary
            },
            22,
        );
        c.bevel((13, 93, 218, 21), palette.header, false);
        c.text(
            23,
            100,
            language.text("MES PLAYLISTS", "MY PLAYLISTS"),
            palette.bright,
            24,
        );
        if self.left_focus && self.focused {
            c.rect((16, 116, 212, 1), palette.accent);
        }
        if s.entries.is_empty() {
            c.text(
                25,
                140,
                if s.loading {
                    language.text("Chargement...", "Loading...")
                } else {
                    language.text("Aucune playlist", "No playlists")
                },
                palette.primary,
                23,
            );
            c.text(
                25,
                159,
                if s.connected {
                    language.text("Actualiser avec F5", "Press F5 to refresh")
                } else {
                    language.text("Connectez le lecteur", "Connect the player")
                },
                palette.muted,
                23,
            );
        }
        for (row, entry) in s
            .entries
            .iter()
            .enumerate()
            .skip(self.left_offset)
            .take(LEFT_ROWS)
        {
            let y = LEFT_TOP + (row - self.left_offset) * 28;
            let active = !self.search_mode && self.selected_id.as_ref() == Some(&entry.id);
            if active {
                c.rect((18, y, 202, 27), palette.selection);
            }
            c.text(
                24,
                y + 4,
                &entry.name,
                if !entry.available {
                    palette.muted
                } else if active {
                    palette.bright
                } else {
                    palette.primary
                },
                24,
            );
            c.text(
                24,
                y + 16,
                if !entry.available {
                    language.text("Indisponible", "Unavailable")
                } else if entry.folder.is_empty() {
                    "Spotify"
                } else {
                    &entry.folder
                },
                palette.muted,
                24,
            );
        }
        c.scrollbar(
            222,
            LEFT_TOP,
            LEFT_ROWS * 28,
            s.entries.len(),
            LEFT_ROWS,
            self.left_offset,
        );
        c.bevel((240, 60, 508, 341), palette.well, true);
        c.bevel((241, 61, 506, 37), palette.header_alt, false);
        c.text(
            252,
            68,
            if self.search_mode {
                language.text("Recherche Spotify", "Spotify search")
            } else {
                selected.map_or(
                    language.text("Sélectionnez une playlist", "Select a playlist"),
                    |e| e.name.as_str(),
                )
            },
            palette.bright,
            60,
        );
        let mut summary = Label::new();
        if self.search_mode {
            if s.search.loading || self.search_pending.is_some() {
                let _ = write!(
                    summary,
                    "{}",
                    language.text("Recherche en cours...", "Searching...")
                );
            } else if !s.search.query.is_empty() {
                let _ = write!(
                    summary,
                    "{} {}",
                    s.search.tracks.len(),
                    language.text("résultats", "results")
                );
            } else {
                let _ = write!(
                    summary,
                    "{}",
                    language.text(
                        "Saisissez un titre ou un artiste.",
                        "Enter a song or artist."
                    )
                );
            }
        } else if let Some(entry) = selected {
            if let Some(count) = entry.track_count {
                let _ = write!(summary, "{count} {}", language.text("titres", "tracks"));
            } else {
                let _ = write!(
                    summary,
                    "{}",
                    language.text("Nombre de titres inconnu", "Track count unknown")
                );
            }
            if let Some(duration) = entry.duration_ms {
                let _ = write!(summary, "  /  {} min", duration / 60_000);
            }
        } else {
            let _ = write!(
                summary,
                "{}",
                language.text(
                    "Les titres sont chargés à l'ouverture.",
                    "Tracks load when a playlist is opened."
                )
            );
        }
        c.text(252, 83, summary.as_str(), palette.muted, 60);
        c.bevel((242, 100, 504, 17), palette.header, false);
        c.text(252, 105, language.text("N°", "No"), palette.bright, 3);
        c.text(
            286,
            105,
            language.text("TITRE / ARTISTE", "TITLE / ARTIST"),
            palette.bright,
            38,
        );
        c.text(
            686,
            105,
            language.text("DURÉE", "LENGTH"),
            palette.bright,
            7,
        );
        if !self.left_focus && !self.input_focus && self.focused {
            c.rect((245, 118, 497, 1), palette.accent);
        }
        if !ready || tracks.is_empty() {
            let message = if self.search_mode {
                if s.search.loading || self.search_pending.is_some() {
                    language.text("Recherche en cours...", "Searching...")
                } else if s.search.query.is_empty() {
                    language.text(
                        "Saisissez un titre ou un artiste.",
                        "Enter a song or artist.",
                    )
                } else {
                    language.text("Aucun résultat.", "No results.")
                }
            } else if s.loading || self.pending_selection.is_some() {
                language.text("Chargement des titres...", "Loading tracks...")
            } else if selected.is_some() {
                language.text("Aucun titre chargé.", "No tracks loaded.")
            } else {
                language.text(
                    "Choisissez une playlist dans la colonne de gauche.",
                    "Choose a playlist in the left column.",
                )
            };
            c.text(260, 151, message, palette.primary, 58);
            c.text(
                260,
                174,
                if self.search_mode {
                    language.text(
                        "Entrée : rechercher    Ctrl+F : recherche",
                        "Enter: search    Ctrl+F: search",
                    )
                } else {
                    language.text(
                        "Entrée : lire    Ctrl+Entrée : ajouter à la file",
                        "Enter: play    Ctrl+Enter: add to queue",
                    )
                },
                palette.muted,
                58,
            );
        } else {
            for (index, track) in tracks.iter().enumerate().skip(offset).take(RIGHT_ROWS) {
                let y = 120 + (index - offset) * 24;
                let active = choices.contains(index);
                if active {
                    c.rect((248, y, 486, 23), palette.selection);
                }
                let mut number = Label::new();
                let _ = write!(number, "{:03}", index + 1);
                c.text(252, y + 8, number.as_str(), palette.muted, 3);
                c.text(
                    286,
                    y + 3,
                    track.display_title(language),
                    if active {
                        palette.bright
                    } else {
                        palette.primary
                    },
                    49,
                );
                c.text(286, y + 14, &track.artist, palette.muted, 49);
                let mut time = Label::new();
                let seconds = track.duration_ms / 1000;
                let _ = write!(time, "{}:{:02}", seconds / 60, seconds % 60);
                c.text(686, y + 8, time.as_str(), palette.primary, 6);
            }
            c.scrollbar(738, 120, 264, tracks.len(), RIGHT_ROWS, offset);
        }
        c.button(
            Button::SendTo,
            language.text("ENVOYER VERS...", "SEND TO..."),
            send_enabled,
            pressed == Some(Target::Button(Button::SendTo)) || self.send_menu.is_some(),
        );
        let mut count = Label::new();
        let _ = write!(count, "{}/{}", choices.len(), tracks.len());
        c.text(674, 421, count.as_str(), palette.chrome_muted, 9);
        c.rect((13, 443, 734, 1), palette.separator_shadow);
        c.rect((13, 444, 734, 1), palette.separator_highlight);
        let status = self.notice.as_ref().map_or(
            if s.playlist_write_busy || !s.playlist_write_status.text(language).is_empty() {
                s.playlist_write_status.text(language)
            } else if self.search_mode {
                s.search.status.text(language)
            } else {
                s.status.text(language)
            },
            |(text, _)| text.text(language),
        );
        c.text(15, 450, status, palette.chrome_text, 91);
        c.text_skip(15, 460, status, palette.chrome_text, 91, 91);
        if let Some(menu) = &self.send_menu {
            menu.render(&mut c, &palette, language, menu_enabled, pressed);
        }
        if let Some(picker) = &self.picker {
            picker.render(
                &mut c,
                palette,
                language,
                pressed,
                [
                    picker_confirm_enabled,
                    picker_new_enabled,
                    picker_create_enabled,
                ],
            );
        }
        true
    }
}

fn inside((x, y, w, h): Rect, px: usize, py: usize) -> bool {
    px >= x && py >= y && px < x + w && py < y + h
}
fn reveal(index: usize, offset: &mut usize, rows: usize) {
    if index < *offset {
        *offset = index;
    } else if index >= *offset + rows {
        *offset = index + 1 - rows;
    }
}
struct Canvas<'a> {
    pixels: &'a mut [u32],
    focused: bool,
    skin: Skin,
}
impl Canvas<'_> {
    fn rect(&mut self, (x, y, w, h): Rect, color: u32) {
        for yy in y.min(HEIGHT)..y.saturating_add(h).min(HEIGHT) {
            self.pixels[yy * WIDTH + x.min(WIDTH)..yy * WIDTH + x.saturating_add(w).min(WIDTH)]
                .fill(color);
        }
    }
    fn bevel(&mut self, r: Rect, fill: u32, inset: bool) {
        if self.skin == Skin::Modern {
            crate::appearance::panel(self.pixels, WIDTH, r, fill, inset);
            return;
        }
        self.rect(r, fill);
        let (x, y, w, h) = r;
        let (light, dark) = if inset {
            (0x11111b, 0x88889f)
        } else {
            (0x88889f, 0x11111b)
        };
        self.rect((x, y, w, 1), light);
        self.rect((x, y, 1, h), light);
        self.rect((x, y + h - 1, w, 1), dark);
        self.rect((x + w - 1, y, 1, h), dark);
    }
    fn text(&mut self, x: usize, y: usize, text: &str, color: u32, limit: usize) {
        self.text_skip(x, y, text, color, 0, limit);
    }
    fn text_skip(&mut self, x: usize, y: usize, text: &str, color: u32, skip: usize, limit: usize) {
        for (index, c) in text.chars().skip(skip).take(limit).enumerate() {
            let glyph = font8x8::BASIC_FONTS
                .get(c)
                .or_else(|| font8x8::LATIN_FONTS.get(c))
                .or_else(|| font8x8::BASIC_FONTS.get('?'));
            if let Some(glyph) = glyph {
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if bits & (1 << col) != 0 {
                            self.rect((x + index * 8 + col, y + row, 1, 1), color);
                        }
                    }
                }
            }
        }
    }
    fn button(&mut self, b: Button, label: &str, enabled: bool, down: bool) {
        let (x, y, w, h) = b.rect();
        if self.skin == Skin::Modern {
            crate::appearance::button(self.pixels, WIDTH, (x, y, w, h), enabled, down);
        } else {
            self.bevel(
                (x, y, w, h),
                if enabled { 0xb6bec8 } else { 0x7a7e90 },
                down,
            );
            self.rect(
                (x + 2, y + 2, w - 4, 2),
                if down {
                    0x646b7d
                } else if enabled {
                    0xd6dee4
                } else {
                    0x8b8fa0
                },
            );
            self.rect((x + 2, y + h - 4, w - 4, 2), 0x747d91);
        }
        let shift = usize::from(down);
        self.text(
            x + w.saturating_sub(label.chars().count() * 8) / 2 + shift,
            y + (h - 8) / 2 + shift,
            label,
            if self.skin == Skin::Modern {
                if matches!(b, Button::Minimize | Button::Close) && !self.focused {
                    0x6a7b8c
                } else if enabled {
                    0x1d4365
                } else {
                    0x5b6c7b
                }
            } else if matches!(b, Button::Minimize | Button::Close) {
                if self.focused { 0x303346 } else { 0x62677a }
            } else if enabled {
                0x242b40
            } else {
                0x51586b
            },
            w / 8,
        );
    }
    fn scrollbar(
        &mut self,
        x: usize,
        y: usize,
        height: usize,
        count: usize,
        rows: usize,
        offset: usize,
    ) {
        if count <= rows {
            return;
        }
        self.rect(
            (x, y, 6, height),
            if self.skin == Skin::Modern {
                0x0b2841
            } else {
                0x171724
            },
        );
        let h = (height * rows / count).max(8);
        let top = offset * (height - h) / (count - rows);
        self.bevel(
            (x + 1, y + top, 4, h),
            if self.skin == Skin::Modern {
                0x8dcbe8
            } else {
                GOLD
            },
            false,
        );
    }
}
// Numeric labels stay on the stack; a repaint never allocates temporary strings.
struct Label {
    bytes: [u8; 128],
    len: usize,
}
impl Label {
    fn new() -> Self {
        Self {
            bytes: [0; 128],
            len: 0,
        }
    }
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}
impl std::fmt::Write for Label {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        if self.len + text.len() > self.bytes.len() {
            return Err(std::fmt::Error);
        }
        self.bytes[self.len..self.len + text.len()].copy_from_slice(text.as_bytes());
        self.len += text.len();
        Ok(())
    }
}
#[cfg(test)]
mod create_playlist_tests;
#[cfg(test)]
mod remove_playlist_tests;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod send_menu_tests;
#[cfg(test)]
mod tests;
