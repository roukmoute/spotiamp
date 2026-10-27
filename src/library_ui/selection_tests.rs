use super::tests::{click, send_menu, snapshot};
use super::*;
use std::sync::Arc;

fn setup(search: bool) -> (View, LibrarySnapshot) {
    let mut s = snapshot(20, 30);
    s.search.revision = 7;
    s.search.query = "music".into();
    s.search.tracks = s.tracks.clone();
    let mut view = View::default();
    view.render(&s);
    if search {
        view.open_search(false);
    }
    (view, s)
}
fn choose(view: &mut View, row: usize, control: bool, shift: bool, s: &LibrarySnapshot) {
    let y = 120 + row * 24 + 8;
    assert!(
        view.pointer_down_with_modifiers(310, y, control, shift, s)
            .is_none()
    );
    assert!(view.pointer_up(Some((310, y)), s).is_none());
}
fn indices(view: &View, s: &LibrarySnapshot) -> Vec<usize> {
    view.selected_tracks(s)
        .iter()
        .map(|track| track.uri.rsplit(':').next().unwrap().parse().unwrap())
        .collect()
}
fn assert_tracks(action: Option<Action>, expected: &[usize], play: bool) {
    let Some(Action::Command(LibraryCommand::QueueTracks {
        tracks,
        replace,
        play: actual_play,
        insert_at,
        ..
    })) = action
    else {
        panic!("expected captured queue selection, got {action:?}");
    };
    assert_eq!(replace, play);
    assert_eq!(actual_play, play);
    assert_eq!(insert_at, None);
    assert_eq!(
        tracks
            .iter()
            .map(|track| track
                .uri
                .rsplit(':')
                .next()
                .unwrap()
                .parse::<usize>()
                .unwrap())
            .collect::<Vec<_>>(),
        expected
    );
}
#[test]
fn control_toggle_and_shift_ranges_capture_selected_rows_in_list_order_in_both_sources() {
    for search in [false, true] {
        let (mut view, s) = setup(search);
        choose(&mut view, 6, false, false, &s);
        choose(&mut view, 1, true, false, &s);
        choose(&mut view, 4, true, false, &s);
        assert_eq!(indices(&view, &s), [1, 4, 6]);
        choose(&mut view, 4, true, false, &s);
        assert_eq!(indices(&view, &s), [1, 6]);
        choose(&mut view, 8, false, true, &s);
        assert_eq!(indices(&view, &s), [4, 5, 6, 7, 8]);
        choose(&mut view, 2, true, true, &s);
        assert_eq!(indices(&view, &s), [2, 3, 4, 5, 6, 7, 8]);
        assert_tracks(
            view.key(Key::Enter, false, false, &s),
            &[2, 3, 4, 5, 6, 7, 8],
            true,
        );
        assert_tracks(
            view.key(Key::Enter, false, true, &s),
            &[2, 3, 4, 5, 6, 7, 8],
            false,
        );
        assert_tracks(send_menu(&mut view, 1, &s), &[2, 3, 4, 5, 6, 7, 8], false);
    }
}
#[test]
fn selected_group_survives_drag_threshold_but_plain_release_collapses_it() {
    let (mut view, s) = setup(false);
    choose(&mut view, 1, false, false, &s);
    choose(&mut view, 4, true, false, &s);
    view.pointer_down_with_modifiers(310, 152, false, false, &s);
    let drag = view.selection_drag_at(310, 152, &s).unwrap();
    assert_eq!(drag.len(), 2);
    assert_eq!(indices(&view, &s), [1, 4]);
    view.cancel_pointer();
    assert_eq!(indices(&view, &s), [1, 4]);
    assert!(view.selection_drag_at(310, 152, &s).is_none());
    choose(&mut view, 1, false, false, &s);
    assert_eq!(indices(&view, &s), [1]);
}
#[test]
fn keyboard_ranges_can_grow_shrink_and_select_all_without_selecting_query_text() {
    let (mut view, s) = setup(true);
    choose(&mut view, 2, false, false, &s);
    view.key(Key::Down, false, true, &s);
    view.key(Key::Down, false, true, &s);
    assert_eq!(indices(&view, &s), [2, 3, 4]);
    view.key(Key::Up, false, true, &s);
    assert_eq!(indices(&view, &s), [2, 3]);
    view.key(Key::End, false, true, &s);
    assert_eq!(indices(&view, &s), (2..30).collect::<Vec<_>>());
    assert_eq!(view.search_offset, 19);
    view.key(Key::A, true, false, &s);
    assert_eq!(indices(&view, &s), (0..30).collect::<Vec<_>>());
    view.key(Key::F, true, false, &s);
    view.key(Key::A, true, false, &s);
    assert_eq!(view.selection(), 0..5);
    assert_eq!(indices(&view, &s).len(), 30);
}
#[test]
fn metadata_updates_preserve_both_selections_and_new_search_resets_only_search() {
    let (mut view, mut s) = setup(false);
    choose(&mut view, 1, false, false, &s);
    choose(&mut view, 4, true, false, &s);
    view.open_search(false);
    choose(&mut view, 2, false, false, &s);
    choose(&mut view, 5, true, false, &s);
    s.revision += 1;
    Arc::make_mut(&mut s.search.tracks)[2].title = "New title".into();
    view.render(&s);
    assert_eq!(indices(&view, &s), [2, 5]);
    assert_eq!(view.selected_tracks(&s)[0].title, "New title");
    view.select_playlist(0, &s);
    assert_eq!(indices(&view, &s), [1, 4]);
    view.open_search(false);
    s.revision += 1;
    s.search.revision += 1;
    view.render(&s);
    assert!(indices(&view, &s).is_empty());
    view.select_playlist(0, &s);
    assert_eq!(indices(&view, &s), [1, 4]);
}
#[test]
fn playlist_reordering_follows_selected_identities_and_removal_cannot_retarget_them() {
    let (mut view, mut s) = setup(false);
    choose(&mut view, 1, false, false, &s);
    choose(&mut view, 4, true, false, &s);
    Arc::make_mut(&mut s.tracks).swap(1, 2);
    // Even before the next render, capture rejects changed row identities.
    assert!(view.selected_tracks(&s).is_empty());
    s.revision += 1;
    view.render(&s);
    assert_eq!(indices(&view, &s), [1, 4]);
    assert_tracks(view.queue(&s, true), &[1, 4], true);
    Arc::make_mut(&mut s.tracks)
        .retain(|row| row.uri != "spotify:track:1" && row.uri != "spotify:track:4");
    s.revision += 1;
    view.render(&s);
    assert!(view.selected_tracks(&s).is_empty());
    assert!(view.queue(&s, true).is_none());
}
#[test]
fn picker_captures_chosen_tracks_and_requires_explicit_destination() {
    let (mut view, mut s) = setup(true);
    choose(&mut view, 2, false, false, &s);
    choose(&mut view, 5, true, false, &s);
    send_menu(&mut view, 2, &s);
    assert!(view.picker.is_some());
    assert!(!view.enabled(Button::PickerConfirm, &s));
    assert!(view.key(Key::Enter, false, false, &s).is_none());
    s.revision += 1;
    s.search.revision += 1;
    s.search.tracks = Arc::new(vec![]);
    view.render(&s);
    view.key(Key::End, false, false, &s);
    let Some(Action::Command(LibraryCommand::AddToPlaylist { id, tracks })) =
        view.key(Key::Enter, false, false, &s)
    else {
        panic!("expected playlist addition");
    };
    assert_eq!(id, s.entries[19].id);
    assert_eq!(tracks, vec![s.tracks[2].clone(), s.tracks[5].clone()]);
    assert!(view.picker.is_none());
}
#[test]
fn picker_cancel_removed_destination_and_disconnect_never_add_tracks() {
    let (mut view, mut s) = setup(true);
    choose(&mut view, 0, false, false, &s);
    view.open_picker(&s);
    view.key(Key::Down, false, false, &s);
    let chosen = view.picker.as_ref().unwrap().destination.clone();
    s.revision += 1;
    Arc::make_mut(&mut s.entries).retain(|entry| Some(&entry.id) != chosen.as_ref());
    assert!(view.key(Key::Enter, false, false, &s).is_none());
    assert!(view.key(Key::Escape, false, false, &s).is_none());
    assert!(view.picker.is_none());
    view.open_picker(&s);
    s.revision += 1;
    s.connected = false;
    view.render(&s);
    assert!(view.picker.is_none());
    assert!(!view.enabled(Button::SendTo, &s));
}
#[test]
fn picker_scroll_and_selection_never_change_source_tracks_or_browse_playlist() {
    let (mut view, s) = setup(true);
    choose(&mut view, 2, false, false, &s);
    view.open_picker(&s);
    view.scroll(-3.0, Some((300, 200)), &s);
    assert!(click(&mut view, 260, 150, &s).is_none());
    assert_eq!(
        view.picker.as_ref().unwrap().destination,
        Some(s.entries[3].id.clone())
    );
    assert_eq!(view.selected_id, Some(s.entries[0].id.clone()));
    assert_eq!(indices(&view, &s), [2]);
    assert!(click(&mut view, 40, 150, &s).is_none());
    assert!(click(&mut view, 490, 359, &s).is_none());
    assert!(view.picker.is_none());
}
#[test]
fn all_selected_rows_are_highlighted_without_extra_idle_rendering_in_both_skins() {
    for skin in [Skin::Classic, Skin::Modern] {
        let (mut view, s) = setup(true);
        view.set_skin(skin);
        choose(&mut view, 1, false, false, &s);
        choose(&mut view, 4, true, false, &s);
        view.render(&s);
        let palette = modern::Palette::new(skin, true);
        for i in 0..8 {
            assert_eq!(
                view.pixels[(120 + i * 24) * WIDTH + 248] == palette.selection,
                [1, 4].contains(&i)
            );
        }
        let buffer = view.pixels.as_ptr();
        assert!(!view.render(&s));
        view.open_picker(&s);
        assert!(view.render(&s));
        assert!(!view.render(&s));
        assert_eq!(view.pixels.as_ptr(), buffer);
    }
}

#[test]
fn returning_to_same_playlist_navigation_plays_the_whole_playlist() {
    let (mut view, s) = setup(false);
    choose(&mut view, 2, false, false, &s);
    choose(&mut view, 5, true, false, &s);
    assert_eq!(indices(&view, &s), [2, 5]);
    assert!(click(&mut view, 40, 128, &s).is_none());
    assert_eq!(indices(&view, &s), [2, 5]);
    for (action, play) in [
        (view.key(Key::Enter, false, false, &s), true),
        (send_menu(&mut view, 1, &s), false),
    ] {
        assert!(
            matches!(action, Some(Action::Command(LibraryCommand::Queue {
            id, replace, play: actual, start: 0,
        })) if id == s.entries[0].id && replace == play && actual == play)
        );
    }
}

#[test]
fn selected_playlist_origin_is_captured_and_search_has_no_playlist_origin() {
    let (mut view, s) = setup(false);
    choose(&mut view, 2, false, false, &s);
    assert_eq!(view.selection_origin(), s.selected_id);
    assert!(
        matches!(view.queue(&s, true), Some(Action::Command(LibraryCommand::QueueTracks {
        origin, ..
    })) if origin == s.selected_id)
    );
    view.open_search(false);
    choose(&mut view, 2, false, false, &s);
    assert_eq!(view.selection_origin(), None);
    assert!(matches!(
        view.queue(&s, true),
        Some(Action::Command(LibraryCommand::QueueTracks {
            origin: None,
            ..
        }))
    ));
}
