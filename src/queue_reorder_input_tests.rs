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
