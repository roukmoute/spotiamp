use super::*;
use crate::model::LibraryCommand;

fn tap(callback: &mut Callback, key: Key) {
    callback.set_key_state(key, true);
    callback.set_key_state(key, false);
}

fn shortcut(callback: &mut Callback, key: Key) {
    callback.set_key_state(Key::LeftCtrl, true);
    tap(callback, key);
    callback.add_char(key as u32); // Shortcut-generated text must be ignored.
    callback.set_key_state(Key::LeftCtrl, false);
}

fn state() -> LibrarySnapshot {
    LibrarySnapshot {
        connected: true,
        revision: 1,
        ..Default::default()
    }
}

fn queries(actions: &[library_ui::Action]) -> Vec<&str> {
    actions
        .iter()
        .map(|action| match action {
            library_ui::Action::Command(LibraryCommand::Search(query)) => query.as_str(),
            other => panic!("unexpected input action: {other:?}"),
        })
        .collect()
}

#[test]
fn fast_focus_typing_and_enter_submit_in_native_callback_order() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut callback = Callback::new(events.clone());
    shortcut(&mut callback, Key::F);
    for c in "Björk 東京".chars() {
        callback.add_char(c as u32);
    }
    tap(&mut callback, Key::Enter);
    let mut view = library_ui::View::default();
    let mut actions = Vec::new();
    process(
        &mut view,
        &state(),
        true,
        &events.borrow(),
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert_eq!(queries(&actions), ["Björk 東京"]);
}

#[test]
fn paste_then_enter_in_one_frame_uses_pasted_text_and_copy_stays_local() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut callback = Callback::new(events.clone());
    shortcut(&mut callback, Key::F);
    shortcut(&mut callback, Key::V);
    shortcut(&mut callback, Key::A);
    shortcut(&mut callback, Key::C);
    tap(&mut callback, Key::Enter);
    let mut clipboard = "Sigur Rós".to_owned();
    let mut copied = None;
    let mut view = library_ui::View::default();
    let mut actions = Vec::new();
    process(
        &mut view,
        &state(),
        true,
        &events.borrow(),
        &[],
        &mut |text| {
            if let Some(text) = text {
                copied = Some(text.clone());
                clipboard = text;
            }
            Ok(clipboard.clone())
        },
        &mut actions,
    );
    assert_eq!(copied.as_deref(), Some("Sigur Rós"));
    assert_eq!(queries(&actions), ["Sigur Rós"]);
}

#[test]
fn held_backspace_does_not_duplicate_its_callback_edge() {
    let mut view = library_ui::View::default();
    let mut actions = Vec::new();
    process(
        &mut view,
        &state(),
        true,
        &[
            Input::Key(Key::F, true, false),
            Input::Char('a'),
            Input::Char('b'),
            Input::Key(Key::Backspace, false, false),
        ],
        &[(Key::Backspace, true, false, false)],
        &mut |_| Err(()),
        &mut actions,
    );
    process(
        &mut view,
        &state(),
        true,
        &[Input::Key(Key::Enter, false, false)],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert_eq!(queries(&actions), ["a"]);
}

#[test]
fn inactive_library_ignores_text_clipboard_and_submit() {
    let mut view = library_ui::View::default();
    let mut actions = Vec::new();
    process(
        &mut view,
        &state(),
        false,
        &[
            Input::Key(Key::F, true, false),
            Input::Key(Key::V, true, false),
            Input::Char('a'),
            Input::Key(Key::Enter, false, false),
        ],
        &[],
        &mut |_| panic!("inactive library accessed clipboard"),
        &mut actions,
    );
    assert!(actions.is_empty());
    assert!(!view.wants_text_input());
}

#[test]
fn bounded_callback_overflow_discards_the_whole_partial_submission() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut callback = Callback::new(events.clone());
    shortcut(&mut callback, Key::F);
    for _ in 0..MAX_PENDING_INPUTS {
        callback.add_char('x' as u32);
    }
    tap(&mut callback, Key::Enter);
    assert!(matches!(events.borrow().as_slice(), [Input::Overflow]));
    let mut view = library_ui::View::default();
    let mut actions = Vec::new();
    process(
        &mut view,
        &state(),
        true,
        &events.borrow(),
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(actions.is_empty());
    events.borrow_mut().clear();
    shortcut(&mut callback, Key::F);
    callback.add_char('y' as u32);
    tap(&mut callback, Key::Enter);
    process(
        &mut view,
        &state(),
        true,
        &events.borrow(),
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert_eq!(queries(&actions), ["y"]);
}

#[test]
fn close_consumes_remaining_keys_before_any_search_is_sent() {
    let mut view = library_ui::View::default();
    let mut actions = Vec::new();
    process(
        &mut view,
        &state(),
        true,
        &[
            Input::Key(Key::F3, false, false),
            Input::Key(Key::F, true, false),
            Input::Char('a'),
            Input::Key(Key::Enter, false, false),
        ],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(matches!(actions.as_slice(), [library_ui::Action::Close]));
}
