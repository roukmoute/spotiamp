use super::*;

#[test]
fn window_buttons_work_on_release_with_dialogs_open_and_cancel_outside() {
    let state = Snapshot::default();
    for modal in [0, 1, 2] {
        for (button, x) in [(Button::Minimize, 522), (Button::Close, 543)] {
            let mut view = View::default();
            if modal == 1 {
                view.open_about();
            }
            if modal == 2 {
                view.help = true;
            }
            let about = view.about;
            let help = view.help;
            view.render(&state);
            let pixels = view.pixels.clone();
            let buffer = view.pixels.as_ptr();
            assert!(view.pointer_down(x, 9, &state).is_none());
            view.render(&state);
            assert_ne!(view.pixels, pixels, "pressed relief must be visible");
            let action = view.pointer_up(Some((x, 9)), &state);
            assert!(matches!(
                (button, action),
                (Button::Minimize, Some(Action::Minimize)) | (Button::Close, Some(Action::Close))
            ));
            assert!(view.pointer_up(Some((x, 9)), &state).is_none());
            assert_eq!((view.about, view.help), (about, help));
            view.render(&state);
            assert_eq!(view.pixels.as_ptr(), buffer);
            assert_eq!(view.pixels, pixels);
            view.pointer_down(x, 9, &state);
            assert!(view.pointer_up(Some((280, 9)), &state).is_none());
            view.pointer_down(x, 9, &state);
            view.set_focus(false);
            view.set_focus(true);
            assert!(view.pointer_up(Some((x, 9)), &state).is_none());
        }
    }
}

#[test]
fn native_drag_area_never_overlaps_window_buttons_or_dialogs() {
    let (left, top, width, height) = TITLEBAR_DRAG_REGION;
    for y in top..top + height {
        for x in left..left + width {
            assert!(Button::at(x, y, None, &UpdateStatus::Idle).is_none());
            assert!(Button::at(x, y, Some(AboutTab::Updates), &UpdateStatus::Idle).is_none());
        }
    }
    assert!(top + height <= 24);
}

#[test]
fn titlebar_help_opens_shortcuts_in_about_without_exposing_playback() {
    let state = Snapshot::default();
    let mut view = View::default();
    view.open_about();
    assert!(view.pointer_down(503, 9, &state).is_none());
    assert!(view.pointer_up(Some((503, 9)), &state).is_none());
    assert_eq!(view.about, Some(AboutTab::Shortcuts));
    view.close_about();
    view.pointer_down(503, 9, &state);
    view.pointer_up(Some((503, 9)), &state);
    assert!(view.help);
    view.pointer_down(503, 9, &state);
    view.pointer_up(Some((503, 9)), &state);
    assert!(!view.help);
}
