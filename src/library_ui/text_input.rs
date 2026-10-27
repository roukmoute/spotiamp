//! Shared, bounded UTF-8 editing for the search field and playlist name.
use super::*;

pub(super) struct TextInput<'a> {
    pub text: &'a mut String,
    pub cursor: &'a mut usize,
    pub anchor: &'a mut usize,
    pub offset: &'a mut usize,
    pub columns: usize,
}
impl TextInput<'_> {
    fn selection(&self) -> std::ops::Range<usize> {
        (*self.cursor).min(*self.anchor)..(*self.cursor).max(*self.anchor)
    }
    pub fn reveal(&mut self) {
        let column = self.text[..*self.cursor].chars().count();
        if column < *self.offset {
            *self.offset = column;
        } else if column >= *self.offset + self.columns {
            *self.offset = column + 1 - self.columns;
        }
    }
    pub fn insert(&mut self, ch: char, max_chars: usize, max_bytes: usize) -> bool {
        if ch.is_control() {
            return false;
        }
        let selected = self.selection();
        let bytes = self.text.len() - selected.len();
        let chars = self.text.chars().count() - self.text[selected.clone()].chars().count();
        if bytes + ch.len_utf8() > max_bytes || chars >= max_chars {
            return false;
        }
        self.text.replace_range(selected.clone(), "");
        self.text.insert(selected.start, ch);
        *self.cursor = selected.start + ch.len_utf8();
        *self.anchor = *self.cursor;
        self.reveal();
        true
    }
    pub fn position(&mut self, column: usize) {
        *self.cursor = self
            .text
            .char_indices()
            .nth(column + *self.offset)
            .map_or(self.text.len(), |(i, _)| i);
        *self.anchor = *self.cursor;
        self.reveal();
    }
    pub fn key(&mut self, key: Key, control: bool, shift: bool) -> Option<Action> {
        let selected = self.selection();
        match key {
            Key::A if control => {
                *self.anchor = 0;
                *self.cursor = self.text.len();
            }
            Key::V if control => return Some(Action::Paste),
            Key::C | Key::X if control => {
                if selected.is_empty() {
                    return None;
                }
                let copied = self.text[selected.clone()].to_owned();
                if key == Key::X {
                    self.text.replace_range(selected.clone(), "");
                    *self.cursor = selected.start;
                    *self.anchor = *self.cursor;
                    self.reveal();
                }
                return Some(Action::Copy(copied));
            }
            Key::Backspace | Key::Delete => {
                let range = if !selected.is_empty() {
                    selected
                } else if key == Key::Backspace {
                    self.text[..*self.cursor]
                        .char_indices()
                        .next_back()
                        .map_or(*self.cursor, |(i, _)| i)..*self.cursor
                } else {
                    *self.cursor
                        ..self.text[*self.cursor..]
                            .chars()
                            .next()
                            .map_or(*self.cursor, |ch| *self.cursor + ch.len_utf8())
                };
                self.text.replace_range(range.clone(), "");
                *self.cursor = range.start;
                *self.anchor = *self.cursor;
            }
            Key::Left | Key::Right | Key::Home | Key::End => {
                *self.cursor = match key {
                    Key::Home => 0,
                    Key::End => self.text.len(),
                    Key::Left if !shift && !selected.is_empty() => selected.start,
                    Key::Right if !shift && !selected.is_empty() => selected.end,
                    Key::Left => self.text[..*self.cursor]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i),
                    _ => self.text[*self.cursor..]
                        .chars()
                        .next()
                        .map_or(*self.cursor, |ch| *self.cursor + ch.len_utf8()),
                };
                if !shift {
                    *self.anchor = *self.cursor;
                }
            }
            _ => return None,
        }
        self.reveal();
        None
    }
}
