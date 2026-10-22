use super::*;

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
