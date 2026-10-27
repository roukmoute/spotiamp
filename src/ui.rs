use crate::appearance::Skin;
use crate::locale::{Language, LanguagePreference};
use crate::localized_text::LocalizedText;
use crate::model::{AudioQuality, Command, Snapshot, TrackRow};
use crate::updater::UpdateStatus;
use font8x8::UnicodeFonts;
use std::{
    fs::File,
    hash::{Hash, Hasher},
    io::Write,
    time::{Duration, Instant},
};

pub const WIDTH: usize = 560;
pub const HEIGHT: usize = 470;
// Logical skin pixels; native backends scale this region for their window system.
pub const TITLEBAR_DRAG_REGION: (usize, usize, usize, usize) = (6, 2, 486, 16);
const BG: u32 = 0x29283d;
const LIGHT: u32 = 0x88889f;
const SHADOW: u32 = 0x11111b;
const GREEN: u32 = 0x5bff35;
const DIM: u32 = 0x697c60;
const WHITE: u32 = 0xc8c9d4;
const BLACK: u32 = 0x050805;
const GOLD: u32 = 0xb3a978;
const ROWS: usize = 8;

const LANGUAGE_CHOICES: [LanguagePreference; 7] = [
    LanguagePreference::Auto,
    LanguagePreference::Specific(Language::French),
    LanguagePreference::Specific(Language::English),
    LanguagePreference::Specific(Language::German),
    LanguagePreference::Specific(Language::Spanish),
    LanguagePreference::Specific(Language::Italian),
    LanguagePreference::Specific(Language::Portuguese),
];

fn language_index(preference: LanguagePreference) -> usize {
    LANGUAGE_CHOICES
        .iter()
        .position(|&value| value == preference)
        .unwrap_or(0)
}

pub enum Action {
    Send(Command),
    QueueToPlaylist { tracks: Vec<TrackRow>, create: bool },
    Paste,
    Submit,
    CopyAboutInfo,
    OpenProject,
    CheckUpdates,
    DownloadUpdate,
    InstallUpdate,
    Minimize,
    Close,
    OpenLibrary,
    SetSkin(Skin),
    SetLanguage(LanguagePreference),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AboutTab {
    Player,
    Credits,
    Shortcuts,
    History,
    Updates,
}

impl AboutTab {
    const ALL: [Self; 5] = [
        Self::Player,
        Self::Credits,
        Self::Shortcuts,
        Self::History,
        Self::Updates,
    ];
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConnectionAction {
    Connect,
    Cancel,
    Logout,
}

impl ConnectionAction {
    fn for_state(s: &Snapshot) -> Self {
        if s.connected {
            Self::Logout
        } else if s.busy {
            Self::Cancel
        } else {
            Self::Connect
        }
    }

    fn label(self, language: Language) -> &'static str {
        match self {
            Self::Connect => language.text("CONNEXION", "CONNECT"),
            Self::Cancel => language.text("ANNULER", "CANCEL"),
            Self::Logout => language.text("DECONNECTER", "DISCONNECT"),
        }
    }

    fn command(self) -> Command {
        match self {
            Self::Connect => Command::Connect,
            Self::Cancel => Command::CancelConnect,
            Self::Logout => Command::Logout,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Button {
    Help,
    Minimize,
    Close,
    About,
    Library,
    AboutTab(AboutTab),
    AboutCloseTop,
    AboutClose,
    AboutCopy,
    AboutProject,
    UpdateCheck,
    UpdateDownload,
    UpdateInstall,
    Connection,
    Previous,
    Play,
    Pause,
    Stop,
    Next,
    Shuffle,
    Repeat,
    Paste,
    Add,
    Remove,
    Clear,
    Quality96,
    Quality160,
    Quality320,
    Skin(Skin),
    Language(LanguagePreference),
}

impl Button {
    const TITLEBAR: [Self; 3] = [Self::Help, Self::Minimize, Self::Close];
    const TRANSPORT: [Self; 5] = [
        Self::Previous,
        Self::Play,
        Self::Pause,
        Self::Stop,
        Self::Next,
    ];

    const ALL: [Self; 19] = [
        Self::Skin(Skin::Classic),
        Self::Skin(Skin::Modern),
        Self::About,
        Self::Library,
        Self::Connection,
        Self::Previous,
        Self::Play,
        Self::Pause,
        Self::Stop,
        Self::Next,
        Self::Shuffle,
        Self::Repeat,
        Self::Paste,
        Self::Add,
        Self::Remove,
        Self::Clear,
        Self::Quality96,
        Self::Quality160,
        Self::Quality320,
    ];

    const MODAL: [Self; 19] = [
        Self::AboutTab(AboutTab::Player),
        Self::AboutTab(AboutTab::Credits),
        Self::AboutTab(AboutTab::Shortcuts),
        Self::AboutTab(AboutTab::History),
        Self::AboutTab(AboutTab::Updates),
        Self::AboutCloseTop,
        Self::AboutClose,
        Self::AboutCopy,
        Self::AboutProject,
        Self::UpdateCheck,
        Self::UpdateDownload,
        Self::UpdateInstall,
        Self::Language(LanguagePreference::Auto),
        Self::Language(LanguagePreference::Specific(Language::French)),
        Self::Language(LanguagePreference::Specific(Language::English)),
        Self::Language(LanguagePreference::Specific(Language::German)),
        Self::Language(LanguagePreference::Specific(Language::Spanish)),
        Self::Language(LanguagePreference::Specific(Language::Italian)),
        Self::Language(LanguagePreference::Specific(Language::Portuguese)),
    ];

    fn rect(self) -> (usize, usize, usize, usize) {
        match self {
            Self::About => (308, 116, 22, 22),
            Self::Library => (448, 145, 100, 16),
            Self::AboutTab(tab) => (40 + tab as usize * 96, 65, 96, 25),
            Self::AboutCloseTop => (505, 33, 20, 18),
            Self::AboutClose => (420, 405, 100, 22),
            Self::AboutCopy => (236, 405, 174, 22),
            Self::AboutProject => (40, 405, 186, 22),
            Self::UpdateCheck => (64, 315, 208, 24),
            Self::UpdateDownload | Self::UpdateInstall => (286, 315, 208, 24),
            Self::Help => (496, 3, 16, 14),
            Self::Minimize => (514, 3, 18, 14),
            Self::Close => (534, 3, 20, 14),
            Self::Connection => (421, 29, 125, 22),
            Self::Previous => (15, 116, 38, 22),
            Self::Play => (57, 116, 38, 22),
            Self::Pause => (99, 116, 38, 22),
            Self::Stop => (141, 116, 38, 22),
            Self::Next => (183, 116, 38, 22),
            Self::Shuffle => (225, 116, 56, 22),
            Self::Repeat => (283, 116, 22, 22),
            Self::Paste => (430, 171, 48, 22),
            Self::Add => (484, 171, 62, 22),
            Self::Remove => (14, 417, 98, 22),
            Self::Clear => (118, 417, 74, 22),
            Self::Quality96 => (350, 417, 60, 21),
            Self::Quality160 => (415, 417, 60, 21),
            Self::Quality320 => (480, 417, 66, 21),
            Self::Skin(Skin::Classic) => (47, 146, 74, 16),
            Self::Skin(Skin::Modern) => (125, 146, 62, 16),
            Self::Language(preference) => {
                let index = language_index(preference);
                if index < 3 {
                    (164 + index * 112, 200, 104, 22)
                } else {
                    (54 + (index - 3) * 112, 228, 104, 22)
                }
            }
        }
    }

    fn at(x: usize, y: usize, about: Option<AboutTab>, update: &UpdateStatus) -> Option<Self> {
        let buttons: &[Self] = if about.is_some() {
            &Self::MODAL
        } else {
            &Self::ALL
        };
        Self::TITLEBAR
            .iter()
            .chain(buttons)
            .copied()
            .find(|button| {
                // Distinct capture identities prevent a Download press from becoming
                // an Install release if the asynchronous update state changes.
                let visible = match button {
                    Self::Language(_) => about == Some(AboutTab::Player),
                    Self::UpdateCheck => about == Some(AboutTab::Updates),
                    Self::UpdateDownload => {
                        about == Some(AboutTab::Updates)
                            && !matches!(update, UpdateStatus::Ready { .. })
                    }
                    Self::UpdateInstall => {
                        about == Some(AboutTab::Updates)
                            && matches!(update, UpdateStatus::Ready { .. })
                    }
                    _ => true,
                };
                let (left, top, width, height) = button.rect();
                visible && (left..left + width).contains(&x) && (top..top + height).contains(&y)
            })
    }

    fn enabled(self, view: &View, s: &Snapshot) -> bool {
        match self {
            Self::Previous | Self::Play | Self::Pause | Self::Stop | Self::Next => s.connected,
            Self::Add => s.connected && !s.busy,
            Self::Remove => view.queue_removal_index(s).is_some(),
            Self::Clear => !s.queue.is_empty(),
            Self::UpdateCheck => view.can_check_update(),
            Self::UpdateDownload => matches!(view.update, UpdateStatus::Available { .. }),
            Self::UpdateInstall => matches!(view.update, UpdateStatus::Ready { .. }),
            _ => true,
        }
    }
}

pub struct View {
    pub pixels: Vec<u32>,
    pub skin: Skin,
    pub language: Language,
    pub language_preference: LanguagePreference,
    pub input: String,
    pub input_active: bool,
    pub select_all: bool,
    pub selected: Option<usize>,
    queue_choices: Option<queue_selection::QueueSelection>,
    queue_menu: Option<queue_menu::QueueMenu>,
    pub help: bool,
    pub about: Option<AboutTab>,
    pub update: UpdateStatus,
    update_indicator_phase: bool,
    about_focus: usize,
    update_check_requested: bool,
    focused: bool,
    pressed_button: Option<Button>,
    pressed_connection: Option<ConnectionAction>,
    hovered_button: Option<Button>,
    offset: usize,
    drop_marker: Option<usize>,
    drop_scroll_tick: Option<Instant>,
    queue_drag: Option<queue_drag::QueueDrag>,
    pending_queue_reorder: Option<queue_drag::PendingQueueReorder>,
    next_queue_reorder_request: u64,
    scroll_remainder: f64,
    observed_playback_revision: u64,
    dragging: u8,
    last_click: Option<(usize, Instant)>,
    transient: Option<(LocalizedText, Instant)>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            pixels: vec![BG; WIDTH * HEIGHT],
            skin: Skin::default(),
            language: Language::French,
            language_preference: LanguagePreference::Auto,
            input: String::new(),
            input_active: false,
            select_all: false,
            selected: None,
            queue_choices: None,
            queue_menu: None,
            help: false,
            about: None,
            update: UpdateStatus::default(),
            update_indicator_phase: false,
            about_focus: 0,
            update_check_requested: false,
            focused: true,
            pressed_button: None,
            pressed_connection: None,
            hovered_button: None,
            offset: 0,
            drop_marker: None,
            drop_scroll_tick: None,
            queue_drag: None,
            pending_queue_reorder: None,
            next_queue_reorder_request: 0,
            scroll_remainder: 0.0,
            observed_playback_revision: 0,
            dragging: 0,
            last_click: None,
            transient: None,
        }
    }
}

impl View {
    /// Center explicit starts once; reveal automatic starts only if offscreen.
    /// Position/metadata updates must leave the user's browsing position alone.
    pub fn sync_playback(&mut self, snapshot: &Snapshot) -> bool {
        self.sync_queue_menu(snapshot);
        self.sync_queue_reorder(snapshot);
        self.sync_queue_selection(snapshot);
        if self.observed_playback_revision == snapshot.playback_revision {
            return false;
        }
        self.observed_playback_revision = snapshot.playback_revision;
        // A context action owns its original occurrence and selection. Audio
        // progression must not move the underlying rows while it is open.
        if self.has_queue_menu() {
            return false;
        }
        let preserve_choices = snapshot.playback_origin == crate::model::PlaybackOrigin::Automatic
            && self
                .queue_choices
                .as_ref()
                .is_some_and(|choices| choices.len() > 1);
        if !preserve_choices {
            self.pending_queue_reorder = None;
        }
        let Some(index) = snapshot
            .current
            .filter(|&index| index < snapshot.queue.len())
        else {
            return false;
        };
        // Row actions and double-clicks must not target the newly followed row.
        // The fixed transport buttons retain their meaning across a track change.
        let transport = self
            .pressed_button
            .filter(|b| Button::TRANSPORT.contains(b));
        let hovered = self.hovered_button;
        self.cancel_pointer();
        if transport.is_some() {
            self.pressed_button = transport;
            self.hovered_button = hovered;
        }
        self.last_click = None;
        self.scroll_remainder = 0.0;
        if !preserve_choices {
            self.choose_queue_row(index, false, false, snapshot);
        }
        if snapshot.playback_origin != crate::model::PlaybackOrigin::Automatic
            || index < self.offset
            || index >= self.offset.saturating_add(ROWS)
        {
            // Normal forward playback moves by half a page when it first
            // leaves the viewport, rather than moving the list on every song.
            self.offset = index
                .saturating_sub(ROWS / 2)
                .min(snapshot.queue.len().saturating_sub(ROWS));
        }
        true
    }

    pub fn set_skin(&mut self, skin: Skin) {
        if self.skin != skin {
            self.cancel_pointer();
            self.last_click = None;
            self.skin = skin;
        }
    }

    /// Change only presentation; queued tracks, input and audio remain intact.
    pub fn set_language(&mut self, preference: LanguagePreference, language: Language) {
        if self.language != language || self.language_preference != preference {
            self.cancel_pointer();
            self.last_click = None;
            self.language = language;
            self.language_preference = preference;
        }
    }

    fn render_skin_selector(&mut self) {
        // Both choices occupy the existing playlist title strip, not a new window.
        self.rect(
            8,
            146,
            182,
            16,
            if self.skin == Skin::Modern {
                0xc3c8d3
            } else {
                BG
            },
        );
        self.tiny_text(
            10,
            151,
            "SKIN",
            if self.skin == Skin::Modern {
                0x303d56
            } else {
                WHITE
            },
        );
        for (skin, label) in [
            (Skin::Classic, self.language.text("CLASSIQUE", "CLASSIC")),
            (Skin::Modern, "MODERN"),
        ] {
            let (x, y, w, h) = Button::Skin(skin).rect();
            let down = self.button_down(x, y);
            self.metal_face(x, y, w, h, true, down || self.skin == skin);
            let d = usize::from(down);
            self.tiny_text(
                x + (w - label.len() * 6) / 2 + d,
                y + 5 + d,
                label,
                0x242b40,
            );
            if self.skin == skin {
                self.rect(
                    x + 5,
                    y + h - 3,
                    w - 10,
                    1,
                    if self.skin == Skin::Modern {
                        0x26518c
                    } else {
                        0x77715c
                    },
                );
            }
        }
    }

    /// Advance the update hint using elapsed time from the existing UI loop.
    /// Only its visible half-cycle affects redraws; no clock or task is owned here.
    pub fn tick_update_indicator(&mut self, elapsed: Duration) {
        self.update_indicator_phase =
            self.update_indicator_visible() && (elapsed.as_millis() / 750).is_multiple_of(2);
    }

    fn update_indicator_visible(&self) -> bool {
        self.about.is_none()
            && !self.help
            && matches!(
                self.update,
                UpdateStatus::Available { .. } | UpdateStatus::Ready { .. }
            )
    }

    fn update_indicator_lit(&self) -> bool {
        self.update_indicator_visible() && self.update_indicator_phase
    }

    fn about_tooltip(&self) -> &'static str {
        match self.update {
            UpdateStatus::Available { .. } => self
                .language
                .text("Mise à jour disponible (F2)", "Update available (F2)"),
            UpdateStatus::Ready { .. } => self.language.text(
                "Mise à jour à installer (F2)",
                "Update ready to install (F2)",
            ),
            UpdateStatus::Downloading { .. } => self
                .language
                .text("Téléchargement en cours (F2)", "Downloading update (F2)"),
            UpdateStatus::Failed(_) => self
                .language
                .text("Échec de mise à jour (F2)", "Update failed (F2)"),
            _ => self
                .language
                .text("À propos de Spotiamp (F2)", "About Spotiamp (F2)"),
        }
    }

    pub fn open_about(&mut self) {
        self.cancel_pointer();
        let tab = if matches!(
            self.update,
            UpdateStatus::Available { .. }
                | UpdateStatus::Downloading { .. }
                | UpdateStatus::Ready { .. }
                | UpdateStatus::Failed(_)
        ) {
            AboutTab::Updates
        } else {
            AboutTab::Player
        };
        self.select_about_tab(tab);
        self.help = false;
        self.input_active = false;
        self.select_all = false;
        self.last_click = None;
        self.transient = None;
    }

    fn select_about_tab(&mut self, tab: AboutTab) {
        if self.about != Some(tab) {
            self.update_check_requested = tab == AboutTab::Updates && self.can_check_on_entry();
        }
        self.about = Some(tab);
        self.about_focus = tab as usize;
    }

    fn can_check_on_entry(&self) -> bool {
        matches!(
            self.update,
            UpdateStatus::Idle | UpdateStatus::Current | UpdateStatus::Failed(_)
        )
    }

    /// Consume the tab-entry request after native input has been processed.
    /// Busy states discard it instead of deferring another check until idle.
    pub fn take_update_check_request(&mut self) -> bool {
        std::mem::take(&mut self.update_check_requested)
            && self.about == Some(AboutTab::Updates)
            && self.can_check_on_entry()
    }

    pub fn close_about(&mut self) {
        self.cancel_pointer();
        self.about = None;
        self.update_check_requested = false;
    }

    pub fn about_key(&mut self, key: minifb::Key, shift: bool) -> Option<Action> {
        use minifb::Key;
        let tab = self.about?;
        self.cancel_pointer();
        match key {
            Key::Escape => self.close_about(),
            Key::F1 => self.select_about_tab(AboutTab::Shortcuts),
            Key::Tab => {
                let order: &[usize] = match tab {
                    AboutTab::Player => &[0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 16, 5, 6, 7],
                    AboutTab::Updates => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                    _ => &[0, 1, 2, 3, 4, 5, 6, 7],
                };
                let index = order
                    .iter()
                    .position(|&focus| focus == self.about_focus)
                    .unwrap_or(0);
                self.about_focus =
                    order[(index + if shift { order.len() - 1 } else { 1 }) % order.len()];
            }
            Key::Left | Key::Up | Key::Right | Key::Down => {
                let delta = if matches!(key, Key::Left | Key::Up) {
                    AboutTab::ALL.len() - 1
                } else {
                    1
                };
                let next = (tab as usize + delta) % AboutTab::ALL.len();
                self.select_about_tab(AboutTab::ALL[next]);
            }
            Key::Enter | Key::Space => return self.activate_about_focus(),
            _ => {}
        }
        None
    }

    fn activate_about_focus(&mut self) -> Option<Action> {
        match self.about_focus {
            0..=4 => self.select_about_tab(AboutTab::ALL[self.about_focus]),
            5 => return Some(Action::OpenProject),
            6 => return Some(Action::CopyAboutInfo),
            7 => self.close_about(),
            10..=16 if self.about == Some(AboutTab::Player) => {
                return Some(Action::SetLanguage(LANGUAGE_CHOICES[self.about_focus - 10]));
            }
            8 if self.about == Some(AboutTab::Updates) && self.can_check_update() => {
                return Some(Action::CheckUpdates);
            }
            9 if self.about == Some(AboutTab::Updates) => match self.update {
                UpdateStatus::Available { .. } => return Some(Action::DownloadUpdate),
                UpdateStatus::Ready { .. } => return Some(Action::InstallUpdate),
                _ => {}
            },
            _ => {}
        }
        None
    }

    fn can_check_update(&self) -> bool {
        matches!(
            self.update,
            UpdateStatus::Idle
                | UpdateStatus::Current
                | UpdateStatus::Available { .. }
                | UpdateStatus::Ready { .. }
                | UpdateStatus::Failed(_)
        )
    }

    pub fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.cancel_pointer();
        }
    }

    pub fn cancel_pointer(&mut self) {
        self.close_queue_menu();
        self.pressed_button = None;
        self.pressed_connection = None;
        self.hovered_button = None;
        self.end_drag();
    }

    pub fn pointer_down(&mut self, x: usize, y: usize, s: &Snapshot) -> Option<Action> {
        self.pointer_down_with_modifiers(x, y, false, false, s)
    }

    pub fn pointer_down_with_modifiers(
        &mut self,
        x: usize,
        y: usize,
        control: bool,
        shift: bool,
        s: &Snapshot,
    ) -> Option<Action> {
        if self.has_queue_menu() {
            self.queue_menu_pointer_down(x, y, s);
            return None;
        }
        self.cancel_pointer();
        self.pending_queue_reorder = None;
        if !self.focused {
            return None;
        }
        let button = Button::at(x, y, self.about, &self.update);
        if self.help && !button.is_some_and(|button| Button::TITLEBAR.contains(&button)) {
            self.help = false;
            return None;
        }
        if let Some(button) = button {
            self.input_active = false;
            self.select_all = false;
            if button.enabled(self, s) {
                self.pressed_button = Some(button);
                self.hovered_button = Some(button);
                if button == Button::Connection {
                    self.pressed_connection = Some(ConnectionAction::for_state(s));
                }
            }
            return None;
        }
        if self.begin_queue_drag(x, y, control, shift, s) {
            return None;
        }
        self.click(x, y, s)
    }

    pub fn transport_pointer_down(&mut self, x: usize, y: usize, s: &Snapshot) {
        if !self.help
            && self.about.is_none()
            && Button::at(x, y, None, &self.update)
                .is_some_and(|button| Button::TRANSPORT.contains(&button))
        {
            self.pointer_down(x, y, s);
        } else {
            self.cancel_pointer();
        }
    }

    pub fn pointer_move(&mut self, position: Option<(usize, usize)>) {
        if self.has_queue_menu() {
            self.queue_menu_pointer_move(position);
            return;
        }
        self.hovered_button =
            position.and_then(|(x, y)| Button::at(x, y, self.about, &self.update));
    }

    pub fn pointer_up(&mut self, position: Option<(usize, usize)>, s: &Snapshot) -> Option<Action> {
        if self.has_queue_menu() {
            return self.queue_menu_pointer_up(position, s);
        }
        if self.queue_drag.is_some() {
            return self.release_queue_drag(position, s);
        }
        let pressed = self.pressed_button.take();
        let connection = self.pressed_connection.take();
        self.end_drag();
        self.pointer_move(position);
        if self.focused
            && let Some(button) = pressed.filter(|b| Some(*b) == self.hovered_button)
            && button.enabled(self, s)
        {
            // Completing startup authentication during a press must not turn
            // an intended Cancel into Logout (which deletes the saved session).
            if button == Button::Connection && connection != Some(ConnectionAction::for_state(s)) {
                return None;
            }
            // Activate the captured control exactly once, on release inside it.
            let (x, y, _, _) = button.rect();
            return self.click(x + 2, y + 2, s);
        }
        None
    }

    fn button_down(&self, x: usize, y: usize) -> bool {
        self.pressed_button.is_some_and(|button| {
            let (left, top, _, _) = button.rect();
            left == x && top == y && self.hovered_button == Some(button)
        })
    }
    pub fn notice(&mut self, text: impl Into<LocalizedText>) {
        self.transient = Some((text.into(), Instant::now()));
    }
    pub fn insert_char(&mut self, c: char) {
        if self.about.is_some() || self.has_queue_menu() {
            return;
        }
        if self.select_all {
            self.input.clear();
            self.select_all = false;
        }
        if self.input.len() < 2048 {
            self.input.push(c);
        }
    }
    /// Position between queue rows, in framebuffer coordinates. A library drop
    /// inserts there without changing the currently playing music.
    pub fn queue_drop_index(&self, position: (f32, f32), len: usize) -> Option<usize> {
        let (x, y) = position;
        if self.about.is_some()
            || self.help
            || self.has_queue_menu()
            || !x.is_finite()
            || !y.is_finite()
            || !(14.0..537.0).contains(&x)
            || !(221.0..413.0).contains(&y)
        {
            return None;
        }
        Some((self.offset + ((y - 221.0 + 12.0) / 24.0) as usize).min(len))
    }

    pub fn show_queue_drop(&mut self, position: Option<(f32, f32)>, len: usize) {
        // The library refreshes its drop target every frame, even with no drag.
        // Do not erase the main queue's own insertion marker.
        if self.queue_drag.is_none() {
            self.update_queue_drop(position, len);
        }
    }

    fn update_queue_drop(&mut self, position: Option<(f32, f32)>, len: usize) {
        self.drop_marker = position.and_then(|point| self.queue_drop_index(point, len));
        let edge = position
            .filter(|_| self.drop_marker.is_some())
            .and_then(|(_, y)| {
                if y < 231.0 {
                    Some(-1)
                } else if y >= 403.0 {
                    Some(1)
                } else {
                    None
                }
            });
        if let Some(direction) = edge {
            let now = Instant::now();
            let previous = self.drop_scroll_tick.get_or_insert(now);
            if previous.elapsed() >= Duration::from_millis(250) {
                self.offset = if direction < 0 {
                    self.offset.saturating_sub(1)
                } else {
                    self.offset.saturating_add(1).min(len.saturating_sub(ROWS))
                };
                self.drop_scroll_tick = Some(now);
                self.drop_marker = position.and_then(|point| self.queue_drop_index(point, len));
            }
        } else {
            self.drop_scroll_tick = None;
        }
    }

    pub fn scroll_at(&mut self, y: f32, position: Option<(usize, usize)>, len: usize) {
        if !self.help
            && position.is_some_and(|(x, y)| (14..546).contains(&x) && (219..414).contains(&y))
        {
            self.scroll(y, len);
        }
    }

    pub fn scroll(&mut self, y: f32, len: usize) {
        if self.about.is_some() || self.has_queue_menu() {
            return;
        }
        let max_offset = len.saturating_sub(ROWS);
        if self.offset > max_offset {
            self.offset = max_offset;
            self.scroll_remainder = 0.0;
        }
        if max_offset == 0 {
            self.scroll_remainder = 0.0;
            return;
        }
        if !y.is_finite() {
            return;
        }
        // Backends report wheel notches (or fractional trackpad movement).
        // Keep fractions across frames instead of rounding each event up.
        self.scroll_remainder += f64::from(y);
        // Backend f32 conversion can leave a complete notch just below an
        // integer (e.g. Win32 deltas 70 + 50). Absorb only that rounding noise.
        let nearest = self.scroll_remainder.round();
        if nearest != 0.0 && (self.scroll_remainder - nearest).abs() < 1e-6 {
            self.scroll_remainder = nearest;
        }
        let lines = self.scroll_remainder.trunc();
        self.scroll_remainder -= lines;
        let step = lines.abs() as usize;
        if lines > 0.0 {
            self.offset = self.offset.saturating_sub(step);
        } else if lines < 0.0 {
            self.offset = self.offset.saturating_add(step).min(max_offset);
        }
        // Scrolling beyond an edge must not delay the next inward movement.
        if (self.offset == 0 && self.scroll_remainder > 0.0)
            || (self.offset == max_offset && self.scroll_remainder < 0.0)
        {
            self.scroll_remainder = 0.0;
        }
    }
    #[cfg(test)]
    pub fn move_selection(&mut self, delta: i32, len: usize) {
        self.cancel_pointer();
        self.pending_queue_reorder = None;
        if self.about.is_some() {
            return;
        }
        if len == 0 {
            self.selected = None;
            self.queue_choices = None;
            return;
        }
        let next = self.selected.map_or(0, |i| {
            (i as i32).saturating_add(delta).clamp(0, len as i32 - 1) as usize
        });
        self.selected = Some(next);
        self.queue_choices = None;
        if next < self.offset {
            self.offset = next;
        }
        if next >= self.offset + ROWS {
            self.offset = next + 1 - ROWS;
        }
    }
    pub fn click(&mut self, x: usize, y: usize, s: &Snapshot) -> Option<Action> {
        if self.has_queue_menu() {
            return None;
        }
        // Window controls stay usable while the in-skin dialogs are open.
        match Button::at(x, y, self.about, &self.update) {
            Some(Button::Minimize) => return Some(Action::Minimize),
            Some(Button::Close) => return Some(Action::Close),
            Some(Button::Skin(skin)) => return Some(Action::SetSkin(skin)),
            Some(Button::Help) => {
                if self.about.is_some() {
                    self.select_about_tab(AboutTab::Shortcuts);
                } else {
                    self.help = !self.help;
                }
                return None;
            }
            _ => {}
        }
        if self.about.is_some() {
            self.about_focus = match Button::at(x, y, self.about, &self.update) {
                Some(Button::AboutTab(tab)) => tab as usize,
                Some(Button::AboutProject) => 5,
                Some(Button::AboutCopy) => 6,
                Some(Button::AboutClose | Button::AboutCloseTop) => 7,
                Some(Button::UpdateCheck) => 8,
                Some(Button::UpdateDownload | Button::UpdateInstall) => 9,
                Some(Button::Language(preference)) => 10 + language_index(preference),
                _ => return None,
            };
            return self.activate_about_focus();
        }
        if Button::at(x, y, None, &self.update) == Some(Button::About) {
            self.open_about();
            return None;
        }
        if self.help {
            self.help = false;
            return None;
        }
        if Button::at(x, y, None, &self.update) == Some(Button::Library) {
            self.cancel_pointer();
            return Some(Action::OpenLibrary);
        }
        self.input_active = (170..195).contains(&y) && (14..422).contains(&x);
        self.select_all = false;
        if (417..438).contains(&y) {
            let quality = match x {
                350..=409 => Some(AudioQuality::Kbps96),
                415..=474 => Some(AudioQuality::Kbps160),
                480..=545 => Some(AudioQuality::Kbps320),
                _ => None,
            };
            if let Some(quality) = quality {
                return Some(Action::Send(Command::SetQuality(quality)));
            }
        }
        if (30..51).contains(&y) && (421..546).contains(&x) {
            return Some(Action::Send(ConnectionAction::for_state(s).command()));
        }
        if (96..106).contains(&y) && (15..545).contains(&x) && s.duration_ms > 0 {
            self.dragging = 1;
            return self.drag(x, y, s);
        }
        if (116..139).contains(&y) {
            let c = match x {
                15..=52 => Some(Command::Previous),
                57..=94 => Some(Command::Play(self.selected.or(s.current).unwrap_or(0))),
                99..=136 => Some(Command::Toggle),
                141..=178 => Some(Command::Stop),
                183..=220 => Some(Command::Next),
                225..=280 => Some(Command::ToggleShuffle),
                283..=304 => Some(Command::ToggleRepeat),
                337..=543 => {
                    self.dragging = 2;
                    return self.drag(x, y, s);
                }
                _ => None,
            };
            return c.map(Action::Send);
        }
        if (170..195).contains(&y) {
            if (430..=477).contains(&x) {
                return Some(Action::Paste);
            }
            if (484..=545).contains(&x) {
                return Some(Action::Submit);
            }
        }
        if (221..413).contains(&y) && (14..535).contains(&x) {
            let i = self.offset + (y - 221) / 24;
            if i < s.queue.len() {
                self.choose_queue_row(i, false, false, s);
                let double = self
                    .last_click
                    .is_some_and(|(old, at)| old == i && at.elapsed() < Duration::from_millis(400));
                self.last_click = Some((i, Instant::now()));
                if double {
                    return Some(Action::Send(Command::PlayIndex(i)));
                }
            }
        }
        if (417..438).contains(&y) {
            if (14..=111).contains(&x) {
                return self
                    .queue_removal_index(s)
                    .map(|i| Action::Send(Command::Remove(i)));
            }
            if (118..=191).contains(&x) {
                return Some(Action::Send(Command::Clear));
            }
        }
        None
    }
    pub fn drag(&self, x: usize, _y: usize, s: &Snapshot) -> Option<Action> {
        if self.about.is_some() || self.has_queue_menu() {
            return None;
        }
        match self.dragging {
            1 => Some(Action::Send(Command::Seek(
                ((x.clamp(15, 545) - 15) as u64 * s.duration_ms as u64 / 530) as u32,
            ))),
            2 => Some(Action::Send(Command::Volume(
                ((x.clamp(337, 543) - 337) * 65535 / 206) as u16,
            ))),
            _ => None,
        }
    }
    pub fn end_drag(&mut self) {
        self.dragging = 0;
        self.cancel_queue_drag();
    }
    pub fn fingerprint(&self, s: &Snapshot) -> u64 {
        let mut hash = std::hash::DefaultHasher::new();
        s.hash(&mut hash);
        self.input.hash(&mut hash);
        self.update.hash(&mut hash);
        self.skin.hash(&mut hash);
        self.drop_marker.hash(&mut hash);
        self.queue_choices.hash(&mut hash);
        self.queue_menu
            .as_ref()
            .map(|menu| (menu.rect, menu.focus, menu.pressed))
            .hash(&mut hash);
        self.language.hash(&mut hash);
        self.language_preference.hash(&mut hash);
        if self.update_indicator_visible() {
            self.update_indicator_phase.hash(&mut hash);
        }
        (
            self.input_active,
            self.select_all,
            self.selected,
            self.help,
            self.offset,
            self.focused,
            self.pressed_button
                .map(|button| (button, self.hovered_button == Some(button))),
            self.dragging,
        )
            .hash(&mut hash);
        (
            self.about,
            self.about_focus,
            self.hovered_button.filter(|button| {
                matches!(button, Button::About | Button::Shuffle | Button::Repeat)
                    && self.about.is_none()
                    && !self.help
            }),
        )
            .hash(&mut hash);
        self.transient
            .as_ref()
            .filter(|(_, t)| t.elapsed() < Duration::from_secs(8))
            .map(|(text, _)| text)
            .hash(&mut hash);
        hash.finish()
    }
}

mod about_skin;
mod modern_skin;
mod queue_drag;
mod queue_menu;
mod queue_selection;
mod skin;

#[cfg(test)]
mod about_tests;

#[cfg(test)]
mod update_tests;

#[cfg(test)]
mod chrome_tests;

#[cfg(test)]
mod scroll_tests;

#[cfg(test)]
mod playback_follow_tests;

#[cfg(test)]
mod transport_tests;

#[cfg(test)]
mod queue_drag_tests;

#[cfg(test)]
mod queue_selection_tests;

#[cfg(test)]
mod queue_menu_tests;

#[cfg(test)]
mod connection_tests;

#[cfg(test)]
mod mode_tests;

#[cfg(test)]
mod skin_tests;

#[cfg(test)]
mod language_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quality_controls_are_available_before_connecting_and_do_not_change_selection() {
        let mut view = View {
            selected: Some(3),
            ..View::default()
        };
        let state = Snapshot::default();
        assert_eq!(state.quality, AudioQuality::Kbps320);
        for (x, expected) in [
            (380, AudioQuality::Kbps96),
            (445, AudioQuality::Kbps160),
            (510, AudioQuality::Kbps320),
        ] {
            assert!(matches!(
                view.click(x, 425, &state),
                Some(Action::Send(Command::SetQuality(quality))) if quality == expected
            ));
            assert_eq!(view.selected, Some(3));
        }
        view.help = true;
        assert!(view.click(510, 425, &state).is_none());
    }
    #[test]
    fn queue_selection_stays_valid_when_scrolling_and_removing() {
        let mut view = View::default();
        view.move_selection(-1, 0);
        assert_eq!(view.selected, None);
        view.move_selection(1, 500);
        assert_eq!(view.selected, Some(0));
        view.move_selection(999, 500);
        assert_eq!(view.selected, Some(499));
        assert_eq!(view.offset, 492);
        view.render(&Snapshot::default());
        assert_eq!(view.selected, None);
        assert_eq!(view.offset, 0);
    }
    #[test]
    fn seeking_and_volume_clamp_to_valid_ranges() {
        let mut view = View::default();
        let state = Snapshot {
            duration_ms: 200_000,
            ..Snapshot::default()
        };
        view.dragging = 1;
        assert!(matches!(
            view.drag(999, 0, &state),
            Some(Action::Send(Command::Seek(200_000)))
        ));
        view.dragging = 2;
        assert!(matches!(
            view.drag(999, 0, &state),
            Some(Action::Send(Command::Volume(65535)))
        ));
    }
}
