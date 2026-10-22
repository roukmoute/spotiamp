use super::*;

fn button_event(x: f32, y: f32, down: bool) -> MouseInput {
    MouseInput::Button {
        button: MouseButton::Left,
        down,
        x,
        y,
    }
}

fn playing_state() -> model::Snapshot {
    model::Snapshot {
        connected: true,
        playing: true,
        queue: vec![model::TrackRow::default(); 30],
        current: Some(0),
        playback_revision: 1,
        ..Default::default()
    }
}

#[test]
fn a_new_playback_consumes_old_input_then_accepts_the_next_batch() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        for (x, y) in [(50., 425.), (100., 233.)] {
            let mut view = ui::View::default();
            view.set_skin(skin);
            let mut state = model::Snapshot {
                queue: vec![model::TrackRow::default(); 30],
                current: Some(0),
                playback_revision: 1,
                ..Default::default()
            };
            let mut actions = Vec::new();
            process_frame_input(&mut view, &state, FrameInput::default(), &mut actions);
            assert_eq!(view.selected, Some(0));
            // These down/up callbacks belong to the screen before track nine
            // started. Neither Remove nor a double-click may be retargeted.
            let click = [
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
            ];
            state.current = Some(8);
            state.playback_revision += 1;
            process_frame_input(
                &mut view,
                &state,
                FrameInput {
                    active: true,
                    mouse: &click,
                    keys: &[(Key::Delete, false, false), (Key::Enter, false, false)],
                    repeat_down: true,
                    wheel: Some(20.),
                    ..Default::default()
                },
                &mut actions,
            );
            assert!(actions.is_empty(), "old input cannot affect the new row");
            assert_eq!(view.selected, Some(8));
            view.render(&state);
            process_frame_input(
                &mut view,
                &state,
                FrameInput {
                    active: true,
                    keys: &[(Key::Delete, false, false)],
                    ..Default::default()
                },
                &mut actions,
            );
            assert!(matches!(
                actions.as_slice(),
                [ui::Action::Send(Command::Remove(8))]
            ));
        }
    }
}

#[test]
fn next_press_survives_a_track_change_between_press_and_release_once() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        let mut view = ui::View::default();
        view.set_skin(skin);
        let mut state = playing_state();
        let mut actions = Vec::new();
        process_frame_input(
            &mut view,
            &state,
            FrameInput {
                active: true,
                ..Default::default()
            },
            &mut actions,
        );
        process_frame_input(
            &mut view,
            &state,
            FrameInput {
                active: true,
                mouse: &[button_event(202., 127., true)],
                ..Default::default()
            },
            &mut actions,
        );
        assert!(actions.is_empty());
        state.current = Some(8);
        state.playback_revision += 1;
        for expected in [1, 0] {
            process_frame_input(
                &mut view,
                &state,
                FrameInput {
                    active: true,
                    mouse: &[button_event(202., 127., false)],
                    ..Default::default()
                },
                &mut actions,
            );
            assert_eq!(actions.len(), expected);
            assert!(
                actions
                    .iter()
                    .all(|action| matches!(action, ui::Action::Send(Command::Next)))
            );
        }
        assert_eq!(view.selected, Some(8));
    }
}

#[test]
fn a_followed_row_keeps_transport_clicks_but_discards_old_queue_actions() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        let mut view = ui::View::default();
        view.set_skin(skin);
        let mut state = playing_state();
        let mut actions = Vec::new();
        process_frame_input(
            &mut view,
            &state,
            FrameInput {
                active: true,
                ..Default::default()
            },
            &mut actions,
        );
        let mut mouse = Vec::new();
        // Old row double-click, all five fixed buttons, then old Remove.
        for (x, y) in [
            (80., 233.),
            (80., 233.),
            (34., 127.),
            (76., 127.),
            (118., 127.),
            (160., 127.),
            (202., 127.),
            (50., 425.),
        ] {
            mouse.extend([button_event(x, y, true), button_event(x, y, false)]);
        }
        state.current = Some(8);
        state.playback_revision += 1;
        process_frame_input(
            &mut view,
            &state,
            FrameInput {
                active: true,
                mouse: &mouse,
                keys: &[(Key::Delete, false, false), (Key::Enter, false, false)],
                ..Default::default()
            },
            &mut actions,
        );
        assert!(matches!(
            actions.as_slice(),
            [
                ui::Action::Send(Command::Previous),
                ui::Action::Send(Command::Play(8)),
                ui::Action::Send(Command::Toggle),
                ui::Action::Send(Command::Stop),
                ui::Action::Send(Command::Next),
            ]
        ));
        assert_eq!(view.selected, Some(8));
    }
}

#[test]
fn followed_transport_still_respects_focus_dialogs_and_text_entry() {
    for blocked in 0..5 {
        let mut view = ui::View::default();
        let mut state = playing_state();
        let mut actions = Vec::new();
        process_frame_input(
            &mut view,
            &state,
            FrameInput {
                active: true,
                ..Default::default()
            },
            &mut actions,
        );
        match blocked {
            1 => view.open_about(),
            2 => view.help = true,
            _ => {}
        }
        state.current = Some(8);
        state.playback_revision += 1;
        let keys = if blocked == 3 {
            &[(Key::F2, false, false)][..]
        } else {
            &[]
        };
        state.connected = blocked != 4;
        process_frame_input(
            &mut view,
            &state,
            FrameInput {
                active: blocked != 0,
                mouse: &[
                    button_event(202., 127., true),
                    button_event(202., 127., false),
                ],
                keys,
                ..Default::default()
            },
            &mut actions,
        );
        assert!(
            actions.is_empty(),
            "transport must remain blocked in case {blocked}"
        );
    }
    let mut view = ui::View::default();
    let mut state = playing_state();
    let mut actions = Vec::new();
    for input_active in [true, false] {
        view.input_active = input_active;
        state.playback_revision += 1;
        process_frame_input(
            &mut view,
            &state,
            FrameInput {
                active: true,
                keys: &[(Key::Space, false, false)],
                ..Default::default()
            },
            &mut actions,
        );
        if input_active {
            assert!(actions.is_empty());
        } else {
            assert!(matches!(
                actions.as_slice(),
                [ui::Action::Send(Command::Toggle)]
            ));
        }
    }
}
