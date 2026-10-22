use super::*;
use appearance::Skin;

#[test]
fn skin_selection_consumes_the_frame_without_playback_or_text_changes() {
    let snapshot = model::Snapshot {
        connected: true,
        playing: true,
        ..Default::default()
    };
    let mut actions = Vec::new();
    for mouse in [false, true] {
        let mut view = ui::View::default();
        view.input = "keep this URL".into();
        let events = [
            MouseInput::Button {
                button: MouseButton::Left,
                down: true,
                x: 150.,
                y: 153.,
            },
            MouseInput::Button {
                button: MouseButton::Left,
                down: false,
                x: 150.,
                y: 153.,
            },
        ];
        process_frame_input(
            &mut view,
            &snapshot,
            FrameInput {
                active: true,
                keys: if mouse {
                    &[(Key::Space, false, false)]
                } else {
                    &[(Key::F4, false, false), (Key::Space, false, false)]
                },
                mouse: if mouse { &events } else { &[] },
                chars: &['x'],
                ..Default::default()
            },
            &mut actions,
        );
        assert!(matches!(
            actions.as_slice(),
            [ui::Action::SetSkin(Skin::Modern)]
        ));
        assert_eq!(view.input, "keep this URL");
    }
}

#[test]
fn f4_respects_focus_and_about_and_toggles_both_ways() {
    let mut view = ui::View::default();
    let mut actions = Vec::new();
    for (active, about) in [(false, false), (true, true), (true, false)] {
        if about {
            view.open_about();
        } else {
            view.close_about();
        }
        process_frame_input(
            &mut view,
            &model::Snapshot::default(),
            FrameInput {
                active,
                keys: &[(Key::F4, false, false)],
                ..Default::default()
            },
            &mut actions,
        );
        if active && !about {
            assert!(matches!(
                actions.as_slice(),
                [ui::Action::SetSkin(Skin::Modern)]
            ));
            view.set_skin(Skin::Modern);
        } else {
            assert!(actions.is_empty());
        }
    }
    process_frame_input(
        &mut view,
        &model::Snapshot::default(),
        FrameInput {
            active: true,
            keys: &[(Key::F4, false, false)],
            ..Default::default()
        },
        &mut actions,
    );
    assert!(matches!(
        actions.as_slice(),
        [ui::Action::SetSkin(Skin::Classic)]
    ));
}

#[test]
fn command_line_skin_override_is_explicit_and_validated() {
    assert_eq!(requested_skin(&[]), Ok(None));
    for (value, skin) in [("classic", Skin::Classic), ("modern", Skin::Modern)] {
        assert_eq!(
            requested_skin(&["--skin".into(), value.into()]),
            Ok(Some(skin))
        );
    }
    for args in [
        vec!["--skin".into()],
        vec!["--skin".into(), "unknown".into()],
        vec!["--skin".into(), "--about".into()],
    ] {
        assert!(requested_skin(&args).is_err());
    }
}
