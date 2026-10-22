use super::*;

fn queue() -> model::Snapshot {
    model::Snapshot {
        connected: true,
        queue: (0..30)
            .map(|index| model::TrackRow {
                title: format!("Titre {index}"),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn inactive_player_scrolls_under_pointer_without_accepting_keyboard_or_clicks() {
    let snapshot = queue();
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        let mut view = ui::View::default();
        view.set_skin(skin);
        view.input = "unchanged".into();
        view.selected = Some(5);
        let buffer = view.pixels.as_ptr();
        let mut actions = Vec::new();
        for _ in 0..2 {
            process_frame_input(
                &mut view,
                &snapshot,
                FrameInput {
                    pointer: Some((100.0, 250.0)),
                    wheel: Some(-0.5),
                    keys: &[(Key::Delete, false, false), (Key::F2, false, false)],
                    chars: &['x'],
                    mouse: &[
                        MouseInput::Button {
                            button: MouseButton::Left,
                            down: true,
                            x: 70.0,
                            y: 125.0,
                        },
                        MouseInput::Button {
                            button: MouseButton::Left,
                            down: false,
                            x: 70.0,
                            y: 125.0,
                        },
                    ],
                    active: false,
                    repeat_down: true,
                    ..Default::default()
                },
                &mut actions,
            );
            assert!(actions.is_empty());
            assert!(view.about.is_none());
            assert_eq!(view.selected, Some(5), "wheel retains selection");
            assert_eq!(view.input, "unchanged");
        }
        view.render(&snapshot);
        assert_eq!(view.pixels.as_ptr(), buffer);
        view.set_focus(true);
        assert!(view.click(100, 233, &snapshot).is_none());
        assert_eq!(
            view.selected,
            Some(1),
            "two half notches expose the second row"
        );
    }
}

#[test]
fn player_wheel_requires_its_visible_list_and_does_not_scroll_behind_dialogs() {
    let snapshot = queue();
    for active in [false, true] {
        for pointer in [
            None,
            Some((-1.0, 250.0)),
            Some((561.0, 250.0)),
            Some((100.0, 125.0)),
            Some((100.0, 450.0)),
            Some((f32::NAN, 250.0)),
        ] {
            let mut view = ui::View::default();
            let mut actions = Vec::new();
            process_frame_input(
                &mut view,
                &snapshot,
                FrameInput {
                    pointer,
                    wheel: Some(-4.0),
                    active,
                    ..Default::default()
                },
                &mut actions,
            );
            view.set_focus(true);
            view.render(&snapshot);
            assert!(view.click(100, 233, &snapshot).is_none());
            assert_eq!(
                view.selected,
                Some(0),
                "wheel outside the list must not move it"
            );
        }
        for about in [false, true] {
            let mut view = ui::View::default();
            if about {
                view.open_about();
            } else {
                view.help = true;
            }
            process_frame_input(
                &mut view,
                &snapshot,
                FrameInput {
                    pointer: Some((100.0, 250.0)),
                    wheel: Some(-4.0),
                    active,
                    ..Default::default()
                },
                &mut Vec::new(),
            );
            view.close_about();
            view.help = false;
            view.set_focus(true);
            view.render(&snapshot);
            assert!(view.click(100, 233, &snapshot).is_none());
            assert_eq!(view.selected, Some(0), "covered list must not scroll");
        }
    }
}
