use super::*;

fn restored() -> (Worker, Arc<RecordingPlayer>) {
    let saved = PlaybackSession {
        queue: vec![
            row("4uLU6hMCjMI75M1A2tKUQC", "Duplicate first"),
            row("4uLU6hMCjMI75M1A2tKUQC", "Duplicate second"),
            row("0000000000000000000001", "Last"),
        ],
        current: Some(1),
        position_ms: 42_500,
        playlist: Some(PlaylistId::Spotify(
            "spotify:playlist:4uLU6hMCjMI75M1A2tKUQC".into(),
        )),
        volume: 12_345,
        shuffle: false,
        repeat: true,
    };
    let mut worker = Worker::new(Arc::new(Mutex::new(Snapshot::default())));
    let player = Arc::new(RecordingPlayer::default());
    worker.player = Some(player.clone());
    worker.restore_session(saved);
    (worker, player)
}

#[test]
fn restored_queue_is_paused_and_first_play_resumes_exact_duplicate_and_position() {
    let (mut worker, player) = restored();
    let before = worker.snapshot();
    assert!(!before.playing);
    assert_eq!(before.current, Some(1));
    assert_eq!(before.position_ms, 42_500);
    assert_eq!(before.duration_ms, 120_000);
    assert_eq!(before.volume, 12_345);
    assert!(before.repeat);
    assert_eq!(before.playback_revision, 1);
    assert!(worker.resume_pending);
    assert!(worker.current_uri.is_none());
    assert!(player.take().is_empty());
    // A separately selected row must not change Play's resume semantics.
    send(&mut worker, Command::Play(2));
    assert_eq!(worker.snapshot().current, Some(1));
    assert_eq!(worker.snapshot().position_ms, 42_500);
    assert_eq!(
        player.take(),
        vec![Call::Load(
            parse_input("spotify:track:4uLU6hMCjMI75M1A2tKUQC").unwrap(),
            true,
            42_500,
            Bitrate::Bitrate320,
        )]
    );
    assert!(!worker.resume_pending);
    assert_eq!(worker.pending_loads, 1);
    worker.player_event(PlayerEvent::PositionChanged {
        track_id: worker.current_uri.clone().unwrap(),
        play_request_id: 71,
        position_ms: 0,
    });
    assert_eq!(worker.snapshot().position_ms, 42_500);
}

#[test]
fn restored_toggle_and_seek_use_saved_position_without_preloading() {
    let (mut worker, player) = restored();
    send(&mut worker, Command::Seek(50_000));
    assert!(player.take().is_empty());
    assert_eq!(worker.snapshot().position_ms, 50_000);
    send(&mut worker, Command::Toggle);
    assert!(matches!(
        player.take().as_slice(),
        [Call::Load(_, true, 50_000, _)]
    ));
}

#[test]
fn explicit_selection_next_stop_and_clear_supersede_restored_resume() {
    for command in [
        Command::PlayIndex(1),
        Command::Next,
        Command::Stop,
        Command::Clear,
    ] {
        let (mut worker, player) = restored();
        let clears = matches!(command, Command::Clear);
        let stops = matches!(command, Command::Stop);
        send(&mut worker, command);
        assert!(!worker.resume_pending);
        assert_eq!(worker.snapshot().position_ms, 0);
        if clears {
            assert!(worker.snapshot().queue.is_empty());
            assert!(worker.queue_playlist.is_none());
            assert!(worker.restore_library_selection.is_none());
        } else if !stops {
            assert!(matches!(
                player.take().as_slice(),
                [Call::Load(_, true, 0, _)]
            ));
        }
    }
}

#[test]
fn failed_play_before_connection_retains_resume_and_metadata_markers_are_truthful() {
    let (mut worker, player) = restored();
    worker.player = None;
    send(&mut worker, Command::Play(2));
    assert!(worker.resume_pending);
    assert_eq!(worker.snapshot().position_ms, 42_500);
    assert!(player.take().is_empty());
    let mut saved =
        playback_settings::from_snapshot(&worker.snapshot(), worker.queue_playlist.clone());
    saved.queue[0].title_kind = TrackTitleKind::Loading;
    worker.restore_session(saved);
    assert_eq!(
        worker.snapshot().queue[0].title_kind,
        TrackTitleKind::Unavailable
    );
    assert_eq!(worker.snapshot().queue[1].title, "Duplicate second");
}

#[tokio::test(flavor = "current_thread")]
async fn disconnect_releases_audio_but_retains_exact_resume_point() {
    let (mut worker, player) = restored();
    worker.disconnect();
    assert_eq!(player.take(), vec![Call::Stop]);
    assert!(worker.resume_pending);
    assert_eq!(worker.snapshot().position_ms, 42_500);
    let replacement = Arc::new(RecordingPlayer::default());
    worker.player = Some(replacement.clone());
    send(&mut worker, Command::Play(0));
    assert!(matches!(
        replacement.take().as_slice(),
        [Call::Load(_, true, 42_500, _)]
    ));
}

#[test]
fn queue_origin_is_independent_of_browsing_and_clears_on_mixed_imports() {
    let (mut worker, _) = restored();
    let origin = worker.queue_playlist.clone();
    worker.library.shared.lock().unwrap().selected_id =
        Some(PlaylistId::Spotify("another-browsed-playlist".into()));
    assert_eq!(
        playback_settings::from_snapshot(&worker.snapshot(), worker.queue_playlist.clone())
            .playlist,
        origin
    );
    worker.import_playlist = None;
    worker.update(
        Update::Rows {
            generation: worker.generation,
            rows: vec![row("0000000000000000000002", "Added link")],
            notice: LocalizedText::default(),
        },
        &mpsc::channel(1).0,
    );
    assert!(worker.queue_playlist.is_none());
    let (mut worker, _) = restored();
    worker.state.lock().unwrap().queue.clear();
    worker.import_playlist = origin.clone();
    worker.update(
        Update::Rows {
            generation: worker.generation,
            rows: vec![row("0000000000000000000002", "Imported playlist")],
            notice: LocalizedText::default(),
        },
        &mpsc::channel(1).0,
    );
    assert_eq!(worker.queue_playlist, origin);
}

#[test]
fn persisted_fingerprint_ignores_status_but_tracks_progress_and_order() {
    let (worker, _) = restored();
    let original = worker.playback_fingerprint();
    edit(&worker.state, |s| {
        s.status = "A transient notice".into();
        s.busy = true;
        s.connected = true;
    });
    assert_eq!(worker.playback_fingerprint(), original);
    edit(&worker.state, |s| s.position_ms += 500);
    assert_ne!(worker.playback_fingerprint(), original);
    edit(&worker.state, |s| {
        s.position_ms -= 500;
        s.queue.swap(0, 1);
    });
    assert_ne!(worker.playback_fingerprint(), original);
}

#[test]
fn accepted_seek_and_restart_are_persistable_before_async_player_ack() {
    for command in [Command::Seek(50_000), Command::Play(1), Command::Previous] {
        let (mut worker, player) = worker(true);
        edit(&worker.state, |s| s.duration_ms = 120_000);
        let expected = if matches!(command, Command::Seek(_)) {
            50_000
        } else {
            0
        };
        send(&mut worker, command);
        assert_eq!(player.take(), vec![Call::Seek(expected)]);
        assert_eq!(
            playback_settings::from_snapshot(&worker.snapshot(), None).position_ms,
            expected
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn disconnect_after_explicit_stop_does_not_recreate_resume_intent() {
    let (mut worker, player) = restored();
    send(&mut worker, Command::Stop);
    assert!(!worker.resume_pending);
    worker.disconnect();
    assert!(!worker.resume_pending);
    assert_eq!(worker.snapshot().position_ms, 0);
    assert_eq!(player.take(), vec![Call::Stop, Call::Stop]);
    let replacement = Arc::new(RecordingPlayer::default());
    worker.player = Some(replacement.clone());
    send(&mut worker, Command::Play(2));
    assert_eq!(worker.snapshot().current, Some(2));
    assert!(matches!(
        replacement.take().as_slice(),
        [Call::Load(_, true, 0, _)]
    ));
}
