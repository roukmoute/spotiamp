use super::tests::snapshot;
use super::*;

fn setup() -> (View, LibrarySnapshot) {
    let mut state = snapshot(3, 8);
    state.selected_owned = true;
    state.selected_playlist_revision = Arc::from([1, 2, 3]);
    let mut view = View::default();
    view.render(&state);
    view.left_focus = false;
    view.choose_track(2, false, false, &state);
    view.choose_track(5, true, false, &state);
    (view, state)
}

fn remove(view: &mut View, state: &LibrarySnapshot) -> Option<Action> {
    view.key(Key::Delete, false, false, state)
}

#[test]
fn delete_captures_only_selected_occurrences_and_shared_playlist_revision_without_playing() {
    for skin in [Skin::Classic, Skin::Modern] {
        let (mut view, mut state) = setup();
        view.set_skin(skin);
        Arc::make_mut(&mut state.tracks)[5].uri = state.tracks[2].uri.clone();
        state.revision += 1;
        view.render(&state);
        view.choose_track(2, false, false, &state);
        view.choose_track(5, true, false, &state);
        let before = state.tracks.clone();
        let Some(Action::Command(LibraryCommand::RemoveFromPlaylist {
            id,
            expected_tracks,
            expected_revision,
            indices,
        })) = remove(&mut view, &state)
        else {
            panic!("expected playlist removal");
        };
        assert_eq!(Some(id), state.selected_id);
        assert_eq!(indices, [2, 5]);
        assert!(Arc::ptr_eq(&expected_tracks, &state.tracks));
        assert!(Arc::ptr_eq(
            &expected_revision,
            &state.selected_playlist_revision
        ));
        assert_eq!(state.tracks, before);
        assert_eq!(view.playlist_choices.len(), 2);
        assert!(remove(&mut view, &state).is_none());
        view.key(Key::Down, false, false, &state);
        assert!(
            remove(&mut view, &state).is_none(),
            "changing selection must not bypass pending write"
        );
    }
}

#[test]
fn delete_needs_owned_fresh_available_playlist_and_title_panel_focus() {
    for blocked in 0..12 {
        let (mut view, mut state) = setup();
        match blocked {
            0 => state.selected_owned = false,
            1 => state.connected = false,
            2 => state.loading = true,
            3 => state.busy = true,
            4 => state.playlist_write_busy = true,
            5 => view.left_focus = true,
            6 => view.search_mode = true,
            7 => view.set_focus(false),
            8 => state.selected_playlist_revision = Arc::from([]),
            9 => {
                view.playlist_choices.clear();
            }
            10 => Arc::make_mut(&mut state.entries)[0].available = false,
            11 => {
                state.selected_id = Some(state.entries[1].id.clone());
                state.loading = true;
            }
            _ => unreachable!(),
        }
        state.revision += 1;
        assert!(
            remove(&mut view, &state).is_none(),
            "blocked case {blocked}"
        );
        assert!(view.pending_removal.is_none());
    }
    for (control, shift) in [(true, false), (false, true), (true, true)] {
        let (mut view, state) = setup();
        assert!(view.key(Key::Delete, control, shift, &state).is_none());
        assert!(view.pending_removal.is_none());
    }
}

#[test]
fn delete_in_query_picker_or_name_editor_never_removes_spotify_tracks() {
    let (mut view, state) = setup();
    view.open_search(true);
    view.paste("Jazz");
    view.key(Key::Home, false, false, &state);
    assert!(remove(&mut view, &state).is_none());
    assert_eq!(view.query, "azz");
    assert!(view.pending_removal.is_none());

    let (mut view, state) = setup();
    view.open_picker(&state);
    assert!(remove(&mut view, &state).is_none());
    view.new_playlist();
    view.paste("Jazz");
    view.key(Key::Home, false, false, &state);
    assert!(remove(&mut view, &state).is_none());
    assert_eq!(view.picker.as_ref().unwrap().name, "azz");
    assert!(view.pending_removal.is_none());
}

#[test]
fn playlist_revision_change_discards_old_occurrence_selection_but_metadata_does_not() {
    let (mut view, mut state) = setup();
    Arc::make_mut(&mut state.tracks)[2].title = "Metadata refreshed".into();
    state.revision += 1;
    view.render(&state);
    assert_eq!(view.playlist_choices.len(), 2);
    state.selected_playlist_revision = Arc::from([4, 5, 6]);
    state.revision += 1;
    assert!(remove(&mut view, &state).is_none());
    assert_eq!(view.playlist_choices.len(), 0);
    view.choose_track(2, false, false, &state);
    assert!(matches!(
        remove(&mut view, &state),
        Some(Action::Command(LibraryCommand::RemoveFromPlaylist { .. }))
    ));
}

#[test]
fn unrelated_snapshot_or_channel_rejection_cannot_release_pending_removal() {
    let (mut view, mut state) = setup();
    let Some(Action::Command(command)) = remove(&mut view, &state) else {
        panic!("removal");
    };
    state.revision += 1;
    Arc::make_mut(&mut state.tracks)[0].title = "Fresh title".into();
    view.render(&state);
    assert!(remove(&mut view, &state).is_none());
    view.command_rejected(&LibraryCommand::Refresh);
    assert!(remove(&mut view, &state).is_none());
    view.command_rejected(&command);
    assert!(view.pending_removal.is_none());
    assert!(matches!(
        remove(&mut view, &state),
        Some(Action::Command(LibraryCommand::RemoveFromPlaylist { .. }))
    ));
}

#[test]
fn completed_removal_clears_old_selection_and_disconnection_releases_pending() {
    let (mut view, mut state) = setup();
    assert!(remove(&mut view, &state).is_some());
    Arc::make_mut(&mut state.tracks).remove(5);
    Arc::make_mut(&mut state.tracks).remove(2);
    state.playlist_remove_revision += 1;
    state.selected_playlist_revision = Arc::from([4, 5, 6]);
    state.revision += 1;
    view.render(&state);
    assert!(view.pending_removal.is_none());
    assert_eq!(view.playlist_choices.len(), 0);
    assert!(remove(&mut view, &state).is_none());
    view.choose_track(1, false, false, &state);
    assert!(remove(&mut view, &state).is_some());
    state.connected = false;
    state.revision += 1;
    view.render(&state);
    assert!(view.pending_removal.is_none());
}
