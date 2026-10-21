use super::*;
use model::{Snapshot, TrackRow};

fn state() -> Snapshot {
    Snapshot {
        connected: true,
        playing: true,
        queue: (0..30)
            .map(|index| TrackRow {
                title: format!("Track {index}"),
                duration_ms: 180_000,
                ..TrackRow::default()
            })
            .collect(),
        current: Some(2),
        duration_ms: 180_000,
        ..Snapshot::default()
    }
}

fn click(x: f32, y: f32) -> [MouseInput; 2] {
    [
        MouseInput::Button {
            button: MouseButton::Left,
            down: true,
            x,
            y,
        },
        MouseInput::Button {
            button: MouseButton::Left,
            down: false,
            x,
            y,
        },
    ]
}

#[test]
fn window_controls_are_allowed_through_about_without_playback_shortcuts() {
    for (x, close) in [(522.0, false), (543.0, true)] {
        let mut view = ui::View::default();
        view.open_about();
        let mut actions = Vec::new();
        process_frame_input(
            &mut view,
            &state(),
            FrameInput {
                mouse: &click(x, 9.0),
                keys: &[(Key::Delete, false, false)],
                active: true,
                ..FrameInput::default()
            },
            &mut actions,
        );
        assert!(
            matches!(actions.as_slice(), [ui::Action::Close]) && close
                || matches!(actions.as_slice(), [ui::Action::Minimize]) && !close
        );
        assert!(view.about.is_some());
    }
}

#[test]
fn modal_consumes_background_shortcuts_text_repeats_wheel_and_drag() {
    let state = state();
    let mut view = ui::View::default();
    view.input = "unsubmitted input".into();
    view.selected = Some(2);
    view.render(&state);
    let before = view.pixels.clone();
    view.open_about();
    let mut actions = Vec::new();
    process_frame_input(
        &mut view,
        &state,
        FrameInput {
            keys: &[
                (Key::V, true, false),
                (Key::A, true, false),
                (Key::Backspace, false, false),
                (Key::Delete, false, false),
                (Key::Space, false, false),
                (Key::Enter, false, false),
            ],
            chars: &['x', 'y'],
            pointer: Some((280.0, 100.0)),
            mouse_down: true,
            active: true,
            wheel: Some(-20.0),
            repeat_backspace: true,
            repeat_up: true,
            repeat_down: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(
        actions.is_empty(),
        "no clipboard or playback action may escape"
    );
    assert!(view.about.is_some());
    assert_eq!(view.selected, Some(2));
    assert_eq!(view.input, "unsubmitted input");
    view.close_about();
    view.render(&state);
    assert_eq!(
        view.pixels, before,
        "the underlying viewport must stay unchanged"
    );
}

#[test]
fn escape_and_space_in_one_frame_close_without_toggling_playback() {
    let state = state();
    let mut view = ui::View::default();
    view.open_about();
    let mut actions = Vec::new();
    process_frame_input(
        &mut view,
        &state,
        FrameInput {
            keys: &[(Key::Escape, false, false), (Key::Space, false, false)],
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(view.about.is_none());
    assert!(actions.is_empty());
    process_frame_input(
        &mut view,
        &state,
        FrameInput {
            keys: &[(Key::Space, false, false)],
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(matches!(
        actions.as_slice(),
        [ui::Action::Send(Command::Toggle)]
    ));
}

#[test]
fn closing_then_clicking_player_in_same_mouse_batch_does_not_click_through() {
    let state = state();
    let mut view = ui::View::default();
    view.open_about();
    let events: Vec<_> = click(510.0, 40.0)
        .into_iter()
        .chain(click(70.0, 125.0))
        .chain(click(380.0, 425.0))
        .collect();
    let mut actions = Vec::new();
    process_frame_input(
        &mut view,
        &state,
        FrameInput {
            mouse: &events,
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(view.about.is_none());
    assert!(actions.is_empty());
    process_frame_input(
        &mut view,
        &state,
        FrameInput {
            mouse: &click(70.0, 125.0),
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(matches!(actions.as_slice(), [ui::Action::Send(_)]));
}

#[test]
fn opening_about_blocks_other_inputs_from_that_frame() {
    let state = state();
    let mut view = ui::View::default();
    view.input = "keep me".into();
    let mut actions = Vec::new();
    process_frame_input(
        &mut view,
        &state,
        FrameInput {
            keys: &[
                (Key::V, true, false),
                (Key::Space, false, false),
                (Key::F2, false, false),
            ],
            chars: &['x'],
            mouse: &click(70.0, 125.0),
            active: true,
            wheel: Some(-10.0),
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(view.about.is_some());
    assert!(actions.is_empty());
    assert_eq!(view.input, "keep me");
}

#[test]
fn shift_tab_uses_modifier_at_keydown_even_after_shift_is_released() {
    let keys = Rc::new(RefCell::new(Vec::new()));
    let mut callback = TextInput {
        chars: Rc::new(RefCell::new(Vec::new())),
        keys: keys.clone(),
        mouse: Rc::new(RefCell::new(Vec::new())),
        held: [false; 512],
        modifiers: 0,
    };
    callback.set_key_state(Key::LeftShift, true);
    callback.add_char('A' as u32);
    callback.set_key_state(Key::Tab, true);
    callback.set_key_state(Key::Tab, false);
    callback.set_key_state(Key::LeftShift, false);
    assert!(keys.borrow().contains(&(Key::Tab, false, true)));
    assert_eq!(&*callback.chars.borrow(), &['A']);
    let mut view = ui::View::default();
    view.open_about();
    let mut actions = Vec::new();
    process_frame_input(
        &mut view,
        &state(),
        FrameInput {
            keys: &keys.borrow(),
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    // Backward from the first tab focuses Close, rather than Credits.
    process_frame_input(
        &mut view,
        &state(),
        FrameInput {
            keys: &[(Key::Enter, false, false)],
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(view.about.is_none());
    assert!(actions.is_empty());
}

#[test]
fn about_cli_selects_each_tab_and_allows_a_following_screenshot_option() {
    for (name, tab) in [
        ("player", ui::AboutTab::Player),
        ("credits", ui::AboutTab::Credits),
        ("shortcuts", ui::AboutTab::Shortcuts),
        ("history", ui::AboutTab::History),
        ("updates", ui::AboutTab::Updates),
    ] {
        assert_eq!(
            requested_about_tab(&["--about".into(), name.into()]),
            Ok(Some(tab))
        );
    }
    assert_eq!(
        requested_about_tab(&["--about".into(), "--screenshot".into(), "out.ppm".into()]),
        Ok(Some(ui::AboutTab::Player))
    );
    assert!(requested_about_tab(&["--about".into(), "unknown".into()]).is_err());
}

#[test]
fn update_actions_survive_modal_filter_without_background_actions() {
    use updater::UpdateStatus;
    let state = state();
    for (update, x, tabs, expected) in [
        (UpdateStatus::Idle, 100.0, 4, 0),
        (
            UpdateStatus::Available {
                version: "0.1.0-alpha.7".into(),
            },
            400.0,
            5,
            1,
        ),
        (
            UpdateStatus::Ready {
                version: "0.1.0-alpha.7".into(),
            },
            400.0,
            5,
            2,
        ),
    ] {
        for keyboard in [false, true] {
            let mut view = ui::View::default();
            view.update = update.clone();
            view.input = "keep this link".into();
            view.selected = Some(2);
            view.open_about();
            view.click(470, 76, &state); // Updates tab; focus begins on its label.
            if keyboard {
                for _ in 0..tabs {
                    view.about_key(Key::Tab, false);
                }
            }
            let mut actions = Vec::new();
            let mouse = click(x, 325.0);
            process_frame_input(
                &mut view,
                &state,
                FrameInput {
                    keys: &[
                        (Key::Space, false, false),
                        (Key::V, true, false),
                        (Key::Delete, false, false),
                    ],
                    mouse: if keyboard { &[] } else { &mouse },
                    chars: &['x'],
                    active: true,
                    wheel: Some(-20.0),
                    repeat_down: true,
                    ..FrameInput::default()
                },
                &mut actions,
            );
            assert!(matches!(
                (expected, actions.as_slice()),
                (0, [ui::Action::CheckUpdates])
                    | (1, [ui::Action::DownloadUpdate])
                    | (2, [ui::Action::InstallUpdate])
            ));
            assert_eq!(view.about, Some(ui::AboutTab::Updates));
            assert_eq!(view.input, "keep this link");
            assert_eq!(view.selected, Some(2));
        }
    }
}

#[test]
fn install_then_close_in_one_mouse_batch_preserves_install_without_click_through() {
    let state = state();
    let mut view = ui::View::default();
    view.update = updater::UpdateStatus::Ready {
        version: "0.1.0-alpha.7".into(),
    };
    view.open_about();
    let events: Vec<_> = click(400.0, 325.0)
        .into_iter()
        .chain(click(510.0, 40.0))
        .chain(click(70.0, 125.0))
        .collect();
    let mut actions = Vec::new();
    process_frame_input(
        &mut view,
        &state,
        FrameInput {
            mouse: &events,
            active: true,
            ..FrameInput::default()
        },
        &mut actions,
    );
    assert!(view.about.is_none());
    assert!(matches!(actions.as_slice(), [ui::Action::InstallUpdate]));
}
