//! Ordered native events: a fast paste/type followed by Enter submits the new text.
use super::*;
use crate::localized_text::LocalizedText;
use minifb::InputCallback;

const MAX_PENDING_INPUTS: usize = 512;

#[derive(Clone, Copy, Debug)]
pub(super) enum Input {
    Key(Key, bool, bool),
    Char(char),
    Mouse(MouseInput),
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
        self.push(Input::Mouse(MouseInput::Button { button, down, x, y }));
    }

    fn mouse_input_cancelled(&mut self) {
        self.push(Input::Mouse(MouseInput::Cancel));
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

pub(super) fn process(
    view: &mut library_ui::View,
    snapshot: &LibrarySnapshot,
    active: bool,
    events: &[Input],
    repeats: &[(Key, bool, bool, bool)],
    clipboard: &mut impl FnMut(Option<String>) -> Result<String, ()>,
    actions: &mut Vec<library_ui::Action>,
) {
    if !active {
        return;
    }
    for &event in events {
        let action = match event {
            Input::Key(Key::F3, _, _) => Some(library_ui::Action::Close),
            Input::Key(key, control, shift) => view.key(key, control, shift, snapshot),
            Input::Char(character) => {
                view.type_char(character);
                None
            }
            Input::Mouse(MouseInput::Button {
                button: MouseButton::Left,
                down,
                x,
                y,
            }) => {
                let position = inside(x, y);
                if down {
                    position.and_then(|(x, y)| view.pointer_down(x, y, snapshot))
                } else {
                    view.pointer_up(position, snapshot)
                }
            }
            Input::Mouse(MouseInput::Cancel) | Input::Overflow => {
                view.cancel_pointer();
                None
            }
            _ => None,
        };
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
