use super::tests::{click, snapshot};
use super::*;

fn searched() -> (View, LibrarySnapshot) {
    let mut s = snapshot(30, 40);
    s.search.revision = 7;
    s.search.query = "jazz".into();
    s.search.tracks = s.tracks.clone();
    let mut view = View::default();
    view.render(&s);
    view.open_search(true);
    (view, s)
}

#[test]
fn search_navigation_preserves_playlist_selection_scroll_and_metadata() {
    let (mut view, s) = searched();
    view.search_mode = false;
    view.input_focus = false;
    view.left_offset = 6;
    view.right_offset = 17;
    view.selected_track = Some(19);
    view.selected_uri = Some(s.tracks[19].uri.clone());
    let buffer = view.pixels.as_ptr();
    click(&mut view, 70, 76, &s);
    assert!(view.wants_text_input());
    assert!(view.search_mode);
    view.key(Key::Down, false, false, &s);
    view.key(Key::End, false, false, &s);
    assert_eq!(view.search_track, Some(39));
    assert_eq!(view.search_offset, 29);
    view.select_playlist(0, &s);
    assert!(!view.search_mode);
    assert_eq!(
        (view.left_offset, view.right_offset, view.selected_track),
        (6, 17, Some(19))
    );
    assert_eq!(view.selected_uri, Some(s.tracks[19].uri.clone()));
    view.open_search(true);
    assert_eq!((view.search_track, view.search_offset), (Some(39), 29));
    assert_eq!(view.pixels.as_ptr(), buffer);
}

#[test]
fn editing_unicode_and_clipboard_selection_keeps_utf8_boundaries() {
    let (mut view, s) = searched();
    view.key(Key::A, true, false, &s);
    view.paste("Beyoncé 🦀");
    assert_eq!(view.query, "Beyoncé 🦀");
    view.key(Key::Left, false, true, &s);
    assert!(matches!(view.key(Key::C, true, false, &s), Some(Action::Copy(text)) if text == "🦀"));
    view.type_char('é');
    assert_eq!(view.query, "Beyoncé é");
    view.key(Key::Backspace, false, false, &s);
    assert_eq!(view.query, "Beyoncé ");
    view.key(Key::Home, false, false, &s);
    view.key(Key::Delete, false, false, &s);
    assert_eq!(view.query, "eyoncé ");
    view.key(Key::End, false, true, &s);
    assert!(
        matches!(view.key(Key::X, true, false, &s), Some(Action::Copy(text)) if text == "eyoncé ")
    );
    assert!(view.query.is_empty());
    assert_eq!((view.cursor, view.anchor), (0, 0));
    assert!(matches!(
        view.key(Key::V, true, false, &s),
        Some(Action::Paste)
    ));
}

#[test]
fn query_limits_unicode_input_and_paste_without_splitting_codepoints() {
    let (mut view, s) = searched();
    view.key(Key::A, true, false, &s);
    view.paste(&"🦀".repeat(300));
    assert_eq!(view.query.chars().count(), SEARCH_QUERY_MAX_CHARS);
    assert_eq!(view.query.len(), SEARCH_QUERY_MAX_BYTES);
    view.type_char('x');
    assert_eq!(view.query.len(), SEARCH_QUERY_MAX_BYTES);
    view.key(Key::A, true, false, &s);
    view.paste("Miles\nDavis\tLive\u{7}");
    assert_eq!(view.query, "Miles Davis Live");
}

#[test]
fn typing_submits_only_on_enter_and_never_queues_a_playlist() {
    let (mut view, s) = searched();
    view.key(Key::A, true, false, &s);
    view.paste("  Miles Davis  ");
    assert!(
        matches!(view.key(Key::Enter, false, false, &s), Some(Action::Command(LibraryCommand::Search(query))) if query == "Miles Davis")
    );
    assert!(!view.active_ready(&s));
    assert!(click(&mut view, 310, 128, &s).is_none());
    assert!(view.search_track.is_none());
    assert!(view.queue(&s, true).is_none());
}

#[test]
fn stale_request_ack_cannot_reenable_rows_before_latest_query_arrives() {
    let (mut view, mut s) = searched();
    view.query = "new query".into();
    view.submit_search(&s);
    s.revision += 1;
    s.search.revision += 1;
    s.search.query = "older query".into();
    view.render(&s);
    assert!(!view.active_ready(&s));
    s.revision += 1;
    s.search.revision += 1;
    s.search.query = "new query".into();
    view.render(&s);
    assert!(view.active_ready(&s));
}

#[test]
fn double_click_and_enter_target_exact_search_result_request() {
    let (mut view, s) = searched();
    assert!(click(&mut view, 310, 152, &s).is_none());
    assert!(matches!(
        click(&mut view, 310, 152, &s),
        Some(Action::Command(LibraryCommand::QueueSearch {
            revision: 7,
            index: 1,
            play: true
        }))
    ));
    assert!(matches!(
        view.key(Key::Enter, true, false, &s),
        Some(Action::Command(LibraryCommand::QueueSearch {
            revision: 7,
            index: 1,
            play: false
        }))
    ));
    assert!(matches!(
        click(&mut view, 390, 425, &s),
        Some(Action::Command(LibraryCommand::QueueSearch {
            revision: 7,
            index: 1,
            play: false
        }))
    ));
}

#[test]
fn result_metadata_updates_preserve_selection_but_new_request_cancels_stale_click() {
    let (mut view, mut s) = searched();
    click(&mut view, 310, 152, &s);
    s.revision += 1;
    std::sync::Arc::make_mut(&mut s.search.tracks)[1].title = "Resolved title".into();
    view.render(&s);
    assert_eq!(view.search_track, Some(1));
    view.pointer_down(310, 152, &s);
    s.revision += 1;
    s.search.revision += 1;
    s.search.query = "different request".into();
    assert!(view.pointer_up(Some((310, 152)), &s).is_none());
    assert!(view.search_track.is_none());
    assert!(click(&mut view, 310, 152, &s).is_none());
}

#[test]
fn clear_removes_text_and_disables_results_even_before_backend_ack() {
    let (mut view, s) = searched();
    view.select_track(3, &s);
    assert!(
        matches!(click(&mut view, 700, 40, &s), Some(Action::Command(LibraryCommand::Search(query))) if query.is_empty())
    );
    assert!(view.query.is_empty());
    assert!(view.wants_text_input());
    assert!(!view.active_ready(&s));
    assert!(view.queue(&s, true).is_none());
}

#[test]
fn text_focus_keyboard_and_inactive_input_never_steal_playback_or_selection() {
    let (mut view, s) = searched();
    view.key(Key::Tab, false, false, &s);
    assert!(!view.wants_text_input());
    view.type_char('x');
    assert_eq!(view.query, "jazz");
    view.key(Key::Down, false, false, &s);
    assert_eq!(view.search_track, Some(0));
    view.key(Key::F, true, false, &s);
    assert!(view.wants_text_input());
    assert_eq!(view.selection(), 0..4);
    assert!(view.key(Key::Escape, false, false, &s).is_none());
    assert!(!view.wants_text_input());
    view.key(Key::Enter, false, false, &s);
    assert!(view.wants_text_input());
    view.set_focus(false);
    view.type_char('x');
    view.paste("oops");
    assert_eq!(view.query, "jazz");
}

#[test]
fn search_view_both_skins_all_languages_reuses_buffer_and_never_repaints_at_idle() {
    let (mut view, s) = searched();
    view.query = "Long title with accents é è ü ø and some extra words".into();
    view.cursor = view.query.len();
    view.anchor = 0;
    view.reveal_cursor();
    assert!(view.input_offset > 0);
    let buffer = view.pixels.as_ptr();
    let tracks = std::sync::Arc::strong_count(&s.search.tracks);
    for skin in [Skin::Classic, Skin::Modern] {
        view.set_skin(skin);
        for language in Language::ALL {
            view.set_language(language);
            view.render(&s);
            assert!(!view.render(&s));
            assert_eq!(view.pixels.as_ptr(), buffer);
            assert_eq!(std::sync::Arc::strong_count(&s.search.tracks), tracks);
        }
    }
}

#[test]
fn empty_request_start_then_query_same_revision_acknowledges_pending_search() {
    let (mut view, mut s) = searched();
    view.query = "new query".into();
    view.submit_search(&s);
    s.revision += 1;
    s.search.revision += 1;
    s.search.query = "".into();
    view.render(&s);
    assert!(!view.active_ready(&s));
    s.revision += 1;
    s.search.query = "new query".into();
    s.search.loading = true;
    view.render(&s);
    assert!(view.search_pending.is_none());
    s.revision += 1;
    s.search.loading = false;
    view.render(&s);
    assert!(view.active_ready(&s));
}

#[test]
fn repeated_enter_is_noop_until_same_query_request_finishes() {
    let (mut view, mut s) = searched();
    assert!(matches!(
        view.submit_search(&s),
        Some(Action::Command(LibraryCommand::Search(_)))
    ));
    assert!(view.submit_search(&s).is_none());
    // A status change carrying the previous query must not acknowledge a reload.
    s.revision += 1;
    view.render(&s);
    assert!(view.search_pending.is_some());
    s.revision += 1;
    s.search.revision += 1;
    s.search.loading = true;
    view.render(&s);
    assert!(view.search_pending.is_none());
    assert!(view.submit_search(&s).is_none());
    s.revision += 1;
    s.search.loading = false;
    view.render(&s);
    assert!(matches!(
        view.submit_search(&s),
        Some(Action::Command(LibraryCommand::Search(_)))
    ));
}

#[test]
fn tab_cycles_search_input_results_navigation_in_both_directions() {
    let (mut view, s) = searched();
    for _ in 0..2 {
        view.key(Key::Tab, false, false, &s);
        assert!(!view.input_focus && !view.left_focus);
        view.key(Key::Tab, false, false, &s);
        assert!(!view.input_focus && view.left_focus);
        view.key(Key::Tab, false, false, &s);
        assert!(view.input_focus && !view.left_focus);
    }
    for _ in 0..2 {
        view.key(Key::Tab, false, true, &s);
        assert!(!view.input_focus && view.left_focus);
        view.key(Key::Tab, false, true, &s);
        assert!(!view.input_focus && !view.left_focus);
        view.key(Key::Tab, false, true, &s);
        assert!(view.input_focus && !view.left_focus);
    }
}

#[test]
fn rejected_or_disconnected_search_can_retry_unchanged_query() {
    let (mut view, mut s) = searched();
    assert!(view.submit_search(&s).is_some());
    view.command_rejected();
    assert!(view.search_pending.is_none());
    assert!(view.submit_search(&s).is_some());
    s.revision += 1;
    s.connected = false;
    view.render(&s);
    assert!(view.search_pending.is_none());
    s.revision += 1;
    s.connected = true;
    view.render(&s);
    assert!(view.submit_search(&s).is_some());
}
