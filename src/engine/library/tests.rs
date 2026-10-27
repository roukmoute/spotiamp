use super::*;

const TRACK: &str = "spotify:track:4uLU6hMCjMI75M1A2tKUQC";
const PLAYLIST: &str = "spotify:playlist:4uLU6hMCjMI75M1A2tKUQC";

#[test]
fn created_playlist_statuses_use_supported_bitmap_glyphs_in_all_languages() {
    use crate::locale::Language;
    use font8x8::UnicodeFonts;
    let error = LocalizedText::from(
        "Ajout a votre bibliotheque non confirme; verifiez vos playlists avant de reessayer",
    );
    for status in [
        playlist_created_adding("New playlist"),
        playlist_created_success("New playlist", 12),
        playlist_created_error("New playlist", &error),
    ] {
        for language in [
            Language::French,
            Language::English,
            Language::German,
            Language::Spanish,
            Language::Italian,
            Language::Portuguese,
        ] {
            for character in status.text(language).chars() {
                assert!(
                    font8x8::BASIC_FONTS
                        .get(character)
                        .or_else(|| font8x8::LATIN_FONTS.get(character))
                        .is_some(),
                    "Unsupported {language:?} status glyph: {character}"
                );
            }
        }
    }
}

#[test]
fn long_created_names_leave_recovery_advice_visible_in_all_languages() {
    use crate::locale::Language;
    let name = "🎵".repeat(100);
    let errors = [
        LocalizedText::from(
            "Ajout a votre bibliotheque non confirme; verifiez vos playlists avant de reessayer",
        ),
        LocalizedText::from("Spotify a refuse l'ajout a votre bibliotheque; aucun titre ajoute"),
        LocalizedText::from("Impossible de verifier vos playlists; aucun titre ajoute"),
        LocalizedText::from("Ajout non confirme; verifiez la playlist avant de reessayer"),
    ];
    for language in [
        Language::French,
        Language::English,
        Language::German,
        Language::Spanish,
        Language::Italian,
        Language::Portuguese,
    ] {
        for error in &errors {
            let status = playlist_created_error(&name, error);
            let text = status.text(language);
            assert!(text.ends_with(error.text(language)), "{text}");
            assert!(text.chars().count() <= 170, "{text}");
        }
        assert!(
            playlist_created_adding(&name)
                .text(language)
                .chars()
                .count()
                <= 170
        );
        assert!(
            playlist_created_success(&name, 500)
                .text(language)
                .chars()
                .count()
                <= 170
        );
    }
    assert_eq!(name.chars().count(), 100);
    assert_eq!(playlist_status_name("Été"), "Été");
    assert_eq!(playlist_status_name(&name).chars().count(), 40);
}

#[test]
fn creation_retains_destination_on_failed_add_and_preserves_audio_and_source_selection() {
    for outcome in [
        Ok(crate::spotify_playlist_write::Receipt { total: Some(3) }),
        Err(LocalizedText::from(
            "Spotify a refuse l'ajout; aucun titre ajoute",
        )),
        Err(LocalizedText::from(
            "Ajout non confirme; verifiez la playlist avant de reessayer",
        )),
    ] {
        let mut worker = worker();
        let before = worker.snapshot();
        let source = worker.library.shared.lock().unwrap().selected_id.clone();
        let id = PlaylistId::Spotify("spotify:playlist:0000000000000000000001".into());
        worker.library.edit(|s| s.playlist_write_busy = true);
        worker.library_update(Update::Creation {
            generation: 0,
            auth_generation: 0,
            name: "Été".into(),
            step: CreationUpdate::Created(id.clone()),
        });
        {
            let library = worker.library.shared.lock().unwrap();
            assert!(library.playlist_write_busy);
            assert_eq!(library.entries[0].id, id);
            assert_eq!(library.entries[0].track_count, Some(0));
        }
        let success = outcome.is_ok();
        worker.library_update(Update::Creation {
            generation: 0,
            auth_generation: 0,
            name: "Été".into(),
            step: CreationUpdate::Finished {
                id: Some(id.clone()),
                count: 3,
                result: outcome,
            },
        });
        let library = worker.library.shared.lock().unwrap();
        assert!(!library.playlist_write_busy);
        assert_eq!(library.selected_id, source);
        assert_eq!(library.entries.len(), 1);
        assert_eq!(library.entries[0].id, id);
        assert_eq!(library.entries[0].track_count, success.then_some(3));
        assert!(
            library
                .playlist_write_status
                .text(crate::locale::Language::English)
                .starts_with("Playlist \"Été\" created.")
                || success
        );
        assert_eq!(worker.snapshot(), before);
    }
}

#[test]
fn stale_creation_cannot_insert_destination_or_release_current_write() {
    let mut worker = worker();
    worker.library.edit(|s| s.playlist_write_busy = true);
    for (generation, auth_generation) in [(1, 0), (0, 1)] {
        worker.library_update(Update::Creation {
            generation,
            auth_generation,
            name: "Stale".into(),
            step: CreationUpdate::Created(PlaylistId::Spotify(PLAYLIST.into())),
        });
        worker.library_update(Update::Creation {
            generation,
            auth_generation,
            name: "Stale".into(),
            step: CreationUpdate::Finished {
                id: None,
                count: 1,
                result: Err("Failure".into()),
            },
        });
    }
    let library = worker.library.shared.lock().unwrap();
    assert!(library.entries.is_empty());
    assert!(library.playlist_write_busy);
}

#[test]
fn earlier_catalogue_reply_cannot_erase_a_newly_created_destination() {
    let mut worker = worker();
    worker.library.edit(|s| {
        s.loading = true;
        s.selected_id = None;
    });
    let id = PlaylistId::Spotify(PLAYLIST.into());
    worker.library_update(Update::Creation {
        generation: 0,
        auth_generation: 0,
        name: "Created".into(),
        step: CreationUpdate::Created(id.clone()),
    });
    worker.library_update(Update::Creation {
        generation: 0,
        auth_generation: 0,
        name: "Created".into(),
        step: CreationUpdate::Finished {
            id: Some(id.clone()),
            count: 3,
            result: Ok(crate::spotify_playlist_write::Receipt { total: Some(3) }),
        },
    });
    worker.library_update(Update::Catalogue {
        generation: 0,
        result: Ok(vec![]),
    });
    let library = worker.library.shared.lock().unwrap();
    assert_eq!(library.entries[0].id, id);
    assert_eq!(library.entries[0].track_count, Some(3));
    assert!(worker.library.catalogue_created.is_empty());
}

#[tokio::test]
async fn duplicate_create_is_blocked_until_the_existing_write_finishes() {
    let mut worker = worker();
    worker.library.write_task = Some(tokio::spawn(std::future::pending()));
    worker.library.write_creating = true;
    worker.library.edit(|s| s.playlist_write_busy = true);
    let generation = worker.library.write_generation;
    let (updates, mut receiver) = mpsc::channel(1);
    worker.library_command(
        LibraryCommand::CreatePlaylist {
            name: "Second".into(),
            tracks: vec![row("Track")],
        },
        &updates,
    );
    assert_eq!(worker.library.write_generation, generation);
    assert!(worker.library.shared.lock().unwrap().playlist_write_busy);
    assert!(receiver.try_recv().is_err());
    worker.reset_library("Connectez Spotify pour afficher vos playlists");
    assert!(!worker.library.shared.lock().unwrap().playlist_write_busy);
    assert_eq!(
        worker.library.shared.lock().unwrap().playlist_write_status,
        "Creation ou ajout non confirme; verifiez vos playlists avant de reessayer"
    );
}

fn row(title: &str) -> TrackRow {
    TrackRow {
        uri: TRACK.into(),
        title: title.into(),
        duration_ms: 123_000,
        ..TrackRow::default()
    }
}

fn worker() -> Worker {
    let mut worker = Worker::new(Arc::new(Mutex::new(Snapshot {
        queue: vec![row("Existing")],
        current: Some(0),
        position_ms: 42_000,
        playing: true,
        ..Snapshot::default()
    })));
    worker.library.open = true;
    worker
        .library
        .edit(|s| s.selected_id = Some(PlaylistId::Spotify(PLAYLIST.into())));
    worker
}

fn preview_message(uris: &[&str]) -> librespot_protocol::playlist4_external::SelectedListContent {
    use librespot_protocol::playlist4_external::{
        Item, ListAttributes, ListItems, SelectedListContent,
    };
    use protobuf::MessageField;

    let mut message = SelectedListContent::new();
    message.set_owner_username("account-id".into());
    message.set_revision(vec![0, 17, 255, 3]);
    message.set_length(uris.len() as i32);
    let mut attributes = ListAttributes::new();
    attributes.set_name("A display name, not an account ID".into());
    message.attributes = MessageField::some(attributes);
    let mut contents = ListItems::new();
    contents.set_pos(0);
    contents.set_truncated(false);
    contents.items = uris
        .iter()
        .map(|uri| {
            let mut item = Item::new();
            item.set_uri((*uri).into());
            item
        })
        .collect();
    message.contents = MessageField::some(contents);
    message
}

fn parsed_preview(
    message: &librespot_protocol::playlist4_external::SelectedListContent,
) -> Playlist {
    // The incoming URI has no owner; the production metadata parser decorates
    // it from owner_username in the server response.
    Playlist::parse(message, &SpotifyUri::from_uri(PLAYLIST).unwrap()).unwrap()
}

#[test]
fn playlist_preview_uses_parsed_owner_id_and_keeps_revision_and_duplicate_occurrences() {
    let message = preview_message(&[TRACK, TRACK]);
    let playlist = parsed_preview(&message);
    assert!(
        matches!(&playlist.id, SpotifyUri::Playlist { user: Some(owner), .. } if owner == "account-id")
    );
    let (tracks, truncated, revision, owned) = playlist_preview(playlist.clone(), "account-id");
    assert!(owned);
    assert!(!truncated);
    assert_eq!(revision, vec![0, 17, 255, 3]);
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0], tracks[1]);
    for username in ["", "ACCOUNT-ID", "another-account", playlist.name()] {
        assert!(
            !playlist_preview(playlist.clone(), username).3,
            "{username}"
        );
    }
    let mut misleading = message.clone();
    misleading
        .attributes
        .as_mut()
        .unwrap()
        .set_name("account-id".into());
    misleading.set_owner_username("different-owner".into());
    assert!(!playlist_preview(parsed_preview(&misleading), "account-id").3);
    misleading.set_owner_username(String::new());
    assert!(!playlist_preview(parsed_preview(&misleading), "account-id").3);
}

#[test]
fn playlist_preview_preserves_readable_tracks_but_refuses_unsafe_occurrence_indices() {
    let complete = preview_message(&[TRACK, TRACK]);
    let mut cases = Vec::new();
    let mut changed = complete.clone();
    changed.contents.as_mut().unwrap().set_truncated(true);
    cases.push(("truncated response", changed, true));
    let mut changed = complete.clone();
    changed.set_length(3);
    cases.push(("missing items", changed, false));
    let mut changed = complete.clone();
    changed.set_length(1);
    cases.push(("extra items", changed, false));
    let mut changed = complete.clone();
    changed.set_length(-1);
    cases.push(("invalid length", changed, true));
    let mut changed = complete.clone();
    changed.contents.as_mut().unwrap().set_pos(1);
    cases.push(("nonzero offset", changed, false));
    let mut changed = complete.clone();
    changed.set_multiple_heads(true);
    cases.push(("conflicting heads", changed, false));
    let mut changed = complete.clone();
    changed
        .attributes
        .as_mut()
        .unwrap()
        .set_deleted_by_owner(true);
    cases.push(("deleted playlist", changed, false));
    let mut changed = complete.clone();
    changed.clear_revision();
    cases.push(("missing revision", changed, false));
    for (name, message, expected_truncation) in cases {
        let (tracks, truncated, revision, owned) =
            playlist_preview(parsed_preview(&message), "account-id");
        assert_eq!(tracks.len(), 2, "{name}");
        assert_eq!(truncated, expected_truncation, "{name}");
        assert_eq!(revision, message.revision(), "{name}");
        assert!(!owned, "{name}");
    }
    for filtered in [
        "spotify:episode:4uLU6hMCjMI75M1A2tKUQC",
        "spotify:local:Artist:Album:Title:123",
    ] {
        let message = preview_message(&[TRACK, filtered, TRACK]);
        let (tracks, truncated, _, owned) =
            playlist_preview(parsed_preview(&message), "account-id");
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0], tracks[1]);
        assert!(!truncated);
        assert!(
            !owned,
            "Filtered URI must not shift removal indices: {filtered}"
        );
    }
    for count in [MAX_QUEUE, MAX_QUEUE + 1] {
        let message = preview_message(&vec![TRACK; count]);
        let (tracks, truncated, _, owned) =
            playlist_preview(parsed_preview(&message), "account-id");
        assert_eq!(tracks.len(), MAX_QUEUE);
        assert_eq!(truncated, count > MAX_QUEUE);
        assert_eq!(owned, count == MAX_QUEUE);
    }
}

#[test]
fn playlist_access_cannot_cross_preview_generation_or_closed_library() {
    for (open, generation, accepted) in [(true, 4, true), (true, 3, false), (false, 4, false)] {
        let mut worker = worker();
        worker.library.open = open;
        worker.library.generation = 4;
        worker.library.edit(|s| {
            s.tracks = Arc::new(vec![row("Still browsed")]);
            s.selected_playlist_revision = Arc::from([1, 2]);
        });
        let before = worker.snapshot();
        let tracks = worker.library.shared.lock().unwrap().tracks.clone();
        worker.library_update(Update::PlaylistAccess {
            generation,
            revision: vec![9, 0, 8],
            owned: true,
        });
        let library = worker.library.shared.lock().unwrap();
        assert_eq!(library.selected_owned, accepted);
        assert_eq!(
            &*library.selected_playlist_revision,
            if accepted {
                &[9, 0, 8][..]
            } else {
                &[1, 2][..]
            }
        );
        assert_eq!(
            library.selected_id,
            Some(PlaylistId::Spotify(PLAYLIST.into()))
        );
        assert!(Arc::ptr_eq(&library.tracks, &tracks));
        assert_eq!(worker.snapshot(), before);
    }
}

#[test]
fn playlist_read_failure_and_reset_clear_previous_removal_authority() {
    let mut worker = worker();
    let before = worker.snapshot();
    for failure in [true, false] {
        worker.library.edit(|s| {
            s.selected_owned = true;
            s.selected_playlist_revision = Arc::from([8, 1]);
        });
        if failure {
            worker.library_update(Update::Loaded {
                generation: worker.library.generation,
                result: Err("Read refused".into()),
            });
        } else {
            worker.reset_playlist("Reset");
        }
        let library = worker.library.shared.lock().unwrap();
        assert!(!library.selected_owned);
        assert!(library.selected_playlist_revision.is_empty());
        assert_eq!(worker.snapshot(), before);
    }
}

#[test]
fn removal_result_preserves_audio_and_browsing_refreshes_only_confirmed_changes() {
    for success in [true, false] {
        for browsing_destination in [true, false] {
            let mut worker = worker();
            let destination = PlaylistId::Spotify(PLAYLIST.into());
            let browsed = if browsing_destination {
                destination.clone()
            } else {
                PlaylistId::Spotify("spotify:playlist:0000000000000000000001".into())
            };
            worker.library.write_removing = true;
            worker.library.edit(|s| {
                s.selected_id = Some(browsed.clone());
                s.selected_owned = true;
                s.selected_playlist_revision = Arc::from([3, 8]);
                s.tracks = Arc::new(vec![row("Browsed"), row("Browsed duplicate")]);
                s.entries = Arc::new(vec![PlaylistEntry {
                    id: destination.clone(),
                    name: "Destination".into(),
                    track_count: Some(5),
                    duration_ms: Some(600_000),
                    folder: "Folder".into(),
                    available: true,
                }]);
                s.playlist_write_busy = true;
                s.playlist_remove_revision = u64::MAX;
                s.status = "Browsing status".into();
            });
            let before = worker.snapshot();
            let tracks = worker.library.shared.lock().unwrap().tracks.clone();
            worker.library_update(Update::Removed {
                generation: 0,
                auth_generation: 0,
                id: destination.clone(),
                count: 2,
                result: if success {
                    Ok(crate::spotify_playlist_write::Receipt { total: Some(3) })
                } else {
                    Err("Retrait non confirme; verifiez la playlist avant de reessayer".into())
                },
            });
            let library = worker.library.shared.lock().unwrap();
            assert!(!worker.library.write_removing);
            assert!(!library.playlist_write_busy);
            assert_eq!(library.playlist_remove_revision, 0);
            assert_eq!(library.selected_id, Some(browsed));
            assert_eq!(library.selected_owned, !browsing_destination);
            assert_eq!(&*library.selected_playlist_revision, &[3, 8]);
            assert!(
                Arc::ptr_eq(&library.tracks, &tracks),
                "Never remove rows optimistically"
            );
            assert_eq!(library.status, "Browsing status");
            assert_eq!(
                library.entries[0].track_count,
                Some(if success { 3 } else { 5 })
            );
            assert_eq!(
                library.entries[0].duration_ms,
                if success { None } else { Some(600_000) }
            );
            assert_eq!(worker.library.write_refresh, success.then_some(destination));
            if success {
                assert_eq!(
                    library.playlist_write_status.text(Language::English),
                    "2 track(s) removed from the Spotify playlist"
                );
            } else {
                assert_eq!(
                    library.playlist_write_status,
                    "Retrait non confirme; verifiez la playlist avant de reessayer"
                );
            }
            assert_eq!(worker.snapshot(), before);
        }
    }
}

#[tokio::test]
async fn stale_removal_cannot_unlock_current_write_or_change_library() {
    let mut worker = worker();
    let destination = PlaylistId::Spotify(PLAYLIST.into());
    worker.library.write_task = Some(tokio::spawn(std::future::pending()));
    worker.library.write_removing = true;
    worker.library.write_generation = 8;
    worker.auth_generation = 6;
    worker.library.edit(|s| {
        s.selected_owned = true;
        s.selected_playlist_revision = Arc::from([2]);
        s.tracks = Arc::new(vec![row("Kept")]);
        s.playlist_write_busy = true;
        s.playlist_remove_revision = 11;
        s.playlist_write_status = "Current operation".into();
    });
    let before = worker.snapshot();
    let tracks = worker.library.shared.lock().unwrap().tracks.clone();
    for (generation, auth_generation) in [(7, 6), (8, 5)] {
        for success in [true, false] {
            worker.library_update(Update::Removed {
                generation,
                auth_generation,
                id: destination.clone(),
                count: 1,
                result: if success {
                    Ok(crate::spotify_playlist_write::Receipt { total: Some(0) })
                } else {
                    Err("Stale failure".into())
                },
            });
            let library = worker.library.shared.lock().unwrap();
            assert!(worker.library.write_task.is_some());
            assert!(worker.library.write_removing);
            assert!(worker.library.write_refresh.is_none());
            assert!(library.playlist_write_busy);
            assert!(library.selected_owned);
            assert_eq!(library.selected_id, Some(destination.clone()));
            assert_eq!(&*library.selected_playlist_revision, &[2]);
            assert_eq!(library.playlist_remove_revision, 11);
            assert_eq!(library.playlist_write_status, "Current operation");
            assert!(Arc::ptr_eq(&library.tracks, &tracks));
            assert_eq!(worker.snapshot(), before);
        }
    }
}

#[tokio::test]
async fn hiding_library_preserves_removal_but_logout_reports_uncertain_outcome() {
    let mut worker = worker();
    worker.library.write_task = Some(tokio::spawn(std::future::pending()));
    worker.library.write_removing = true;
    worker.library.edit(|s| {
        s.playlist_write_busy = true;
        s.playlist_write_status = "Retrait de la playlist Spotify en cours...".into();
        s.selected_owned = true;
        s.selected_playlist_revision = Arc::from([1]);
        s.playlist_remove_revision = 9;
    });
    let before = worker.snapshot();
    let generation = worker.library.write_generation;
    let (updates, _) = mpsc::channel(1);
    worker.library_command(LibraryCommand::Close, &updates);
    assert!(!worker.library.open);
    assert!(worker.library.write_task.is_some());
    assert!(worker.library.write_removing);
    assert_eq!(worker.library.write_generation, generation);
    {
        let library = worker.library.shared.lock().unwrap();
        assert!(library.playlist_write_busy);
        assert_eq!(library.playlist_remove_revision, 9);
        assert_eq!(
            library.playlist_write_status,
            "Retrait de la playlist Spotify en cours..."
        );
        assert!(!library.selected_owned);
        assert!(library.selected_playlist_revision.is_empty());
    }
    worker.reset_library("Connectez Spotify pour afficher vos playlists");
    assert!(worker.library.write_task.is_none());
    assert!(!worker.library.write_removing);
    assert!(!worker.library.write_creating);
    assert_eq!(worker.library.write_generation, generation.wrapping_add(1));
    assert!(worker.library.write_refresh.is_none());
    let library = worker.library.shared.lock().unwrap();
    assert!(!library.playlist_write_busy);
    assert_eq!(library.playlist_remove_revision, 10);
    assert_eq!(
        library.playlist_write_status,
        "Retrait non confirme; verifiez la playlist avant de reessayer"
    );
    assert_eq!(worker.snapshot(), before);
}

#[test]
fn removal_success_statuses_are_distinct_and_bitmap_renderable_in_six_languages() {
    use font8x8::UnicodeFonts;
    let status = playlist_remove_success(12);
    let mut translations = std::collections::BTreeSet::new();
    for language in Language::ALL {
        let text = status.text(language);
        assert!(text.starts_with("12 "));
        assert!(translations.insert(text));
        for character in text.chars() {
            assert!(
                font8x8::BASIC_FONTS
                    .get(character)
                    .or_else(|| font8x8::LATIN_FONTS.get(character))
                    .is_some(),
                "Unsupported {language:?} status glyph: {character}"
            );
        }
    }
}

fn request(worker: &Worker, replace: bool) -> QueueRequest {
    QueueRequest {
        replace,
        play: false,
        start: 0,
        audio_generation: worker.generation,
    }
}

#[test]
fn failed_empty_or_invalid_playlist_preserves_queue_and_playback() {
    let mut worker = worker();
    let before = worker.snapshot();
    worker.library.pending_queue = Some(request(&worker, true));
    worker.library_update(Update::Loaded {
        generation: 0,
        result: Err("Acces refuse".into()),
    });
    assert_eq!(worker.snapshot(), before);
    assert!(worker.library.pending_queue.is_none());
    assert_eq!(worker.library.shared.lock().unwrap().status, "Acces refuse");

    worker.apply_library_queue(&[], request(&worker, true));
    assert_eq!(worker.snapshot(), before);
    worker.apply_library_queue(
        &[row("New")],
        QueueRequest {
            start: 1,
            ..request(&worker, true)
        },
    );
    assert_eq!(worker.snapshot(), before);
    worker.apply_library_queue(
        &[row("New")],
        QueueRequest {
            play: true,
            ..request(&worker, true)
        },
    );
    assert_eq!(worker.snapshot(), before);
}

#[test]
fn successful_load_commits_replacement_once_and_metadata_preserves_order() {
    let mut worker = worker();
    worker.library.pending_queue = Some(request(&worker, true));
    worker.library_update(Update::Loaded {
        generation: 0,
        result: Ok((vec![row("New"), row("New")], false)),
    });
    assert_eq!(worker.snapshot().queue, vec![row("New"), row("New")]);
    assert_eq!(worker.snapshot().current, None);
    assert!(!worker.snapshot().playing);
    assert_eq!(worker.snapshot().position_ms, 0);
    assert_eq!(worker.library.shared.lock().unwrap().queue_len, 2);
    assert!(worker.library.pending_queue.is_none());
    worker.library_update(Update::Metadata {
        generation: 0,
        rows: vec![row("Resolved")],
    });
    assert_eq!(
        worker.snapshot().queue,
        vec![row("Resolved"), row("Resolved")]
    );
    assert_eq!(
        worker.library.shared.lock().unwrap().tracks.as_ref(),
        &vec![row("Resolved"), row("Resolved")]
    );
}

#[test]
fn clear_close_and_selection_epochs_reject_in_flight_results() {
    let mut worker = worker();
    let (updates, _) = mpsc::channel(1);
    worker.library.pending_queue = Some(request(&worker, true));
    worker.command(Command::Clear, &updates);
    worker.library_update(Update::Loaded {
        generation: 0,
        result: Ok((vec![row("Too late")], false)),
    });
    assert!(worker.snapshot().queue.is_empty());
    assert_eq!(worker.library.shared.lock().unwrap().tracks.len(), 1);

    let held = Arc::downgrade(&worker.library.shared.lock().unwrap().tracks);
    worker.library_command(LibraryCommand::Close, &updates);
    assert!(held.upgrade().is_none());
    worker.library_update(Update::Loaded {
        generation: 0,
        result: Ok((vec![row("Stale")], false)),
    });
    assert!(worker.library.shared.lock().unwrap().tracks.is_empty());
    assert!(worker.library.shared.lock().unwrap().entries.is_empty());

    worker.library.open = true;
    let previous = worker.library.generation;
    worker.library.cancel();
    worker.library_update(Update::Metadata {
        generation: previous,
        rows: vec![row("Stale")],
    });
    assert!(worker.library.shared.lock().unwrap().tracks.is_empty());
    worker.library_update(Update::Catalogue {
        generation: previous,
        result: Ok(vec![PlaylistEntry {
            id: PlaylistId::Spotify(PLAYLIST.into()),
            name: "Stale account".into(),
            track_count: None,
            duration_ms: None,
            folder: String::new(),
            available: true,
        }]),
    });
    assert!(worker.library.shared.lock().unwrap().entries.is_empty());
}

#[test]
fn append_is_bounded_and_preserves_current_audio_state() {
    let mut worker = worker();
    edit(&worker.state, |s| {
        s.queue = vec![row("Existing"); MAX_QUEUE - 2]
    });
    worker.apply_library_queue(&vec![row("New"); 10], request(&worker, false));
    let audio = worker.snapshot();
    assert_eq!(audio.queue.len(), MAX_QUEUE);
    assert_eq!(audio.queue[MAX_QUEUE - 1].title, "New");
    assert_eq!(audio.current, Some(0));
    assert!(audio.playing);
    assert_eq!(audio.position_ms, 42_000);
    assert_eq!(worker.library.shared.lock().unwrap().queue_len, MAX_QUEUE);
    let before = worker.snapshot();
    worker.apply_library_queue(&[row("Rejected")], request(&worker, false));
    assert_eq!(worker.snapshot(), before);
}

#[test]
fn newer_stop_and_import_supersede_a_delayed_library_play() {
    let mut worker = worker();
    let (updates, _) = mpsc::channel(1);
    worker.library.pending_queue = Some(QueueRequest {
        play: true,
        ..request(&worker, true)
    });
    worker.command(Command::Stop, &updates);
    worker.library_update(Update::Loaded {
        generation: 0,
        result: Ok((vec![row("Too late")], false)),
    });
    assert_eq!(worker.snapshot().queue, vec![row("Existing")]);
    assert!(!worker.snapshot().playing);
    assert_eq!(
        worker.library.shared.lock().unwrap().tracks[0].title,
        "Too late"
    );

    // Even if an import starts after requesting the playlist, a library
    // response cannot replace the import's newer queue contents.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        worker.import_task = Some(tokio::spawn(std::future::pending()));
        let before = worker.snapshot();
        worker.apply_library_queue(&[row("Rejected")], request(&worker, true));
        assert_eq!(worker.snapshot(), before);
        worker.import_task.take().unwrap().abort();
    });
}

#[test]
fn closing_aborts_read_job_and_releases_its_retained_data() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut worker = worker();
        let retained = Arc::new(vec![row("Pending response")]);
        let weak = Arc::downgrade(&retained);
        worker.library.task = Some(tokio::spawn(async move {
            std::future::pending::<()>().await;
            drop(retained);
        }));
        tokio::task::yield_now().await;
        let (updates, _) = mpsc::channel(1);
        worker.library_command(LibraryCommand::Close, &updates);
        tokio::task::yield_now().await;
        assert!(weak.upgrade().is_none());
        assert!(worker.library.task.is_none());
    });
}

#[test]
fn audio_summary_changes_do_not_clone_library_or_tick_revision_on_position_updates() {
    let worker = worker();
    worker
        .library
        .edit(|s| s.tracks = Arc::new(vec![row("Selected")]));
    worker.sync_library_summary();
    let library = worker.library.shared.lock().unwrap().clone();
    edit(&worker.state, |s| s.position_ms += 500);
    worker.sync_library_summary();
    let after = worker.library.shared.lock().unwrap().clone();
    assert_eq!(after.revision, library.revision);
    assert!(Arc::ptr_eq(&after.tracks, &library.tracks));
    assert!(Arc::ptr_eq(&after.entries, &library.entries));
    edit(&worker.state, |s| s.connected = true);
    worker.sync_library_summary();
    assert_eq!(
        worker.library.shared.lock().unwrap().revision,
        after.revision + 1
    );
}

#[derive(Debug, PartialEq)]
enum TransportCall {
    Load(SpotifyUri, bool, u32, Bitrate),
    Stop,
}

#[derive(Default)]
struct RecordingPlayer(Mutex<Vec<TransportCall>>);
impl RecordingPlayer {
    fn take(&self) -> Vec<TransportCall> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}
impl PlayerTransport for RecordingPlayer {
    fn load_with_bitrate(&self, uri: SpotifyUri, play: bool, position: u32, bitrate: Bitrate) {
        self.0
            .lock()
            .unwrap()
            .push(TransportCall::Load(uri, play, position, bitrate));
    }
    fn stop(&self) {
        self.0.lock().unwrap().push(TransportCall::Stop);
    }
    fn play(&self) {
        panic!("playlist play must load its requested track");
    }
    fn pause(&self) {
        panic!("playlist loading must not pause current playback");
    }
    fn seek(&self, _: u32) {
        panic!("playlist loading must not seek current playback");
    }
    fn is_invalid(&self) -> bool {
        false
    }
}

fn connected_worker() -> (Worker, Arc<RecordingPlayer>, PlaylistId) {
    let mut worker = worker();
    worker.session = Some(Session::new(SessionConfig::default(), None));
    let player = Arc::new(RecordingPlayer::default());
    worker.player = Some(player.clone());
    let id = PlaylistId::Spotify(PLAYLIST.into());
    worker.library.edit(|s| {
        s.entries = Arc::new(vec![PlaylistEntry {
            id: id.clone(),
            name: "Selected playlist".into(),
            track_count: Some(1),
            duration_ms: None,
            folder: String::new(),
            available: true,
        }]);
    });
    (worker, player, id)
}

fn queue_command(id: PlaylistId) -> Command {
    Command::Library(LibraryCommand::Queue {
        id,
        replace: true,
        play: true,
        start: 0,
    })
}

#[test]
fn preview_then_play_reuses_one_read_before_or_after_the_response_arrives() {
    // Enter but never poll this runtime: exercise real command dispatch and
    // task identities without allowing any Spotify request to execute.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    for response_first in [false, true] {
        let (mut worker, player, id) = connected_worker();
        let (updates, _) = mpsc::channel(1);
        let before = worker.snapshot();
        worker.command(
            Command::Library(LibraryCommand::Select(id.clone())),
            &updates,
        );
        let generation = worker.library.generation;
        let task = worker.library.task.as_ref().unwrap().id();
        worker.command(
            Command::Library(LibraryCommand::Select(id.clone())),
            &updates,
        );
        assert_eq!(worker.library.generation, generation);
        assert_eq!(worker.library.task.as_ref().unwrap().id(), task);
        assert_eq!(worker.snapshot(), before);
        assert!(player.take().is_empty());

        if response_first {
            worker.library_update(Update::Loaded {
                generation,
                result: Ok((vec![row("Playlist track")], false)),
            });
            assert_eq!(worker.snapshot(), before, "single click only previews");
            assert!(player.take().is_empty());
        }
        worker.command(queue_command(id), &updates);
        assert_eq!(worker.library.generation, generation);
        assert_eq!(worker.library.task.as_ref().unwrap().id(), task);
        if !response_first {
            assert!(worker.library.shared.lock().unwrap().busy);
            assert_eq!(worker.snapshot(), before);
            assert!(player.take().is_empty());
            worker.library_update(Update::Loaded {
                generation,
                result: Ok((vec![row("Playlist track")], false)),
            });
        }
        assert_eq!(
            player.take(),
            vec![
                TransportCall::Stop,
                TransportCall::Load(parse_input(TRACK).unwrap(), true, 0, Bitrate::Bitrate320),
            ]
        );
        assert_eq!(worker.snapshot().queue, vec![row("Playlist track")]);
        assert_eq!(worker.snapshot().current, Some(0));
        assert!(worker.library.pending_queue.is_none());
        assert!(!worker.library.shared.lock().unwrap().busy);
    }
}

#[test]
fn reused_preview_play_still_obeys_stop_clear_close_and_new_selection() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    for action in ["stop", "clear", "close", "select"] {
        let (mut worker, player, id) = connected_worker();
        let (updates, _) = mpsc::channel(1);
        worker.command(
            Command::Library(LibraryCommand::Select(id.clone())),
            &updates,
        );
        let generation = worker.library.generation;
        worker.command(queue_command(id), &updates);
        match action {
            "stop" => worker.command(Command::Stop, &updates),
            "clear" => worker.command(Command::Clear, &updates),
            "close" => worker.command(Command::Library(LibraryCommand::Close), &updates),
            "select" => {
                let next = PlaylistId::Spotify("spotify:playlist:0000000000000000000001".into());
                worker.library.edit(|s| {
                    let mut entry = s.entries[0].clone();
                    entry.id = next.clone();
                    Arc::make_mut(&mut s.entries).push(entry);
                });
                worker.command(Command::Library(LibraryCommand::Select(next)), &updates);
            }
            _ => unreachable!(),
        }
        let after_newer_intent = worker.snapshot();
        player.take();
        worker.library_update(Update::Loaded {
            generation,
            result: Ok((vec![row("Late response")], false)),
        });
        assert_eq!(worker.snapshot(), after_newer_intent, "{action}");
        assert!(player.take().is_empty(), "{action}");
        assert!(worker.library.pending_queue.is_none());
        assert_eq!(
            worker.library.shared.lock().unwrap().tracks.is_empty(),
            matches!(action, "close" | "select"),
            "{action}"
        );
    }
}

#[test]
fn latest_queue_intent_reuses_read_and_failure_allows_a_fresh_retry() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    let (mut worker, player, id) = connected_worker();
    let (updates, _) = mpsc::channel(1);
    worker.command(
        Command::Library(LibraryCommand::Select(id.clone())),
        &updates,
    );
    let generation = worker.library.generation;
    let task = worker.library.task.as_ref().unwrap().id();
    worker.command(queue_command(id.clone()), &updates);
    worker.command(
        Command::Library(LibraryCommand::Queue {
            id: id.clone(),
            replace: false,
            play: true,
            start: 1,
        }),
        &updates,
    );
    assert_eq!(worker.library.task.as_ref().unwrap().id(), task);
    let selected = TrackRow {
        uri: "spotify:track:0000000000000000000002".into(),
        ..row("Second")
    };
    worker.library_update(Update::Loaded {
        generation,
        result: Ok((vec![row("First"), selected.clone()], false)),
    });
    assert_eq!(
        worker.snapshot().queue,
        vec![row("Existing"), row("First"), selected.clone()]
    );
    assert_eq!(worker.snapshot().current, Some(2));
    assert_eq!(
        player.take(),
        vec![TransportCall::Load(
            parse_input(&selected.uri).unwrap(),
            true,
            0,
            Bitrate::Bitrate320
        )]
    );

    // Invalidate the cached preview and dispatch a fresh selection, then fail
    // its shared preview/play request. Failure cannot clear the existing queue.
    worker.library.cancel();
    worker.command(
        Command::Library(LibraryCommand::Select(id.clone())),
        &updates,
    );
    worker.command(queue_command(id.clone()), &updates);
    let failed_generation = worker.library.generation;
    let before = worker.snapshot();
    worker.library_update(Update::Loaded {
        generation: failed_generation,
        result: Err("Playlist indisponible".into()),
    });
    assert_eq!(worker.snapshot(), before);
    assert!(player.take().is_empty());
    assert!(worker.library.task.is_none());
    assert!(worker.library.pending_queue.is_none());
    assert!(!worker.library.shared.lock().unwrap().busy);
    worker.command(queue_command(id), &updates);
    assert!(worker.library.generation > failed_generation);
    assert!(worker.library.task.is_some());
    assert!(worker.library.shared.lock().unwrap().busy);
}

#[test]
fn restored_source_playlist_waits_for_catalogue_and_user_browsing_takes_precedence() {
    let mut worker = worker();
    let origin = PlaylistId::Spotify(PLAYLIST.into());
    worker.queue_playlist = Some(origin.clone());
    worker.restore_library_selection = Some(origin.clone());
    worker.library.shared.lock().unwrap().entries = Arc::new(vec![crate::model::PlaylistEntry {
        id: origin.clone(),
        name: "Original playlist".into(),
        track_count: None,
        duration_ms: None,
        folder: String::new(),
        available: true,
    }]);
    let (updates, _) = mpsc::channel(1);
    worker.restore_library_preview(&updates);
    assert_eq!(worker.restore_library_selection, Some(origin.clone()));
    assert_eq!(worker.take_restored_playlist(), Some(origin.clone()));
    assert_eq!(worker.take_restored_playlist(), None);
    worker.restore_library_selection = Some(origin.clone());
    // Selecting the already-loaded preview exercises user intent without network.
    worker.library.ready = Some((origin.clone(), false));
    worker.library_command(LibraryCommand::Select(origin.clone()), &updates);
    assert!(worker.restore_library_selection.is_none());
    assert_eq!(worker.queue_playlist, Some(origin));
}

#[test]
fn existing_playlist_notice_retains_count_in_all_six_languages() {
    let notice = playlist_notice(42, true);
    let expected = [
        "42 titres; apercu limite a 500 ou tronque par Spotify",
        "42 tracks; preview limited to 500 or truncated by Spotify",
        "42 Titel; Vorschau auf 500 begrenzt oder von Spotify gekürzt",
        "42 canciones; vista limitada a 500 o recortada por Spotify",
        "42 brani; anteprima limitata a 500 o troncata da Spotify",
        "42 faixas; prévia limitada a 500 ou truncada pelo Spotify",
    ];
    for (language, expected) in Language::ALL.into_iter().zip(expected) {
        assert_eq!(notice.text(language), expected);
    }
}

fn search_loaded(worker: &mut Worker, rows: Vec<TrackRow>) -> u64 {
    worker.library.cancel_search();
    let generation = worker.library.search_generation;
    worker.library_update(Update::Search {
        generation,
        update: SearchUpdate::Loaded(Ok((rows, false))),
    });
    generation
}

#[test]
fn search_updates_do_not_replace_playlist_preview_or_interrupt_audio() {
    let mut worker = worker();
    worker.library.edit(|s| {
        s.tracks = Arc::new(vec![row("Playlist preview")]);
        s.status = "Playlist status".into();
    });
    let before = worker.snapshot();
    let preview = worker.library.shared.lock().unwrap().clone();
    let generation = search_loaded(&mut worker, vec![row("Search result")]);
    let library = worker.library.shared.lock().unwrap().clone();
    assert_eq!(worker.snapshot(), before);
    assert!(Arc::ptr_eq(&preview.tracks, &library.tracks));
    assert!(Arc::ptr_eq(&preview.entries, &library.entries));
    assert_eq!(library.selected_id, preview.selected_id);
    assert_eq!(library.status, preview.status);
    assert_eq!(library.search.revision, generation);
    assert_eq!(library.search.tracks[0].title, "Search result");
    worker.library.cancel();
    worker.library_update(Update::Search {
        generation,
        update: SearchUpdate::Metadata(vec![row("Search metadata")]),
    });
    let library = worker.library.shared.lock().unwrap();
    assert_eq!(library.search.tracks[0].title, "Search metadata");
    assert_eq!(library.search.revision, generation);
    assert_eq!(library.tracks[0].title, "Playlist preview");
}

#[test]
fn search_play_queues_only_clicked_result_and_append_preserves_playback() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    for play in [false, true] {
        let (mut worker, player, id) = connected_worker();
        worker.queue_playlist = Some(id.clone());
        worker.library.ready = Some((id, false));
        let second = TrackRow {
            uri: "spotify:track:0000000000000000000002".into(),
            ..row("Chosen")
        };
        let generation = search_loaded(&mut worker, vec![row("Other result"), second.clone()]);
        let (updates, _) = mpsc::channel(1);
        worker.command(
            Command::Library(LibraryCommand::QueueSearch {
                revision: generation,
                index: 1,
                play,
            }),
            &updates,
        );
        let snapshot = worker.snapshot();
        if play {
            assert_eq!(snapshot.queue, vec![second.clone()]);
            assert_eq!(snapshot.current, Some(0));
            assert_eq!(
                player.take(),
                vec![
                    TransportCall::Stop,
                    TransportCall::Load(
                        parse_input(&second.uri).unwrap(),
                        true,
                        0,
                        Bitrate::Bitrate320
                    )
                ]
            );
        } else {
            assert_eq!(snapshot.queue, vec![row("Existing"), second]);
            assert!(snapshot.playing);
            assert_eq!(snapshot.current, Some(0));
            assert_eq!(snapshot.position_ms, 42_000);
            assert!(player.take().is_empty());
        }
        assert_eq!(
            worker.queue_playlist, None,
            "search is not the previewed playlist"
        );
    }
}

#[test]
fn stale_invalid_or_full_search_queue_requests_preserve_audio() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    let (mut worker, player, _) = connected_worker();
    let generation = search_loaded(&mut worker, vec![row("Chosen")]);
    let before = worker.snapshot();
    worker.queue_search_result(generation.wrapping_sub(1), 0, true);
    worker.queue_search_result(generation, 1, true);
    assert_eq!(worker.snapshot(), before);
    assert!(player.take().is_empty());
    edit(&worker.state, |s| {
        s.queue.resize(MAX_QUEUE, row("Existing"))
    });
    let before = worker.snapshot();
    worker.queue_search_result(generation, 0, false);
    assert_eq!(worker.snapshot(), before);
    assert!(player.take().is_empty());
}

#[test]
fn new_search_clear_close_and_logout_abort_work_and_reject_late_results() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    for action in ["search", "clear", "close", "logout"] {
        let (mut worker, _, _) = connected_worker();
        let (updates, _) = mpsc::channel(1);
        worker.library_command(LibraryCommand::Search("old".into()), &updates);
        let old = worker.library.search_generation;
        let old_task = worker.library.search_task.as_ref().unwrap().id();
        worker.library_command(LibraryCommand::Search("old".into()), &updates);
        assert_eq!(worker.library.search_task.as_ref().unwrap().id(), old_task);
        match action {
            "search" => worker.library_command(LibraryCommand::Search("new".into()), &updates),
            "clear" => worker.library_command(LibraryCommand::Search(" \t ".into()), &updates),
            "close" => worker.library_command(LibraryCommand::Close, &updates),
            "logout" => worker.reset_library("Logged out"),
            _ => unreachable!(),
        }
        assert_ne!(worker.library.search_generation, old);
        worker.library_update(Update::Search {
            generation: old,
            update: SearchUpdate::Loaded(Ok((vec![row("Stale")], false))),
        });
        worker.library_update(Update::Search {
            generation: old,
            update: SearchUpdate::Finished { timed_out: false },
        });
        let search = worker.library.shared.lock().unwrap().search.clone();
        assert!(search.tracks.is_empty());
        if action == "search" {
            assert_eq!(search.query.as_ref(), "new");
            assert!(search.loading);
            assert!(worker.library.search_task.is_some());
        } else {
            assert_eq!(search.query.as_ref(), "");
            assert!(!search.loading);
            assert!(worker.library.search_task.is_none());
        }
    }
}

#[test]
fn failed_search_and_metadata_timeout_leave_no_infinite_loading_rows() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    let (mut worker, _, _) = connected_worker();
    let generation = search_loaded(
        &mut worker,
        vec![TrackRow {
            title_kind: TrackTitleKind::Loading,
            ..row("")
        }],
    );
    worker.queue_search_result(generation, 0, false);
    worker.library_update(Update::Search {
        generation,
        update: SearchUpdate::Finished { timed_out: true },
    });
    assert_eq!(
        worker.library.shared.lock().unwrap().search.tracks[0].title_kind,
        TrackTitleKind::Unavailable
    );
    assert_eq!(
        worker.snapshot().queue[1].title_kind,
        TrackTitleKind::Unavailable
    );
    let before = worker.snapshot();
    worker.library_update(Update::Search {
        generation,
        update: SearchUpdate::Loaded(Err("Unavailable".into())),
    });
    assert_eq!(worker.snapshot(), before);
    assert!(!worker.library.shared.lock().unwrap().search.loading);
}

#[test]
fn search_requests_without_connection_or_with_invalid_queries_never_touch_playback() {
    let mut worker = worker();
    let (updates, _) = mpsc::channel(1);
    let before = worker.snapshot();
    for query in ["A song".to_owned(), "x".repeat(257)] {
        worker.library_command(LibraryCommand::Search(query), &updates);
        assert!(worker.library.search_task.is_none());
        assert!(!worker.library.shared.lock().unwrap().search.loading);
        assert_eq!(worker.snapshot(), before);
    }
}

#[test]
fn repeated_query_after_results_arrive_starts_a_new_atomic_request() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    let (mut worker, _, _) = connected_worker();
    let (updates, _) = mpsc::channel(1);
    worker.library_command(LibraryCommand::Search("same query".into()), &updates);
    let previous = worker.library.search_generation;
    worker.library_update(Update::Search {
        generation: previous,
        update: SearchUpdate::Loaded(Ok((vec![row("First response")], false))),
    });
    // Metadata is still in flight, but another explicit submission must be
    // acknowledged by a fresh request revision rather than leave UI pending.
    assert!(worker.library.search_task.is_some());
    let library_revision = worker.library.shared.lock().unwrap().revision;
    worker.library_command(LibraryCommand::Search("same query".into()), &updates);
    let current = worker.library.shared.lock().unwrap().clone();
    assert_ne!(current.search.revision, previous);
    assert_eq!(current.search.query.as_ref(), "same query");
    assert!(current.search.loading);
    assert!(current.search.tracks.is_empty());
    assert_eq!(
        current.revision,
        library_revision + 1,
        "request published in one edit"
    );
    let cloned = current.clone();
    assert!(Arc::ptr_eq(&current.search.query, &cloned.search.query));
    assert!(Arc::ptr_eq(&current.search.tracks, &cloned.search.tracks));
}

#[test]
fn clearing_search_finishes_queued_placeholders_and_failed_enrichment_keeps_known_names() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    let (mut worker, _, _) = connected_worker();
    let generation = search_loaded(
        &mut worker,
        vec![TrackRow {
            title_kind: TrackTitleKind::Loading,
            ..row("")
        }],
    );
    worker.queue_search_result(generation, 0, false);
    let (updates, _) = mpsc::channel(1);
    worker.library_command(LibraryCommand::Search("".into()), &updates);
    assert_eq!(
        worker.snapshot().queue[1].title_kind,
        TrackTitleKind::Unavailable
    );
    let generation = search_loaded(&mut worker, vec![row("Known title")]);
    worker.library_update(Update::Search {
        generation,
        update: SearchUpdate::Metadata(vec![TrackRow {
            title_kind: TrackTitleKind::Unavailable,
            ..row("")
        }]),
    });
    assert_eq!(
        worker.library.shared.lock().unwrap().search.tracks[0].title,
        "Known title"
    );
    assert_eq!(worker.snapshot().queue[0].title, "Existing");
}

#[test]
fn canceled_search_releases_in_flight_task_data() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (mut worker, _, _) = connected_worker();
        let retained = Arc::new(vec![row("In-flight response")]);
        let weak = Arc::downgrade(&retained);
        worker.library.search_task = Some(tokio::spawn(async move {
            std::future::pending::<()>().await;
            drop(retained);
        }));
        tokio::task::yield_now().await;
        let (updates, _) = mpsc::channel(1);
        worker.library_command(LibraryCommand::Search("".into()), &updates);
        tokio::task::yield_now().await;
        assert!(weak.upgrade().is_none());
        assert!(worker.library.search_task.is_none());
    });
}

#[test]
fn late_failed_playlist_metadata_preserves_named_search_results_in_the_queue() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    for play in [false, true] {
        let (mut worker, player, _) = connected_worker();
        let chosen = TrackRow {
            artist: "Known artist".into(),
            ..row("Named search result")
        };
        let generation = search_loaded(&mut worker, vec![chosen.clone()]);
        worker.queue_search_result(generation, 0, play);
        let before = worker.snapshot();
        player.take();
        worker.library_update(Update::Metadata {
            generation: worker.library.generation,
            rows: vec![TrackRow {
                uri: TRACK.into(),
                title_kind: TrackTitleKind::Unavailable,
                ..TrackRow::default()
            }],
        });
        assert_eq!(worker.snapshot(), before);
        assert!(player.take().is_empty());
        assert_eq!(worker.snapshot().queue.last(), Some(&chosen));
    }
}

#[test]
fn selected_insert_preserves_order_duplicates_and_current_playback() {
    let mut worker = worker();
    edit(&worker.state, |s| {
        s.queue = vec![row("Before"), row("Playing"), row("After")];
        s.current = Some(1);
        s.shuffle = true;
        s.repeat = true;
    });
    worker.current_uri = Some(SpotifyUri::from_uri(TRACK).unwrap());
    worker.reset_order(Some(1));
    let before = worker.snapshot();
    let transport_uri = worker.current_uri.clone();
    worker.queue_selected_tracks(
        vec![row("First"), row("Second"), row("First")],
        false,
        false,
        Some(1),
        None,
        None,
    );
    let after = worker.snapshot();
    assert_eq!(
        after
            .queue
            .iter()
            .map(|row| row.title.as_str())
            .collect::<Vec<_>>(),
        ["Before", "First", "Second", "First", "Playing", "After"]
    );
    assert_eq!(after.current, Some(4));
    assert_eq!(
        after.queue[after.current.unwrap()],
        before.queue[before.current.unwrap()]
    );
    assert_eq!(after.position_ms, before.position_ms);
    assert_eq!(after.playing, before.playing);
    assert_eq!(after.playback_revision, before.playback_revision);
    assert_eq!(worker.current_uri, transport_uri);
    assert!(after.shuffle && after.repeat);
    assert_eq!(worker.library.shared.lock().unwrap().queue_len, 6);
}

#[test]
fn selected_queue_rejects_whole_batch_before_touching_audio() {
    let mut worker = worker();
    let before = worker.snapshot();
    let invalid = TrackRow {
        uri: "spotify:episode:4uLU6hMCjMI75M1A2tKUQC".into(),
        ..row("Podcast")
    };
    for (rows, replace, at) in [
        (vec![], true, None),
        (vec![row("Good"), invalid], true, None),
        (vec![row("Large"); MAX_QUEUE], false, None),
        (vec![row("Large"); MAX_QUEUE + 1], true, None),
        (vec![row("Invalid position")], false, Some(2)),
        (vec![row("Invalid replace")], true, Some(0)),
    ] {
        worker.queue_selected_tracks(rows, replace, false, at, None, None);
        assert_eq!(worker.snapshot(), before);
    }
}

#[test]
fn selected_queue_snapshot_is_independent_of_later_browsing_and_pending_load() {
    let mut worker = worker();
    worker.library.pending_queue = Some(request(&worker, true));
    let captured = vec![row("Selected 2"), row("Selected 4")];
    worker.library.edit(|s| {
        s.tracks = Arc::new(vec![row("Different playlist")]);
        s.search.tracks = Arc::new(vec![row("Different search")]);
    });
    worker.queue_selected_tracks(captured.clone(), false, false, None, None, None);
    assert_eq!(&worker.snapshot().queue[1..], captured.as_slice());
    assert!(worker.library.pending_queue.is_none());
    worker.library_update(Update::Loaded {
        generation: 0,
        result: Ok((vec![row("Late playlist")], false)),
    });
    assert_eq!(&worker.snapshot().queue[1..], captured.as_slice());
}

#[test]
fn playlist_write_result_does_not_change_audio_or_browsing_and_rejects_old_accounts() {
    let mut worker = worker();
    let before = worker.snapshot();
    let destination = PlaylistId::Spotify(PLAYLIST.into());
    worker.library.edit(|s| {
        s.playlist_write_busy = true;
        s.entries = Arc::new(vec![PlaylistEntry {
            id: destination.clone(),
            name: "Destination".into(),
            track_count: Some(1),
            duration_ms: Some(1),
            folder: String::new(),
            available: true,
        }]);
    });
    worker.library_update(Update::Written {
        generation: 0,
        auth_generation: 1,
        id: destination.clone(),
        count: 3,
        result: Ok(crate::spotify_playlist_write::Receipt { total: Some(4) }),
    });
    assert!(worker.library.shared.lock().unwrap().playlist_write_busy);
    worker.library_update(Update::Written {
        generation: 0,
        auth_generation: 0,
        id: destination.clone(),
        count: 3,
        result: Ok(crate::spotify_playlist_write::Receipt { total: Some(4) }),
    });
    assert_eq!(worker.snapshot(), before);
    let library = worker.library.shared.lock().unwrap();
    assert!(!library.playlist_write_busy);
    assert_eq!(library.entries[0].track_count, Some(4));
    assert_eq!(library.entries[0].duration_ms, None);
    assert_eq!(library.selected_id, Some(destination.clone()));
    assert_eq!(worker.library.write_refresh, Some(destination));
}

#[tokio::test]
async fn closing_library_keeps_submitted_write_but_logout_cancels_with_uncertain_notice() {
    let mut worker = worker();
    worker.library.write_task = Some(tokio::spawn(std::future::pending()));
    worker.library.edit(|s| s.playlist_write_busy = true);
    let (updates, _) = mpsc::channel(1);
    worker.library_command(LibraryCommand::Close, &updates);
    assert!(worker.library.write_task.is_some());
    assert!(worker.library.shared.lock().unwrap().playlist_write_busy);
    worker.reset_library("Connectez Spotify pour afficher vos playlists");
    assert!(worker.library.write_task.is_none());
    assert!(!worker.library.shared.lock().unwrap().playlist_write_busy);
    assert_eq!(
        worker.library.shared.lock().unwrap().playlist_write_status,
        "Ajout non confirme; verifiez la playlist avant de reessayer"
    );
}

#[test]
fn completed_playlist_write_does_not_hide_next_search_or_full_queue_error() {
    let mut worker = worker();
    let (updates, _) = mpsc::channel(1);
    worker
        .library
        .edit(|s| s.playlist_write_status = playlist_write_success(3));
    worker.library_command(LibraryCommand::Search("new search".into()), &updates);
    {
        let library = worker.library.shared.lock().unwrap();
        assert!(
            library
                .playlist_write_status
                .text(Language::French)
                .is_empty()
        );
        assert_eq!(
            library.search.status,
            "Connectez Spotify pour rechercher des titres"
        );
    }
    worker
        .library
        .edit(|s| s.playlist_write_status = playlist_write_success(3));
    edit(&worker.state, |s| {
        s.queue = vec![row("Existing"); MAX_QUEUE]
    });
    worker.library_command(
        LibraryCommand::QueueTracks {
            tracks: vec![row("Does not fit")],
            origin: None,
            expected_queue: None,
            replace: false,
            play: false,
            insert_at: None,
        },
        &updates,
    );
    let library = worker.library.shared.lock().unwrap();
    assert!(
        library
            .playlist_write_status
            .text(Language::French)
            .is_empty()
    );
    assert_eq!(
        library.status,
        "File pleine : 500 titres maximum; la file a ete conservee"
    );
    assert_eq!(library.search.status, library.status);
}

#[test]
fn browsing_clears_finished_write_result_but_keeps_in_flight_progress() {
    let mut worker = worker();
    let (updates, _) = mpsc::channel(1);
    for command in [
        LibraryCommand::Select(PlaylistId::Spotify(PLAYLIST.into())),
        LibraryCommand::Refresh,
        LibraryCommand::QueueSearch {
            revision: 0,
            index: 0,
            play: false,
        },
        LibraryCommand::Queue {
            id: PlaylistId::Spotify(PLAYLIST.into()),
            replace: false,
            play: false,
            start: 0,
        },
    ] {
        worker
            .library
            .edit(|s| s.playlist_write_status = playlist_write_success(3));
        worker.library_command(command, &updates);
        assert!(
            worker
                .library
                .shared
                .lock()
                .unwrap()
                .playlist_write_status
                .text(Language::French)
                .is_empty()
        );
    }
    worker.library.edit(|s| {
        s.playlist_write_busy = true;
        s.playlist_write_status = "Ajout a la playlist Spotify en cours...".into();
    });
    worker.library_command(LibraryCommand::Search("new search".into()), &updates);
    let library = worker.library.shared.lock().unwrap();
    assert!(library.playlist_write_busy);
    assert_eq!(
        library.playlist_write_status,
        "Ajout a la playlist Spotify en cours..."
    );
}

#[test]
fn logout_clears_completed_write_result_so_connection_notice_is_visible() {
    let mut worker = worker();
    worker
        .library
        .edit(|s| s.playlist_write_status = playlist_write_success(3));
    worker.reset_library("Connectez Spotify pour afficher vos playlists");
    let library = worker.library.shared.lock().unwrap();
    assert!(
        library
            .playlist_write_status
            .text(Language::French)
            .is_empty()
    );
    assert!(!library.playlist_write_busy);
    assert_eq!(
        library.status,
        "Connectez Spotify pour afficher vos playlists"
    );
}

#[test]
fn captured_playlist_origin_survives_browsing_persistence_and_restart() {
    let mut worker = worker();
    let source = PlaylistId::Spotify(PLAYLIST.into());
    let other = PlaylistId::Spotify("spotify:playlist:0000000000000000000001".into());
    worker.library.edit(|s| s.selected_id = Some(other));
    worker.queue_selected_tracks(
        vec![row("Chosen first"), row("Chosen second")],
        true,
        false,
        None,
        Some(source.clone()),
        None,
    );
    assert_eq!(worker.queue_playlist, Some(source.clone()));
    edit(&worker.state, |s| {
        s.current = Some(1);
        s.position_ms = 42_000;
    });
    let saved = playback_settings::from_snapshot(&worker.snapshot(), worker.queue_playlist.clone());
    assert_eq!(saved.playlist, Some(source.clone()));
    let mut restarted = super::super::Worker::new(Arc::new(Mutex::new(Snapshot::default())));
    restarted.restore_session(saved);
    assert_eq!(restarted.snapshot().queue, worker.snapshot().queue);
    assert_eq!(restarted.snapshot().current, Some(1));
    assert_eq!(restarted.snapshot().position_ms, 42_000);
    assert_eq!(restarted.restore_library_selection, Some(source.clone()));
    assert_eq!(restarted.queue_playlist, Some(source));
    assert!(!restarted.snapshot().playing);
}

#[test]
fn selection_origin_remains_for_same_playlist_but_clears_for_search_or_mixed_sources() {
    let mut worker = worker();
    let source = PlaylistId::Spotify(PLAYLIST.into());
    worker.queue_selected_tracks(
        vec![row("First")],
        true,
        false,
        None,
        Some(source.clone()),
        None,
    );
    worker.queue_selected_tracks(
        vec![row("More from same")],
        false,
        false,
        None,
        Some(source.clone()),
        None,
    );
    assert_eq!(worker.queue_playlist, Some(source.clone()));
    worker.queue_selected_tracks(vec![row("Search result")], false, false, None, None, None);
    assert_eq!(worker.queue_playlist, None);
    worker.queue_selected_tracks(
        vec![row("Source again")],
        true,
        false,
        None,
        Some(source),
        None,
    );
    worker.queue_selected_tracks(
        vec![row("Different playlist")],
        false,
        false,
        Some(0),
        Some(PlaylistId::Spotify(
            "spotify:playlist:0000000000000000000001".into(),
        )),
        None,
    );
    assert_eq!(worker.queue_playlist, None);
}

#[test]
fn drop_on_replaced_same_length_queue_is_rejected_without_retargeting_insertion() {
    let mut worker = worker();
    let expected = worker
        .snapshot()
        .queue
        .iter()
        .map(|row| row.uri.clone())
        .collect();
    edit(&worker.state, |s| {
        s.queue[0].uri = "spotify:track:0000000000000000000001".into()
    });
    let before = worker.snapshot();
    worker.queue_selected_tracks(
        vec![row("Captured selection")],
        false,
        false,
        Some(0),
        None,
        Some(expected),
    );
    assert_eq!(worker.snapshot(), before);
    assert_eq!(
        worker.library.shared.lock().unwrap().status,
        "Position d'insertion invalide; la file a ete conservee"
    );
    let matching = before.queue.iter().map(|row| row.uri.clone()).collect();
    worker.queue_selected_tracks(
        vec![row("Captured selection")],
        false,
        false,
        Some(0),
        None,
        Some(matching),
    );
    assert_eq!(worker.snapshot().queue.len(), 2);
    assert_eq!(worker.snapshot().queue[1], before.queue[0]);
    assert_eq!(worker.snapshot().current, Some(1));
}
