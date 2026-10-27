use super::tests::{click, snapshot};
use super::*;
use std::sync::Arc;

#[test]
fn queue_picker_keeps_captured_tracks_as_a_fresh_catalogue_arrives() {
    let mut s = snapshot(3, 6);
    let destinations = s.entries.clone();
    let tracks = vec![
        s.tracks[1].clone(),
        s.tracks[4].clone(),
        s.tracks[1].clone(),
    ];
    s.entries = Arc::default();
    s.loading = true;
    let mut view = View::default();
    assert!(view.open_queue_playlist_picker(&s, tracks.clone(), false));
    assert!(view.picker.as_ref().unwrap().destinations.is_empty());
    s.entries = destinations;
    s.loading = false;
    s.revision += 1;
    view.render(&s);
    let picker = view.picker.as_mut().unwrap();
    assert_eq!(picker.tracks, tracks);
    assert_eq!(picker.destinations.len(), 3);
    picker.choose(1);
    let destination = picker.destination.clone().unwrap();
    let address = picker.destinations.as_ptr();
    s.revision += 1;
    view.render(&s);
    assert_eq!(view.picker.as_ref().unwrap().destinations.as_ptr(), address);
    assert!(matches!(view.confirm_picker(&s), Some(Action::Command(
        LibraryCommand::AddToPlaylist { id, tracks: captured }
    )) if id == destination && captured == tracks));
}

#[test]
fn queue_picker_does_not_overwrite_a_dialog_and_keeps_new_name_during_catalogue_refresh() {
    let mut s = snapshot(2, 6);
    let tracks = vec![s.tracks[2].clone(), s.tracks[3].clone()];
    let mut view = View::default();
    assert!(view.open_queue_playlist_picker(&s, tracks.clone(), true));
    view.paste("Été et musique");
    assert!(!view.open_queue_playlist_picker(&s, vec![s.tracks[0].clone()], false));
    s.entries = Arc::new(s.entries.iter().take(1).cloned().collect());
    s.revision += 1;
    view.render(&s);
    let picker = view.picker.as_ref().unwrap();
    assert!(picker.creating);
    assert_eq!(picker.name, "Été et musique");
    assert_eq!(picker.tracks, tracks);
    assert!(matches!(view.create_playlist(&s), Some(Action::Command(
        LibraryCommand::CreatePlaylist { name, tracks: captured }
    )) if name == "Été et musique" && captured == tracks));
}

#[test]
fn queue_picker_rejects_unavailable_writes_and_invalidates_removed_destinations() {
    let mut s = snapshot(2, 4);
    let mut view = View::default();
    let tracks = vec![s.tracks[1].clone()];
    for (connected, busy) in [(false, false), (true, true)] {
        s.connected = connected;
        s.playlist_write_busy = busy;
        s.revision += 1;
        assert!(!view.open_queue_playlist_picker(&s, tracks.clone(), false));
        assert!(view.picker.is_none());
    }
    s.connected = true;
    s.playlist_write_busy = false;
    s.revision += 1;
    assert!(view.open_queue_playlist_picker(&s, tracks, false));
    view.picker.as_mut().unwrap().choose(1);
    s.entries = Arc::new(s.entries.iter().take(1).cloned().collect());
    s.revision += 1;
    view.render(&s);
    assert!(view.picker.as_ref().unwrap().destination.is_none());
    assert!(view.confirm_picker(&s).is_none());
}

fn setup(search: bool) -> (View, LibrarySnapshot) {
    let mut s = snapshot(20, 30);
    s.search.revision = 7;
    s.search.query = "jazz".into();
    s.search.tracks = s.tracks.clone();
    let mut view = View::default();
    view.render(&s);
    if search {
        view.open_search(false);
    }
    view.choose_track(2, false, false, &s);
    view.choose_track(5, true, false, &s);
    view.open_picker(&s);
    assert!(click(&mut view, 240, 359, &s).is_none());
    assert!(view.picker.as_ref().unwrap().creating);
    assert!(view.wants_text_input());
    (view, s)
}

#[test]
fn create_uses_captured_selection_in_both_sources_and_preserves_query() {
    for search in [false, true] {
        let (mut view, mut s) = setup(search);
        let query = view.query.clone();
        view.paste("  Été à São Paulo  ");
        s.revision += 1;
        s.search.revision += 1;
        s.search.tracks = Arc::new(vec![]);
        s.tracks = Arc::new(vec![]);
        view.render(&s);
        assert!(view.enabled(Button::PickerCreate, &s));
        let Some(Action::Command(LibraryCommand::CreatePlaylist { name, tracks })) =
            view.key(Key::Enter, false, false, &s)
        else {
            panic!("expected explicit playlist creation");
        };
        assert_eq!(name, "Été à São Paulo");
        assert_eq!(
            tracks
                .iter()
                .map(|row| row.uri.as_str())
                .collect::<Vec<_>>(),
            ["spotify:track:2", "spotify:track:5"]
        );
        assert_eq!(view.query, query);
        assert!(view.picker.is_none());
    }
}

#[test]
fn empty_whitespace_and_overlong_names_disable_creation_without_truncating_to_valid_names() {
    let (mut view, s) = setup(true);
    assert!(!view.enabled(Button::PickerCreate, &s));
    assert!(view.key(Key::Enter, false, false, &s).is_none());
    view.paste(" \t\n ");
    assert!(!view.enabled(Button::PickerCreate, &s));
    view.key(Key::A, true, false, &s);
    view.paste(&"é".repeat(101));
    assert_eq!(view.picker.as_ref().unwrap().name.chars().count(), 101);
    assert!(!view.enabled(Button::PickerCreate, &s));
    assert!(click(&mut view, 370, 359, &s).is_none());
    view.key(Key::Backspace, false, false, &s);
    assert!(view.enabled(Button::PickerCreate, &s));
    view.key(Key::A, true, false, &s);
    view.paste(&"🦙".repeat(100));
    assert_eq!(view.picker.as_ref().unwrap().name.len(), 400);
    assert!(view.enabled(Button::PickerCreate, &s));
    view.type_char('🦙');
    assert!(!view.enabled(Button::PickerCreate, &s));
}

#[test]
fn name_unicode_cursor_clipboard_and_search_text_are_independent() {
    let (mut view, s) = setup(true);
    let original_query = view.query.clone();
    view.paste("Été à 東京");
    view.key(Key::Home, false, false, &s);
    view.key(Key::Right, false, true, &s);
    assert!(matches!(view.key(Key::C, true, false, &s), Some(Action::Copy(ref s)) if s == "É"));
    assert!(matches!(view.key(Key::X, true, false, &s), Some(Action::Copy(ref s)) if s == "É"));
    assert!(matches!(
        view.key(Key::V, true, false, &s),
        Some(Action::Paste)
    ));
    view.paste("Une ");
    assert_eq!(view.picker.as_ref().unwrap().name, "Une té à 東京");
    view.key(Key::End, false, false, &s);
    view.key(Key::Left, false, true, &s);
    view.type_char('京');
    assert_eq!(view.picker.as_ref().unwrap().name, "Une té à 東京");
    view.key(Key::Home, false, false, &s);
    view.key(Key::Delete, false, false, &s);
    assert_eq!(view.picker.as_ref().unwrap().name, "ne té à 東京");
    assert_eq!(view.query, original_query);
    assert!(click(&mut view, 197, 174, &s).is_none());
    view.type_char('U');
    assert_eq!(view.picker.as_ref().unwrap().name, "Une té à 東京");
}

#[test]
fn back_preserves_name_tracks_and_destination_then_escape_cancels_without_commands() {
    let (mut view, s) = setup(false);
    view.paste("Jazz du soir");
    assert!(view.key(Key::Escape, false, false, &s).is_none());
    assert!(!view.wants_text_input());
    view.key(Key::Down, false, false, &s);
    let destination = view.picker.as_ref().unwrap().destination.clone();
    assert!(view.key(Key::N, true, false, &s).is_none());
    assert_eq!(view.picker.as_ref().unwrap().name, "Jazz du soir");
    assert!(click(&mut view, 490, 359, &s).is_none());
    assert_eq!(view.picker.as_ref().unwrap().destination, destination);
    assert!(view.key(Key::Escape, false, false, &s).is_none());
    assert!(view.picker.is_none());
}

#[test]
fn creation_is_disabled_when_busy_or_disconnected_and_channel_rejection_preserves_retry() {
    let (mut view, mut s) = setup(true);
    view.paste("My playlist");
    s.revision += 1;
    s.playlist_write_busy = true;
    assert!(view.key(Key::Enter, false, false, &s).is_none());
    assert!(view.picker.is_some());
    s.revision += 1;
    s.playlist_write_busy = false;
    let Some(Action::Command(command @ LibraryCommand::CreatePlaylist { .. })) =
        view.key(Key::Enter, false, false, &s)
    else {
        panic!("expected creation");
    };
    view.command_rejected(&command);
    assert_eq!(view.picker.as_ref().unwrap().name, "My playlist");
    assert_eq!(view.picker.as_ref().unwrap().tracks.len(), 2);
    assert!(view.enabled(Button::PickerCreate, &s));
    s.revision += 1;
    s.connected = false;
    view.render(&s);
    assert!(view.picker.is_none());
    assert!(!view.enabled(Button::PickerCreate, &s));
    assert!(!view.wants_text_input());
}

#[test]
fn no_existing_playlists_are_needed_to_create_and_name_dialog_has_no_idle_repaints() {
    for skin in [Skin::Classic, Skin::Modern] {
        let (mut view, mut s) = setup(true);
        view.set_skin(skin);
        s.entries = Arc::new(vec![]);
        s.revision += 1;
        view.paste("Jazz");
        assert!(view.render(&s));
        let buffer = view.pixels.as_ptr();
        assert!(!view.render(&s));
        assert_eq!(view.pixels.as_ptr(), buffer);
        assert!(view.enabled(Button::PickerCreate, &s));
        view.set_focus(false);
        view.type_char('x');
        assert_eq!(view.picker.as_ref().unwrap().name, "Jazz");
        assert!(view.key(Key::Enter, false, false, &s).is_none());
    }
}

#[test]
fn later_unrelated_command_rejection_cannot_restore_a_submitted_creation() {
    let (mut view, s) = setup(true);
    view.paste("Created playlist");
    assert!(matches!(
        view.key(Key::Enter, false, false, &s),
        Some(Action::Command(LibraryCommand::CreatePlaylist { .. }))
    ));
    view.key(Key::F, true, false, &s);
    view.command_rejected(&LibraryCommand::Refresh);
    assert!(view.picker.is_none());
}
