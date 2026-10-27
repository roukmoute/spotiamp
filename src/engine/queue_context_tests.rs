use super::*;

fn identity(worker: &Worker) -> Vec<String> {
    worker
        .snapshot()
        .queue
        .iter()
        .map(|row| row.uri.clone())
        .collect()
}

fn edit_command(worker: &Worker, indices: Vec<usize>, keep_selected: bool) -> Command {
    Command::EditQueue {
        expected_queue: identity(worker),
        expected_reorder_request: worker.snapshot().queue_reorder_request,
        indices,
        keep_selected,
    }
}

#[test]
fn queue_context_edits_keep_exact_duplicate_decoder_progress_and_saved_resume() {
    for playing in [false, true] {
        for keep_selected in [false, true] {
            let (mut worker, player) = queue(6);
            edit(&worker.state, |s| {
                s.current = Some(3);
                s.position_ms = 42_500;
                s.duration_ms = 120_000;
                s.playing = playing;
            });
            worker.wants_playback = playing;
            let playlist = Some(PlaylistId::Spotify("spotify:playlist:origin".into()));
            worker.queue_playlist = playlist.clone();
            worker.restore_library_selection = playlist.clone();
            let before = worker.snapshot();
            let fingerprint = worker.playback_fingerprint();
            let command = edit_command(
                &worker,
                if keep_selected {
                    vec![1, 3, 5]
                } else {
                    vec![0, 2, 4]
                },
                keep_selected,
            );
            send(&mut worker, command);
            let after = worker.snapshot();
            assert_eq!(
                after
                    .queue
                    .iter()
                    .map(|r| r.title.as_str())
                    .collect::<Vec<_>>(),
                ["Track 1", "Track 3", "Track 5"]
            );
            assert_eq!(after.current, Some(1));
            assert_eq!(after.position_ms, before.position_ms);
            assert_eq!(after.duration_ms, before.duration_ms);
            assert_eq!(after.playing, before.playing);
            assert_eq!(after.playback_revision, before.playback_revision);
            assert_eq!(worker.request_id, Some(7));
            assert_eq!(worker.pending_loads, 0);
            assert_eq!(worker.wants_playback, playing);
            assert_eq!(worker.queue_playlist, playlist);
            assert_eq!(worker.restore_library_selection, playlist);
            assert_ne!(worker.playback_fingerprint(), fingerprint);
            assert!(player.take().is_empty());

            let saved = playback_settings::from_snapshot(&after, worker.queue_playlist.clone());
            assert_eq!(saved.current, Some(1));
            assert_eq!(saved.position_ms, 42_500);
            assert_eq!(saved.queue, after.queue);
            worker.restore_session(saved);
            send(&mut worker, Command::Play(0));
            assert_eq!(worker.snapshot().current, Some(1));
            assert!(matches!(
                player.take().as_slice(),
                [Call::Load(_, true, 42_500, _)]
            ));
        }
    }
}

#[test]
fn queue_context_removing_current_stops_once_and_empty_clears_origin() {
    for keep_selected in [false, true] {
        let (mut worker, player) = queue(6);
        edit(&worker.state, |s| {
            s.current = Some(3);
            s.duration_ms = 120_000;
        });
        worker.queue_playlist = Some(PlaylistId::Spotify("origin".into()));
        worker.restore_library_selection = worker.queue_playlist.clone();
        let command = edit_command(
            &worker,
            if keep_selected {
                vec![1, 5]
            } else {
                vec![0, 2, 3, 4]
            },
            keep_selected,
        );
        send(&mut worker, command);
        let after = worker.snapshot();
        assert_eq!(
            after
                .queue
                .iter()
                .map(|r| r.title.as_str())
                .collect::<Vec<_>>(),
            ["Track 1", "Track 5"]
        );
        assert_eq!(after.current, None);
        assert_eq!(after.position_ms, 0);
        assert_eq!(after.duration_ms, 0);
        assert!(!after.playing);
        assert!(worker.stopped);
        assert!(worker.current_uri.is_none());
        assert_eq!(player.take(), vec![Call::Stop]);
        assert!(worker.queue_playlist.is_some());

        let command = edit_command(&worker, vec![0, 1], false);
        send(&mut worker, command);
        assert!(worker.snapshot().queue.is_empty());
        assert!(worker.queue_playlist.is_none());
        assert!(worker.restore_library_selection.is_none());
        assert!(player.take().is_empty(), "no decoder remains to stop");
    }
}

#[test]
fn queue_context_retains_or_cancels_the_exact_deferred_load() {
    for keep_current in [true, false] {
        let (mut worker, player) = queue(6);
        let (updates, _) = mpsc::channel(1);
        worker.command(Command::PlayIndex(3), &updates);
        let load = worker.deferred_load.as_ref().unwrap();
        let (uri, deadline, position, quality) = (
            load.uri.clone(),
            load.deadline,
            load.position_ms,
            load.quality,
        );
        let command = edit_command(
            &worker,
            if keep_current {
                vec![1, 3, 5]
            } else {
                vec![1, 5]
            },
            true,
        );
        worker.command(command, &updates);
        if keep_current {
            let load = worker.deferred_load.as_ref().unwrap();
            assert_eq!(
                (&load.uri, load.deadline, load.position_ms, load.quality),
                (&uri, deadline, position, quality)
            );
            assert_eq!(worker.snapshot().current, Some(1));
            assert!(player.take().is_empty());
            worker.dispatch_due_load(deadline);
            assert!(
                matches!(player.take().as_slice(), [Call::Load(track, true, 0, _)] if track == &uri)
            );
        } else {
            assert!(worker.deferred_load.is_none());
            assert!(worker.snapshot().current.is_none());
            assert_eq!(player.take(), vec![Call::Stop]);
            worker.dispatch_due_load(deadline);
            assert!(player.take().is_empty());
        }
    }
}

#[test]
fn queue_context_preserves_unstarted_saved_resume_when_its_occurrence_is_kept() {
    let (mut worker, player) = queue(5);
    let mut saved = playback_settings::from_snapshot(&worker.snapshot(), None);
    saved.current = Some(3);
    saved.position_ms = 42_500;
    worker.current_uri = None;
    worker.restore_session(saved);
    let command = edit_command(&worker, vec![1, 3], true);
    send(&mut worker, command);
    assert!(worker.resume_pending);
    assert!(worker.current_uri.is_none());
    assert!(player.take().is_empty());
    send(&mut worker, Command::Play(0));
    assert_eq!(worker.snapshot().current, Some(1));
    assert!(matches!(
        player.take().as_slice(),
        [Call::Load(_, true, 42_500, _)]
    ));
}

#[test]
fn queue_context_invalid_stale_and_noop_edits_leave_pending_transport_intact() {
    let (mut worker, player) = worker(true);
    let (updates, _) = mpsc::channel(1);
    worker.command(Command::PlayIndex(1), &updates);
    let deadline = worker.deferred_load.as_ref().unwrap().deadline;
    let before = worker.snapshot();
    let fingerprint = worker.playback_fingerprint();
    for command in [
        edit_command(&worker, vec![], false),
        edit_command(&worker, vec![1, 0], false),
        edit_command(&worker, vec![0, 0], true),
        edit_command(&worker, vec![2], false),
        edit_command(&worker, vec![0, 1], true),
        Command::EditQueue {
            expected_queue: vec!["stale".into()],
            expected_reorder_request: 0,
            indices: vec![0],
            keep_selected: false,
        },
        Command::EditQueue {
            expected_queue: identity(&worker).into_iter().rev().collect(),
            expected_reorder_request: 0,
            indices: vec![0],
            keep_selected: true,
        },
        Command::PlayQueueIndex {
            expected_queue: identity(&worker),
            expected_reorder_request: 0,
            index: usize::MAX,
        },
        Command::PlayQueueIndex {
            expected_queue: vec!["stale".into()],
            expected_reorder_request: 0,
            index: 0,
        },
    ] {
        worker.command(command, &updates);
        let after = worker.snapshot();
        assert_eq!(after.queue, before.queue);
        assert_eq!(after.current, before.current);
        assert_eq!(after.position_ms, before.position_ms);
        assert_eq!(after.playback_revision, before.playback_revision);
        assert_eq!(worker.playback_fingerprint(), fingerprint);
        assert_eq!(worker.deferred_load.as_ref().unwrap().deadline, deadline);
        assert!(player.take().is_empty());
    }
    worker.dispatch_due_load(deadline);
    assert!(matches!(
        player.take().as_slice(),
        [Call::Load(_, true, 0, _)]
    ));
}

#[test]
fn queue_context_play_selects_exact_existing_occurrence_without_replacing_queue() {
    for playing in [false, true] {
        let (mut worker, player) = queue(5);
        edit(&worker.state, |s| s.playing = playing);
        let before = worker.snapshot();
        let command = Command::PlayQueueIndex {
            expected_queue: identity(&worker),
            expected_reorder_request: 0,
            index: 3,
        };
        send(&mut worker, command);
        let after = worker.snapshot();
        assert_eq!(after.queue, before.queue);
        assert_eq!(after.current, Some(3));
        assert_eq!(after.position_ms, 0);
        assert_eq!(after.playback_revision, before.playback_revision + 1);
        assert!(matches!(
            player.take().as_slice(),
            [Call::Load(_, true, 0, _)]
        ));
    }
}

#[test]
fn queue_context_rejects_reordered_identical_uris_from_an_older_menu() {
    let (mut worker, player) = queue(4);
    let expected_queue = identity(&worker);
    let command = Command::ReorderQueue {
        request_id: 91,
        expected_queue: expected_queue.clone(),
        indices: vec![0, 2],
        insert_before: 4,
    };
    send(&mut worker, command);
    let before = worker.snapshot();
    for command in [
        Command::EditQueue {
            expected_queue: expected_queue.clone(),
            expected_reorder_request: 0,
            indices: vec![0],
            keep_selected: false,
        },
        Command::PlayQueueIndex {
            expected_queue: expected_queue.clone(),
            expected_reorder_request: 0,
            index: 0,
        },
    ] {
        send(&mut worker, command);
        let after = worker.snapshot();
        assert_eq!(after.queue, before.queue);
        assert_eq!(after.current, before.current);
        assert_eq!(after.playback_revision, before.playback_revision);
        assert!(player.take().is_empty());
    }
}

#[test]
fn queue_context_edit_keeps_shuffle_previous_and_next_on_the_same_occurrences() {
    for _ in 0..10 {
        let (mut worker, player) = queue(6);
        send(&mut worker, Command::ToggleShuffle);
        send(&mut worker, Command::Next);
        let second = worker.snapshot().current.unwrap();
        send(&mut worker, Command::Next);
        let third = worker.snapshot().current.unwrap();
        send(&mut worker, Command::Previous);
        player.take();
        let mut kept = vec![0, second, third];
        kept.sort_unstable();
        let command = edit_command(&worker, kept.clone(), true);
        send(&mut worker, command);
        assert!(player.take().is_empty());
        assert_eq!(
            worker.snapshot().current,
            kept.iter().position(|&old| old == second)
        );
        for (command, old) in [
            (Command::Next, third),
            (Command::Previous, second),
            (Command::Previous, 0),
        ] {
            send(&mut worker, command);
            let after = worker.snapshot();
            assert_eq!(
                after.queue[after.current.unwrap()].title,
                format!("Track {old}")
            );
        }
    }
}
