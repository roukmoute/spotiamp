use super::*;
use minifb::Key;

fn release(view: &mut View, x: usize, y: usize, state: &Snapshot) -> Option<Action> {
    assert!(view.pointer_down(x, y, state).is_none());
    view.pointer_up(Some((x, y)), state)
}

#[test]
fn about_opens_on_release_and_preserves_player_state() {
    let mut view = View {
        input: "spotify:playlist:test".into(),
        selected: Some(12),
        input_active: true,
        offset: 8,
        dragging: 2,
        ..View::default()
    };
    let state = Snapshot::default();
    view.pointer_down(318, 126, &state);
    assert!(view.about.is_none());
    view.pointer_up(Some((318, 126)), &state);
    assert_eq!(view.about, Some(AboutTab::Player));
    assert_eq!(view.input, "spotify:playlist:test");
    assert_eq!(view.selected, Some(12));
    assert_eq!(view.offset, 8);
    assert!(!view.input_active);
    assert_eq!(view.dragging, 0);
    assert!(view.pressed_button.is_none());
    assert!(view.last_click.is_none());
}

#[test]
fn modal_does_not_pass_pointer_wheel_or_text_to_player() {
    let mut view = View {
        input: "keep this".into(),
        selected: Some(12),
        offset: 8,
        ..View::default()
    };
    view.open_about();
    let state = Snapshot {
        connected: true,
        duration_ms: 200_000,
        ..Snapshot::default()
    };
    // Player connection, progress, transport, playlist, quality and empty area.
    for (x, y) in [
        (540, 40),
        (20, 100),
        (20, 125),
        (20, 250),
        (540, 425),
        (4, 4),
    ] {
        assert!(release(&mut view, x, y, &state).is_none());
        assert!(view.about.is_some());
    }
    view.scroll(-8.0, 100);
    view.move_selection(10, 100);
    view.insert_char('x');
    view.dragging = 2; // Even an old capture cannot drive volume behind the modal.
    assert!(view.drag(500, 126, &state).is_none());
    assert_eq!(view.offset, 8);
    assert_eq!(view.selected, Some(12));
    assert_eq!(view.input, "keep this");
}

#[test]
fn modal_buttons_require_release_inside_and_cancel_on_focus_loss() {
    let state = Snapshot::default();
    let mut view = View::default();
    view.open_about();
    view.pointer_down(260, 415, &state);
    assert!(view.button_down(236, 405));
    assert!(view.pointer_up(Some((10, 10)), &state).is_none());
    view.pointer_down(260, 415, &state);
    view.set_focus(false);
    view.set_focus(true);
    assert!(view.pointer_up(Some((260, 415)), &state).is_none());
    assert!(matches!(
        release(&mut view, 260, 415, &state),
        Some(Action::CopyAboutInfo)
    ));
    assert!(matches!(
        release(&mut view, 100, 415, &state),
        Some(Action::OpenProject)
    ));
    assert!(release(&mut view, 450, 415, &state).is_none());
    assert!(view.about.is_none());
    view.open_about();
    release(&mut view, 513, 40, &state);
    assert!(view.about.is_none());
}

#[test]
fn all_tabs_work_with_mouse_and_keyboard_and_focus_wraps() {
    let state = Snapshot::default();
    let mut view = View::default();
    view.open_about();
    for (index, tab) in AboutTab::ALL.into_iter().enumerate() {
        release(&mut view, 55 + index * 96, 76, &state);
        assert_eq!(view.about, Some(tab));
        assert_eq!(view.about_focus, index);
    }
    view.about_key(Key::Right, false);
    assert_eq!(view.about, Some(AboutTab::Player));
    view.about_key(Key::Left, false);
    assert_eq!(view.about, Some(AboutTab::Updates));
    view.about_key(Key::F1, false);
    assert_eq!(view.about, Some(AboutTab::Shortcuts));
    view.open_about();
    view.about_key(Key::Tab, true);
    assert_eq!(view.about_focus, 7);
    view.about_key(Key::Tab, false);
    assert_eq!(view.about_focus, 0);
    for _ in 0..6 {
        view.about_key(Key::Tab, false);
    }
    assert!(matches!(
        view.about_key(Key::Space, false),
        Some(Action::CopyAboutInfo)
    ));
    view.about_key(Key::Tab, false);
    view.about_key(Key::Enter, false);
    assert!(view.about.is_none());
}

#[test]
fn about_reuses_framebuffer_and_redraws_only_on_visible_changes() {
    let state = Snapshot::default();
    let mut view = View::default();
    view.render(&state);
    let buffer = view.pixels.as_ptr();
    let base = view.fingerprint(&state);
    view.pointer_move(Some((318, 126)));
    assert_ne!(base, view.fingerprint(&state)); // Tooltip appears.
    view.open_about();
    let opened = view.fingerprint(&state);
    assert_ne!(base, opened);
    for tab in AboutTab::ALL {
        view.about = Some(tab);
        view.render(&state);
        assert_eq!(buffer, view.pixels.as_ptr());
        assert_eq!(view.pixels.len(), WIDTH * HEIGHT);
    }
    view.about = Some(AboutTab::Player);
    assert_eq!(opened, view.fingerprint(&state));
    view.about_key(Key::Tab, false);
    assert_ne!(opened, view.fingerprint(&state)); // Keyboard focus is visible.
}
