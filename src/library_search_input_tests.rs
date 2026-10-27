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

fn mouse(down: bool, x: f32, y: f32) -> Input {
    Input::Mouse(
        MouseInput::Button {
            button: MouseButton::Left,
            down,
            x,
            y,
        },
        false,
        false,
    )
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
    let events = [
        Input::Key(Key::F5, false, false),
        mouse(true, 330.0, 425.0),
        mouse(false, 330.0, 425.0),
        mouse(true, 390.0, 366.0),
        mouse(false, 390.0, 366.0),
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

fn owned_playlist() -> (library_ui::View, LibrarySnapshot) {
    use crate::model::{PlaylistEntry, PlaylistId, TrackRow};
    use std::sync::Arc;
    let id = PlaylistId::Spotify("owned-playlist".into());
    let snapshot = LibrarySnapshot {
        selected_id: Some(id.clone()),
        selected_owned: true,
        selected_playlist_revision: Arc::from([1, 2, 3]),
        entries: Arc::new(vec![PlaylistEntry {
            id,
            name: "My playlist".into(),
            available: true,
            track_count: Some(3),
            duration_ms: None,
            folder: String::new(),
        }]),
        tracks: Arc::new(
            (0..3)
                .map(|index| TrackRow {
                    uri: format!("spotify:track:{index:022}"),
                    ..Default::default()
                })
                .collect(),
        ),
        ..state()
    };
    let mut view = library_ui::View::default();
    view.render(&snapshot);
    view.pointer_down(320, 132, &snapshot);
    assert!(view.pointer_up(Some((320, 132)), &snapshot).is_none());
    (view, snapshot)
}

#[test]
fn outside_menu_click_is_consumed_without_selecting_or_dragging_the_covered_list() {
    let (mut view, snapshot) = owned_playlist();
    let mut actions = Vec::new();
    let mut drag = PointerDrag::default();
    process_with_drag(
        &mut view,
        &snapshot,
        true,
        &[
            mouse(true, 330.0, 425.0),
            mouse(false, 330.0, 425.0),
            mouse(true, 320.0, 156.0),
            mouse(false, -30.0, 269.0),
        ],
        &[],
        &mut |_| panic!("menu accessed clipboard"),
        &mut actions,
        &mut drag,
    );
    assert!(actions.is_empty());
    assert!(!view.has_overlay());
    assert_eq!(
        view.selected_tracks(&snapshot),
        vec![snapshot.tracks[0].clone()]
    );
    assert!(drag.origin.is_none() && !drag.active && drag.completed.is_empty());
}

#[test]
fn menu_consumes_background_shortcuts_text_clipboard_and_escape_without_actions() {
    let (mut view, snapshot) = owned_playlist();
    let mut actions = Vec::new();
    process(
        &mut view,
        &snapshot,
        true,
        &[
            Input::Key(Key::Menu, false, false),
            Input::Key(Key::F, true, false),
            Input::Key(Key::V, true, false),
            Input::Char('x'),
            Input::Key(Key::Delete, false, false),
            Input::Key(Key::F5, false, false),
        ],
        &[],
        &mut |_| panic!("menu accessed clipboard"),
        &mut actions,
    );
    assert!(actions.is_empty());
    assert!(view.has_overlay() && !view.wants_text_input());
    assert_eq!(
        view.selected_tracks(&snapshot),
        vec![snapshot.tracks[0].clone()]
    );
    process(
        &mut view,
        &snapshot,
        true,
        &[Input::Key(Key::Escape, false, false)],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(!view.has_overlay() && actions.is_empty());
    process(
        &mut view,
        &snapshot,
        true,
        &[
            Input::Key(Key::Menu, false, false),
            Input::Key(Key::F3, false, false),
            Input::Key(Key::Enter, false, false),
        ],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(matches!(actions.as_slice(), [library_ui::Action::Close]));
}

#[test]
fn keyboard_opened_menu_cancels_a_row_drag_before_an_outside_release() {
    let (mut view, snapshot) = owned_playlist();
    let mut actions = Vec::new();
    let mut drag = PointerDrag::default();
    process_with_drag(
        &mut view,
        &snapshot,
        true,
        &[mouse(true, 320.0, 132.0), Input::Key(Key::F10, false, true)],
        &[],
        &mut |_| Err(()),
        &mut actions,
        &mut drag,
    );
    assert!(view.has_overlay());
    assert!(drag.origin.is_none() && !drag.active);
    drag.moved(Some((-30.0, 269.0)), &mut view);
    process_with_drag(
        &mut view,
        &snapshot,
        true,
        &[mouse(false, -30.0, 269.0)],
        &[],
        &mut |_| Err(()),
        &mut actions,
        &mut drag,
    );
    assert!(actions.is_empty() && drag.completed.is_empty());
}

#[test]
fn stale_menu_activation_is_consumed_when_the_source_playlist_changes() {
    let (mut view, mut snapshot) = owned_playlist();
    view.pointer_down_with_modifiers(320, 180, true, false, &snapshot);
    view.pointer_up(Some((320, 180)), &snapshot);
    let mut actions = Vec::new();
    process(
        &mut view,
        &snapshot,
        true,
        &[Input::Key(Key::Menu, false, false)],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(view.has_overlay());
    snapshot.revision += 1;
    snapshot.selected_playlist_revision = std::sync::Arc::from([4, 5, 6]);
    std::sync::Arc::make_mut(&mut snapshot.tracks).reverse();
    process(
        &mut view,
        &snapshot,
        true,
        &[mouse(true, 390.0, 366.0), mouse(false, 390.0, 366.0)],
        &[],
        &mut |_| panic!("stale menu accessed clipboard"),
        &mut actions,
    );
    assert!(actions.is_empty() && !view.has_overlay());
    assert!(!view.wants_text_input());
}

#[test]
fn menu_removal_is_consumed_once_and_focus_loss_cancels_a_pressed_item() {
    let (mut view, snapshot) = owned_playlist();
    let mut actions = Vec::new();
    process(
        &mut view,
        &snapshot,
        true,
        &[
            Input::Key(Key::Menu, false, false),
            mouse(true, 390.0, 394.0),
        ],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    view.set_focus(false);
    process(
        &mut view,
        &snapshot,
        false,
        &[mouse(false, 390.0, 394.0)],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(actions.is_empty() && !view.has_overlay());
    view.set_focus(true);
    process(
        &mut view,
        &snapshot,
        true,
        &[
            Input::Key(Key::Menu, false, false),
            mouse(true, 390.0, 394.0),
            mouse(false, 390.0, 394.0),
            Input::Key(Key::Delete, false, false),
        ],
        &[(Key::Delete, true, false, false)],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(
        matches!(actions.as_slice(), [library_ui::Action::Command(LibraryCommand::RemoveFromPlaylist { indices, expected_tracks, expected_revision, .. })]
        if indices == &[0] && std::sync::Arc::ptr_eq(expected_tracks, &snapshot.tracks) && expected_revision == &snapshot.selected_playlist_revision)
    );
}

#[test]
fn repeated_delete_in_one_batch_or_native_held_timer_never_removes_another_playlist_row() {
    let (mut view, mut snapshot) = owned_playlist();
    let mut actions = Vec::new();
    process(
        &mut view,
        &snapshot,
        true,
        &[
            Input::Key(Key::Delete, false, false),
            Input::Key(Key::Delete, false, false),
        ],
        &[(Key::Delete, true, false, false)],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(
        matches!(actions.as_slice(), [library_ui::Action::Command(LibraryCommand::RemoveFromPlaylist { indices, .. })] if indices == &[0])
    );
    actions.clear();
    // Even unrelated metadata changes must not release the local pending lock.
    snapshot.revision += 1;
    process(
        &mut view,
        &snapshot,
        true,
        &[Input::Key(Key::Delete, false, false)],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(actions.is_empty());

    // Model a completed operation and a fresh user selection while Delete is
    // still physically held: only its next fresh key-down can remove anything.
    snapshot.revision += 1;
    snapshot.playlist_remove_revision += 1;
    snapshot.selected_playlist_revision = std::sync::Arc::from([4, 5, 6]);
    std::sync::Arc::make_mut(&mut snapshot.tracks).remove(0);
    view.render(&snapshot);
    view.pointer_down(320, 132, &snapshot);
    view.pointer_up(Some((320, 132)), &snapshot);
    process(
        &mut view,
        &snapshot,
        true,
        &[],
        &[(Key::Delete, true, false, false)],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(actions.is_empty());
    process(
        &mut view,
        &snapshot,
        true,
        &[Input::Key(Key::Delete, false, false)],
        &[],
        &mut |_| Err(()),
        &mut actions,
    );
    assert!(
        matches!(actions.as_slice(), [library_ui::Action::Command(LibraryCommand::RemoveFromPlaylist { indices, .. })] if indices == &[0])
    );
}

#[test]
fn repeated_delete_still_edits_text_without_issuing_a_playlist_command() {
    let (mut view, snapshot) = owned_playlist();
    view.key(Key::F, true, false, &snapshot);
    view.paste("Jazz");
    view.key(Key::Home, false, false, &snapshot);
    let mut actions = Vec::new();
    process(
        &mut view,
        &snapshot,
        true,
        &[],
        &[(Key::Delete, true, false, false)],
        &mut |_| Err(()),
        &mut actions,
    );
    let mut copied = None;
    process(
        &mut view,
        &snapshot,
        true,
        &[
            Input::Key(Key::A, true, false),
            Input::Key(Key::C, true, false),
        ],
        &[],
        &mut |text| {
            copied = text;
            Ok(String::new())
        },
        &mut actions,
    );
    assert_eq!(copied.as_deref(), Some("azz"));
    assert!(actions.is_empty());
}
