use super::*;
use crate::model::{PlaybackOrigin, TrackRow};

fn queue() -> Snapshot {
    Snapshot {
        queue: (0..20)
            .map(|index| TrackRow {
                uri: format!("spotify:track:{index}"),
                title: format!("Track {index}"),
                ..Default::default()
            })
            .collect(),
        current: Some(2),
        playing: true,
        position_ms: 23_456,
        ..Default::default()
    }
}

fn click(view: &mut View, index: usize, control: bool, shift: bool, s: &Snapshot) {
    let y = 221 + (index - view.offset) * 24 + 12;
    assert!(
        view.pointer_down_with_modifiers(80, y, control, shift, s)
            .is_none()
    );
    assert!(view.pointer_up(Some((80, y)), s).is_none());
}

#[test]
fn queue_control_shift_and_additive_range_keep_the_focus_separate_from_the_group() {
    let s = queue();
    let mut view = View::default();
    click(&mut view, 1, false, false, &s);
    click(&mut view, 3, true, false, &s);
    assert_eq!(view.selected_queue_indices(&s), [1, 3]);
    click(&mut view, 5, true, true, &s);
    assert_eq!(view.selected_queue_indices(&s), [1, 3, 4, 5]);
    click(&mut view, 4, false, true, &s);
    assert_eq!(view.selected_queue_indices(&s), [3, 4]);
    click(&mut view, 4, true, false, &s);
    assert_eq!(view.selected_queue_indices(&s), [3]);
    assert_eq!(view.selected, Some(4));
    click(&mut view, 3, true, false, &s);
    assert!(view.selected_queue_indices(&s).is_empty());
    assert_eq!(view.selected, Some(3));
    // Repeated modifiers can never become playback double-clicks.
    click(&mut view, 3, true, false, &s);
    click(&mut view, 3, true, false, &s);
    assert!(view.last_click.is_none());
}

#[test]
fn plain_press_keeps_a_group_for_drag_and_plain_release_collapses_it() {
    let s = queue();
    let mut view = View::default();
    click(&mut view, 1, false, false, &s);
    click(&mut view, 3, true, false, &s);
    view.pointer_down(80, 257, &s);
    assert_eq!(view.selected_queue_indices(&s), [1, 3]);
    assert!(view.pointer_up(Some((80, 257)), &s).is_none());
    assert_eq!(view.selected_queue_indices(&s), [1]);
    assert!(matches!(view.pointer_down(80, 257, &s), None));
    assert!(matches!(
        view.pointer_up(Some((80, 257)), &s),
        Some(Action::Send(Command::PlayIndex(1)))
    ));
    click(&mut view, 3, true, false, &s);
    click(&mut view, 5, false, false, &s);
    assert_eq!(view.selected_queue_indices(&s), [5]);
}

#[test]
fn remove_requires_a_chosen_focus_row_and_never_removes_a_deselected_track() {
    for skin in [Skin::Classic, Skin::Modern] {
        let s = queue();
        let mut view = View::default();
        view.set_skin(skin);
        click(&mut view, 1, false, false, &s);
        click(&mut view, 3, true, false, &s);
        assert_eq!(view.queue_removal_index(&s), Some(3));
        click(&mut view, 3, true, false, &s);
        assert_eq!(view.selected, Some(3));
        assert_eq!(view.selected_queue_indices(&s), [1]);
        assert_eq!(view.queue_removal_index(&s), None);
        assert!(!Button::Remove.enabled(&view, &s));
        assert!(view.click(50, 425, &s).is_none());
        view.pointer_down(50, 425, &s);
        assert!(view.pressed_button.is_none());
        assert!(view.pointer_up(Some((50, 425)), &s).is_none());
        click(&mut view, 1, true, false, &s);
        assert!(view.selected_queue_indices(&s).is_empty());
        assert_eq!(view.queue_removal_index(&s), None);
        assert!(view.click(50, 425, &s).is_none());
        click(&mut view, 2, false, false, &s);
        view.move_queue_selection(1, true, false, &s);
        assert_eq!(view.selected_queue_indices(&s), [2]);
        assert_eq!(view.selected, Some(3));
        assert_eq!(view.queue_removal_index(&s), None);
        view.move_queue_selection(-1, true, false, &s);
        assert_eq!(view.queue_removal_index(&s), Some(2));
        assert!(Button::Remove.enabled(&view, &s));
        view.pointer_down(50, 425, &s);
        assert!(matches!(
            view.pointer_up(Some((50, 425)), &s),
            Some(Action::Send(Command::Remove(2)))
        ));
        view.render(&s);
    }
}

#[test]
fn keyboard_range_select_all_and_focus_navigation_keep_selection_and_viewport_consistent() {
    let s = queue();
    let mut view = View::default();
    view.move_queue_selection(1, false, false, &s);
    view.move_queue_selection(3, false, true, &s);
    assert_eq!(view.selected_queue_indices(&s), [0, 1, 2, 3]);
    view.move_queue_selection(2, true, false, &s);
    assert_eq!(view.selected, Some(5));
    assert_eq!(view.selected_queue_indices(&s), [0, 1, 2, 3]);
    view.move_queue_selection(1, true, true, &s);
    assert_eq!(view.selected_queue_indices(&s), [0, 1, 2, 3, 4, 5, 6]);
    view.move_queue_selection(i32::MAX, false, true, &s);
    assert_eq!(view.selected_queue_indices(&s), (0..20).collect::<Vec<_>>());
    assert_eq!((view.selected, view.offset), (Some(19), 12));
    view.move_queue_selection(i32::MIN, false, false, &s);
    assert_eq!(view.selected_queue_indices(&s), [0]);
    view.select_all_queue(&s);
    assert_eq!(view.selected_queue_indices(&s).len(), 20);
    view.move_queue_selection(1, false, false, &s);
    assert_eq!(view.selected_queue_indices(&s), [1]);
    view.input_active = true;
    view.select_all_queue(&s);
    view.move_queue_selection(1, false, true, &s);
    assert_eq!(view.selected_queue_indices(&s), [1]);
    view.input_active = false;
    view.move_queue_selection(1, false, true, &Snapshot::default());
    assert_eq!(view.selected, None);
}

#[test]
fn moving_a_disjoint_group_remaps_every_duplicate_position_and_anchor_after_exact_ack() {
    let mut s = queue();
    s.queue[4].uri = s.queue[2].uri.clone();
    let mut view = View::default();
    click(&mut view, 2, false, false, &s);
    click(&mut view, 4, true, false, &s);
    view.pointer_down(80, 281, &s);
    let Some(Action::Send(Command::ReorderQueue {
        request_id,
        indices,
        insert_before,
        ..
    })) = view.pointer_up(Some((80, 389)), &s)
    else {
        panic!("expected group reorder")
    };
    assert_eq!(indices, [2, 4]);
    assert_eq!(insert_before, 7);
    assert_eq!(view.selected_queue_indices(&s), [2, 4]);
    let order = crate::model::reorder_indices(s.queue.len(), &indices, insert_before).unwrap();
    s.queue = order.iter().map(|&index| s.queue[index].clone()).collect();
    s.current = Some(5);
    s.queue_reorder_request = request_id;
    view.sync_playback(&s);
    assert_eq!(view.selected_queue_indices(&s), [5, 6]);
    assert_eq!(view.selected, Some(5));
    assert_eq!(view.queue_choices.as_ref().unwrap().anchor, Some(6));
    assert_eq!(s.position_ms, 23_456);
    view.move_queue_selection(2, false, true, &s);
    assert_eq!(
        view.selected_queue_indices(&s),
        [6, 7],
        "Shift must use the remapped anchor, not its old position"
    );
}

#[test]
fn duplicate_only_groups_wait_for_ack_and_a_new_choice_wins_over_old_pending_selection() {
    let mut s = queue();
    for row in &mut s.queue {
        row.uri = "spotify:track:duplicate".into();
    }
    let mut view = View::default();
    click(&mut view, 1, false, false, &s);
    click(&mut view, 3, true, false, &s);
    view.pointer_down(80, 257, &s);
    let Some(Action::Send(Command::ReorderQueue { request_id, .. })) =
        view.pointer_up(Some((80, 389)), &s)
    else {
        panic!("expected reorder")
    };
    view.sync_playback(&s);
    assert_eq!(view.selected_queue_indices(&s), [1, 3]);
    s.queue_reorder_request = request_id + 1;
    view.sync_playback(&s);
    assert_eq!(view.selected_queue_indices(&s), [1, 3]);
    click(&mut view, 0, false, false, &s);
    click(&mut view, 2, true, false, &s);
    s.queue_reorder_request = request_id;
    view.sync_playback(&s);
    assert_eq!(view.selected_queue_indices(&s), [0, 2]);
    assert_eq!(view.selected, Some(2));
}

#[test]
fn cancelled_noop_and_invalid_group_drags_never_change_playback_or_dispatch() {
    let mut s = queue();
    let mut view = View::default();
    click(&mut view, 1, false, false, &s);
    click(&mut view, 3, false, true, &s);
    view.pointer_down(80, 281, &s);
    assert!(view.pointer_up(Some((100, 281)), &s).is_none());
    assert_eq!(view.selected_queue_indices(&s), [1, 2, 3]);
    view.pointer_down(80, 281, &s);
    view.cancel_pointer();
    assert!(view.pointer_up(Some((80, 389)), &s).is_none());
    assert_eq!(view.selected_queue_indices(&s), [1, 2, 3]);
    view.pointer_down(80, 281, &s);
    s.queue.swap(8, 9);
    assert!(view.pointer_up(Some((80, 389)), &s).is_none());
    // Toggling a row off does not start dragging the other selected rows.
    view.pointer_down_with_modifiers(80, 281, true, false, &s);
    assert!(view.pointer_up(Some((80, 389)), &s).is_none());
    assert_eq!(view.selected_queue_indices(&s), [1, 3]);
    assert_eq!(
        (s.current, s.position_ms, s.playing),
        (Some(2), 23_456, true)
    );
}

#[test]
fn automatic_playback_preserves_the_group_while_explicit_playback_selects_its_target() {
    let mut s = queue();
    s.playback_revision = 1;
    let mut view = View::default();
    view.sync_playback(&s);
    click(&mut view, 4, true, false, &s);
    s.playback_origin = PlaybackOrigin::Automatic;
    s.current = Some(8);
    s.playback_revision += 1;
    view.sync_playback(&s);
    assert_eq!(view.selected_queue_indices(&s), [2, 4]);
    assert_eq!((view.selected, view.offset), (Some(4), 4));
    s.playback_origin = PlaybackOrigin::Explicit;
    s.playback_revision += 1;
    view.sync_playback(&s);
    assert_eq!(view.selected_queue_indices(&s), [8]);
    assert_eq!(view.selected, Some(8));
}

#[test]
fn both_skins_render_all_chosen_rows_and_metadata_ticks_reuse_selection_storage() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut s = queue();
        let mut view = View::default();
        view.set_skin(skin);
        click(&mut view, 1, false, false, &s);
        click(&mut view, 3, true, false, &s);
        view.render(&s);
        assert_eq!(
            view.pixels[246 * WIDTH + 400],
            view.pixels[294 * WIDTH + 400]
        );
        assert_ne!(
            view.pixels[246 * WIDTH + 400],
            view.pixels[270 * WIDTH + 400]
        );
        let choices = view.queue_choices.as_ref().unwrap();
        let address = choices.rows.as_ptr();
        let capacity = choices.rows.capacity();
        let fingerprint = view.fingerprint(&s);
        view.sync_playback(&s);
        view.render(&s);
        assert_eq!(view.fingerprint(&s), fingerprint);
        s.queue[1].title = "Updated metadata".into();
        s.queue.push(TrackRow::default());
        view.sync_playback(&s);
        assert_eq!(view.selected_queue_indices(&s), [1, 3]);
        let choices = view.queue_choices.as_ref().unwrap();
        assert_eq!(choices.rows.as_ptr(), address);
        assert_eq!(choices.rows.capacity(), capacity);
        // An unrelated external replacement cannot drag the previous identities.
        s.queue[1].uri = "spotify:track:replacement".into();
        assert!(view.selected_queue_indices(&s).is_empty());
        view.sync_playback(&s);
        assert_eq!(view.selected_queue_indices(&s), [3]);
    }
}

#[test]
#[ignore = "Exports a queue multiselection preview when SPOTIAMP_QUEUE_MULTI_PREVIEW is set"]
fn export_queue_multiselection_preview() {
    let destination = std::env::var("SPOTIAMP_QUEUE_MULTI_PREVIEW").expect("preview destination");
    let mut s = queue();
    s.connected = true;
    s.duration_ms = 210_000;
    s.volume = 44_000;
    for (row, (title, artist)) in s.queue.iter_mut().zip([
        ("Around the World", "Daft Punk"),
        ("Midnight City", "M83"),
        ("Nightcall", "Kavinsky"),
        ("Digital Love", "Daft Punk"),
        ("Instant Crush", "Daft Punk"),
        ("A Real Hero", "College"),
        ("D.A.N.C.E.", "Justice"),
        ("Safe and Sound", "Justice"),
    ]) {
        row.title = title.into();
        row.artist = artist.into();
        row.duration_ms = 210_000;
    }
    let mut view = View::default();
    if std::env::var("SPOTIAMP_QUEUE_MULTI_SKIN").as_deref() == Ok("modern") {
        view.set_skin(Skin::Modern);
    }
    click(&mut view, 1, false, false, &s);
    click(&mut view, 3, true, false, &s);
    click(&mut view, 4, true, false, &s);
    view.pointer_down(80, 257, &s);
    view.poll_queue_drag(Some((450, 389)), &s);
    view.render(&s);
    view.save_ppm(&destination).unwrap();
}
