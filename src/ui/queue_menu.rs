//! A transient software-drawn menu; captures allocate only when explicitly opened.
use super::*;
use minifb::Key;

const MENU_WIDTH: usize = 330;
const MENU_HEIGHT: usize = 144;
const ITEMS: usize = 5;
type Rect = (usize, usize, usize, usize);

pub(super) struct QueueMenu {
    pub(super) rect: Rect,
    pub(super) focus: usize,
    pub(super) pressed: Option<usize>,
    pointer_position: Option<(usize, usize)>,
    expected_queue: Vec<String>,
    reorder_request: u64,
    index: usize,
    indices: Vec<usize>,
    tracks: Vec<TrackRow>,
}

fn inside(rect: Rect, x: usize, y: usize) -> bool {
    (rect.0..rect.0 + rect.2).contains(&x) && (rect.1..rect.1 + rect.3).contains(&y)
}

impl QueueMenu {
    fn item_rect(&self, index: usize) -> Rect {
        (
            self.rect.0 + 4,
            self.rect.1 + 4 + index * 28,
            MENU_WIDTH - 8,
            26,
        )
    }

    fn item_at(&self, x: usize, y: usize) -> Option<usize> {
        (0..ITEMS).find(|&index| inside(self.item_rect(index), x, y))
    }

    fn source_matches(&self, s: &Snapshot) -> bool {
        self.reorder_request == s.queue_reorder_request
            && self.expected_queue.len() == s.queue.len()
            && self
                .expected_queue
                .iter()
                .zip(&s.queue)
                .all(|(uri, row)| uri == &row.uri)
    }

    fn enabled(&self, index: usize, s: &Snapshot) -> bool {
        !s.busy
            && !self.indices.is_empty()
            && match index {
                0..=2 => s.connected,
                3 => true,
                4 => self.indices.len() < s.queue.len(),
                _ => false,
            }
    }
}

impl View {
    pub fn has_queue_menu(&self) -> bool {
        self.queue_menu.is_some()
    }

    pub fn close_queue_menu(&mut self) {
        if self.queue_menu.take().is_some() {
            self.last_click = None;
        }
    }

    pub(super) fn sync_queue_menu(&mut self, s: &Snapshot) {
        if self.queue_menu.as_ref().is_some_and(|menu| {
            !self.focused || self.about.is_some() || self.help || !menu.source_matches(s)
        }) {
            self.close_queue_menu();
        }
    }

    /// A second context press is consumed by the existing menu, never by a
    /// covered row. Native right releases must remain inert in the input layer.
    pub fn context_menu_at(&mut self, x: usize, y: usize, s: &Snapshot) {
        let had_menu = self.has_queue_menu();
        self.sync_queue_menu(s);
        if had_menu {
            if let Some(menu) = &mut self.queue_menu {
                menu.pressed = None;
            }
            if self
                .queue_menu
                .as_ref()
                .is_some_and(|menu| !inside(menu.rect, x, y))
            {
                self.close_queue_menu();
            }
            return;
        }
        self.cancel_pointer();
        self.pending_queue_reorder = None;
        self.last_click = None;
        if !self.focused || self.about.is_some() || self.help {
            return;
        }
        let Some(index) = self.queue_row_at((x, y), s.queue.len()) else {
            return;
        };
        self.sync_queue_selection(s);
        if self.queue_row_selected(index) {
            self.selected = Some(index);
        } else {
            self.choose_queue_row(index, false, false, s);
        }
        self.open_queue_menu(index, x, y, s);
    }

    fn open_queue_menu(&mut self, index: usize, x: usize, y: usize, s: &Snapshot) {
        let indices = self.selected_queue_indices(s);
        if indices.is_empty() {
            return;
        }
        self.input_active = false;
        self.select_all = false;
        let mut menu = QueueMenu {
            rect: (
                x.clamp(4, WIDTH - MENU_WIDTH - 4),
                y.clamp(4, HEIGHT - MENU_HEIGHT - 4),
                MENU_WIDTH,
                MENU_HEIGHT,
            ),
            focus: 0,
            pressed: None,
            pointer_position: Some((x, y)),
            expected_queue: s.queue.iter().map(|row| row.uri.clone()).collect(),
            reorder_request: s.queue_reorder_request,
            index,
            tracks: indices
                .iter()
                .map(|&index| s.queue[index].clone())
                .collect(),
            indices,
        };
        menu.focus = (0..ITEMS)
            .find(|&index| menu.enabled(index, s))
            .unwrap_or(0);
        self.queue_menu = Some(menu);
    }

    pub(super) fn queue_menu_pointer_down(&mut self, x: usize, y: usize, s: &Snapshot) {
        self.sync_queue_menu(s);
        let Some(menu) = &mut self.queue_menu else {
            return;
        };
        if !inside(menu.rect, x, y) {
            self.close_queue_menu();
            return;
        }
        menu.pointer_position = Some((x, y));
        menu.pressed = menu.item_at(x, y).filter(|&index| menu.enabled(index, s));
        if let Some(index) = menu.item_at(x, y) {
            menu.focus = index;
        }
    }

    pub(super) fn queue_menu_pointer_move(&mut self, position: Option<(usize, usize)>) {
        let Some(menu) = &mut self.queue_menu else {
            return;
        };
        // Polling a stationary mouse must not erase arrow-key navigation.
        if position != menu.pointer_position {
            menu.pointer_position = position;
            if let Some(index) = position.and_then(|(x, y)| menu.item_at(x, y)) {
                menu.focus = index;
            }
        }
    }

    pub(super) fn queue_menu_pointer_up(
        &mut self,
        position: Option<(usize, usize)>,
        s: &Snapshot,
    ) -> Option<Action> {
        self.sync_queue_menu(s);
        let menu = self.queue_menu.as_mut()?;
        let pressed = menu.pressed.take()?;
        if position.and_then(|(x, y)| menu.item_at(x, y)) == Some(pressed) {
            return self.activate_queue_menu(pressed, s);
        }
        None
    }

    fn activate_queue_menu(&mut self, index: usize, s: &Snapshot) -> Option<Action> {
        self.sync_queue_menu(s);
        if !self.queue_menu.as_ref()?.enabled(index, s) {
            return None;
        }
        let menu = self.queue_menu.take()?;
        self.last_click = None;
        match index {
            0 => Some(Action::Send(Command::PlayQueueIndex {
                expected_queue: menu.expected_queue,
                expected_reorder_request: menu.reorder_request,
                index: menu.index,
            })),
            1 | 2 => Some(Action::QueueToPlaylist {
                tracks: menu.tracks,
                create: index == 2,
            }),
            3 | 4 => Some(Action::Send(Command::EditQueue {
                expected_queue: menu.expected_queue,
                expected_reorder_request: menu.reorder_request,
                indices: menu.indices,
                keep_selected: index == 4,
            })),
            _ => None,
        }
    }

    /// The caller consumes the complete key event whenever a menu was already
    /// open, even if this method closes a now-stale capture without an action.
    pub fn queue_menu_key(
        &mut self,
        key: Key,
        control: bool,
        shift: bool,
        s: &Snapshot,
    ) -> Option<Action> {
        let had_menu = self.has_queue_menu();
        self.sync_queue_menu(s);
        if !had_menu {
            if control
                || !(key == Key::Menu || (key == Key::F10 && shift))
                || !self.focused
                || self.about.is_some()
                || self.help
                || self.input_active
            {
                return None;
            }
            self.cancel_pointer();
            self.pending_queue_reorder = None;
            self.last_click = None;
            self.sync_queue_selection(s);
            let chosen = self.selected_queue_indices(s);
            let Some(index) = self
                .selected
                .filter(|index| chosen.contains(index))
                .or_else(|| chosen.first().copied())
                .or_else(|| s.current.filter(|&index| index < s.queue.len()))
                .or_else(|| (!s.queue.is_empty()).then_some(0))
            else {
                return None;
            };
            if !self.queue_row_selected(index) {
                self.choose_queue_row(index, false, false, s);
            }
            self.selected = Some(index);
            if index < self.offset {
                self.offset = index;
            } else if index >= self.offset + ROWS {
                self.offset = index + 1 - ROWS;
            }
            self.open_queue_menu(index, 30, 221 + (index - self.offset) * 24 + 12, s);
            return None;
        }
        let menu = self.queue_menu.as_mut()?;
        menu.pressed = None;
        match key {
            Key::Escape | Key::Menu | Key::Tab => self.close_queue_menu(),
            Key::F10 if shift => self.close_queue_menu(),
            Key::Enter | Key::Space => {
                let index = menu.focus;
                return self.activate_queue_menu(index, s);
            }
            Key::Up | Key::Down | Key::Home | Key::End => {
                let next = match key {
                    Key::Home => (0..ITEMS).find(|&index| menu.enabled(index, s)),
                    Key::End => (0..ITEMS).rev().find(|&index| menu.enabled(index, s)),
                    _ => (1..=ITEMS)
                        .map(|step| {
                            (menu.focus + if key == Key::Up { ITEMS - step } else { step }) % ITEMS
                        })
                        .find(|&index| menu.enabled(index, s)),
                };
                if let Some(index) = next {
                    menu.focus = index;
                }
            }
            _ => (),
        }
        None
    }

    pub(super) fn render_queue_menu(&mut self, s: &Snapshot) {
        let Some(menu) = &self.queue_menu else { return };
        let rect = menu.rect;
        let focus = menu.focus;
        let pressed = menu.pressed;
        let enabled: [bool; ITEMS] = std::array::from_fn(|index| menu.enabled(index, s));
        let (background, selection, text, muted, shadow) = if self.skin == Skin::Modern {
            (0xc3ceda, 0x225372, 0x1d4365, 0x40566b, 0x7f95ab)
        } else {
            (BG, 0x252d52, GOLD, DIM, SHADOW)
        };
        self.rect(rect.0 + 3, rect.1 + 3, rect.2, rect.3, shadow);
        self.bevel(rect.0, rect.1, rect.2, rect.3, background, false);
        let labels = [
            self.language.text("Lire ce titre", "Play this track"),
            self.language
                .text("Vers une playlist...", "To a playlist..."),
            self.language
                .text("Nouvelle playlist...", "New playlist..."),
            self.language
                .text("Retirer de la file", "Remove from queue"),
            self.language
                .text("Garder uniquement la sélection", "Keep only selection"),
        ];
        for (index, label) in labels.into_iter().enumerate() {
            let x = rect.0 + 4;
            let y = rect.1 + 4 + index * 28;
            if focus == index && enabled[index] {
                self.rect(x, y, MENU_WIDTH - 8, 26, selection);
                if pressed == Some(index) {
                    self.bevel(x, y, MENU_WIDTH - 8, 26, selection, true);
                }
            }
            self.text(
                x + 8,
                y + 9,
                label,
                if !enabled[index] {
                    muted
                } else if focus == index {
                    0xffffff
                } else {
                    text
                },
                38,
            );
        }
    }
}
