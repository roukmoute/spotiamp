use super::tests::{click, snapshot};
use super::*;

fn setup(search: bool) -> (View, LibrarySnapshot) {
    let mut s = snapshot(3, 30);
    s.selected_owned = true;
    s.selected_playlist_revision = Arc::from([1, 2, 3]);
    s.search.revision = 7;
    s.search.query = "jazz".into();
    s.search.tracks = s.tracks.clone();
    let mut view = View::default();
    view.render(&s);
    if search {
        view.open_search(false);
    }
    click(&mut view, 300, 152, &s);
    view.choose_track(4, true, false, &s);
    (view, s)
}

fn item(view: &mut View, index: usize, s: &LibrarySnapshot) -> Option<Action> {
    let (x, y, _, _) = view.send_menu.as_ref().unwrap().item_rect(index);
    click(view, x + 16, y + 13, s)
}

#[test]
fn context_click_keeps_selected_group_and_selects_an_unselected_track_without_playing() {
    for skin in [Skin::Classic, Skin::Modern] {
        for search in [false, true] {
            for row in [1, 2, 4] {
                let (mut view, s) = setup(search);
                view.set_skin(skin);
                view.context_menu_at(310, 132 + row * 24, &s);
                assert!(view.has_overlay());
                assert!(view.pressed.is_none());
                assert!(view.selection_drag_at(310, 132 + row * 24, &s).is_none());
                assert!(view.pointer_up(Some((310, 132 + row * 24)), &s).is_none());
                let expected = if row == 2 {
                    vec![s.tracks[2].clone()]
                } else {
                    vec![s.tracks[1].clone(), s.tracks[4].clone()]
                };
                assert_eq!(view.selected_tracks(&s), expected);
                let Some(Action::Command(LibraryCommand::QueueTracks {
                    tracks,
                    origin,
                    replace,
                    play,
                    insert_at,
                    ..
                })) = item(&mut view, 1, &s)
                else {
                    panic!("append context selection");
                };
                assert_eq!(tracks, expected);
                assert_eq!(origin, if search { None } else { s.selected_id.clone() });
                assert!(!replace && !play);
                assert_eq!(insert_at, None);
            }
        }
    }
}

#[test]
fn context_menu_focus_targets_tracks_even_after_playlist_navigation_or_query_editing() {
    for search in [false, true] {
        let (mut view, s) = setup(search);
        if search {
            view.open_search(true);
        } else {
            click(&mut view, 40, 128, &s);
        }
        view.context_menu_at(310, 156, &s);
        assert!(!view.left_focus && !view.input_focus);
        let Some(Action::Command(LibraryCommand::QueueTracks {
            tracks, play: true, ..
        })) = item(&mut view, 0, &s)
        else {
            panic!("play tracks, not entire playlist");
        };
        assert_eq!(tracks, vec![s.tracks[1].clone(), s.tracks[4].clone()]);
    }
}

#[test]
fn context_menu_is_clamped_to_window_and_hit_testing_uses_its_actual_position() {
    for (x, y, expected) in [(260, 132, (260, 132)), (730, 370, (426, 322))] {
        let (mut view, s) = setup(false);
        view.context_menu_at(x, y, &s);
        let menu = view.send_menu.as_ref().unwrap();
        let (mx, my, width, height) = menu.rect;
        assert_eq!((mx, my), expected);
        assert!(mx + width + 3 < WIDTH && my + height + 3 < HEIGHT);
        for index in 0..5 {
            let (ix, iy, iw, ih) = menu.item_rect(index);
            assert!(ix >= mx && iy >= my && ix + iw <= mx + width && iy + ih <= my + height);
            assert_eq!(view.target(ix + 2, iy + 2), Some(Target::MenuItem(index)));
        }
        assert!(item(&mut view, 3, &s).is_none());
        assert!(view.picker.as_ref().unwrap().creating);
    }
}

#[test]
fn context_click_rejects_blank_rows_headers_sidebar_and_inactive_or_unready_sources() {
    for (x, y) in [
        (40, 128),
        (300, 105),
        (740, 150),
        (300, 425),
        (usize::MAX, usize::MAX),
    ] {
        let (mut view, s) = setup(false);
        let selected = view.selected_tracks(&s);
        view.context_menu_at(x, y, &s);
        assert!(!view.has_overlay());
        assert_eq!(view.selected_tracks(&s), selected);
    }
    let mut s = snapshot(1, 2);
    let mut view = View::default();
    view.render(&s);
    view.context_menu_at(310, 228, &s);
    assert!(!view.has_overlay());
    s.connected = false;
    s.revision += 1;
    view.context_menu_at(310, 132, &s);
    assert!(!view.has_overlay());
    s.connected = true;
    s.loading = true;
    s.revision += 1;
    view.context_menu_at(310, 132, &s);
    assert!(!view.has_overlay());
    s.loading = false;
    s.revision += 1;
    view.set_focus(false);
    view.context_menu_at(310, 132, &s);
    assert!(!view.has_overlay());
}

#[test]
fn context_click_never_retargets_tracks_behind_existing_menu_or_playlist_picker() {
    let (mut view, s) = setup(false);
    let selected = view.selected_tracks(&s);
    view.context_menu_at(310, 156, &s);
    // This position covers another track underneath, but belongs to the menu.
    view.context_menu_at(320, 180, &s);
    assert!(view.has_overlay());
    assert_eq!(view.selected_tracks(&s), selected);
    // A click outside closes only the menu, without changing the group.
    view.context_menu_at(310, 132, &s);
    assert!(!view.has_overlay());
    assert_eq!(view.selected_tracks(&s), selected);
    view.open_picker(&s);
    view.context_menu_at(310, 132, &s);
    assert!(view.picker.is_some() && view.send_menu.is_none());
    assert_eq!(view.selected_tracks(&s), selected);
}

#[test]
fn context_selection_survives_scroll_offset_and_stale_revision_cancels_removal() {
    let (mut view, mut s) = setup(false);
    view.right_offset = 15;
    view.context_menu_at(310, 156, &s);
    assert_eq!(view.selected_tracks(&s), vec![s.tracks[16].clone()]);
    let (x, y, _, _) = view.send_menu.as_ref().unwrap().item_rect(4);
    view.pointer_down(x + 8, y + 10, &s);
    s.selected_playlist_revision = Arc::from([4, 5, 6]);
    s.revision += 1;
    assert!(view.pointer_up(Some((x + 8, y + 10)), &s).is_none());
    assert!(view.pending_removal.is_none());
    assert!(!view.has_overlay());
}

#[test]
fn context_menu_reuses_frame_buffer_and_stationary_pointer_preserves_keyboard_choice() {
    for skin in [Skin::Classic, Skin::Modern] {
        let (mut view, s) = setup(true);
        view.set_skin(skin);
        let buffer = view.pixels.as_ptr();
        view.context_menu_at(310, 156, &s);
        assert!(view.render(&s));
        assert!(!view.render(&s));
        let (x, y, _, _) = view.send_menu.as_ref().unwrap().item_rect(0);
        view.pointer_move(Some((x + 8, y + 10)));
        view.key(Key::Down, false, false, &s);
        view.pointer_move(Some((x + 8, y + 10)));
        let Some(Action::Command(LibraryCommand::QueueTracks { play, replace, .. })) =
            view.key(Key::Enter, false, false, &s)
        else {
            panic!("append selection");
        };
        assert!(!play && !replace);
        assert!(view.render(&s));
        assert!(!view.render(&s));
        assert_eq!(view.pixels.as_ptr(), buffer);
    }
}
