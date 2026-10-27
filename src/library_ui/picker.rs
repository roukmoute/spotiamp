use super::*;
const ROWS: usize = 8;
const LIST: Rect = (192, 142, 376, ROWS * 24);

pub(super) struct PlaylistPicker {
    pub(super) tracks: Vec<TrackRow>,
    pub(super) destinations: Vec<(PlaylistId, String)>,
    pub(super) destination: Option<PlaylistId>,
    offset: usize,
    remainder: f64,
}
impl PlaylistPicker {
    pub(super) fn target(&self, x: usize, y: usize) -> Option<Target> {
        for button in [
            Button::Minimize,
            Button::Close,
            Button::PickerConfirm,
            Button::PickerCancel,
        ] {
            if inside(button.rect(), x, y) {
                return Some(Target::Button(button));
            }
        }
        inside(LIST, x, y).then(|| Target::Destination(self.offset + (y - LIST.1) / 24))
    }
    fn selected(&self) -> Option<usize> {
        self.destinations
            .iter()
            .position(|(id, _)| Some(id) == self.destination.as_ref())
    }
    pub(super) fn choose(&mut self, index: usize) {
        if let Some((id, _)) = self.destinations.get(index) {
            self.destination = Some(id.clone());
            reveal(index, &mut self.offset, ROWS);
        }
    }
    pub(super) fn render(
        &self,
        c: &mut Canvas<'_>,
        palette: modern::Palette,
        language: Language,
        pressed: Option<Target>,
        enabled: bool,
    ) {
        c.rect((180, 89, 408, 302), palette.separator_shadow);
        c.bevel((176, 85, 408, 302), palette.background, false);
        c.bevel((182, 91, 396, 22), palette.header, false);
        c.text(
            192,
            99,
            language.text("AJOUTER À UNE PLAYLIST", "ADD TO A PLAYLIST"),
            palette.bright,
            46,
        );
        let mut caption = Label::new();
        let _ = write!(
            caption,
            "{} {}",
            self.tracks.len(),
            language.text("titres sélectionnés", "selected tracks")
        );
        c.text(192, 124, caption.as_str(), palette.chrome_text, 46);
        c.bevel((190, 140, 380, 196), palette.well, true);
        if self.destinations.is_empty() {
            c.text(
                202,
                166,
                language.text("Aucune playlist disponible.", "No playlists available."),
                palette.primary,
                44,
            );
        }
        for (index, (id, name)) in self
            .destinations
            .iter()
            .enumerate()
            .skip(self.offset)
            .take(ROWS)
        {
            let y = LIST.1 + (index - self.offset) * 24;
            let selected = self.destination.as_ref() == Some(id);
            if selected {
                c.rect((192, y, 366, 23), palette.selection);
            }
            c.text(
                200,
                y + 8,
                name,
                if selected {
                    palette.bright
                } else {
                    palette.primary
                },
                44,
            );
        }
        c.scrollbar(
            560,
            LIST.1,
            LIST.3,
            self.destinations.len(),
            ROWS,
            self.offset,
        );
        c.button(
            Button::PickerConfirm,
            language.text("AJOUTER", "ADD"),
            enabled,
            pressed == Some(Target::Button(Button::PickerConfirm)),
        );
        c.button(
            Button::PickerCancel,
            language.text("ANNULER", "CANCEL"),
            true,
            pressed == Some(Target::Button(Button::PickerCancel)),
        );
    }
}
impl View {
    pub(super) fn open_picker(&mut self, s: &LibrarySnapshot) {
        let tracks = self.selected_tracks(s);
        if tracks.is_empty() || !s.connected || s.playlist_write_busy {
            return;
        }
        let destinations = s
            .entries
            .iter()
            .filter(|entry| entry.available)
            .map(|entry| (entry.id.clone(), entry.name.clone()))
            .collect();
        self.picker = Some(PlaylistPicker {
            tracks,
            destinations,
            destination: None,
            offset: 0,
            remainder: 0.0,
        });
        self.clear_press();
        self.dirty();
    }
    pub(super) fn confirm_picker(&mut self, s: &LibrarySnapshot) -> Option<Action> {
        if !self.enabled(Button::PickerConfirm, s) {
            return None;
        }
        let picker = self.picker.take()?;
        self.dirty();
        Some(Action::Command(LibraryCommand::AddToPlaylist {
            id: picker.destination?,
            tracks: picker.tracks,
        }))
    }
    pub(super) fn picker_key(&mut self, key: Key, s: &LibrarySnapshot) -> Option<Action> {
        match key {
            Key::Escape => {
                self.picker = None;
                self.dirty();
            }
            Key::Enter => return self.confirm_picker(s),
            Key::Up | Key::Down | Key::Home | Key::End | Key::PageUp | Key::PageDown => {
                let picker = self.picker.as_mut()?;
                let count = picker.destinations.len();
                if count == 0 {
                    return None;
                }
                let old = picker.selected();
                let index = match key {
                    Key::Home => 0,
                    Key::End => count - 1,
                    Key::Up => old.unwrap_or(1).saturating_sub(1),
                    Key::Down => old.map_or(0, |i| (i + 1).min(count - 1)),
                    Key::PageUp => old.unwrap_or(0).saturating_sub(ROWS),
                    _ => (old.unwrap_or(0) + ROWS).min(count - 1),
                };
                picker.choose(index);
                self.dirty();
            }
            _ => (),
        }
        None
    }
    pub(super) fn picker_scroll(&mut self, delta: f32, position: Option<(usize, usize)>) {
        let Some(picker) = &mut self.picker else {
            return;
        };
        if !position.is_some_and(|(x, y)| inside(LIST, x, y)) {
            return;
        }
        let max = picker.destinations.len().saturating_sub(ROWS);
        let old = picker.offset;
        picker.remainder += f64::from(delta).clamp(-1000.0, 1000.0);
        let whole = (picker.remainder.abs() + 1e-6)
            .floor()
            .copysign(picker.remainder) as i64;
        picker.remainder -= whole as f64;
        picker.offset = (old as i64 - whole).clamp(0, max as i64) as usize;
        if (picker.offset == 0 && picker.remainder > 0.0)
            || (picker.offset == max && picker.remainder < 0.0)
        {
            picker.remainder = 0.0;
        }
        if old != picker.offset {
            self.cancel_pointer();
            self.dirty();
        }
    }
}
