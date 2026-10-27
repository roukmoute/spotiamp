use super::*;
const ROWS: usize = 8;
const LIST: Rect = (192, 142, 376, ROWS * 24);
const NAME_INPUT: Rect = (192, 166, 376, 24);
const NAME_COLUMNS: usize = 45;

pub(super) struct PlaylistPicker {
    pub(super) tracks: Vec<TrackRow>,
    pub(super) destinations: Vec<(PlaylistId, String)>,
    pub(super) destination: Option<PlaylistId>,
    offset: usize,
    remainder: f64,
    pub(super) creating: bool,
    pub(super) name: String,
    name_cursor: usize,
    name_anchor: usize,
    name_offset: usize,
}
impl PlaylistPicker {
    pub(super) fn target(&self, x: usize, y: usize) -> Option<Target> {
        let buttons = if self.creating {
            [
                Button::Minimize,
                Button::Close,
                Button::PickerCreate,
                Button::PickerCancel,
            ]
        } else {
            [
                Button::Minimize,
                Button::Close,
                Button::PickerConfirm,
                Button::PickerCancel,
            ]
        };
        for button in buttons {
            if inside(button.rect(), x, y) {
                return Some(Target::Button(button));
            }
        }
        if self.creating {
            return inside(NAME_INPUT, x, y).then_some(Target::PlaylistName);
        }
        if inside(Button::PickerNew.rect(), x, y) {
            return Some(Target::Button(Button::PickerNew));
        }
        inside(LIST, x, y).then(|| Target::Destination(self.offset + (y - LIST.1) / 24))
    }
    fn name_input(&mut self) -> TextInput<'_> {
        TextInput {
            text: &mut self.name,
            cursor: &mut self.name_cursor,
            anchor: &mut self.name_anchor,
            offset: &mut self.name_offset,
            columns: NAME_COLUMNS,
        }
    }
    pub(super) fn type_char(&mut self, ch: char) -> bool {
        // Keep overlong input visible and invalid instead of silently creating
        // a different, truncated name. The shared editor still caps memory.
        self.creating
            && self
                .name_input()
                .insert(ch, SEARCH_QUERY_MAX_CHARS, SEARCH_QUERY_MAX_BYTES)
    }
    pub(super) fn position_name(&mut self, x: usize) {
        self.name_input()
            .position(x.saturating_sub(NAME_INPUT.0 + 5) / 8);
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
        enabled: [bool; 3],
    ) {
        c.rect((180, 89, 408, 302), palette.separator_shadow);
        c.bevel((176, 85, 408, 302), palette.background, false);
        c.bevel((182, 91, 396, 22), palette.header, false);
        c.text(
            192,
            99,
            if self.creating {
                language.text("NOUVELLE PLAYLIST", "NEW PLAYLIST")
            } else {
                language.text("AJOUTER À UNE PLAYLIST", "ADD TO A PLAYLIST")
            },
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
        if self.creating {
            self.render_name(c, palette, language);
        } else {
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
                Button::PickerNew,
                language.text("NOUVELLE...", "NEW..."),
                enabled[1],
                pressed == Some(Target::Button(Button::PickerNew)),
            );
        }
        let confirm = if self.creating {
            Button::PickerCreate
        } else {
            Button::PickerConfirm
        };
        c.button(
            confirm,
            if self.creating {
                language.text("CRÉER", "CREATE")
            } else {
                language.text("AJOUTER", "ADD")
            },
            if self.creating {
                enabled[2]
            } else {
                enabled[0]
            },
            pressed == Some(Target::Button(confirm)),
        );
        c.button(
            Button::PickerCancel,
            if self.creating {
                language.text("RETOUR", "BACK")
            } else {
                language.text("ANNULER", "CANCEL")
            },
            true,
            pressed == Some(Target::Button(Button::PickerCancel)),
        );
    }
    fn render_name(&self, c: &mut Canvas<'_>, palette: modern::Palette, language: Language) {
        c.text(
            192,
            149,
            language.text("Nom de la playlist", "Playlist name"),
            palette.chrome_text,
            46,
        );
        c.bevel(NAME_INPUT, palette.well, true);
        let selected =
            self.name_cursor.min(self.name_anchor)..self.name_cursor.max(self.name_anchor);
        if c.focused && !selected.is_empty() {
            let start = self.name[..selected.start]
                .chars()
                .count()
                .saturating_sub(self.name_offset)
                .min(NAME_COLUMNS);
            let end = self.name[..selected.end]
                .chars()
                .count()
                .saturating_sub(self.name_offset)
                .min(NAME_COLUMNS);
            c.rect(
                (197 + start * 8, 171, (end - start) * 8, 13),
                palette.selection,
            );
        }
        c.text_skip(
            197,
            174,
            &self.name,
            palette.primary,
            self.name_offset,
            NAME_COLUMNS,
        );
        if c.focused {
            let caret = self.name[..self.name_cursor]
                .chars()
                .count()
                .saturating_sub(self.name_offset)
                .min(NAME_COLUMNS);
            c.rect((197 + caret * 8, 171, 1, 13), palette.accent);
        }
        c.text(
            192,
            201,
            language.text("100 caractères maximum.", "100 characters maximum."),
            palette.chrome_muted,
            46,
        );
        c.bevel((192, 224, 376, 101), palette.well, true);
        for (index, row) in self.tracks.iter().take(4).enumerate() {
            c.text(
                200,
                235 + index * 22,
                row.display_title(language),
                palette.primary,
                44,
            );
        }
    }
}
impl View {
    #[cfg(test)]
    pub(super) fn open_picker(&mut self, s: &LibrarySnapshot) {
        self.open_picker_from_tracks(s, self.selected_tracks(s));
    }
    pub(super) fn open_picker_from_tracks(&mut self, s: &LibrarySnapshot, tracks: Vec<TrackRow>) {
        if tracks.is_empty()
            || !s.connected
            || s.playlist_write_busy
            || self.submitted_picker.is_some()
            || self.pending_removal.is_some()
        {
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
            creating: false,
            name: String::new(),
            name_cursor: 0,
            name_anchor: 0,
            name_offset: 0,
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
    pub(super) fn new_playlist(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.creating = true;
            picker.name_anchor = 0;
            picker.name_cursor = picker.name.len();
            picker.name_input().reveal();
            self.clear_press();
            self.dirty();
        }
    }
    pub(super) fn cancel_picker(&mut self) {
        if let Some(picker) = &mut self.picker {
            if picker.creating {
                picker.creating = false;
            } else {
                self.picker = None;
            }
            self.clear_press();
            self.dirty();
        }
    }
    pub(super) fn create_playlist(&mut self, s: &LibrarySnapshot) -> Option<Action> {
        if !self.enabled(Button::PickerCreate, s) {
            return None;
        }
        let picker = self.picker.take()?;
        let action = Action::Command(LibraryCommand::CreatePlaylist {
            name: normalized_playlist_name(&picker.name)?.to_owned(),
            tracks: picker.tracks.clone(),
        });
        // The native command channel may reject a full queue. Preserve the
        // captured selection and typed name for that immediate retry path.
        self.submitted_picker = Some(picker);
        self.dirty();
        Some(action)
    }
    pub(super) fn picker_key(
        &mut self,
        key: Key,
        control: bool,
        shift: bool,
        s: &LibrarySnapshot,
    ) -> Option<Action> {
        if self.picker.as_ref().is_some_and(|picker| picker.creating) {
            return match key {
                Key::Escape => {
                    self.cancel_picker();
                    None
                }
                Key::Enter => self.create_playlist(s),
                _ => {
                    let action = self.picker.as_mut()?.name_input().key(key, control, shift);
                    self.dirty();
                    action
                }
            };
        }
        match key {
            Key::N if control && self.enabled(Button::PickerNew, s) => self.new_playlist(),
            Key::Escape => {
                self.cancel_picker();
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
        if picker.creating || !position.is_some_and(|(x, y)| inside(LIST, x, y)) {
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
