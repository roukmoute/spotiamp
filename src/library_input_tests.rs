use super::*;

#[cfg(feature = "spotify")]
#[test]
fn closing_library_retries_a_full_channel_until_the_worker_can_release_it() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tx.try_send(Command::Stop).unwrap();
    let engine = engine::Engine {
        tx,
        state: Default::default(),
        library: Default::default(),
    };
    let mut pending = true;
    flush_library_close(&mut pending, &engine);
    assert!(pending);
    assert!(matches!(rx.try_recv(), Ok(Command::Stop)));
    flush_library_close(&mut pending, &engine);
    assert!(!pending);
    assert!(matches!(
        rx.try_recv(),
        Ok(Command::Library(model::LibraryCommand::Close))
    ));
    flush_library_close(&mut pending, &engine);
    assert!(rx.try_recv().is_err(), "Close must not be duplicated");
}

#[test]
fn library_shortcut_consumes_the_frame_without_starting_playback() {
    let mut view = ui::View::default();
    let snapshot = model::Snapshot::default();
    let mut actions = Vec::new();
    process_frame_input(
        &mut view,
        &snapshot,
        FrameInput {
            keys: &[(Key::F3, false, false), (Key::Space, false, false)],
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], ui::Action::OpenLibrary));
    view.open_about();
    process_frame_input(
        &mut view,
        &snapshot,
        FrameInput {
            keys: &[(Key::F3, false, false)],
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(actions.is_empty(), "About must retain keyboard focus");
}

#[test]
fn library_button_uses_release_and_cancels_outside_or_after_focus_loss() {
    let mut view = ui::View::default();
    let snapshot = model::Snapshot::default();
    assert!(view.pointer_down(485, 152, &snapshot).is_none());
    assert!(matches!(
        view.pointer_up(Some((485, 152)), &snapshot),
        Some(ui::Action::OpenLibrary)
    ));
    view.pointer_down(485, 152, &snapshot);
    assert!(view.pointer_up(Some((300, 152)), &snapshot).is_none());
    view.pointer_down(485, 152, &snapshot);
    view.set_focus(false);
    view.set_focus(true);
    assert!(view.pointer_up(Some((485, 152)), &snapshot).is_none());
}

#[test]
fn library_window_hit_testing_excludes_outside_and_non_finite_positions() {
    assert_eq!(library_window::inside(759.0, 469.0), Some((759, 469)));
    for (x, y) in [
        (-1.0, 10.0),
        (760.0, 20.0),
        (10.0, 470.0),
        (f32::NAN, 1.0),
        (1.0, f32::INFINITY),
    ] {
        assert!(library_window::inside(x, y).is_none());
    }
}
