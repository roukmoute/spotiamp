use super::*;
use crate::model::TrackRow;

fn queue() -> Snapshot {
    Snapshot {
        queue: (0..12)
            .map(|i| TrackRow {
                uri: format!("spotify:track:{i}"),
                ..Default::default()
            })
            .collect(),
        current: Some(2),
        playing: true,
        position_ms: 123_456,
        ..Default::default()
    }
}

#[test]
fn queue_drag_draws_the_insertion_marker_in_both_skins_without_idle_redraws() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut view = View::default();
        view.set_skin(skin);
        let s = queue();
        view.pointer_down(80, 233, &s);
        view.poll_queue_drag(Some((80, 290)), &s);
        assert_eq!(view.drop_marker, Some(3));
        // The library sends this every frame even when it has no captured drag.
        view.show_queue_drop(None, s.queue.len());
        assert_eq!(view.drop_marker, Some(3));
        view.render(&s);
        assert_eq!(
            view.pixels[293 * WIDTH + 80],
            if skin == Skin::Modern {
                0x55e4ff
            } else {
                GREEN
            }
        );
        let before = view.fingerprint(&s);
        view.poll_queue_drag(Some((80, 290)), &s);
        assert_eq!(view.fingerprint(&s), before);
        assert!(matches!(
            view.pointer_up(Some((80, 290)), &s),
            Some(Action::Send(Command::ReorderQueue { .. }))
        ));
        assert_eq!(view.drop_marker, None);
    }
}

#[test]
fn queue_drag_autoscrolls_at_each_edge_and_stops_when_pointer_leaves() {
    let mut view = View::default();
    let s = queue();
    view.pointer_down(80, 233, &s);
    view.poll_queue_drag(Some((80, 406)), &s);
    assert_eq!(view.offset, 0);
    view.drop_scroll_tick = Some(Instant::now() - Duration::from_millis(251));
    view.poll_queue_drag(Some((80, 406)), &s);
    assert_eq!(view.offset, 1);
    assert_eq!(view.drop_marker, Some(9));
    view.poll_queue_drag(Some((80, 224)), &s);
    view.drop_scroll_tick = Some(Instant::now() - Duration::from_millis(251));
    view.poll_queue_drag(Some((80, 224)), &s);
    assert_eq!(view.offset, 0);
    view.poll_queue_drag(Some((80, 450)), &s);
    assert_eq!(view.drop_marker, None);
    assert_eq!(view.drop_scroll_tick, None);
}

#[test]
fn queued_reorder_remaps_the_selected_duplicate_occurrence_after_acceptance() {
    let mut s = queue();
    s.queue[4].uri = s.queue[2].uri.clone();
    let mut view = View::default();
    view.pointer_down(80, 281, &s); // The first occurrence of that track.
    let action = view.pointer_up(Some((80, 368)), &s);
    let Some(Action::Send(Command::ReorderQueue {
        request_id,
        expected_queue,
        indices,
        insert_before,
    })) = action
    else {
        panic!("reorder expected")
    };
    assert_eq!(indices, [2]);
    assert_eq!(insert_before, 6);
    assert_eq!(
        expected_queue,
        s.queue
            .iter()
            .map(|row| row.uri.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        view.selected,
        Some(2),
        "do not move the highlight in the old snapshot"
    );
    view.sync_playback(&s);
    assert_eq!(view.selected, Some(2));
    let order = crate::model::reorder_indices(s.queue.len(), &indices, insert_before).unwrap();
    s.queue = order.iter().map(|&index| s.queue[index].clone()).collect();
    s.current = Some(5); // Engine remaps the same playing occurrence.
    s.queue_reorder_request = request_id;
    assert!(
        !view.sync_playback(&s),
        "reordering is not a playback start"
    );
    assert_eq!(
        view.selected,
        Some(5),
        "the moved occurrence must stay selected"
    );
    assert_eq!(s.current, Some(5));
    assert_eq!(s.position_ms, 123_456);
    assert_eq!(view.offset, 0);
    assert!(view.pending_queue_reorder.is_none());
}

#[test]
fn a_changed_queue_cancels_the_drag_and_a_rejected_order_cannot_later_move_selection() {
    let mut view = View::default();
    let mut s = queue();
    view.pointer_down(80, 233, &s);
    s.queue.swap(3, 4);
    assert!(view.pointer_up(Some((80, 330)), &s).is_none());
    assert!(view.queue_drag.is_none());
    assert!(view.last_click.is_none());
    view.pointer_down(80, 233, &s);
    assert!(view.pointer_up(Some((80, 330)), &s).is_some());
    s.queue.pop();
    view.sync_playback(&s);
    assert!(view.pending_queue_reorder.is_none());
    assert_eq!(view.selected, Some(0));
}

#[test]
fn queue_drag_cancelled_by_overlay_or_selection_navigation_never_reorders() {
    for block in 0..5 {
        let mut view = View::default();
        let s = queue();
        view.pointer_down(80, 233, &s);
        view.poll_queue_drag(Some((80, 330)), &s);
        match block {
            0 => view.open_about(),
            1 => view.help = true,
            2 => view.set_focus(false),
            3 => view.move_selection(1, s.queue.len()),
            _ => view.set_skin(Skin::Modern),
        }
        assert!(view.pointer_up(Some((80, 330)), &s).is_none());
        assert!(view.queue_drag.is_none());
        assert!(view.drop_marker.is_none());
    }
}

#[test]
fn new_pointer_selection_wins_over_delayed_reorder_acknowledgement() {
    let mut view = View::default();
    let mut s = queue();
    view.pointer_down(80, 233, &s);
    view.pointer_up(Some((80, 330)), &s);
    view.pointer_down(80, 257, &s);
    view.pointer_up(Some((80, 257)), &s);
    let order = crate::model::reorder_indices(s.queue.len(), &[0], 5).unwrap();
    s.queue = order.iter().map(|&i| s.queue[i].clone()).collect();
    view.sync_playback(&s);
    assert_eq!(view.selected, Some(1));
}

#[test]
fn identical_duplicate_uris_require_the_exact_acknowledgement_before_remapping() {
    let mut s = queue();
    for row in &mut s.queue {
        row.uri = "spotify:track:duplicate".into();
    }
    s.queue_reorder_request = 1;
    let mut view = View::default();
    view.pointer_down(80, 281, &s);
    let Some(Action::Send(Command::ReorderQueue { request_id, .. })) =
        view.pointer_up(Some((80, 368)), &s)
    else {
        panic!("reorder expected");
    };
    assert_ne!(request_id, s.queue_reorder_request);
    view.sync_playback(&s);
    assert_eq!(
        view.selected,
        Some(2),
        "equal URIs do not acknowledge a move"
    );
    s.queue_reorder_request = request_id.wrapping_add(1);
    view.sync_playback(&s);
    assert_eq!(
        view.selected,
        Some(2),
        "another operation cannot acknowledge it"
    );
    s.queue_reorder_request = request_id;
    view.sync_playback(&s);
    assert_eq!(view.selected, Some(5));
}

#[test]
fn keyboard_selection_and_send_failure_both_cancel_pending_highlight_remap() {
    for rejected in [false, true] {
        let mut s = queue();
        let mut view = View::default();
        view.pointer_down(80, 233, &s);
        let Some(Action::Send(Command::ReorderQueue {
            request_id,
            indices,
            insert_before,
            ..
        })) = view.pointer_up(Some((80, 330)), &s)
        else {
            panic!("reorder expected");
        };
        if rejected {
            view.reject_queue_reorder(request_id);
        } else {
            view.move_selection(1, s.queue.len());
        }
        let selected = view.selected;
        let order = crate::model::reorder_indices(s.queue.len(), &indices, insert_before).unwrap();
        s.queue = order.iter().map(|&i| s.queue[i].clone()).collect();
        s.queue_reorder_request = request_id;
        view.sync_playback(&s);
        assert_eq!(view.selected, selected);
        assert!(view.pending_queue_reorder.is_none());
    }
}
