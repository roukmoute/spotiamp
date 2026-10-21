use crate::model::{AudioQuality, Command, Snapshot};
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

pub enum Action {
    Send(Command),
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
    Paste,
    Add,
    Remove,
    Clear,
    Quality96,
    Quality160,
    Quality320,
}

impl Button {
    const TITLEBAR: [Self; 3] = [Self::Help, Self::Minimize, Self::Close];

    const ALL: [Self; 15] = [
        Self::About,
        Self::Library,
        Self::Connection,
        Self::Previous,
        Self::Play,
        Self::Pause,
        Self::Stop,
        Self::Next,
        Self::Paste,
        Self::Add,
        Self::Remove,
        Self::Clear,
        Self::Quality96,
        Self::Quality160,
        Self::Quality320,
    ];

    const MODAL: [Self; 12] = [
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
            Self::Paste => (430, 171, 48, 22),
            Self::Add => (484, 171, 62, 22),
            Self::Remove => (14, 417, 98, 22),
            Self::Clear => (118, 417, 74, 22),
            Self::Quality96 => (350, 417, 60, 21),
            Self::Quality160 => (415, 417, 60, 21),
            Self::Quality320 => (480, 417, 66, 21),
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
            Self::Remove => view.selected.is_some_and(|i| i < s.queue.len()),
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
    pub input: String,
    pub input_active: bool,
    pub select_all: bool,
    pub selected: Option<usize>,
    pub help: bool,
    pub about: Option<AboutTab>,
    pub update: UpdateStatus,
    about_focus: usize,
    focused: bool,
    pressed_button: Option<Button>,
    hovered_button: Option<Button>,
    offset: usize,
    scroll_remainder: f64,
    dragging: u8,
    last_click: Option<(usize, Instant)>,
    transient: Option<(String, Instant)>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            pixels: vec![BG; WIDTH * HEIGHT],
            input: String::new(),
            input_active: false,
            select_all: false,
            selected: None,
            help: false,
            about: None,
            update: UpdateStatus::default(),
            about_focus: 0,
            focused: true,
            pressed_button: None,
            hovered_button: None,
            offset: 0,
            scroll_remainder: 0.0,
            dragging: 0,
            last_click: None,
            transient: None,
        }
    }
}

impl View {
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
        self.about = Some(tab);
        self.about_focus = tab as usize;
        self.help = false;
        self.input_active = false;
        self.select_all = false;
        self.last_click = None;
        self.transient = None;
    }

    pub fn close_about(&mut self) {
        self.cancel_pointer();
        self.about = None;
    }

    pub fn about_key(&mut self, key: minifb::Key, shift: bool) -> Option<Action> {
        use minifb::Key;
        let tab = self.about?;
        self.cancel_pointer();
        match key {
            Key::Escape => self.close_about(),
            Key::F1 => {
                self.about = Some(AboutTab::Shortcuts);
                self.about_focus = 2;
            }
            Key::Tab => {
                let count = if tab == AboutTab::Updates { 10 } else { 8 };
                self.about_focus = (self.about_focus + if shift { count - 1 } else { 1 }) % count;
            }
            Key::Left | Key::Up | Key::Right | Key::Down => {
                let delta = if matches!(key, Key::Left | Key::Up) {
                    AboutTab::ALL.len() - 1
                } else {
                    1
                };
                self.about_focus = (tab as usize + delta) % AboutTab::ALL.len();
                self.about = Some(AboutTab::ALL[self.about_focus]);
            }
            Key::Enter | Key::Space => return self.activate_about_focus(),
            _ => {}
        }
        None
    }

    fn activate_about_focus(&mut self) -> Option<Action> {
        match self.about_focus {
            0..=4 => self.about = Some(AboutTab::ALL[self.about_focus]),
            5 => return Some(Action::OpenProject),
            6 => return Some(Action::CopyAboutInfo),
            7 => self.close_about(),
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
        self.pressed_button = None;
        self.hovered_button = None;
        self.end_drag();
    }

    pub fn pointer_down(&mut self, x: usize, y: usize, s: &Snapshot) -> Option<Action> {
        self.cancel_pointer();
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
            }
            return None;
        }
        self.click(x, y, s)
    }

    pub fn pointer_move(&mut self, position: Option<(usize, usize)>) {
        self.hovered_button =
            position.and_then(|(x, y)| Button::at(x, y, self.about, &self.update));
    }

    pub fn pointer_up(&mut self, position: Option<(usize, usize)>, s: &Snapshot) -> Option<Action> {
        let pressed = self.pressed_button.take();
        self.end_drag();
        self.pointer_move(position);
        if self.focused
            && let Some(button) = pressed.filter(|b| Some(*b) == self.hovered_button)
            && button.enabled(self, s)
        {
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
    pub fn notice(&mut self, text: &str) {
        self.transient = Some((text.into(), Instant::now()));
    }
    pub fn insert_char(&mut self, c: char) {
        if self.about.is_some() {
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
    pub fn scroll(&mut self, y: f32, len: usize) {
        if self.about.is_some() {
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
    pub fn move_selection(&mut self, delta: i32, len: usize) {
        if self.about.is_some() {
            return;
        }
        if len == 0 {
            self.selected = None;
            return;
        }
        let next = self
            .selected
            .map_or(0, |i| (i as i32 + delta).clamp(0, len as i32 - 1) as usize);
        self.selected = Some(next);
        if next < self.offset {
            self.offset = next;
        }
        if next >= self.offset + ROWS {
            self.offset = next + 1 - ROWS;
        }
    }
    pub fn click(&mut self, x: usize, y: usize, s: &Snapshot) -> Option<Action> {
        // Window controls stay usable while the in-skin dialogs are open.
        match Button::at(x, y, self.about, &self.update) {
            Some(Button::Minimize) => return Some(Action::Minimize),
            Some(Button::Close) => return Some(Action::Close),
            Some(Button::Help) => {
                if self.about.is_some() {
                    self.about = Some(AboutTab::Shortcuts);
                    self.about_focus = AboutTab::Shortcuts as usize;
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
            return Some(Action::Send(if s.connected || s.busy {
                Command::Logout
            } else {
                Command::Connect
            }));
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
                self.selected = Some(i);
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
                return self.selected.map(|i| Action::Send(Command::Remove(i)));
            }
            if (118..=191).contains(&x) {
                return Some(Action::Send(Command::Clear));
            }
        }
        None
    }
    pub fn drag(&self, x: usize, _y: usize, s: &Snapshot) -> Option<Action> {
        if self.about.is_some() {
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
    }
    pub fn fingerprint(&self, s: &Snapshot) -> u64 {
        let mut hash = std::hash::DefaultHasher::new();
        s.hash(&mut hash);
        self.input.hash(&mut hash);
        self.update.hash(&mut hash);
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
            self.hovered_button == Some(Button::About),
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
mod transport_tests;

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
