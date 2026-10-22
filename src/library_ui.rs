//! A native bitmap library. The framebuffer exists only while its window is open.
use crate::model::{LibraryCommand, LibrarySnapshot, PlaylistEntry, PlaylistId};
use font8x8::UnicodeFonts;
use minifb::Key;
use std::{
    fmt::Write,
    time::{Duration, Instant},
};

pub const WIDTH: usize = 760;
pub const HEIGHT: usize = 470;
pub const TITLEBAR_DRAG_REGION: (usize, usize, usize, usize) = (6, 2, 690, 16);
const LEFT_ROWS: usize = 12;
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Button {
    Minimize,
    Close,
    Refresh,
    Play,
    Append,
}
impl Button {
    fn rect(self) -> Rect {
        match self {
            Self::Minimize => (704, 3, 20, 14),
            Self::Close => (728, 3, 20, 14),
            Self::Refresh => (12, 29, 164, 23),
            Self::Play => (246, 413, 106, 24),
            Self::Append => (360, 413, 128, 24),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Button(Button),
    Playlist(usize),
    Track(usize),
}

pub struct View {
    pub pixels: Vec<u32>,
    selected_id: Option<PlaylistId>,
    pending_selection: Option<PlaylistId>,
    observed_id: Option<PlaylistId>,
    selected_track: Option<usize>,
    selected_uri: Option<String>,
    left_focus: bool,
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
    notice: Option<(String, Instant)>,
    ui_revision: u64,
    snapshot_revision: Option<u64>,
    rendered: Option<(u64, u64)>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            pixels: vec![BG; WIDTH * HEIGHT],
            selected_id: None,
            pending_selection: None,
            observed_id: None,
            selected_track: None,
            selected_uri: None,
            left_focus: true,
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
    pub fn notice(&mut self, text: &str) {
        self.notice = Some((text.into(), Instant::now()));
        self.dirty();
    }
    pub fn set_focus(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.dirty();
        }
        if !focused {
            self.cancel_pointer();
            self.last_click = None;
        }
    }
    pub fn cancel_pointer(&mut self) {
        // Win32 also reports capture loss after a normal button release.
        // Only an interrupted press should break the double-click sequence.
        if self.pressed.is_some() {
            self.last_click = None;
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
    fn sync(&mut self, s: &LibrarySnapshot) {
        if self.snapshot_revision == Some(s.revision) {
            return;
        }
        let initial = self.snapshot_revision.is_none();
        self.snapshot_revision = Some(s.revision);
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
                    self.selected_uri = None;
                    self.right_offset = 0;
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
            self.selected_uri = None;
            self.right_offset = 0;
        }
        if self.selected_id.is_some() && self.selected(s).is_none() {
            self.selected_id = None;
            self.pending_selection = None;
            self.selected_track = None;
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
            self.selected_uri = None;
            self.right_offset = 0;
        }
    }
    fn enabled(&self, b: Button, s: &LibrarySnapshot) -> bool {
        match b {
            Button::Minimize | Button::Close => true,
            Button::Refresh => s.connected && !s.busy && !s.loading,
            Button::Play | Button::Append => {
                s.connected
                    && !s.busy
                    && self.tracks_ready(s)
                    && !s.tracks.is_empty()
                    && self.selected(s).is_some_and(|e| e.available)
            }
        }
    }
    fn target(&self, x: usize, y: usize) -> Option<Target> {
        for b in [
            Button::Minimize,
            Button::Close,
            Button::Refresh,
            Button::Play,
            Button::Append,
        ] {
            if inside(b.rect(), x, y) {
                return Some(Target::Button(b));
            }
        }
        if inside((18, 91, 202, LEFT_ROWS * 28), x, y) {
            return Some(Target::Playlist(self.left_offset + (y - 91) / 28));
        }
        if inside((252, 120, 480, RIGHT_ROWS * 24), x, y) {
            return Some(Target::Track(self.right_offset + (y - 120) / 24));
        }
        None
    }
    pub fn pointer_down(&mut self, x: usize, y: usize, s: &LibrarySnapshot) -> Option<Action> {
        self.sync(s);
        self.clear_press();
        if !self.focused {
            return None;
        }
        let target = self.target(x, y);
        let allowed = match target {
            Some(Target::Button(b)) => self.enabled(b, s),
            Some(Target::Playlist(i)) => s.entries.get(i).is_some_and(|e| e.available),
            Some(Target::Track(i)) => self.tracks_ready(s) && i < s.tracks.len(),
            None => false,
        };
        if allowed {
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
        }
        None
    }
    pub fn pointer_move(&mut self, position: Option<(usize, usize)>) {
        let target = position.and_then(|(x, y)| self.target(x, y));
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
            return None;
        }
        let target = pressed?;
        if self.press_revision != s.revision
            && !matches!(
                target,
                Target::Playlist(_) | Target::Button(Button::Close | Button::Minimize)
            )
        {
            self.last_click = None;
            return None;
        }
        match target {
            Target::Button(b) if self.enabled(b, s) => self.button(b, s),
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
            Target::Track(index) if self.tracks_ready(s) && index < s.tracks.len() => {
                self.left_focus = false;
                self.select_track(index, s);
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
        self.last_click = None;
        if self.selected_id.as_ref() == Some(&entry.id) {
            return None;
        }
        self.selected_id = Some(entry.id.clone());
        self.pending_selection = Some(entry.id.clone());
        self.selected_track = None;
        self.selected_uri = None;
        self.right_offset = 0;
        self.right_remainder = 0.0;
        reveal(index, &mut self.left_offset, LEFT_ROWS);
        Some(Action::Command(LibraryCommand::Select(entry.id.clone())))
    }
    fn select_track(&mut self, index: usize, s: &LibrarySnapshot) {
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
        Some(Action::Command(LibraryCommand::Queue {
            id: self.selected_id.clone()?,
            replace: play,
            play,
            start: if play {
                self.selected_track.unwrap_or(0)
            } else {
                0
            },
        }))
    }
    fn button(&mut self, button: Button, s: &LibrarySnapshot) -> Option<Action> {
        self.last_click = None;
        match button {
            Button::Minimize => Some(Action::Minimize),
            Button::Close => Some(Action::Close),
            Button::Refresh => Some(Action::Command(LibraryCommand::Refresh)),
            Button::Play => self.queue(s, true),
            Button::Append => self.queue(s, false),
        }
    }
    pub fn key(
        &mut self,
        key: Key,
        control: bool,
        _shift: bool,
        s: &LibrarySnapshot,
    ) -> Option<Action> {
        self.sync(s);
        self.cancel_pointer();
        self.last_click = None;
        if !self.focused {
            return None;
        }
        match key {
            Key::Escape => return Some(Action::Close),
            Key::F5 if self.enabled(Button::Refresh, s) => return self.button(Button::Refresh, s),
            Key::Tab => {
                self.left_focus = !self.left_focus;
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
        if matches!(
            key,
            Key::Up | Key::Down | Key::Home | Key::End | Key::PageUp | Key::PageDown
        ) {
            let length = if self.left_focus {
                s.entries.len()
            } else if self.tracks_ready(s) {
                s.tracks.len()
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
                self.selected_track
            };
            let page = if self.left_focus {
                LEFT_ROWS
            } else {
                RIGHT_ROWS
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
            self.select_track(index, s);
            return None;
        }
        if key == Key::Enter {
            return self.queue(s, !control);
        }
        None
    }
    pub fn scroll(&mut self, y: f32, position: Option<(usize, usize)>, s: &LibrarySnapshot) {
        self.sync(s);
        if !self.focused || !y.is_finite() {
            return;
        }
        let left = position.map_or(self.left_focus, |(x, _)| x < 238);
        let len = if left {
            s.entries.len()
        } else if self.tracks_ready(s) {
            s.tracks.len()
        } else {
            0
        };
        let (offset, remainder, rows) = if left {
            (&mut self.left_offset, &mut self.left_remainder, LEFT_ROWS)
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
        let ready = self.tracks_ready(s);
        let enabled = [
            self.enabled(Button::Refresh, s),
            self.enabled(Button::Play, s),
            self.enabled(Button::Append, s),
        ];
        let pressed = self.pressed.filter(|p| Some(*p) == self.hovered);
        let mut c = Canvas {
            pixels: &mut self.pixels,
        };
        c.pixels.fill(BG);
        c.bevel((0, 0, WIDTH, HEIGHT), BG, false);
        c.bevel((3, 3, WIDTH - 6, HEIGHT - 6), BG, true);
        c.bevel((6, 4, 688, 14), 0x343344, true);
        for y in [7, 10, 13] {
            c.rect((12, y, 208, 1), GOLD);
            c.rect((480, y, 208, 1), GOLD);
        }
        c.text(232, 7, "SPOTIAMP  /  BIBLIOTHÈQUE", WHITE, 30);
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
            "ACTUALISER",
            enabled[0],
            pressed == Some(Target::Button(Button::Refresh)),
        );
        c.text(192, 37, "VOS PLAYLISTS SPOTIFY", GOLD, 36);
        c.text(
            574,
            37,
            if s.connected {
                "LECTURE SEULE"
            } else {
                "HORS CONNEXION"
            },
            DIM,
            21,
        );
        c.bevel((12, 60, 220, 377), BLACK, true);
        c.bevel((13, 61, 218, 23), 0x454360, false);
        c.text(23, 69, "MES PLAYLISTS", WHITE, 24);
        if self.left_focus && self.focused {
            c.rect((16, 86, 212, 1), GOLD);
        }
        if s.entries.is_empty() {
            c.text(
                25,
                112,
                if s.loading {
                    "Chargement..."
                } else {
                    "Aucune playlist"
                },
                GREEN,
                23,
            );
            c.text(
                25,
                131,
                if s.connected {
                    "Actualiser avec F5"
                } else {
                    "Connectez le lecteur"
                },
                DIM,
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
            let y = 91 + (row - self.left_offset) * 28;
            let active = self.selected_id.as_ref() == Some(&entry.id);
            if active {
                c.rect((18, y, 202, 27), 0x252d52);
            }
            c.text(
                24,
                y + 4,
                &entry.name,
                if !entry.available {
                    DIM
                } else if active {
                    WHITE
                } else {
                    GREEN
                },
                24,
            );
            c.text(
                24,
                y + 16,
                if !entry.available {
                    "Indisponible"
                } else if entry.folder.is_empty() {
                    "Spotify"
                } else {
                    &entry.folder
                },
                DIM,
                24,
            );
        }
        c.scrollbar(222, 91, 336, s.entries.len(), LEFT_ROWS, self.left_offset);
        c.bevel((240, 60, 508, 341), BLACK, true);
        c.bevel((241, 61, 506, 37), 0x343344, false);
        c.text(
            252,
            68,
            selected.map_or("Sélectionnez une playlist", |e| e.name.as_str()),
            WHITE,
            60,
        );
        let mut summary = Label::new();
        if let Some(entry) = selected {
            if let Some(count) = entry.track_count {
                let _ = write!(summary, "{count} titres");
            } else {
                let _ = write!(summary, "Nombre de titres inconnu");
            }
            if let Some(duration) = entry.duration_ms {
                let _ = write!(summary, "  /  {} min", duration / 60_000);
            }
        } else {
            let _ = write!(summary, "Les titres sont chargés à l'ouverture.");
        }
        c.text(252, 83, summary.as_str(), DIM, 60);
        c.bevel((242, 100, 504, 17), 0x454360, false);
        c.text(252, 105, "N°", WHITE, 3);
        c.text(286, 105, "TITRE / ARTISTE", WHITE, 38);
        c.text(686, 105, "DURÉE", WHITE, 7);
        if !self.left_focus && self.focused {
            c.rect((245, 118, 497, 1), GOLD);
        }
        if !ready || s.tracks.is_empty() {
            let message = if s.loading || self.pending_selection.is_some() {
                "Chargement des titres..."
            } else if selected.is_some() {
                "Aucun titre chargé."
            } else {
                "Choisissez une playlist dans la colonne de gauche."
            };
            c.text(260, 151, message, GREEN, 58);
            c.text(
                260,
                174,
                "Entrée : lire    Ctrl+Entrée : ajouter à la file",
                DIM,
                58,
            );
        } else {
            for (index, track) in s
                .tracks
                .iter()
                .enumerate()
                .skip(self.right_offset)
                .take(RIGHT_ROWS)
            {
                let y = 120 + (index - self.right_offset) * 24;
                let active = self.selected_track == Some(index);
                if active {
                    c.rect((248, y, 486, 23), 0x252d52);
                }
                let mut number = Label::new();
                let _ = write!(number, "{:03}", index + 1);
                c.text(252, y + 8, number.as_str(), DIM, 3);
                c.text(
                    286,
                    y + 3,
                    &track.title,
                    if active { WHITE } else { GREEN },
                    49,
                );
                c.text(286, y + 14, &track.artist, DIM, 49);
                let mut time = Label::new();
                let seconds = track.duration_ms / 1000;
                let _ = write!(time, "{}:{:02}", seconds / 60, seconds % 60);
                c.text(686, y + 8, time.as_str(), GREEN, 6);
            }
            c.scrollbar(738, 120, 264, s.tracks.len(), RIGHT_ROWS, self.right_offset);
        }
        c.button(
            Button::Play,
            "LIRE",
            enabled[1],
            pressed == Some(Target::Button(Button::Play)),
        );
        c.button(
            Button::Append,
            "À LA FILE",
            enabled[2],
            pressed == Some(Target::Button(Button::Append)),
        );
        c.text(506, 416, "Double-clic : lire", DIM, 30);
        let mut queue = Label::new();
        let _ = write!(queue, "FILE : {} TITRES", s.queue_len);
        c.text(506, 428, queue.as_str(), DIM, 30);
        c.rect((13, 443, 734, 1), 0x11111b);
        c.rect((13, 444, 734, 1), 0x656174);
        let status = self
            .notice
            .as_ref()
            .map_or(s.status.as_str(), |(text, _)| text.as_str());
        c.text(15, 450, status, GOLD, 91);
        c.text_skip(15, 460, status, GOLD, 91, 91);
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
}
impl Canvas<'_> {
    fn rect(&mut self, (x, y, w, h): Rect, color: u32) {
        for yy in y.min(HEIGHT)..y.saturating_add(h).min(HEIGHT) {
            self.pixels[yy * WIDTH + x.min(WIDTH)..yy * WIDTH + x.saturating_add(w).min(WIDTH)]
                .fill(color);
        }
    }
    fn bevel(&mut self, r: Rect, fill: u32, inset: bool) {
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
        let shift = usize::from(down);
        self.text(
            x + w.saturating_sub(label.chars().count() * 8) / 2 + shift,
            y + (h - 8) / 2 + shift,
            label,
            if enabled { 0x242b40 } else { 0x51586b },
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
        self.rect((x, y, 6, height), 0x171724);
        let h = (height * rows / count).max(8);
        let top = offset * (height - h) / (count - rows);
        self.bevel((x + 1, y + top, 4, h), GOLD, false);
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
mod tests;
