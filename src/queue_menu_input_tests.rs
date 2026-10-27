//! Context-menu capture and native input batches through the real player loop.
use super::*;

fn state() -> model::Snapshot {
    let mut snapshot = model::Snapshot {
        connected: true,
        playing: true,
        current: Some(2),
        queue: (0..20)
            .map(|index| model::TrackRow {
                uri: format!("spotify:track:{index}"),
                title: format!("Track {index}"),
                ..Default::default()
            })
            .collect(),
        queue_reorder_request: 7,
        position_ms: 42_000,
        ..Default::default()
    };
    snapshot.queue[4].uri = snapshot.queue[2].uri.clone();
    snapshot
}

fn mouse(button: MouseButton, down: bool, x: f32, y: f32) -> MouseInput {
    MouseInput::Button { button, down, x, y }
}

fn frame(view: &mut ui::View, state: &model::Snapshot, input: FrameInput<'_>) -> Vec<ui::Action> {
    let mut actions = Vec::new();
    process_frame_input(view, state, input, &mut actions);
    actions
}

fn active(view: &mut ui::View, state: &model::Snapshot, input: FrameInput<'_>) -> Vec<ui::Action> {
    frame(
        view,
        state,
        FrameInput {
            active: true,
            ..input
        },
    )
}

fn group(view: &mut ui::View, state: &model::Snapshot) {
    for (index, control) in [(0, false), (4, true), (2, true)] {
        let y = (233 + index * 24) as f32;
        assert!(
            active(
                view,
                state,
                FrameInput {
                    mouse: &[
                        MouseInput::ModifiedButton {
                            button: MouseButton::Left,
                            down: true,
                            x: 80.,
                            y,
                            control,
                            shift: false
                        },
                        MouseInput::ModifiedButton {
                            button: MouseButton::Left,
                            down: false,
                            x: 80.,
                            y,
                            control,
                            shift: false
                        },
                    ],
                    ..Default::default()
                }
            )
            .is_empty()
        );
    }
    assert_eq!(view.selected_queue_indices(state), [0, 2, 4]);
}

fn open(view: &mut ui::View, state: &model::Snapshot) {
    assert!(
        active(
            view,
            state,
            FrameInput {
                mouse: &[
                    mouse(MouseButton::Right, true, 80., 281.),
                    mouse(MouseButton::Right, false, 80., 281.)
                ],
                pointer: Some((80., 281.)),
                ..Default::default()
            }
        )
        .is_empty()
    );
    assert!(view.has_queue_menu());
}

#[test]
fn queue_context_native_right_edges_keep_group_or_retarget_once_without_playing() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        let s = state();
        let mut view = ui::View::default();
        view.set_skin(skin);
        group(&mut view, &s);
        open(&mut view, &s);
        assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
        assert!(
            active(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(MouseButton::Right, false, 100., 297.)],
                    ..Default::default()
                }
            )
            .is_empty()
        );
        assert!(view.has_queue_menu());
        active(
            &mut view,
            &s,
            FrameInput {
                keys: &[(Key::Escape, false, false)],
                ..Default::default()
            },
        );
        assert!(
            active(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[
                        mouse(MouseButton::Right, true, 80., 305.),
                        mouse(MouseButton::Right, false, 80., 305.)
                    ],
                    ..Default::default()
                }
            )
            .is_empty()
        );
        assert_eq!(view.selected_queue_indices(&s), [3]);
        let actions = active(
            &mut view,
            &s,
            FrameInput {
                keys: &[(Key::Enter, false, false)],
                ..Default::default()
            },
        );
        assert!(
            matches!(actions.as_slice(), [ui::Action::Send(Command::PlayQueueIndex {
            index: 3, expected_reorder_request: 7, expected_queue,
        })] if expected_queue == &s.queue.iter().map(|row| row.uri.clone()).collect::<Vec<_>>())
        );
        assert!(!view.has_queue_menu());
    }
}

#[test]
fn queue_context_dismissal_and_action_consume_all_same_batch_background_clicks() {
    let s = state();
    for activate in [false, true] {
        let mut view = ui::View::default();
        group(&mut view, &s);
        open(&mut view, &s);
        let (x, y) = if activate { (100., 381.) } else { (500., 233.) };
        let actions = active(
            &mut view,
            &s,
            FrameInput {
                mouse: &[
                    mouse(MouseButton::Left, true, x, y),
                    mouse(MouseButton::Left, false, x, y),
                    mouse(MouseButton::Left, true, 80., 233.),
                    mouse(MouseButton::Left, false, 80., 233.),
                    mouse(MouseButton::Left, true, 80., 233.),
                    mouse(MouseButton::Left, false, 80., 233.),
                ],
                ..Default::default()
            },
        );
        if activate {
            assert!(
                matches!(actions.as_slice(), [ui::Action::Send(Command::EditQueue {
                indices, keep_selected: false, expected_reorder_request: 7, expected_queue,
            })] if indices == &[0, 2, 4] && expected_queue.len() == 20)
            );
        } else {
            assert!(actions.is_empty());
        }
        assert!(!view.has_queue_menu());
        assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
        assert_eq!(view.selected, Some(2));
    }
}

#[test]
fn queue_context_cancellation_wins_over_pressed_action_and_queued_release() {
    let s = state();
    for cancel in 0..8 {
        let mut view = ui::View::default();
        group(&mut view, &s);
        open(&mut view, &s);
        active(
            &mut view,
            &s,
            FrameInput {
                mouse: &[mouse(MouseButton::Left, true, 100., 381.)],
                ..Default::default()
            },
        );
        let cancellation = [Key::Escape, Key::F1, Key::F2, Key::F3, Key::F4, Key::Tab];
        let keys = if cancel < 6 {
            vec![(cancellation[cancel], false, false)]
        } else {
            vec![]
        };
        let mut mouse_events = vec![mouse(MouseButton::Left, false, 100., 381.)];
        if cancel == 7 {
            // Even a Cancel delivered after the release has precedence.
            mouse_events.push(MouseInput::Cancel);
        }
        assert!(
            frame(
                &mut view,
                &s,
                FrameInput {
                    active: cancel != 6,
                    keys: &keys,
                    mouse: &mouse_events,
                    ..Default::default()
                }
            )
            .is_empty(),
            "cancellation {cancel}"
        );
        assert!(!view.has_queue_menu());
        assert!(view.about.is_none());
        assert!(!view.help);
        assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
        assert!(
            active(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(MouseButton::Left, false, 100., 381.)],
                    ..Default::default()
                }
            )
            .is_empty()
        );
    }
}

#[test]
fn queue_context_same_batch_open_and_native_cancel_cannot_execute_menu_action() {
    let s = state();
    let mut view = ui::View::default();
    group(&mut view, &s);
    assert!(
        active(
            &mut view,
            &s,
            FrameInput {
                mouse: &[
                    mouse(MouseButton::Right, true, 80., 281.),
                    mouse(MouseButton::Right, false, 80., 281.),
                    mouse(MouseButton::Left, true, 100., 381.),
                    mouse(MouseButton::Left, false, 100., 381.),
                    MouseInput::Cancel,
                ],
                ..Default::default()
            }
        )
        .is_empty()
    );
    assert!(!view.has_queue_menu());
    assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
}

#[test]
fn queue_context_capture_survives_metadata_and_track_advance_for_all_actions() {
    for item in 0..5 {
        let mut s = state();
        let original = s.queue.clone();
        let mut view = ui::View::default();
        group(&mut view, &s);
        open(&mut view, &s);
        s.queue[2].title = "Resolved title".into();
        s.current = Some(14);
        s.playback_revision += 1;
        s.playback_origin = model::PlaybackOrigin::Automatic;
        s.position_ms = 1234;
        let mut keys = vec![(Key::Home, false, false)];
        keys.extend((0..item).map(|_| (Key::Down, false, false)));
        keys.push((Key::Enter, false, false));
        let actions = active(
            &mut view,
            &s,
            FrameInput {
                keys: &keys,
                pointer: Some((80., 281.)),
                ..Default::default()
            },
        );
        assert_eq!(actions.len(), 1);
        match actions.into_iter().next().unwrap() {
            ui::Action::Send(Command::PlayQueueIndex {
                index,
                expected_queue,
                expected_reorder_request,
            }) => {
                assert_eq!(item, 0);
                assert_eq!(index, 2);
                assert_eq!(expected_reorder_request, 7);
                assert_eq!(
                    expected_queue,
                    original
                        .iter()
                        .map(|row| row.uri.clone())
                        .collect::<Vec<_>>()
                );
            }
            ui::Action::QueueToPlaylist { tracks, create } => {
                assert!(item == 1 || item == 2);
                assert_eq!(create, item == 2);
                assert_eq!(
                    tracks,
                    [
                        original[0].clone(),
                        original[2].clone(),
                        original[4].clone()
                    ]
                );
            }
            ui::Action::Send(Command::EditQueue {
                indices,
                keep_selected,
                expected_queue,
                expected_reorder_request,
            }) => {
                assert!(item == 3 || item == 4);
                assert_eq!(indices, [0, 2, 4]);
                assert_eq!(keep_selected, item == 4);
                assert_eq!(expected_reorder_request, 7);
                assert_eq!(
                    expected_queue,
                    original
                        .iter()
                        .map(|row| row.uri.clone())
                        .collect::<Vec<_>>()
                );
            }
            _ => panic!("unexpected context action"),
        }
        assert!(!view.has_queue_menu());
        assert_eq!(view.queue_drop_index((80., 221.), s.queue.len()), Some(0));
        assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
    }
}

#[test]
fn queue_context_structural_change_consumes_pending_keys_clicks_and_stale_release() {
    for change in 0..4 {
        let mut s = state();
        let mut view = ui::View::default();
        group(&mut view, &s);
        open(&mut view, &s);
        active(
            &mut view,
            &s,
            FrameInput {
                mouse: &[mouse(MouseButton::Left, true, 100., 381.)],
                ..Default::default()
            },
        );
        match change {
            0 => s.queue.swap(8, 9),
            1 => {
                s.queue.pop();
            }
            2 => s.queue.push(s.queue[0].clone()),
            // Swapping equal URIs must still invalidate captured occurrences.
            _ => {
                s.queue.swap(2, 4);
                s.queue_reorder_request += 1;
            }
        }
        assert!(
            active(
                &mut view,
                &s,
                FrameInput {
                    keys: &[
                        (Key::Enter, false, false),
                        (Key::Delete, false, false),
                        (Key::V, true, false)
                    ],
                    mouse: &[
                        mouse(MouseButton::Left, false, 100., 381.),
                        mouse(MouseButton::Left, true, 80., 233.),
                        mouse(MouseButton::Left, false, 80., 233.),
                    ],
                    ..Default::default()
                }
            )
            .is_empty()
        );
        assert!(!view.has_queue_menu());
        assert!(
            active(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(MouseButton::Left, false, 100., 381.)],
                    ..Default::default()
                }
            )
            .is_empty()
        );
    }
}

#[test]
fn queue_context_keyboard_opening_owns_batch_and_keeps_selection_with_stationary_pointer() {
    let s = state();
    for opening in [(Key::Menu, false, false), (Key::F10, false, true)] {
        let mut view = ui::View::default();
        group(&mut view, &s);
        assert!(
            active(
                &mut view,
                &s,
                FrameInput {
                    keys: &[opening, (Key::Enter, false, false)],
                    mouse: &[
                        mouse(MouseButton::Left, true, 500., 233.),
                        mouse(MouseButton::Left, false, 500., 233.)
                    ],
                    ..Default::default()
                }
            )
            .is_empty()
        );
        assert!(view.has_queue_menu());
        assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
        active(
            &mut view,
            &s,
            FrameInput {
                keys: &[(Key::End, false, false)],
                pointer: Some((30., 281.)),
                ..Default::default()
            },
        );
        let actions = active(
            &mut view,
            &s,
            FrameInput {
                keys: &[(Key::Enter, false, false)],
                pointer: Some((30., 281.)),
                ..Default::default()
            },
        );
        assert!(
            matches!(actions.as_slice(), [ui::Action::Send(Command::EditQueue { indices, keep_selected: true, .. })] if indices == &[0, 2, 4])
        );
        view.input_active = true;
        assert!(
            active(
                &mut view,
                &s,
                FrameInput {
                    keys: &[opening],
                    ..Default::default()
                }
            )
            .is_empty()
        );
        assert!(!view.has_queue_menu(), "text input owns context-menu key");
    }
}

#[test]
fn queue_context_isolates_paste_typing_wheel_repeats_and_background_shortcuts() {
    let s = state();
    let mut view = ui::View::default();
    view.input = "kept input".into();
    group(&mut view, &s);
    open(&mut view, &s);
    assert!(
        active(
            &mut view,
            &s,
            FrameInput {
                keys: &[
                    (Key::V, true, false),
                    (Key::A, true, false),
                    (Key::Backspace, false, false),
                    (Key::Delete, false, false)
                ],
                chars: &['x', 'y'],
                pointer: Some((80., 281.)),
                wheel: Some(-20.),
                mouse_down: true,
                repeat_backspace: true,
                repeat_up: true,
                repeat_down: true,
                repeat_shift: true,
                ..Default::default()
            }
        )
        .is_empty()
    );
    assert!(view.has_queue_menu());
    assert_eq!(view.input, "kept input");
    assert!(!view.input_active);
    assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
    view.close_queue_menu();
    assert_eq!(view.queue_drop_index((80., 221.), s.queue.len()), Some(0));
}
