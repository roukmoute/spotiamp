use super::*;

const TRACK: &str = "spotify:track:4uLU6hMCjMI75M1A2tKUQC";
const PLAYLIST: &str = "spotify:playlist:4uLU6hMCjMI75M1A2tKUQC";

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
