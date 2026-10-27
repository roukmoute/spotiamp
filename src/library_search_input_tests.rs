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

#[test]
fn drag_capture_preserves_selected_rows_and_cancel_never_emits_a_drop() {
    use std::sync::Arc;
    let mut snapshot = state();
    snapshot.search.revision = 1;
    snapshot.search.tracks = Arc::new(
        (0..5)
            .map(|i| crate::model::TrackRow {
                uri: format!("spotify:track:{i:022}"),
                title: format!("Track {i}"),
                ..Default::default()
            })
            .collect(),
    );
    let mut view = library_ui::View::default();
    view.render(&snapshot);
    view.key(Key::F, true, false, &snapshot);
    let mut drag = PointerDrag::default();
    let mut actions = Vec::new();
    let event = |down, y, ctrl| {
        Input::Mouse(
            MouseInput::Button {
                button: MouseButton::Left,
                down,
                x: 320.0,
                y,
            },
            ctrl,
            false,
        )
    };
    process_with_drag(
        &mut view,
        &snapshot,
        true,
        &[
            event(true, 132.0, false),
            event(false, 132.0, false),
            event(true, 180.0, true),
            event(false, 180.0, true),
            event(true, 132.0, false),
        ],
        &[],
        &mut |_| Err(()),
        &mut actions,
        &mut drag,
    );
    assert_eq!(
        drag.tracks.iter().map(|r| &r.uri).collect::<Vec<_>>(),
        vec![
            &snapshot.search.tracks[0].uri,
            &snapshot.search.tracks[2].uri
        ]
    );
    assert!(!drag.active);
    drag.moved(Some((-20.0, 269.0)), &mut view);
    assert!(drag.active);
    // Capture is immutable if the live results change during the gesture.
    snapshot.search.tracks = Arc::new(Vec::new());
    process_with_drag(
        &mut view,
        &snapshot,
        true,
        &[Input::Mouse(
            MouseInput::Button {
                button: MouseButton::Left,
                down: false,
                x: -20.0,
                y: 269.0,
            },
            false,
            false,
        )],
        &[],
        &mut |_| Err(()),
        &mut actions,
        &mut drag,
    );
    assert_eq!(drag.completed.len(), 1);
    assert_eq!(drag.completed[0].0.len(), 2);
    assert!(!drag.active);
    drag.completed.clear();
    drag.origin = Some((1.0, 1.0));
    drag.active = true;
    process_with_drag(
        &mut view,
        &snapshot,
        true,
        &[Input::Mouse(MouseInput::Cancel, false, false)],
        &[],
        &mut |_| Err(()),
        &mut actions,
        &mut drag,
    );
    assert!(drag.completed.is_empty());
    assert!(!drag.active && drag.origin.is_none());
}

#[test]
#[cfg(feature = "spotify")]
fn creation_recovery_matches_dispatch_after_paste_enter_and_later_keys_in_one_batch() {
    let mut snapshot = state();
    let tracks: Vec<_> = (0..3)
        .map(|index| crate::model::TrackRow {
            uri: format!("spotify:track:{index:022}"),
            title: format!("Track {index}"),
            ..Default::default()
        })
        .collect();
    snapshot.search.revision = 1;
    snapshot.search.query = "jazz".into();
    snapshot.search.tracks = std::sync::Arc::new(tracks.clone());
    let mut view = library_ui::View::default();
    view.render(&snapshot);
    view.key(Key::F, true, false, &snapshot);
    for (y, control) in [(132, false), (180, true)] {
        view.pointer_down_with_modifiers(320, y, control, false, &snapshot);
        assert!(view.pointer_up(Some((320, y)), &snapshot).is_none());
    }
    let mouse = |down| {
        Input::Mouse(
            MouseInput::Button {
                button: MouseButton::Left,
                down,
                x: 550.0,
                y: 425.0,
            },
            false,
            false,
        )
    };
    let events = [
        Input::Key(Key::F5, false, false),
        mouse(true),
        mouse(false),
        Input::Key(Key::N, true, false),
        Input::Key(Key::V, true, false),
        Input::Key(Key::Enter, false, false),
        Input::Key(Key::LeftShift, false, true),
        Input::Key(Key::Left, false, false),
    ];
    let mut actions = Vec::new();
    process(
        &mut view,
        &snapshot,
        true,
        &events,
        &[],
        &mut |_| Ok("  Été à São Paulo  ".into()),
        &mut actions,
    );
    assert!(
        matches!(actions.as_slice(), [library_ui::Action::Command(LibraryCommand::Refresh), library_ui::Action::Command(LibraryCommand::CreatePlaylist { name, tracks: chosen })]
        if name == "Été à São Paulo" && chosen == &vec![tracks[0].clone(), tracks[2].clone()])
    );
    assert!(!view.wants_text_input());

    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    sender
        .try_send(crate::model::Command::Library(LibraryCommand::Refresh))
        .unwrap();
    let mut commands = actions.into_iter().map(|action| match action {
        library_ui::Action::Command(command) => command,
        other => panic!("unexpected action {other:?}"),
    });
    // A preceding rejected refresh must not restore a later creation dialog.
    super::super::dispatch_command(&mut view, &sender, commands.next().unwrap());
    assert!(!view.wants_text_input());
    // The exact creation must remain recoverable despite keys after Enter.
    super::super::dispatch_command(&mut view, &sender, commands.next().unwrap());
    assert!(view.wants_text_input());
    let Some(library_ui::Action::Command(retry)) = view.key(Key::Enter, false, false, &snapshot)
    else {
        panic!("rejected creation should preserve its name and selected tracks");
    };
    assert!(
        matches!(&retry, LibraryCommand::CreatePlaylist { name, tracks: chosen }
        if name == "Été à São Paulo" && chosen == &vec![tracks[0].clone(), tracks[2].clone()])
    );
    receiver.try_recv().unwrap();
    super::super::dispatch_command(&mut view, &sender, retry);
    assert!(matches!(
        receiver.try_recv().unwrap(),
        crate::model::Command::Library(LibraryCommand::CreatePlaylist { .. })
    ));
    assert!(!view.wants_text_input());

    // Accepted creation recovery is retired; later errors cannot reopen it.
    drop(receiver);
    super::super::dispatch_command(&mut view, &sender, LibraryCommand::Refresh);
    assert!(!view.wants_text_input());
}
