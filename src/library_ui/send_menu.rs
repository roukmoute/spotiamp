//! Transient native bitmap menu. It owns only the explicitly captured selection.
use super::*;

const MENU_RECT: Rect = (246, 265, 330, 144);
const ITEMS: usize = 5;

pub(super) struct SendMenu {
    pub(super) focus: usize,
    pub(super) pointer_position: Option<(usize, usize)>,
    tracks: Vec<TrackRow>,
    indices: Vec<usize>,
    expected_tracks: Arc<Vec<TrackRow>>,
    playlist_revision: Arc<[u8]>,
    origin: Option<PlaylistId>,
    search_revision: Option<u64>,
    whole_playlist: bool,
}

impl SendMenu {
    pub(super) fn item_rect(index: usize) -> Rect {
        (250, 269 + index * 28, 322, 26)
    }

    pub(super) fn target(&self, x: usize, y: usize) -> Target {
        (0..ITEMS)
            .find(|&index| inside(Self::item_rect(index), x, y))
            .map_or(Target::MenuOutside, Target::MenuItem)
    }

    pub(super) fn source_matches(&self, view: &View, s: &LibrarySnapshot) -> bool {
        if !s.connected || !view.active_ready(s) {
            return false;
        }
        match self.search_revision {
            Some(revision) if !view.search_mode || revision != s.search.revision => return false,
            None if view.search_mode
                || !view.selected(s).is_some_and(|entry| entry.available)
                || self.origin != view.selected_id
                || self.origin != s.selected_id
                || self.playlist_revision != s.selected_playlist_revision =>
            {
                return false;
            }
            _ => (),
        }
        let current = if self.search_revision.is_some() {
            &s.search.tracks
        } else {
            &s.tracks
        };
        Arc::ptr_eq(&self.expected_tracks, current)
            || (self.expected_tracks.len() == current.len()
                && self
                    .expected_tracks
                    .iter()
                    .zip(current.iter())
                    .all(|(a, b)| a.uri == b.uri))
    }

    pub(super) fn render(
        &self,
        c: &mut Canvas<'_>,
        palette: &modern::Palette,
        language: Language,
        enabled: [bool; ITEMS],
        pressed: Option<Target>,
    ) {
        c.rect(
            (MENU_RECT.0 + 3, MENU_RECT.1 + 3, MENU_RECT.2, MENU_RECT.3),
            palette.separator_shadow,
        );
        c.bevel(MENU_RECT, palette.background, false);
        let labels = [
            language.text("Lire la sélection", "Play selection"),
            language.text("Ajouter à la file", "Add to queue"),
            language.text("Vers une playlist...", "To a playlist..."),
            language.text("Nouvelle playlist...", "New playlist..."),
            language.text("Retirer de cette playlist", "Remove from this playlist"),
        ];
        for (index, label) in labels.into_iter().enumerate() {
            let rect = Self::item_rect(index);
            if self.focus == index && enabled[index] {
                c.rect(rect, palette.selection);
                if pressed == Some(Target::MenuItem(index)) {
                    c.bevel(rect, palette.selection, true);
                }
            }
            c.text(
                rect.0 + 8,
                rect.1 + 9,
                label,
                if !enabled[index] {
                    palette.chrome_muted
                } else if self.focus == index {
                    palette.bright
                } else {
                    palette.chrome_text
                },
                38,
            );
        }
    }
}

impl View {
    pub fn has_overlay(&self) -> bool {
        self.send_menu.is_some() || self.picker.is_some()
    }

    pub(super) fn open_send_menu(&mut self, s: &LibrarySnapshot) {
        if !self.enabled(Button::SendTo, s) || self.picker.is_some() {
            return;
        }
        let whole_playlist = !self.search_mode && self.left_focus;
        let choices = self.active_choices();
        let source = if self.search_mode {
            &s.search.tracks
        } else {
            &s.tracks
        };
        let indices = choices.indices(source);
        let tracks = choices.tracks(source);
        self.send_menu = Some(SendMenu {
            focus: 0,
            pointer_position: None,
            tracks,
            indices,
            expected_tracks: source.clone(),
            playlist_revision: s.selected_playlist_revision.clone(),
            origin: self.selection_origin(),
            search_revision: self.search_mode.then_some(s.search.revision),
            whole_playlist,
        });
        let focus = (0..ITEMS)
            .find(|&index| self.menu_item_enabled(index, s))
            .unwrap_or(0);
        self.send_menu.as_mut().unwrap().focus = focus;
        self.clear_press();
        self.last_click = None;
        self.search_click = None;
        self.dirty();
    }

    pub(super) fn close_send_menu(&mut self) {
        if self.send_menu.take().is_some() {
            self.clear_press();
            self.dirty();
        }
    }

    pub(super) fn menu_item_enabled(&self, index: usize, s: &LibrarySnapshot) -> bool {
        let Some(menu) = &self.send_menu else {
            return false;
        };
        if !menu.source_matches(self, s) {
            return false;
        }
        match index {
            0 | 1 => !s.busy && (menu.whole_playlist || !menu.tracks.is_empty()),
            2 | 3 => {
                !menu.tracks.is_empty()
                    && !s.playlist_write_busy
                    && self.pending_removal.is_none()
                    && self.submitted_picker.is_none()
            }
            4 => !menu.whole_playlist && !menu.indices.is_empty() && self.can_remove_selected(s),
            _ => false,
        }
    }

    pub(super) fn activate_menu_item(
        &mut self,
        index: usize,
        s: &LibrarySnapshot,
    ) -> Option<Action> {
        if !self.menu_item_enabled(index, s) {
            return None;
        }
        let menu = self.send_menu.take()?;
        self.clear_press();
        self.dirty();
        match index {
            0 | 1 => {
                let play = index == 0;
                Some(Action::Command(if menu.whole_playlist {
                    LibraryCommand::Queue {
                        id: menu.origin?,
                        replace: play,
                        play,
                        start: 0,
                    }
                } else {
                    LibraryCommand::QueueTracks {
                        tracks: menu.tracks,
                        origin: menu.origin,
                        expected_queue: None,
                        replace: play,
                        play,
                        insert_at: None,
                    }
                }))
            }
            2 | 3 => {
                self.open_picker_from_tracks(s, menu.tracks);
                if index == 3 {
                    self.new_playlist();
                }
                None
            }
            4 => self.remove_captured(
                s,
                menu.origin?,
                menu.expected_tracks,
                menu.playlist_revision,
                menu.indices,
            ),
            _ => None,
        }
    }

    pub(super) fn send_menu_key(&mut self, key: Key, s: &LibrarySnapshot) -> Option<Action> {
        match key {
            Key::Escape | Key::Menu | Key::Tab => self.close_send_menu(),
            Key::Enter | Key::Space => {
                return self.activate_menu_item(self.send_menu.as_ref()?.focus, s);
            }
            Key::Up | Key::Down | Key::Home | Key::End => {
                let focus = self.send_menu.as_ref()?.focus;
                let next = match key {
                    Key::Home => (0..ITEMS).find(|&index| self.menu_item_enabled(index, s)),
                    Key::End => (0..ITEMS)
                        .rev()
                        .find(|&index| self.menu_item_enabled(index, s)),
                    _ => (1..=ITEMS)
                        .map(|step| {
                            (focus + if key == Key::Up { ITEMS - step } else { step }) % ITEMS
                        })
                        .find(|&index| self.menu_item_enabled(index, s)),
                };
                if let Some(index) = next {
                    self.send_menu.as_mut()?.focus = index;
                    self.dirty();
                }
            }
            _ => (),
        }
        None
    }
}
