//! Follow a new playback request once without taking control of later browsing.
use super::*;
use crate::model::{PlaybackOrigin, TrackRow};

fn queue(len: usize, current: Option<usize>, revision: u64) -> Snapshot {
    Snapshot {
        queue: (0..len)
            .map(|index| TrackRow {
                uri: format!("spotify:track:{index}"),
                title: format!("Titre {}", index + 1),
                ..TrackRow::default()
            })
            .collect(),
        current,
        playback_revision: revision,
        ..Snapshot::default()
    }
}

#[test]
fn ninth_track_is_selected_centered_and_mouse_accessible_in_both_skins() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut view = View::default();
        view.set_skin(skin);
        view.set_focus(false); // Playback was launched from the library.
        view.selected = Some(0);
        let address = view.pixels.as_ptr();
        let capacity = view.pixels.capacity();
        let state = queue(30, Some(8), 1);
        view.sync_playback(&state);
        view.render(&state);
        assert_eq!(view.selected, Some(8));
        assert_eq!(view.offset, 4, "center row nine among rows five to twelve");
        assert!(!view.focused, "following playback must not steal focus");
        assert_eq!(view.pixels.as_ptr(), address);
        assert_eq!(view.pixels.capacity(), capacity);
        view.set_focus(true);
        let y = 221 + (ROWS / 2) * 24 + 12;
        assert!(view.click(100, y, &state).is_none());
        assert!(matches!(
            view.click(100, y, &state),
            Some(Action::Send(Command::PlayIndex(8)))
        ));
    }
}

#[test]
fn browsing_stays_free_through_ticks_pause_metadata_and_queue_appends() {
    let mut view = View::default();
    let mut state = queue(30, Some(20), 1);
    view.sync_playback(&state);
    view.scroll(100.0, state.queue.len());
    assert!(view.click(100, 233, &state).is_none());
    assert_eq!((view.selected, view.offset), (Some(0), 0));
    view.scroll(-0.5, state.queue.len());
    for tick in 0..6 {
        state.position_ms = tick * 1000;
        state.playing = tick % 2 == 0;
        state.volume = 1234 + tick as u16;
        state.queue[20].title = format!("Métadonnées {tick}");
        state.queue.push(TrackRow::default());
        view.sync_playback(&state);
        view.render(&state);
        assert_eq!((view.selected, view.offset), (Some(0), 0));
        assert_eq!(view.scroll_remainder, -0.5);
    }
    let fingerprint = view.fingerprint(&state);
    view.sync_playback(&state);
    assert_eq!(view.fingerprint(&state), fingerprint);
    state.current = Some(21);
    state.playback_revision += 1;
    view.sync_playback(&state);
    assert_eq!((view.selected, view.offset), (Some(21), 17));
    assert_eq!(view.scroll_remainder, 0.0);
}

#[test]
fn replaying_the_same_row_or_replacing_a_playlist_at_the_same_index_follows_again() {
    let mut view = View::default();
    let mut state = queue(30, Some(8), 1);
    view.sync_playback(&state);
    view.scroll(-20.0, state.queue.len());
    state.playback_revision += 1;
    view.sync_playback(&state);
    assert_eq!((view.selected, view.offset), (Some(8), 4));

    view.scroll(100.0, state.queue.len());
    state = queue(15, Some(8), 3);
    state.queue[8].uri = "spotify:track:another-playlist-track".into();
    view.sync_playback(&state);
    assert_eq!((view.selected, view.offset), (Some(8), 4));

    // A replacement can also contain the same URI at the same row.
    view.scroll(100.0, state.queue.len());
    state.playback_revision += 1;
    view.sync_playback(&state);
    assert_eq!((view.selected, view.offset), (Some(8), 4));
}

#[test]
fn following_handles_short_empty_invalid_and_wrapped_snapshots() {
    let mut view = View::default();
    view.sync_playback(&queue(500, Some(499), u64::MAX));
    assert_eq!((view.selected, view.offset), (Some(499), 500 - ROWS));
    view.sync_playback(&queue(3, Some(2), 0));
    assert_eq!((view.selected, view.offset), (Some(2), 0));
    view.sync_playback(&queue(0, None, 1));
    view.render(&queue(0, None, 1));
    assert_eq!((view.selected, view.offset), (None, 0));
    view.sync_playback(&queue(3, Some(usize::MAX), 2));
    assert_eq!((view.selected, view.offset), (None, 0));
    view.sync_playback(&queue(3, Some(0), 3));
    assert_eq!((view.selected, view.offset), (Some(0), 0));
}

#[test]
fn new_playback_does_not_retarget_a_captured_remove_or_old_double_click() {
    let mut view = View::default();
    let mut state = queue(30, Some(0), 1);
    view.sync_playback(&state);
    assert!(view.click(100, 233, &state).is_none());
    assert!(view.last_click.is_some());
    view.pointer_down(50, 425, &state);
    assert_eq!(view.pressed_button, Some(Button::Remove));
    state.current = Some(8);
    state.playback_revision += 1;
    view.sync_playback(&state);
    assert!(view.last_click.is_none());
    assert!(view.pointer_up(Some((50, 425)), &state).is_none());
    assert!(view.click(100, 329, &state).is_none());
    assert_eq!(view.selected, Some(8));
}

#[test]
fn following_behind_about_keeps_the_dialog_and_link_input_intact() {
    let mut view = View::default();
    view.input = "https://open.spotify.com/playlist/example".into();
    view.open_about();
    let tab = view.about;
    let state = queue(30, Some(20), 1);
    view.sync_playback(&state);
    assert_eq!(view.about, tab);
    assert_eq!(view.input, "https://open.spotify.com/playlist/example");
    assert_eq!((view.selected, view.offset), (Some(20), 16));
    view.close_about();
    view.render(&state);
    assert_eq!((view.selected, view.offset), (Some(20), 16));
}

#[test]
fn centering_stops_at_both_ends_without_leaving_empty_rows() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut view = View::default();
        view.set_skin(skin);
        for (revision, (index, expected_offset)) in [(0, 0), (3, 0), (8, 4), (16, 12), (19, 12)]
            .into_iter()
            .enumerate()
        {
            let state = queue(20, Some(index), revision as u64 + 1);
            view.sync_playback(&state);
            view.render(&state);
            assert_eq!(view.selected, Some(index));
            assert_eq!(view.offset, expected_offset);
            assert!(view.offset <= index && index < view.offset + ROWS);
            assert!(view.offset + ROWS <= state.queue.len());
        }
    }
}

#[test]
fn automatic_ninth_track_moves_half_a_page_and_visible_tracks_keep_the_viewport() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut view = View::default();
        view.set_skin(skin);
        view.set_focus(false);
        let mut state = queue(30, Some(0), 1);
        view.sync_playback(&state);
        state.playback_origin = PlaybackOrigin::Automatic;
        for index in 1..12 {
            state.current = Some(index);
            state.playback_revision += 1;
            assert!(view.sync_playback(&state));
            view.render(&state);
            assert_eq!(view.selected, Some(index));
            assert_eq!(view.offset, if index < 8 { 0 } else { 4 });
            assert!(!view.focused);
        }
        // Track 13 leaves rows 5–12: advance four more rows, once.
        state.current = Some(12);
        state.playback_revision += 1;
        view.sync_playback(&state);
        assert_eq!(view.offset, 8);
        view.scroll(100.0, state.queue.len());
        state.position_ms = 5_000;
        assert!(!view.sync_playback(&state));
        view.render(&state);
        assert_eq!(
            view.offset, 0,
            "manual browsing remains free between tracks"
        );
    }
}

#[test]
fn automatic_shuffle_and_wrap_reveal_offscreen_rows_but_explicit_starts_always_center() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut view = View::default();
        view.set_skin(skin);
        let mut state = queue(30, Some(8), 1);
        view.sync_playback(&state);
        assert_eq!(view.offset, 4);
        state.playback_origin = PlaybackOrigin::Automatic;
        for (index, offset) in [(4, 4), (11, 4), (3, 0), (25, 21), (29, 22), (0, 0)] {
            state.current = Some(index);
            state.playback_revision += 1;
            view.sync_playback(&state);
            view.render(&state);
            assert_eq!((view.selected, view.offset), (Some(index), offset));
        }
        // Track 7 is visible already: only an explicit request centers it.
        state.current = Some(6);
        state.playback_revision += 1;
        view.sync_playback(&state);
        assert_eq!(view.offset, 0);
        state.playback_origin = PlaybackOrigin::Explicit;
        state.playback_revision += 1;
        view.sync_playback(&state);
        assert_eq!(view.offset, 2);
    }
}

#[test]
fn queue_drop_target_uses_rows_and_refuses_controls_or_overlays() {
    let mut view = View::default();
    assert_eq!(view.queue_drop_index((100.0, 221.0), 20), Some(0));
    assert_eq!(view.queue_drop_index((100.0, 269.0), 20), Some(2));
    assert_eq!(view.queue_drop_index((100.0, 405.0), 3), Some(3));
    assert_eq!(view.queue_drop_index((100.0, 269.0), 0), Some(0));
    assert_eq!(view.queue_drop_index((f32::NAN, 269.0), 20), None);
    assert_eq!(view.queue_drop_index((100.0, 425.0), 20), None);
    view.offset = 10;
    assert_eq!(view.queue_drop_index((100.0, 269.0), 20), Some(12));
    view.help = true;
    assert_eq!(view.queue_drop_index((100.0, 269.0), 20), None);
    view.help = false;
    view.open_about();
    assert_eq!(view.queue_drop_index((100.0, 269.0), 20), None);
}
