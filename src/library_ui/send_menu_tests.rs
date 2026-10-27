use super::tests::{click, send_menu, snapshot};
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

#[test]
fn send_menu_queues_the_selected_rows_in_order_from_both_sources_and_skins() {
    for skin in [Skin::Classic, Skin::Modern] {
        for search in [false, true] {
            for item in 0..2 {
                let (mut view, s) = setup(search);
                view.set_skin(skin);
                let Some(Action::Command(LibraryCommand::QueueTracks {
                    tracks,
                    origin,
                    replace,
                    play,
                    insert_at,
                    ..
                })) = send_menu(&mut view, item, &s)
                else {
                    panic!("menu must capture selection");
                };
                assert_eq!(tracks, vec![s.tracks[1].clone(), s.tracks[4].clone()]);
                assert_eq!(origin, if search { None } else { s.selected_id.clone() });
                assert_eq!(replace, item == 0);
                assert_eq!(play, item == 0);
                assert_eq!(insert_at, None);
                assert!(view.pointer_up(Some((280, 282 + item * 28)), &s).is_none());
            }
        }
    }
}

#[test]
fn menu_creation_opens_name_editor_directly_and_captures_selected_tracks() {
    for search in [false, true] {
        let (mut view, s) = setup(search);
        assert!(send_menu(&mut view, 3, &s).is_none());
        assert!(view.picker.as_ref().unwrap().creating);
        view.paste("Découvertes");
        let Some(Action::Command(LibraryCommand::CreatePlaylist { name, tracks })) =
            view.key(Key::Enter, false, false, &s)
        else {
            panic!("create playlist");
        };
        assert_eq!(name, "Découvertes");
        assert_eq!(tracks, vec![s.tracks[1].clone(), s.tracks[4].clone()]);
    }
}

#[test]
fn menu_removal_preserves_duplicate_occurrences_and_requires_ownership() {
    let (mut view, mut s) = setup(false);
    Arc::make_mut(&mut s.tracks)[4].uri = s.tracks[1].uri.clone();
    s.revision += 1;
    view.render(&s);
    view.choose_track(1, false, false, &s);
    view.choose_track(4, true, false, &s);
    let Some(Action::Command(LibraryCommand::RemoveFromPlaylist {
        id,
        indices,
        expected_tracks,
        expected_revision,
    })) = send_menu(&mut view, 4, &s)
    else {
        panic!("remove selection");
    };
    assert_eq!(Some(id), s.selected_id);
    assert_eq!(indices, vec![1, 4]);
    assert!(Arc::ptr_eq(&expected_tracks, &s.tracks));
    assert!(Arc::ptr_eq(
        &expected_revision,
        &s.selected_playlist_revision
    ));

    for search in [false, true] {
        let (mut view, mut s) = setup(search);
        s.selected_owned = false;
        s.revision += 1;
        assert!(send_menu(&mut view, 4, &s).is_none());
        assert!(view.pending_removal.is_none());
    }
}

#[test]
fn outside_menu_click_is_consumed_without_selection_or_drag() {
    let (mut view, s) = setup(false);
    let selected = view.selected_tracks(&s);
    click(&mut view, 280, 425, &s);
    assert!(view.pointer_down(300, 224, &s).is_none());
    assert!(view.selection_drag_at(300, 224, &s).is_none());
    assert!(view.pointer_up(Some((300, 224)), &s).is_none());
    assert_eq!(view.selected_tracks(&s), selected);
    // The next click is a normal selection again.
    click(&mut view, 300, 248, &s);
    assert_eq!(view.selected_tracks(&s), vec![s.tracks[5].clone()]);
}

#[test]
fn menu_scroll_and_text_do_not_change_underlying_query_or_selection() {
    let (mut view, s) = setup(true);
    let selected = view.selected_tracks(&s);
    let query = view.query.clone();
    click(&mut view, 280, 425, &s);
    view.scroll(-4.0, Some((310, 160)), &s);
    view.type_char('x');
    view.paste("unwanted paste");
    assert!(view.key(Key::F, true, false, &s).is_none());
    assert!(view.key(Key::V, true, false, &s).is_none());
    assert_eq!(view.query, query);
    assert_eq!(view.selected_tracks(&s), selected);
    assert_eq!(view.right_offset, 0);
    assert!(view.key(Key::Escape, false, false, &s).is_none());
    assert!(matches!(
        view.key(Key::Escape, false, false, &s),
        Some(Action::Close)
    ));
}

#[test]
fn menu_cannot_apply_actions_after_source_disconnect_revision_change_or_focus_loss() {
    for invalidation in 0..6 {
        let (mut view, mut s) = setup(false);
        click(&mut view, 280, 425, &s);
        view.pointer_down(280, 394, &s);
        match invalidation {
            0 => s.connected = false,
            1 => s.selected_playlist_revision = Arc::from([4, 5, 6]),
            2 => s.selected_id = Some(s.entries[1].id.clone()),
            3 => {
                Arc::make_mut(&mut s.tracks).remove(1);
            }
            4 => view.set_focus(false),
            5 => Arc::make_mut(&mut s.entries)[0].available = false,
            _ => unreachable!(),
        }
        s.revision += 1;
        assert!(
            view.pointer_up(Some((280, 394)), &s).is_none(),
            "case {invalidation}"
        );
        assert!(view.pending_removal.is_none());
    }
}

#[test]
fn send_menu_reuses_the_frame_buffer_and_idle_render_is_skipped_in_all_languages() {
    for skin in [Skin::Classic, Skin::Modern] {
        for language in Language::ALL {
            let (mut view, s) = setup(false);
            view.set_skin(skin);
            view.set_language(language);
            let pointer = view.pixels.as_ptr();
            click(&mut view, 280, 425, &s);
            assert!(view.render(&s));
            assert!(!view.render(&s));
            view.pointer_move(Some((300, 338)));
            assert!(view.render(&s));
            assert!(!view.render(&s));
            assert_eq!(view.pixels.as_ptr(), pointer);
            assert!(view.key(Key::Escape, false, false, &s).is_none());
            assert!(view.render(&s));
            assert!(!view.render(&s));
        }
    }
}

#[test]
fn keyboard_menu_navigation_activates_only_the_focused_enabled_action() {
    for opener in [(Key::Menu, false), (Key::F10, true)] {
        let (mut view, s) = setup(true);
        assert!(view.key(opener.0, false, opener.1, &s).is_none());
        assert!(view.has_overlay());
        assert!(view.key(Key::Down, false, false, &s).is_none());
        let Some(Action::Command(LibraryCommand::QueueTracks {
            tracks,
            play,
            replace,
            ..
        })) = view.key(Key::Enter, false, false, &s)
        else {
            panic!("append selection");
        };
        assert_eq!(tracks, vec![s.tracks[1].clone(), s.tracks[4].clone()]);
        assert!(!play && !replace);
        assert!(!view.has_overlay());
    }
}

#[test]
fn stationary_pointer_does_not_override_keyboard_menu_navigation() {
    let (mut view, s) = setup(true);
    click(&mut view, 280, 425, &s);
    view.pointer_move(Some((300, 282)));
    view.key(Key::Down, false, false, &s);
    view.pointer_move(Some((300, 282)));
    let Some(Action::Command(LibraryCommand::QueueTracks { play, replace, .. })) =
        view.key(Key::Enter, false, false, &s)
    else {
        panic!("append selection");
    };
    assert!(
        !play && !replace,
        "stationary pointer must not undo keyboard choice"
    );
}
