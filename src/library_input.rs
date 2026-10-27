//! Ordered native events: a fast paste/type followed by Enter submits the new text.
use super::*;
use crate::localized_text::LocalizedText;
use minifb::InputCallback;

const MAX_PENDING_INPUTS: usize = 512;

#[derive(Clone, Copy, Debug)]
pub(super) enum Input {
    Key(Key, bool, bool),
    Char(char),
    Mouse(MouseInput, bool, bool),
    Overflow,
}

pub(super) struct Callback {
    pub(super) events: Rc<RefCell<Vec<Input>>>,
    held: [bool; 512],
    modifiers: u8,
}

impl Callback {
    pub(super) fn new(events: Rc<RefCell<Vec<Input>>>) -> Self {
        Self {
            events,
            held: [false; 512],
            modifiers: 0,
        }
    }

    fn push(&mut self, input: Input) {
        let mut events = self.events.borrow_mut();
        if matches!(events.first(), Some(Input::Overflow)) {
            return;
        }
        if events.len() == MAX_PENDING_INPUTS {
            // Discard the entire batch rather than submit partially captured text.
            events.clear();
            events.push(Input::Overflow);
        } else {
            events.push(input);
        }
    }
}

impl InputCallback for Callback {
    fn add_char(&mut self, code: u32) {
        if self.modifiers & 0x0f == 0
            && let Some(character) = char::from_u32(code).filter(|c| !c.is_control())
        {
            self.push(Input::Char(character));
        }
    }

    fn set_key_state(&mut self, key: Key, down: bool) {
        let bit = match key {
            Key::LeftCtrl => 1,
            Key::RightCtrl => 2,
            Key::LeftSuper => 4,
            Key::RightSuper => 8,
            Key::LeftShift => 16,
            Key::RightShift => 32,
            _ => 0,
        };
        if down {
            self.modifiers |= bit;
        } else {
            self.modifiers &= !bit;
        }
        let index = key as usize;
        if down && !self.held[index] {
            self.push(Input::Key(
                key,
                self.modifiers & 0x0f != 0,
                self.modifiers & 0x30 != 0,
            ));
        }
        self.held[index] = down;
    }

    fn set_mouse_button_state(&mut self, button: MouseButton, down: bool, x: f32, y: f32) {
        self.push(Input::Mouse(
            MouseInput::Button { button, down, x, y },
            self.modifiers & 0x0f != 0,
            self.modifiers & 0x30 != 0,
        ));
    }

    fn mouse_input_cancelled(&mut self) {
        self.push(Input::Mouse(MouseInput::Cancel, false, false));
    }
}

/// Clipboard actions resolve immediately, before a subsequent Enter in this batch.
fn dispatch(
    view: &mut library_ui::View,
    action: library_ui::Action,
    clipboard: &mut impl FnMut(Option<String>) -> Result<String, ()>,
    actions: &mut Vec<library_ui::Action>,
) {
    match action {
        library_ui::Action::Paste => match clipboard(None) {
            Ok(text) => view.paste(&text),
            Err(()) => view.notice(LocalizedText::new(
                "Presse-papiers texte indisponible.",
                "Text clipboard unavailable.",
            )),
        },
        library_ui::Action::Copy(text) => {
            if clipboard(Some(text)).is_err() {
                view.notice(LocalizedText::new(
                    "Copie impossible : presse-papiers indisponible.",
                    "Cannot copy: clipboard unavailable.",
                ));
            }
        }
        other => actions.push(other),
    }
}

#[cfg(test)]
pub(super) fn process(
    view: &mut library_ui::View,
    snapshot: &LibrarySnapshot,
    active: bool,
    events: &[Input],
    repeats: &[(Key, bool, bool, bool)],
    clipboard: &mut impl FnMut(Option<String>) -> Result<String, ()>,
    actions: &mut Vec<library_ui::Action>,
) {
    process_with_drag(
        view,
        snapshot,
        active,
        events,
        repeats,
        clipboard,
        actions,
        &mut PointerDrag::default(),
    );
}

#[derive(Default)]
pub(super) struct PointerDrag {
    origin: Option<(f32, f32)>,
    tracks: Vec<crate::model::TrackRow>,
    playlist: Option<crate::model::PlaylistId>,
    pub(super) active: bool,
    pub(super) completed: Vec<(
        Vec<crate::model::TrackRow>,
        Option<crate::model::PlaylistId>,
        (f32, f32),
    )>,
}

impl PointerDrag {
    pub(super) fn cancel(&mut self) {
        self.origin = None;
        self.tracks.clear();
        self.playlist = None;
        self.active = false;
    }

    pub(super) fn moved(&mut self, position: Option<(f32, f32)>, view: &mut library_ui::View) {
        if !self.active
            && let (Some((x, y)), Some((start_x, start_y))) = (position, self.origin)
            && (x - start_x).abs().max((y - start_y).abs()) >= 5.0
        {
            self.active = true;
            view.cancel_pointer();
        }
    }
}

pub(super) fn process_with_drag(
    view: &mut library_ui::View,
    snapshot: &LibrarySnapshot,
    active: bool,
    events: &[Input],
    repeats: &[(Key, bool, bool, bool)],
    clipboard: &mut impl FnMut(Option<String>) -> Result<String, ()>,
    actions: &mut Vec<library_ui::Action>,
    drag: &mut PointerDrag,
) {
    if !active {
        drag.cancel();
        return;
    }
    for &event in events {
        let overlay_was_open = view.has_overlay();
        if overlay_was_open {
            drag.cancel();
        }
        let action = match event {
            Input::Key(Key::Escape, _, _) if drag.origin.is_some() => {
                drag.cancel();
                view.cancel_pointer();
                None
            }
            Input::Key(Key::F3, _, _) => {
                drag.cancel();
                Some(library_ui::Action::Close)
            }
            Input::Key(key, control, shift) => view.key(key, control, shift, snapshot),
            Input::Char(character) => {
                view.type_char(character);
                None
            }
            Input::Mouse(
                MouseInput::Button {
                    button: MouseButton::Left,
                    down,
                    x,
                    y,
                },
                control,
                shift,
            ) => {
                let position = inside(x, y);
                if down {
                    drag.cancel();
                    let action = position.and_then(|(x, y)| {
                        view.pointer_down_with_modifiers(x, y, control, shift, snapshot)
                    });
                    if let Some(tracks) =
                        position.and_then(|(x, y)| view.selection_drag_at(x, y, snapshot))
                    {
                        drag.origin = Some((x, y));
                        drag.tracks = tracks;
                        drag.playlist = view.selection_origin();
                    }
                    action
                } else {
                    // The release may be the only event after a very fast drag.
                    drag.moved(Some((x, y)), view);
                    if drag.active {
                        drag.completed.push((
                            std::mem::take(&mut drag.tracks),
                            drag.playlist.take(),
                            (x, y),
                        ));
                        drag.cancel();
                        view.cancel_pointer();
                        None
                    } else {
                        drag.cancel();
                        view.pointer_up(position, snapshot)
                    }
                }
            }
            Input::Mouse(MouseInput::Cancel, _, _) | Input::Overflow => {
                drag.cancel();
                view.cancel_pointer();
                None
            }
            _ => None,
        };
        // A keyboard-opened menu can arrive while a row is still pressed.
        // Its captured drag belongs to the covered list, not to the overlay.
        // The event dismissing an overlay must not start a background drag.
        if overlay_was_open || view.has_overlay() {
            drag.cancel();
        }
        if let Some(action) = action {
            let close = matches!(action, library_ui::Action::Close);
            dispatch(view, action, clipboard, actions);
            if close {
                return;
            }
        }
    }
    if matches!(events.first(), Some(Input::Overflow)) {
        return;
    }
    for &(key, repeated, control, shift) in repeats {
        if repeated
            // Repeating Delete edits text only; a Spotify removal always
            // requires a fresh key-down, never the held-key timer.
            && (key != Key::Delete || view.wants_text_input())
            && !events
                .iter()
                .any(|input| matches!(input, Input::Key(candidate, _, _) if *candidate == key))
            && let Some(action) = view.key(key, control, shift, snapshot)
        {
            dispatch(view, action, clipboard, actions);
        }
    }
}

#[cfg(test)]
#[path = "library_search_input_tests.rs"]
mod tests;
