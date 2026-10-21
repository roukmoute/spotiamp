use super::*;

fn center(button: Button) -> (usize, usize) {
    let (x, y, w, h) = button.rect();
    (x + w / 2, y + h / 2)
}

fn mode_pixels(view: &View, button: Button) -> Vec<u32> {
    let (x, y, w, h) = button.rect();
    (y..y + h)
        .flat_map(|row| {
            view.pixels[row * WIDTH + x..row * WIDTH + x + w]
                .iter()
                .copied()
        })
        .collect()
}

#[test]
fn mode_controls_toggle_once_on_release_including_before_connection() {
    for connected in [false, true] {
        for active in [false, true] {
            let state = Snapshot {
                connected,
                shuffle: active,
                repeat: active,
                ..Snapshot::default()
            };
            for button in [Button::Shuffle, Button::Repeat] {
                let mut view = View {
                    selected: Some(2),
                    ..View::default()
                };
                let (x, y) = center(button);
                assert!(view.pointer_down(x, y, &state).is_none());
                let action = view.pointer_up(Some((x, y)), &state);
                assert!(matches!(
                    (button, action),
                    (Button::Shuffle, Some(Action::Send(Command::ToggleShuffle)))
                        | (Button::Repeat, Some(Action::Send(Command::ToggleRepeat)))
                ));
                assert!(view.pointer_up(Some((x, y)), &state).is_none());
                assert_eq!(view.selected, Some(2));
            }
        }
    }
}

#[test]
fn mode_capture_cancels_outside_between_controls_on_focus_loss_and_in_dialogs() {
    let state = Snapshot::default();
    for button in [Button::Shuffle, Button::Repeat] {
        let (x, y) = center(button);
        let other = if button == Button::Shuffle {
            Button::Repeat
        } else {
            Button::Shuffle
        };
        let mut view = View::default();
        view.pointer_down(x, y, &state);
        assert!(view.pointer_up(Some(center(other)), &state).is_none());
        assert!(view.pointer_up(Some((x, y)), &state).is_none());
        view.pointer_down(x, y, &state);
        assert!(view.pointer_up(None, &state).is_none());
        view.pointer_down(x, y, &state);
        view.set_focus(false);
        view.set_focus(true);
        assert!(view.pointer_up(Some((x, y)), &state).is_none());
        view.pointer_down(x, y, &state);
        view.open_about();
        assert!(view.pointer_up(Some((x, y)), &state).is_none());
        assert!(view.pointer_down(x, y, &state).is_none());
        assert!(view.pointer_up(Some((x, y)), &state).is_none());
        view.close_about();
        view.help = true;
        assert!(view.pointer_down(x, y, &state).is_none());
        assert!(view.pointer_up(Some((x, y)), &state).is_none());
        assert!(!view.help);
    }
}

#[test]
fn mode_buttons_fit_between_next_and_about_without_changing_the_volume() {
    let state = UpdateStatus::Idle;
    for (button, expected) in [
        (Button::Next, (183, 116, 38, 22)),
        (Button::Shuffle, (225, 116, 56, 22)),
        (Button::Repeat, (283, 116, 22, 22)),
        (Button::About, (308, 116, 22, 22)),
    ] {
        assert_eq!(button.rect(), expected);
        let (x, y, w, h) = expected;
        for point in [(x, y), (x + w - 1, y + h - 1), center(button)] {
            assert_eq!(Button::at(point.0, point.1, None, &state), Some(button));
        }
    }
    for x in [221, 224, 281, 282, 305, 307, 330, 336] {
        assert!(Button::at(x, 127, None, &state).is_none());
    }
    let mut view = View::default();
    assert!(matches!(
        view.pointer_down(337, 127, &Snapshot::default()),
        Some(Action::Send(Command::Volume(0)))
    ));
}

#[test]
fn mode_state_is_visible_persistent_and_part_of_the_redraw_fingerprint() {
    let mut view = View::default();
    let buffer = view.pixels.as_ptr();
    let mut fingerprints = std::collections::HashSet::new();
    for (shuffle, repeat) in [(false, false), (true, false), (false, true), (true, true)] {
        let state = Snapshot {
            shuffle,
            repeat,
            ..Snapshot::default()
        };
        assert!(fingerprints.insert(view.fingerprint(&state)));
        view.render(&state);
        assert_eq!(view.pixels.as_ptr(), buffer);
        for (button, active) in [(Button::Shuffle, shuffle), (Button::Repeat, repeat)] {
            let original = mode_pixels(&view, button);
            assert_eq!(original.contains(&GREEN), active);
            let (x, y) = center(button);
            view.pointer_down(x, y, &state);
            view.render(&state);
            assert_ne!(
                mode_pixels(&view, button),
                original,
                "mouse press must remain visible even when the mode is active"
            );
            view.pointer_up(Some((x, y)), &state);
            view.pointer_move(None);
            view.render(&state);
            assert_eq!(
                mode_pixels(&view, button),
                original,
                "only a new engine snapshot changes the persistent mode indicator"
            );
        }
    }
}

#[test]
fn french_mode_tooltips_redraw_on_entry_and_exit_without_continuous_redraws() {
    let state = Snapshot::default();
    let mut view = View::default();
    view.render(&state);
    let idle = view.fingerprint(&state);
    for button in [Button::Shuffle, Button::Repeat] {
        let (x, y) = center(button);
        view.pointer_move(Some((x, y)));
        let hovered = view.fingerprint(&state);
        assert_ne!(hovered, idle);
        view.render(&state);
        assert_eq!(view.pixels[142 * WIDTH + 227], 0xefe7bc);
        view.pointer_move(Some((x + 1, y)));
        assert_eq!(hovered, view.fingerprint(&state));
        view.pointer_move(None);
        assert_eq!(idle, view.fingerprint(&state));
        view.render(&state);
        assert_ne!(view.pixels[142 * WIDTH + 227], 0xefe7bc);
    }
}

#[test]
#[ignore = "Exports mode previews only when SPOTIAMP_UI_PREVIEW_DIR is set"]
fn export_mode_previews() {
    let directory = std::env::var_os("SPOTIAMP_UI_PREVIEW_DIR").expect("preview output directory");
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let mut view = View::default();
    let mut state = Snapshot {
        connected: true,
        playing: true,
        current: Some(0),
        position_ms: 74_000,
        duration_ms: 241_000,
        status: "Aperçu des commandes de lecture (données de démonstration).".into(),
        queue: vec![crate::model::TrackRow {
            title: "Un titre dans votre file".into(),
            artist: "Votre musique".into(),
            duration_ms: 241_000,
            ..crate::model::TrackRow::default()
        }],
        ..Snapshot::default()
    };
    for name in ["normal-off", "both-active", "pressed", "tooltip"] {
        if name != "normal-off" {
            state.shuffle = true;
            state.repeat = true;
        }
        if name == "pressed" {
            let (x, y) = center(Button::Shuffle);
            view.pointer_down(x, y, &state);
        } else if name == "tooltip" {
            view.cancel_pointer();
            view.pointer_move(Some(center(Button::Repeat)));
        }
        view.render(&state);
        view.save_ppm(directory.join(format!("{name}.ppm")).to_str().unwrap())
            .unwrap();
    }
}
