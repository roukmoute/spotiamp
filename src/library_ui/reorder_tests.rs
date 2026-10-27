use super::tests::snapshot;
use super::*;

fn setup() -> (View, LibrarySnapshot) {
    let mut s = snapshot(2, 30);
    s.selected_owned = true;
    s.selected_playlist_revision = Arc::from([1, 2, 3]);
    let mut view = View::default();
    view.render(&s);
    view.choose_track(1, false, false, &s);
    view.choose_track(4, true, false, &s);
    (view, s)
}

fn source(view: &mut View, s: &LibrarySnapshot) -> ReorderDrag {
    assert!(view.pointer_down(320, 156, s).is_none());
    let source = view.capture_reorder_drag(s).expect("owned playlist drag");
    view.cancel_pointer();
    source
}

#[test]
fn reorder_capture_keeps_selected_occurrences_and_uses_gap_in_original_list() {
    for skin in [Skin::Classic, Skin::Modern] {
        let (mut view, s) = setup();
        view.set_skin(skin);
        let source = source(&mut view, &s);
        assert_eq!(source.indices, [1, 4]);
        let Some(Action::Command(LibraryCommand::ReorderPlaylist {
            id,
            expected_tracks,
            expected_revision,
            indices,
            insert_before,
        })) = view.finish_reorder_drag(source, Some((320.0, 265.0)), &s)
        else {
            panic!("reorder command")
        };
        assert_eq!(Some(id), s.selected_id);
        assert!(Arc::ptr_eq(&expected_tracks, &s.tracks));
        assert!(Arc::ptr_eq(
            &expected_revision,
            &s.selected_playlist_revision
        ));
        assert_eq!(indices, [1, 4]);
        assert_eq!(insert_before, 6);
        assert_eq!(view.playlist_choices.indices(&s.tracks), [1, 4]);
        assert!(view.capture_reorder_drag(&s).is_none());
    }
}

#[test]
fn reorder_requires_owned_ready_source_and_rejects_stale_snapshot_or_overlay() {
    for blocked in 0..10 {
        let (mut view, mut s) = setup();
        let source = source(&mut view, &s);
        match blocked {
            0 => s.selected_owned = false,
            1 => s.connected = false,
            2 => s.loading = true,
            3 => s.busy = true,
            4 => s.playlist_write_busy = true,
            5 => s.selected_playlist_revision = Arc::from([9]),
            6 => Arc::make_mut(&mut s.tracks).swap(0, 1),
            7 => view.open_search(false),
            8 => view.set_focus(false),
            9 => view.context_menu_at(320, 156, &s),
            _ => unreachable!(),
        }
        s.revision += 1;
        view.show_reorder_drop(Some(&source), Some((320.0, 265.0)), &s);
        assert_eq!(view.reorder_gap, None, "blocked {blocked}");
        assert!(
            view.finish_reorder_drag(source, Some((320.0, 265.0)), &s)
                .is_none(),
            "blocked {blocked}"
        );
        assert!(view.pending_reorder.is_none());
    }
}

#[test]
fn insertion_marker_is_static_between_moves_and_edge_scroll_is_paced_and_clamped() {
    for skin in [Skin::Classic, Skin::Modern] {
        let (mut view, s) = setup();
        view.set_skin(skin);
        let source = source(&mut view, &s);
        let start = Instant::now();
        view.show_reorder_drop_at(Some(&source), Some((320.0, 265.0)), &s, start);
        assert_eq!(view.reorder_gap, Some(6));
        assert!(view.render(&s));
        let pixels = view.pixels.as_ptr();
        view.show_reorder_drop_at(
            Some(&source),
            Some((320.0, 265.0)),
            &s,
            start + Duration::from_secs(1),
        );
        assert!(!view.render(&s));
        view.show_reorder_drop_at(
            Some(&source),
            Some((320.0, 380.0)),
            &s,
            start + Duration::from_secs(2),
        );
        assert_eq!(view.right_offset, 0);
        view.show_reorder_drop_at(
            Some(&source),
            Some((320.0, 380.0)),
            &s,
            start + Duration::from_millis(2100),
        );
        assert_eq!(view.right_offset, 0);
        view.show_reorder_drop_at(
            Some(&source),
            Some((320.0, 380.0)),
            &s,
            start + Duration::from_millis(2110),
        );
        assert_eq!((view.right_offset, view.reorder_gap), (1, Some(12)));
        view.right_offset = 19;
        view.show_reorder_drop_at(
            Some(&source),
            Some((320.0, 380.0)),
            &s,
            start + Duration::from_secs(3),
        );
        assert_eq!((view.right_offset, view.reorder_gap), (19, Some(30)));
        view.show_reorder_drop_at(
            Some(&source),
            Some((320.0, 120.0)),
            &s,
            start + Duration::from_secs(4),
        );
        view.show_reorder_drop_at(
            Some(&source),
            Some((320.0, 120.0)),
            &s,
            start + Duration::from_millis(4110),
        );
        assert_eq!((view.right_offset, view.reorder_gap), (18, Some(18)));
        view.show_reorder_drop(Some(&source), Some((200.0, 120.0)), &s);
        assert!(view.reorder_gap.is_none() && view.reorder_scroll.is_none());
        assert_eq!(view.pixels.as_ptr(), pixels);
    }
}

#[test]
fn no_op_and_outside_drops_emit_nothing_and_metadata_only_updates_preserve_drag() {
    let (mut view, mut s) = setup();
    view.choose_track(1, false, false, &s);
    let source = source(&mut view, &s);
    for point in [
        (320.0, 144.0),
        (320.0, 168.0),
        (220.0, 180.0),
        (320.0, 400.0),
    ] {
        assert!(
            view.finish_reorder_drag(source.clone(), Some(point), &s)
                .is_none()
        );
        assert!(view.pending_reorder.is_none());
    }
    Arc::make_mut(&mut s.tracks)[1].title = "Fresh metadata".into();
    s.revision += 1;
    assert!(matches!(
        view.finish_reorder_drag(source, Some((320.0, 120.0)), &s),
        Some(Action::Command(LibraryCommand::ReorderPlaylist {
            insert_before: 0,
            ..
        }))
    ));
}

fn complete(view: &mut View, s: &mut LibrarySnapshot, acknowledged: bool) {
    let pending = view.pending_reorder.as_ref().unwrap();
    let order = pending.order.clone();
    let revision: Arc<[u8]> = Arc::from([4, 5, 6]);
    if acknowledged {
        s.playlist_reorder_result = Some(Arc::new(crate::model::PlaylistReorderResult {
            id: pending.source.id.clone(),
            previous_revision: pending.source.revision.clone(),
            revision: revision.clone(),
            order: order.clone(),
        }));
    }
    s.tracks = Arc::new(order.iter().map(|old| s.tracks[*old].clone()).collect());
    s.selected_playlist_revision = revision;
    s.playlist_remove_revision += 1;
    s.revision += 1;
    view.render(s);
}

#[test]
fn confirmed_reorder_remaps_duplicate_occurrences_only_after_matching_ack_and_reload() {
    for acknowledged in [false, true] {
        let (mut view, mut s) = setup();
        let duplicate = s.tracks[1].clone();
        Arc::make_mut(&mut s.tracks)[4] = duplicate;
        s.revision += 1;
        view.render(&s);
        view.choose_track(1, false, false, &s);
        view.choose_track(4, true, false, &s);
        let source = source(&mut view, &s);
        assert!(
            view.finish_reorder_drag(source, Some((320.0, 265.0)), &s)
                .is_some()
        );
        complete(&mut view, &mut s, acknowledged);
        assert!(view.pending_reorder.is_none());
        if acknowledged {
            assert_eq!(view.playlist_choices.indices(&s.tracks), [4, 5]);
            assert_eq!(view.selected_track, Some(5));
        } else {
            assert_eq!(view.playlist_choices.len(), 0);
            assert_eq!(view.selected_track, None);
        }
    }
}

#[test]
fn reorder_rejection_failure_navigation_and_later_selection_do_not_restore_old_choice() {
    for case in 0..5 {
        let (mut view, mut s) = setup();
        let source = source(&mut view, &s);
        let Some(Action::Command(command)) =
            view.finish_reorder_drag(source, Some((320.0, 265.0)), &s)
        else {
            panic!("reorder")
        };
        match case {
            0 => {
                view.command_rejected(&LibraryCommand::Refresh);
                assert!(view.pending_reorder.is_some());
                view.command_rejected(&command);
            }
            1 => {
                s.playlist_remove_revision += 1;
                s.revision += 1;
                view.render(&s);
            }
            2 => {
                s.connected = false;
                s.revision += 1;
                view.render(&s);
            }
            3 => {
                s.selected_id = Some(s.entries[1].id.clone());
                s.revision += 1;
                view.render(&s);
            }
            4 => {
                view.choose_track(8, false, false, &s);
                complete(&mut view, &mut s, true);
                assert_eq!(view.playlist_choices.len(), 0);
            }
            _ => unreachable!(),
        }
        assert!(view.pending_reorder.is_none(), "case {case}");
    }
}

#[test]
fn acknowledged_move_waits_for_reloaded_rows_and_distinguishes_identical_occurrences() {
    for failed in [false, true] {
        let (mut view, mut s) = setup();
        let duplicate = s.tracks[0].clone();
        for row in Arc::make_mut(&mut s.tracks) {
            *row = duplicate.clone();
        }
        s.revision += 1;
        view.render(&s);
        view.choose_track(1, false, false, &s);
        view.choose_track(4, true, false, &s);
        let source = source(&mut view, &s);
        let rows = source.tracks.clone();
        view.right_offset = 19;
        assert!(
            view.finish_reorder_drag(source, Some((320.0, 265.0)), &s)
                .is_some()
        );
        let pending = view.pending_reorder.as_ref().unwrap();
        s.playlist_reorder_result = Some(Arc::new(crate::model::PlaylistReorderResult {
            id: pending.source.id.clone(),
            previous_revision: pending.source.revision.clone(),
            revision: Arc::from([7, 8, 9]),
            order: pending.order.clone(),
        }));
        s.playlist_remove_revision += 1;
        s.revision += 1;
        view.render(&s);
        assert!(
            view.pending_reorder.is_some(),
            "ACK alone is not a refreshed playlist"
        );
        s.loading = true;
        s.selected_owned = false;
        s.selected_playlist_revision = Arc::from([]);
        s.tracks = Arc::new(Vec::new());
        s.revision += 1;
        view.render(&s);
        assert!(view.pending_reorder.is_some());
        s.loading = false;
        s.revision += 1;
        if failed {
            s.playlist_reorder_result = None;
        } else {
            s.selected_owned = true;
            s.selected_playlist_revision = Arc::from([7, 8, 9]);
            // URI sequence is literally unchanged: only explicit occurrence
            // acknowledgement can prove which duplicate has moved.
            s.tracks = rows;
        }
        view.render(&s);
        assert!(view.pending_reorder.is_none());
        if failed {
            assert_eq!(view.playlist_choices.len(), 0);
        } else {
            assert_eq!(view.playlist_choices.indices(&s.tracks), [23, 24]);
            assert_eq!(view.right_offset, 19);
            assert_eq!(view.selected_track, Some(24));
        }
    }
}
