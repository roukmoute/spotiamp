use super::*;

fn state(connected: bool, busy: bool) -> Snapshot {
    Snapshot {
        connected,
        busy,
        ..Snapshot::default()
    }
}

#[test]
fn connection_control_distinguishes_connect_cancel_and_explicit_logout() {
    for (connected, busy, label) in [
        (false, false, "CONNEXION"),
        (false, true, "ANNULER"),
        (true, false, "DECONNECTER"),
        (true, true, "DECONNECTER"),
    ] {
        let s = state(connected, busy);
        let mut view = View::default();
        assert_eq!(
            ConnectionAction::for_state(&s).label(Language::French),
            label
        );
        assert!(view.pointer_down(470, 40, &s).is_none());
        let action = view.pointer_up(Some((470, 40)), &s);
        assert!(matches!(
            (connected, busy, action),
            (false, false, Some(Action::Send(Command::Connect)))
                | (false, true, Some(Action::Send(Command::CancelConnect)))
                | (true, _, Some(Action::Send(Command::Logout)))
        ));
        assert!(view.pointer_up(Some((470, 40)), &s).is_none());
    }
}

#[test]
fn asynchronous_auth_changes_cannot_turn_a_captured_press_into_another_action() {
    let states = [state(false, false), state(false, true), state(true, false)];
    for before in &states {
        for after in &states {
            if ConnectionAction::for_state(before) == ConnectionAction::for_state(after) {
                continue;
            }
            let mut view = View::default();
            view.pointer_down(470, 40, before);
            assert!(view.pointer_up(Some((470, 40)), after).is_none());
            assert!(view.pointer_up(Some((470, 40)), after).is_none());
        }
    }
}

#[test]
fn cancelling_a_connection_press_does_not_leave_a_pending_logout() {
    let s = state(true, false);
    let mut view = View::default();
    view.pointer_down(470, 40, &s);
    assert!(view.pointer_up(Some((400, 40)), &s).is_none());
    view.pointer_down(470, 40, &s);
    view.set_focus(false);
    view.set_focus(true);
    assert!(view.pointer_up(Some((470, 40)), &s).is_none());
    view.pointer_down(470, 40, &s);
    view.open_about();
    assert!(view.pointer_up(Some((470, 40)), &s).is_none());
}
