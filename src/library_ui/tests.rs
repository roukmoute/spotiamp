use super::*;
use crate::model::TrackRow;
use std::sync::Arc;

fn snapshot(playlists: usize, tracks: usize) -> LibrarySnapshot {
    LibrarySnapshot {
        revision: 1,
        entries: Arc::new(
            (0..playlists)
                .map(|i| PlaylistEntry {
                    id: PlaylistId::Spotify(format!("playlist-{i}")),
                    name: format!("Playlist {i}"),
                    track_count: Some(tracks),
                    duration_ms: Some(tracks as u64 * 180_000),
                    folder: "Mes dossiers / Musique".into(),
                    available: true,
                })
                .collect(),
        ),
        selected_id: (playlists > 0).then(|| PlaylistId::Spotify("playlist-0".into())),
        tracks: Arc::new(
            (0..tracks)
                .map(|i| TrackRow {
                    uri: format!("spotify:track:{i}"),
                    title: format!("Titre {i}"),
                    artist: "Artiste de démonstration".into(),
                    duration_ms: 180_000,
                })
                .collect(),
        ),
        connected: true,
        status: "Bibliothèque Spotify.".into(),
        ..Default::default()
    }
}
fn click(view: &mut View, x: usize, y: usize, s: &LibrarySnapshot) -> Option<Action> {
    assert!(view.pointer_down(x, y, s).is_none());
    view.pointer_up(Some((x, y)), s)
}

#[test]
fn render_reuses_its_buffer_and_shared_tracks_and_skips_unchanged_state() {
    let mut view = View::default();
    let mut s = snapshot(200, 500);
    let buffer = view.pixels.as_ptr();
    let capacity = view.pixels.capacity();
    let tracks = Arc::strong_count(&s.tracks);
    let entries = Arc::strong_count(&s.entries);
    assert!(view.render(&s));
    assert!(!view.render(&s));
    view.pointer_move(Some((10, 10)));
    assert!(!view.render(&s));
    s.revision += 1;
    assert!(view.render(&s));
    assert_eq!(view.pixels.as_ptr(), buffer);
    assert_eq!(view.pixels.capacity(), capacity);
    assert_eq!(Arc::strong_count(&s.tracks), tracks);
    assert_eq!(Arc::strong_count(&s.entries), entries);
}

#[test]
fn scrolling_each_panel_uses_one_notch_per_row_and_accumulates_fractions() {
    let mut view = View::default();
    let s = snapshot(100, 100);
    view.render(&s);
    view.scroll(-1.0, Some((100, 200)), &s);
    assert_eq!(view.left_offset, 1);
    assert_eq!(view.right_offset, 0);
    view.scroll(-0.4, Some((400, 200)), &s);
    assert_eq!(view.right_offset, 0);
    view.scroll(-0.6, Some((400, 200)), &s);
    assert_eq!(view.right_offset, 1);
    view.scroll(f32::NAN, Some((400, 200)), &s);
    view.scroll(f32::INFINITY, Some((100, 200)), &s);
    assert_eq!((view.left_offset, view.right_offset), (1, 1));
    view.scroll(-1000.0, Some((400, 200)), &s);
    assert_eq!(view.right_offset, 100 - RIGHT_ROWS);
    view.scroll(1.0, Some((400, 200)), &s);
    assert_eq!(view.right_offset, 100 - RIGHT_ROWS - 1);
}

#[test]
fn playlist_selection_hides_old_tracks_while_new_selection_is_pending() {
    let mut view = View::default();
    let mut s = snapshot(3, 5);
    view.render(&s);
    assert!(
        matches!(click(&mut view,40,124,&s),Some(Action::Command(LibraryCommand::Select(PlaylistId::Spotify(id)))) if id=="playlist-1")
    );
    assert!(!view.tracks_ready(&s));
    assert!(click(&mut view, 280, 425, &s).is_none());
    s.revision += 1;
    s.selected_id = Some(PlaylistId::Spotify("playlist-1".into()));
    s.loading = true;
    view.render(&s);
    assert!(click(&mut view, 300, 128, &s).is_none());
    s.revision += 1;
    s.loading = false;
    view.render(&s);
    assert!(view.tracks_ready(&s));
    assert!(matches!(
        click(&mut view, 280, 425, &s),
        Some(Action::Command(LibraryCommand::Queue {
            start: 0,
            replace: true,
            play: true,
            ..
        }))
    ));
}

#[test]
fn double_click_plays_only_the_same_track_and_selected_playlist() {
    let mut view = View::default();
    let s = snapshot(2, 20);
    view.render(&s);
    assert!(click(&mut view, 310, 128, &s).is_none());
    assert!(click(&mut view, 310, 152, &s).is_none());
    assert!(
        matches!(click(&mut view,310,152,&s),Some(Action::Command(LibraryCommand::Queue {id:PlaylistId::Spotify(id),start:1,replace:true,play:true})) if id=="playlist-0")
    );
    assert!(click(&mut view, 310, 152, &s).is_none());
    view.set_focus(false);
    view.set_focus(true);
    assert!(click(&mut view, 310, 152, &s).is_none());
}

fn assert_playlist_play(action: Option<Action>, expected: &str) {
    assert!(
        matches!(action, Some(Action::Command(LibraryCommand::Queue {
            id: PlaylistId::Spotify(ref id), start: 0, replace: true, play: true
        })) if id == expected),
        "expected playback of {expected} from the first track, got {action:?}"
    );
}

#[test]
fn playlist_double_click_plays_before_during_or_after_async_loading() {
    for loading_stage in 0..3 {
        let mut view = View::default();
        let mut s = snapshot(3, 5);
        assert!(matches!(
            click(&mut view, 40, 124, &s),
            Some(Action::Command(LibraryCommand::Select(_)))
        ));
        // Normal Win32 ReleaseCapture follows mouse-up, including each click
        // in a double-click. It must not erase the completed first click.
        view.cancel_pointer();
        if loading_stage > 0 {
            s.revision += 1;
            s.selected_id = Some(s.entries[1].id.clone());
            s.loading = loading_stage == 1;
            s.tracks = Arc::default();
            view.render(&s);
        }
        view.pointer_down(40, 124, &s);
        if loading_stage > 0 {
            // The worker can report progress while the second click is held.
            s.revision += 1;
            s.status = "Titres reçus".into();
        }
        assert_playlist_play(view.pointer_up(Some((40, 124)), &s), "playlist-1");
        view.cancel_pointer();
        assert!(click(&mut view, 40, 124, &s).is_none(), "third click");
    }
}

#[test]
fn playlist_double_click_starts_first_track_even_with_another_track_selected() {
    let mut view = View::default();
    let s = snapshot(2, 20);
    view.key(Key::Right, false, false, &s);
    view.key(Key::End, false, false, &s);
    assert_eq!(view.selected_track, Some(19));
    assert!(click(&mut view, 40, 96, &s).is_none());
    assert_playlist_play(click(&mut view, 40, 96, &s), "playlist-0");
    assert_eq!(view.selected_track, None);
    assert_eq!(view.right_offset, 0);
    assert!(view.render(&s));
    assert!(!view.render(&s), "no idle repaint");
}

#[test]
fn playlist_double_click_never_plays_a_different_row_or_reordered_id() {
    let mut view = View::default();
    let mut s = snapshot(3, 5);
    click(&mut view, 40, 96, &s);
    assert!(matches!(
        click(&mut view, 40, 124, &s),
        Some(Action::Command(LibraryCommand::Select(_)))
    ));
    view.pointer_down(40, 124, &s);
    Arc::make_mut(&mut s.entries).swap(0, 1);
    s.revision += 1;
    assert!(view.pointer_up(Some((40, 124)), &s).is_none());
    assert!(!matches!(
        click(&mut view, 40, 124, &s),
        Some(Action::Command(LibraryCommand::Queue { .. }))
    ));
    // Moving the original playlist to another row also requires two fresh clicks.
    assert!(matches!(
        click(&mut view, 40, 96, &s),
        Some(Action::Command(LibraryCommand::Select(_)))
    ));
}

#[test]
fn playlist_double_click_is_cancelled_by_interrupted_input_and_timeout() {
    for interruption in 0..7 {
        let mut view = View::default();
        let s = snapshot(20, 5);
        assert!(click(&mut view, 40, 96, &s).is_none());
        match interruption {
            0 => {
                view.set_focus(false);
                view.set_focus(true);
            }
            1 => {
                view.pointer_down(40, 96, &s);
                view.cancel_pointer();
            }
            2 => {
                view.pointer_down(40, 96, &s);
                assert!(view.pointer_up(Some((230, 96)), &s).is_none());
            }
            3 => {
                assert!(click(&mut view, 230, 96, &s).is_none());
            }
            4 => {
                view.key(Key::Tab, false, false, &s);
            }
            5 => {
                view.scroll(-1.0, Some((40, 200)), &s);
                view.scroll(1.0, Some((40, 200)), &s);
            }
            _ => view.last_click.as_mut().unwrap().2 = Instant::now() - Duration::from_secs(1),
        }
        assert!(
            click(&mut view, 40, 96, &s).is_none(),
            "case {interruption}"
        );
    }
}

#[test]
fn playlist_and_track_clicks_do_not_combine_into_a_double_click() {
    let mut view = View::default();
    let s = snapshot(2, 5);
    assert!(click(&mut view, 40, 96, &s).is_none());
    assert!(click(&mut view, 310, 128, &s).is_none());
    assert!(click(&mut view, 40, 96, &s).is_none());
    assert_playlist_play(click(&mut view, 40, 96, &s), "playlist-0");
}

#[test]
fn unavailable_or_disconnected_playlist_cannot_autoplay() {
    for state in 0..3 {
        let mut view = View::default();
        let mut s = snapshot(2, 5);
        click(&mut view, 40, 96, &s);
        view.pointer_down(40, 96, &s);
        match state {
            0 => s.connected = false,
            1 => Arc::make_mut(&mut s.entries)[0].available = false,
            _ => s.entries = Arc::default(),
        }
        s.revision += 1;
        assert!(!matches!(
            view.pointer_up(Some((40, 96)), &s),
            Some(Action::Command(LibraryCommand::Queue { .. }))
        ));
    }
}

#[test]
fn double_click_can_replace_a_previous_play_request_within_one_frame() {
    let mut view = View::default();
    let mut s = snapshot(2, 0);
    s.loading = true;
    s.busy = true; // Playlist 0 is already loading with a pending play request.
    assert!(matches!(
        click(&mut view, 40, 124, &s),
        Some(Action::Command(LibraryCommand::Select(_)))
    ));
    // No snapshot update yet: the worker will receive Select(1), then Queue(1).
    assert_playlist_play(click(&mut view, 40, 124, &s), "playlist-1");
}

#[test]
fn row_reordering_preserves_ids_and_removal_clears_selection() {
    let mut view = View::default();
    let mut s = snapshot(3, 5);
    view.render(&s);
    click(&mut view, 300, 152, &s);
    Arc::make_mut(&mut s.entries).reverse();
    Arc::make_mut(&mut s.tracks).reverse();
    s.revision += 1;
    view.render(&s);
    assert_eq!(view.selected_id, s.selected_id);
    assert_eq!(view.selected_track, Some(3));
    Arc::make_mut(&mut s.entries).retain(|e| e.id != PlaylistId::Spotify("playlist-0".into()));
    s.revision += 1;
    view.render(&s);
    assert!(view.selected_id.is_none());
    assert!(view.selected_track.is_none());
    assert!(view.key(Key::Enter, false, false, &s).is_none());
}

#[test]
fn native_button_capture_cancels_outside_on_focus_loss_and_on_data_change() {
    let mut view = View::default();
    let mut s = snapshot(2, 3);
    view.render(&s);
    view.pointer_down(280, 425, &s);
    assert!(view.pointer_up(Some((280, 400)), &s).is_none());
    view.pointer_down(280, 425, &s);
    view.set_focus(false);
    assert!(view.pointer_up(Some((280, 425)), &s).is_none());
    view.set_focus(true);
    view.pointer_down(280, 425, &s);
    s.revision += 1;
    assert!(view.pointer_up(Some((280, 425)), &s).is_none());
    assert!(matches!(click(&mut view, 738, 10, &s), Some(Action::Close)));
    assert!(matches!(
        click(&mut view, 714, 10, &s),
        Some(Action::Minimize)
    ));
    assert!(view.pointer_up(Some((714, 10)), &s).is_none());
}

#[test]
fn keyboard_navigation_reveals_rows_and_append_never_replaces_or_plays() {
    let mut view = View::default();
    let s = snapshot(30, 100);
    view.render(&s);
    view.key(Key::Right, false, false, &s);
    view.key(Key::End, false, false, &s);
    assert_eq!(view.selected_track, Some(99));
    assert_eq!(view.right_offset, 89);
    assert!(matches!(
        view.key(Key::Enter, false, false, &s),
        Some(Action::Command(LibraryCommand::Queue {
            start: 99,
            play: true,
            replace: true,
            ..
        }))
    ));
    assert!(matches!(
        view.key(Key::Enter, true, false, &s),
        Some(Action::Command(LibraryCommand::Queue {
            start: 0,
            play: false,
            replace: false,
            ..
        }))
    ));
    view.key(Key::Left, false, false, &s);
    assert!(
        matches!(view.key(Key::End,false,false,&s),Some(Action::Command(LibraryCommand::Select(PlaylistId::Spotify(id)))) if id=="playlist-29")
    );
    assert_eq!(view.left_offset, 30 - LEFT_ROWS);
}

#[test]
fn loading_busy_disconnected_and_unavailable_states_cannot_trigger_playback() {
    for state in 0..4 {
        let mut view = View::default();
        let mut s = snapshot(1, 3);
        match state {
            0 => s.loading = true,
            1 => s.busy = true,
            2 => s.connected = false,
            _ => Arc::make_mut(&mut s.entries)[0].available = false,
        }
        view.render(&s);
        assert!(click(&mut view, 280, 425, &s).is_none());
        assert!(view.key(Key::Enter, false, false, &s).is_none());
        assert!(click(&mut view, 310, 128, &s).is_none());
        assert!(click(&mut view, 310, 128, &s).is_none());
    }
    let mut view = View::default();
    let s = LibrarySnapshot::default();
    assert!(view.render(&s));
    assert!(view.key(Key::End, false, false, &s).is_none());
    assert!(view.key(Key::F5, false, false, &s).is_none());
}

#[test]
fn keyboard_skips_unavailable_playlists_instead_of_getting_stuck() {
    let mut view = View::default();
    let mut s = snapshot(3, 5);
    Arc::make_mut(&mut s.entries)[1].available = false;
    view.render(&s);
    assert!(
        matches!(view.key(Key::Down, false, false, &s), Some(Action::Command(LibraryCommand::Select(PlaylistId::Spotify(id)))) if id == "playlist-2")
    );
    assert!(
        matches!(view.key(Key::Up, false, false, &s), Some(Action::Command(LibraryCommand::Select(PlaylistId::Spotify(id)))) if id == "playlist-0")
    );
}

#[test]
#[ignore = "exports a deterministic UI preview when SPOTIAMP_LIBRARY_PREVIEW is set"]
fn export_library_preview() {
    let path = std::env::var_os("SPOTIAMP_LIBRARY_PREVIEW").expect("preview path");
    let mut view = View::default();
    let mut s = snapshot(16, 28);
    let names = [
        "À écouter cette semaine",
        "Jazz au calme",
        "Nuits électroniques",
        "Piano et concentration",
        "Routes de nuit",
        "Rock indépendant",
        "Soul & groove",
        "Les découvertes",
        "Favoris des années 90",
        "Ambient / paysages",
        "Été à Marseille",
        "Concerts à revoir",
        "Matins tranquilles",
        "Disques du dimanche",
        "Chansons françaises",
        "Les archives",
    ];
    for (e, name) in Arc::make_mut(&mut s.entries).iter_mut().zip(names) {
        e.name = name.into();
        e.folder = if name.contains("Jazz") || name.contains("Piano") {
            "Concentration".into()
        } else {
            "Ma collection".into()
        };
    }
    let tracks = [
        ("Midnight Blue", "Kenny Burrell"),
        ("Blue in Green", "Miles Davis"),
        ("Waltz for Debby", "Bill Evans"),
        ("My Favorite Things", "John Coltrane"),
        ("The Girl from Ipanema", "Stan Getz & João Gilberto"),
        ("Cantaloupe Island", "Herbie Hancock"),
        ("Take Five", "The Dave Brubeck Quartet"),
        ("Feeling Good", "Nina Simone"),
        ("Moanin'", "Art Blakey"),
        ("Song for My Father", "Horace Silver"),
        ("In a Sentimental Mood", "Duke Ellington & John Coltrane"),
    ];
    s.selected_id = Some(PlaylistId::Spotify("playlist-1".into()));
    for (row, (title, artist)) in Arc::make_mut(&mut s.tracks).iter_mut().zip(tracks) {
        row.title = title.into();
        row.artist = artist.into();
    }
    s.status = "16 playlists / lecture seule / F5 : actualiser / Tab : changer de panneau".into();
    view.render(&s);
    click(&mut view, 300, 176, &s);
    view.render(&s);
    let mut out = std::fs::File::create(path).unwrap();
    use std::io::Write;
    write!(out, "P6\n{WIDTH} {HEIGHT}\n255\n").unwrap();
    for p in &view.pixels {
        out.write_all(&[(*p >> 16) as u8, (*p >> 8) as u8, *p as u8])
            .unwrap();
    }
}
