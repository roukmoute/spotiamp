//! Exercise native down/up batches and pointer polling through the player loop.
use super::*;

fn state() -> model::Snapshot {
    model::Snapshot {
        connected: true,
        playing: true,
        current: Some(2),
        queue: (0..20)
            .map(|i| model::TrackRow {
                uri: format!("spotify:track:{i}"),
                ..Default::default()
            })
            .collect(),
        position_ms: 42_000,
        ..Default::default()
    }
}

fn mouse(down: bool, x: f32, y: f32) -> MouseInput {
    MouseInput::Button {
        button: MouseButton::Left,
        down,
        x,
        y,
    }
}

fn frame(view: &mut ui::View, state: &model::Snapshot, input: FrameInput<'_>) -> Vec<ui::Action> {
    let mut actions = Vec::new();
    process_frame_input(
        view,
        state,
        FrameInput {
            active: true,
            ..input
        },
        &mut actions,
    );
    actions
}

#[test]
fn queue_reorder_fast_release_moves_once_and_never_plays() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        let mut view = ui::View::default();
        view.set_skin(skin);
        let s = state();
        // No intermediate frame or motion callback: the release position owns the drop.
        let actions = frame(
            &mut view,
            &s,
            FrameInput {
                mouse: &[mouse(true, 100., 233.), mouse(false, 100., 330.)],
                ..Default::default()
            },
        );
        assert!(
            matches!(actions.as_slice(), [ui::Action::Send(Command::ReorderQueue { indices, insert_before: 5, expected_queue, .. })] if indices == &[0] && expected_queue.len() == 20)
        );
        assert!(
            frame(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(false, 100., 330.)],
                    ..Default::default()
                }
            )
            .is_empty()
        );
        assert_eq!(s.current, Some(2));
        assert_eq!(s.position_ms, 42_000);
    }
}

#[test]
fn queue_double_click_waits_for_release_and_dragging_the_second_press_never_plays() {
    let s = state();
    for drag_second in [false, true] {
        let mut view = ui::View::default();
        assert!(
            frame(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(true, 100., 233.), mouse(false, 100., 233.)],
                    ..Default::default()
                }
            )
            .is_empty()
        );
        assert!(
            frame(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(true, 100., 233.)],
                    mouse_down: true,
                    pointer: Some((100., 233.)),
                    ..Default::default()
                }
            )
            .is_empty()
        );
        let y = if drag_second { 330. } else { 233. };
        let actions = frame(
            &mut view,
            &s,
            FrameInput {
                mouse: &[mouse(false, 100., y)],
                ..Default::default()
            },
        );
        if drag_second {
            assert!(matches!(
                actions.as_slice(),
                [ui::Action::Send(Command::ReorderQueue { .. })]
            ));
        } else {
            assert!(matches!(
                actions.as_slice(),
                [ui::Action::Send(Command::PlayIndex(0))]
            ));
        }
    }
}

#[test]
fn queue_drag_no_op_small_motion_and_outside_release_do_not_dispatch() {
    let s = state();
    for (x, y) in [
        (103., 236.),
        (130., 233.),
        (100., 450.),
        (-10., 330.),
        (100., 100.),
    ] {
        let mut view = ui::View::default();
        assert!(
            frame(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(true, 100., 233.), mouse(false, x, y)],
                    ..Default::default()
                }
            )
            .is_empty()
        );
    }
    let mut view = ui::View::default();
    let short = model::Snapshot {
        queue: s.queue[..2].to_vec(),
        ..s
    };
    assert!(
        frame(
            &mut view,
            &short,
            FrameInput {
                mouse: &[mouse(true, 100., 330.), mouse(false, 100., 233.)],
                ..Default::default()
            }
        )
        .is_empty(),
        "blank rows cannot be drag sources"
    );
}

#[test]
fn queue_drag_escape_focus_loss_right_click_and_native_cancel_consume_release() {
    for cancel in 0..8 {
        let mut view = ui::View::default();
        let s = state();
        assert!(
            frame(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(true, 100., 233.)],
                    mouse_down: true,
                    pointer: Some((100., 330.)),
                    ..Default::default()
                }
            )
            .is_empty()
        );
        let mut actions = Vec::new();
        match cancel {
            0 => {
                actions = frame(
                    &mut view,
                    &s,
                    FrameInput {
                        keys: &[(Key::Escape, false, false)],
                        mouse: &[mouse(false, 100., 330.)],
                        ..Default::default()
                    },
                );
            }
            1 => {
                process_frame_input(
                    &mut view,
                    &s,
                    FrameInput {
                        active: false,
                        mouse: &[mouse(false, 100., 330.)],
                        ..Default::default()
                    },
                    &mut actions,
                );
            }
            2 => {
                actions = frame(
                    &mut view,
                    &s,
                    FrameInput {
                        mouse: &[
                            MouseInput::Button {
                                button: MouseButton::Right,
                                down: true,
                                x: 100.,
                                y: 330.,
                            },
                            mouse(false, 100., 330.),
                        ],
                        ..Default::default()
                    },
                );
            }
            3 => {
                actions = frame(
                    &mut view,
                    &s,
                    FrameInput {
                        mouse: &[MouseInput::Cancel, mouse(false, 100., 330.)],
                        ..Default::default()
                    },
                );
            }
            4 => {
                actions = frame(
                    &mut view,
                    &s,
                    FrameInput {
                        keys: &[(Key::F2, false, false)],
                        mouse: &[mouse(false, 100., 330.)],
                        ..Default::default()
                    },
                );
            }
            5 => {
                actions = frame(
                    &mut view,
                    &s,
                    FrameInput {
                        keys: &[(Key::F1, false, false)],
                        mouse: &[mouse(false, 100., 330.)],
                        ..Default::default()
                    },
                );
            }
            6 => {
                actions = frame(
                    &mut view,
                    &s,
                    FrameInput {
                        keys: &[(Key::Tab, false, false)],
                        mouse: &[mouse(false, 100., 330.)],
                        ..Default::default()
                    },
                );
            }
            _ => {
                let mut changed = s.clone();
                changed.queue.swap(10, 11);
                actions = frame(
                    &mut view,
                    &changed,
                    FrameInput {
                        mouse: &[mouse(false, 100., 330.)],
                        ..Default::default()
                    },
                );
            }
        }
        assert!(actions.is_empty(), "cancellation scenario {cancel}");
        assert!(
            frame(
                &mut view,
                &s,
                FrameInput {
                    mouse: &[mouse(false, 100., 330.)],
                    ..Default::default()
                }
            )
            .is_empty()
        );
    }
}

#[test]
fn queue_drag_escape_in_same_batch_as_press_and_release_cannot_reorder() {
    let mut view = ui::View::default();
    assert!(
        frame(
            &mut view,
            &state(),
            FrameInput {
                keys: &[(Key::Escape, false, false)],
                mouse: &[mouse(true, 100., 233.), mouse(false, 100., 330.)],
                ..Default::default()
            }
        )
        .is_empty()
    );
}

fn callback_frame(
    view: &mut ui::View,
    s: &model::Snapshot,
    callback: &mut TextInput,
) -> Vec<ui::Action> {
    let actions = frame(
        view,
        s,
        FrameInput {
            keys: &callback.keys.borrow(),
            mouse: &callback.mouse.borrow(),
            ..Default::default()
        },
    );
    callback.keys.borrow_mut().clear();
    callback.mouse.borrow_mut().clear();
    actions
}

fn callback_click(callback: &mut TextInput, row: usize) {
    let y = (233 + row * 24) as f32;
    callback.set_mouse_button_state(MouseButton::Left, true, 100., y);
    callback.set_mouse_button_state(MouseButton::Left, false, 100., y);
}

#[test]
fn queue_native_modifier_snapshot_survives_fast_release_and_group_drag() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        for modifier in [
            Key::LeftCtrl,
            Key::RightCtrl,
            Key::LeftSuper,
            Key::RightSuper,
        ] {
            let mut view = ui::View::default();
            view.set_skin(skin);
            let s = state();
            let mut keys = keyboard_input_tests::Keyboard::new();
            callback_click(&mut keys.callback, 0);
            assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
            keys.down(modifier);
            callback_click(&mut keys.callback, 4);
            callback_click(&mut keys.callback, 2);
            keys.up(modifier);
            // Both modified clicks and the modifier release arrive before the frame.
            assert_eq!(keys.callback.modifiers, 0);
            assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
            assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
            // Releasing the modifier before moving an existing member must not
            // collapse the group, even if the whole drag fits in one frame.
            keys.callback
                .set_mouse_button_state(MouseButton::Left, true, 100., 281.);
            keys.callback
                .set_mouse_button_state(MouseButton::Left, false, 100., 368.);
            let actions = callback_frame(&mut view, &s, &mut keys.callback);
            assert!(
                matches!(actions.as_slice(), [ui::Action::Send(Command::ReorderQueue {
                indices, insert_before: 6, expected_queue, ..
            })] if indices == &[0, 2, 4] && expected_queue.len() == 20)
            );
            assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
            assert_eq!(s.position_ms, 42_000);
            assert_eq!(s.current, Some(2));
        }
    }
}

#[test]
fn queue_shift_range_and_control_toggle_clicks_never_double_click_play() {
    let s = state();
    let mut view = ui::View::default();
    let mut keys = keyboard_input_tests::Keyboard::new();
    callback_click(&mut keys.callback, 1);
    assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
    keys.down(Key::LeftShift);
    callback_click(&mut keys.callback, 4);
    keys.up(Key::LeftShift);
    assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
    assert_eq!(view.selected_queue_indices(&s), [1, 2, 3, 4]);
    keys.down(Key::LeftCtrl);
    callback_click(&mut keys.callback, 3);
    callback_click(&mut keys.callback, 3);
    callback_click(&mut keys.callback, 3);
    keys.up(Key::LeftCtrl);
    assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
    assert_eq!(view.selected_queue_indices(&s), [1, 2, 4]);
    // A plain tap on a member selects it alone only once release confirms
    // this was a click, while its press still permits a group drag.
    keys.callback
        .set_mouse_button_state(MouseButton::Left, true, 100., 281.);
    assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
    assert_eq!(view.selected_queue_indices(&s), [1, 2, 4]);
    keys.callback
        .set_mouse_button_state(MouseButton::Left, false, 100., 281.);
    assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
    assert_eq!(view.selected_queue_indices(&s), [2]);
}

#[test]
fn queue_group_drag_cancel_and_modified_right_press_never_reorder() {
    let s = state();
    for cancel in 0..4 {
        let mut view = ui::View::default();
        let mut keys = keyboard_input_tests::Keyboard::new();
        callback_click(&mut keys.callback, 0);
        keys.down(Key::LeftCtrl);
        callback_click(&mut keys.callback, 2);
        callback_click(&mut keys.callback, 4);
        keys.up(Key::LeftCtrl);
        assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
        assert_eq!(view.selected_queue_indices(&s), [0, 2, 4]);
        keys.callback
            .set_mouse_button_state(MouseButton::Left, true, 100., 281.);
        assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
        match cancel {
            0 => keys.tap(Key::Escape),
            1 => keys.callback.mouse_input_cancelled(),
            2 => {
                keys.down(Key::LeftCtrl);
                keys.callback
                    .set_mouse_button_state(MouseButton::Right, true, 100., 368.);
                keys.up(Key::LeftCtrl);
            }
            _ => view.set_focus(false),
        }
        keys.callback
            .set_mouse_button_state(MouseButton::Left, false, 100., 368.);
        assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
    }
}

#[test]
fn modified_transport_clicks_survive_playback_follow_without_retargeting_rows() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        let mut view = ui::View::default();
        view.set_skin(skin);
        let mut s = state();
        let mut keys = keyboard_input_tests::Keyboard::new();
        s.playback_revision = 1;
        frame(&mut view, &s, FrameInput::default());
        keys.down(Key::LeftCtrl);
        keys.callback
            .set_mouse_button_state(MouseButton::Left, true, 202., 127.);
        keys.callback
            .set_mouse_button_state(MouseButton::Left, false, 202., 127.);
        callback_click(&mut keys.callback, 4);
        keys.up(Key::LeftCtrl);
        s.current = Some(8);
        s.playback_revision += 1;
        assert!(matches!(
            callback_frame(&mut view, &s, &mut keys.callback).as_slice(),
            [ui::Action::Send(Command::Next)]
        ));
        assert_eq!(view.selected_queue_indices(&s), [8]);
    }
}

#[test]
fn delete_only_removes_a_chosen_focus_and_never_a_toggled_off_row() {
    let s = state();
    let mut view = ui::View::default();
    let mut keys = keyboard_input_tests::Keyboard::new();
    callback_click(&mut keys.callback, 1);
    keys.down(Key::LeftCtrl);
    callback_click(&mut keys.callback, 3);
    keys.up(Key::LeftCtrl);
    callback_frame(&mut view, &s, &mut keys.callback);
    assert_eq!(view.selected_queue_indices(&s), [1, 3]);
    keys.tap(Key::Delete);
    assert!(matches!(
        callback_frame(&mut view, &s, &mut keys.callback).as_slice(),
        [ui::Action::Send(Command::Remove(3))]
    ));
    keys.down(Key::LeftCtrl);
    callback_click(&mut keys.callback, 3);
    keys.up(Key::LeftCtrl);
    callback_frame(&mut view, &s, &mut keys.callback);
    assert_eq!(view.selected_queue_indices(&s), [1]);
    assert_eq!(view.selected, Some(3));
    keys.tap(Key::Delete);
    assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
    keys.down(Key::LeftCtrl);
    keys.tap(Key::Down);
    keys.up(Key::LeftCtrl);
    callback_frame(&mut view, &s, &mut keys.callback);
    assert_eq!(view.selected, Some(4));
    keys.tap(Key::Delete);
    assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
    keys.down(Key::LeftCtrl);
    callback_click(&mut keys.callback, 1);
    keys.up(Key::LeftCtrl);
    callback_frame(&mut view, &s, &mut keys.callback);
    assert!(view.selected_queue_indices(&s).is_empty());
    keys.tap(Key::Delete);
    assert!(callback_frame(&mut view, &s, &mut keys.callback).is_empty());
}
