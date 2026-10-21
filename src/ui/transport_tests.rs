use super::*;
use crate::model::TrackRow;

fn active_state(playing: bool) -> Snapshot {
    Snapshot {
        connected: true,
        playing,
        current: Some(0),
        position_ms: 42_000,
        duration_ms: 180_000,
        queue: vec![TrackRow::default(); 3],
        ..Snapshot::default()
    }
}

fn button_position(button: Button) -> (usize, usize) {
    let (x, y, width, height) = button.rect();
    (x + width / 2, y + height / 2)
}

fn press_button(view: &mut View, button: Button, state: &Snapshot) -> Option<Action> {
    let position = button_position(button);
    assert!(view.pointer_down(position.0, position.1, state).is_none());
    let action = view.pointer_up(Some(position), state);
    assert!(view.pointer_up(Some(position), state).is_none());
    action
}

#[test]
fn play_button_uses_current_track_policy_even_with_another_row_selected() {
    for playing in [false, true] {
        let state = active_state(playing);
        let mut view = View::default();
        // A single click highlights another row without changing the current track.
        let other_row = (80, 221 + 2 * 24 + 12);
        assert!(
            view.pointer_down(other_row.0, other_row.1, &state)
                .is_none()
        );
        assert!(view.pointer_up(Some(other_row), &state).is_none());
        assert_eq!(view.selected, Some(2));

        // The engine owns resume/restart policy; this index is only its fallback
        // when stopped. PlayIndex here would incorrectly jump to the highlighted row.
        assert!(matches!(
            press_button(&mut view, Button::Play, &state),
            Some(Action::Send(Command::Play(2)))
        ));
        assert_eq!(view.selected, Some(2));
    }
}

#[test]
fn play_button_preserves_the_stopped_start_index_fallbacks() {
    for (selected, current, expected) in [
        (None, None, 0),
        (None, Some(1), 1),
        (Some(2), None, 2),
        (Some(2), Some(1), 2),
    ] {
        let state = Snapshot {
            current,
            position_ms: 0,
            ..active_state(false)
        };
        let mut view = View {
            selected,
            ..View::default()
        };
        assert!(matches!(
            press_button(&mut view, Button::Play, &state),
            Some(Action::Send(Command::Play(index))) if index == expected
        ));
    }
}

#[test]
fn play_press_cancels_outside_on_focus_loss_and_on_disconnect() {
    let state = active_state(true);
    let mut view = View::default();
    let position = button_position(Button::Play);

    view.pointer_down(position.0, position.1, &state);
    assert!(view.pointer_up(Some((280, 160)), &state).is_none());
    assert!(view.pointer_up(Some(position), &state).is_none());

    view.pointer_down(position.0, position.1, &state);
    view.set_focus(false);
    view.set_focus(true);
    assert!(view.pointer_up(Some(position), &state).is_none());

    view.pointer_down(position.0, position.1, &state);
    let disconnected = Snapshot {
        connected: false,
        ..state
    };
    assert!(view.pointer_up(Some(position), &disconnected).is_none());
}

#[test]
fn double_click_still_explicitly_starts_the_selected_row() {
    for playing in [false, true] {
        let state = active_state(playing);
        let mut view = View::default();
        let other_row = (80, 221 + 2 * 24 + 12);
        assert!(
            view.pointer_down(other_row.0, other_row.1, &state)
                .is_none()
        );
        assert!(view.pointer_up(Some(other_row), &state).is_none());
        assert!(matches!(
            view.pointer_down(other_row.0, other_row.1, &state),
            Some(Action::Send(Command::PlayIndex(2)))
        ));
        assert!(view.pointer_up(Some(other_row), &state).is_none());
    }
}

#[test]
fn pause_button_still_toggles_independently_of_row_selection() {
    for playing in [false, true] {
        let state = active_state(playing);
        let mut view = View {
            selected: Some(2),
            ..View::default()
        };
        assert!(matches!(
            press_button(&mut view, Button::Pause, &state),
            Some(Action::Send(Command::Toggle))
        ));
    }
}
