//! Debounce navigation at the real Worker/Player boundary, without Spotify I/O.
use super::*;

fn command(worker: &mut Worker, command: Command) {
    let (updates, _) = mpsc::channel(1);
    worker.command(command, &updates);
}

fn populated(len: usize) -> (Worker, Arc<RecordingPlayer>) {
    let (mut worker, player) = queue(len);
    edit(&worker.state, |s| {
        s.queue = (0..len)
            .map(|index| row(&format!("{index:022}"), &format!("Track {index}")))
            .collect();
    });
    worker.current_uri = Some(parse_input(&worker.snapshot().queue[0].uri).unwrap());
    (worker, player)
}

fn acknowledge(worker: &mut Worker, id: u64) {
    worker.player_event(PlayerEvent::PlayRequestIdChanged {
        play_request_id: id,
    });
}

fn unavailable(worker: &mut Worker, id: u64) {
    worker.player_event(PlayerEvent::Unavailable {
        play_request_id: id,
        track_id: worker.current_uri.clone().unwrap(),
    });
}

#[test]
fn first_start_is_immediate_and_a_navigation_burst_sends_only_its_last_selection() {
    let (mut worker, player) = populated(20);
    worker.stopped = true;
    worker.current_uri = None;
    command(&mut worker, Command::PlayIndex(3));
    assert!(worker.deferred_load.is_none());
    let first_uri = parse_input(&worker.snapshot().queue[3].uri).unwrap();
    assert_eq!(
        player.take(),
        vec![Call::Load(first_uri, true, 0, Bitrate::Bitrate160)]
    );
    for command_value in [
        Command::Next,
        Command::Next,
        Command::Previous,
        Command::PlayIndex(12),
        Command::Previous,
    ] {
        command(&mut worker, command_value);
    }
    assert_eq!(worker.snapshot().current, Some(11));
    assert_eq!(worker.snapshot().playback_revision, 6);
    let deadline = worker.deferred_load.as_ref().unwrap().deadline;
    command(&mut worker, Command::Play(0));
    assert_eq!(
        worker.deferred_load.as_ref().unwrap().deadline,
        deadline,
        "Play while loading must not restart or postpone the selected request"
    );
    worker.dispatch_due_load(deadline - Duration::from_nanos(1));
    assert!(player.take().is_empty());
    assert_eq!(
        worker.pending_loads, 1,
        "only the first actual load awaits an ACK"
    );
    worker.dispatch_due_load(deadline);
    assert_eq!(
        player.take(),
        vec![Call::Load(
            parse_input(&worker.snapshot().queue[11].uri).unwrap(),
            true,
            0,
            Bitrate::Bitrate160,
        )]
    );
    assert_eq!(worker.pending_loads, 2);
    acknowledge(&mut worker, 8);
    assert_eq!(worker.pending_loads, 1);
    acknowledge(&mut worker, 9);
    assert_eq!(
        worker.pending_loads, 0,
        "coalesced requests must not leave phantom ACKs"
    );
}

#[test]
fn pending_pause_resume_seek_quality_and_row_removal_target_the_new_track_only() {
    for resume in [false, true] {
        let (mut worker, player) = populated(4);
        command(&mut worker, Command::Next);
        let selected = parse_input(&worker.snapshot().queue[1].uri).unwrap();
        let deadline = worker.deferred_load.as_ref().unwrap().deadline;
        command(&mut worker, Command::Toggle);
        command(&mut worker, Command::Seek(42_000));
        worker.set_quality(AudioQuality::Kbps320);
        command(&mut worker, Command::Remove(0));
        assert_eq!(worker.snapshot().current, Some(0));
        if resume {
            command(&mut worker, Command::Play(2));
        }
        assert_eq!(
            player.take(),
            vec![Call::Pause],
            "never seek or resume the old decoder"
        );
        worker.dispatch_due_load(deadline);
        assert_eq!(
            player.take(),
            vec![Call::Load(selected, resume, 42_000, Bitrate::Bitrate160)]
        );
        assert_eq!(worker.load_quality, AudioQuality::Kbps160);
        assert_eq!(worker.snapshot().quality, AudioQuality::Kbps320);
    }
}

#[test]
fn previous_after_pending_seek_restarts_the_selected_request_not_the_old_decoder() {
    let (mut worker, player) = populated(4);
    command(&mut worker, Command::Next);
    let deadline = worker.deferred_load.as_ref().unwrap().deadline;
    let selected = parse_input(&worker.snapshot().queue[1].uri).unwrap();
    command(&mut worker, Command::Seek(42_000));
    command(&mut worker, Command::Previous);
    assert_eq!(worker.snapshot().current, Some(1));
    assert_eq!(worker.snapshot().position_ms, 0);
    assert_eq!(worker.snapshot().playback_revision, 2);
    assert!(player.take().is_empty());
    worker.dispatch_due_load(deadline);
    assert_eq!(
        player.take(),
        vec![Call::Load(selected, true, 0, Bitrate::Bitrate160)]
    );
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn stop_clear_current_removal_and_disconnect_cancel_a_pending_load() {
    for action in ["stop", "clear", "remove", "disconnect"] {
        let (mut worker, player) = populated(4);
        command(&mut worker, Command::Next);
        let deadline = worker.deferred_load.as_ref().unwrap().deadline;
        match action {
            "stop" => command(&mut worker, Command::Stop),
            "clear" => command(&mut worker, Command::Clear),
            "remove" => command(&mut worker, Command::Remove(1)),
            _ => worker.disconnect(),
        }
        assert!(worker.deferred_load.is_none(), "{action}");
        worker.dispatch_due_load(deadline + Duration::from_secs(10));
        assert_eq!(player.take(), vec![Call::Stop], "{action}");
        assert!(worker.stopped);
        assert_eq!(worker.pending_loads, 0);
    }
}

#[test]
fn stale_same_uri_errors_are_ignored_but_current_failure_is_visible_and_retryable() {
    // Duplicate queue rows intentionally share the exact URI.
    let (mut worker, player) = queue(3);
    command(&mut worker, Command::Next);
    let deadline = worker.deferred_load.as_ref().unwrap().deadline;
    unavailable(&mut worker, 7);
    assert!(
        !worker.stopped,
        "old failure must not cancel the pending selection"
    );
    worker.dispatch_due_load(deadline);
    unavailable(&mut worker, 7);
    assert!(!worker.stopped, "no failure is current before the load ACK");
    acknowledge(&mut worker, 8);
    unavailable(&mut worker, 7);
    assert!(
        !worker.stopped,
        "old same-URI request is still stale after the new ACK"
    );
    unavailable(&mut worker, 8);
    assert!(worker.stopped);
    assert!(worker.snapshot().status.contains("indisponible"));
    player.take();
    command(&mut worker, Command::Play(1));
    assert!(
        worker.deferred_load.is_none(),
        "retry after failure starts immediately"
    );
    assert!(matches!(
        player.take().as_slice(),
        [Call::Load(_, true, 0, _)]
    ));
    acknowledge(&mut worker, 9);
    unavailable(&mut worker, 8);
    assert!(!worker.stopped);
    worker.player_event(PlayerEvent::Playing {
        play_request_id: 9,
        track_id: worker.current_uri.clone().unwrap(),
        position_ms: 1_000,
    });
    assert!(worker.snapshot().playing);
    assert!(!worker.snapshot().status.contains("indisponible"));
}

async fn settled() {
    // Keep this task runnable: Tokio must not auto-advance to the next timer
    // while we wait for the worker to consume its bounded command queue.
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
}

fn run_worker() -> (
    mpsc::Sender<Command>,
    Arc<Mutex<Snapshot>>,
    Arc<RecordingPlayer>,
    JoinHandle<Result<(), LocalizedText>>,
) {
    let (mut worker, player) = populated(20);
    // Suppress startup reconnect using a disconnected synthetic Session. Never
    // call Session::connect, Engine::spawn, keyring or OAuth in this fixture.
    worker.session = Some(Session::new(SessionConfig::default(), None));
    let state = worker.state.clone();
    let (tx, rx) = mpsc::channel(16);
    (tx, state, player, tokio::spawn(worker.run(rx)))
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn real_worker_timer_waits_for_quiet_period_and_latest_selection_wins() {
    let (tx, state, player, task) = run_worker();
    for command in [
        Command::Next,
        Command::Next,
        Command::Previous,
        Command::PlayIndex(5),
        Command::Play(0),
    ] {
        tx.send(command).await.unwrap();
    }
    settled().await;
    assert_eq!(state.lock().unwrap().current, Some(5));
    assert!(player.take().is_empty());
    tokio::time::advance(LOAD_SETTLE_TIME - Duration::from_millis(1)).await;
    settled().await;
    assert!(player.take().is_empty());
    tx.send(Command::Next).await.unwrap();
    settled().await;
    assert_eq!(state.lock().unwrap().current, Some(6));
    tokio::time::advance(LOAD_SETTLE_TIME - Duration::from_millis(1)).await;
    settled().await;
    assert!(
        player.take().is_empty(),
        "later input resets the quiet window"
    );
    tokio::time::advance(Duration::from_millis(1)).await;
    settled().await;
    assert_eq!(
        player.take(),
        vec![Call::Load(
            parse_input(&state.lock().unwrap().queue[6].uri).unwrap(),
            true,
            0,
            Bitrate::Bitrate160,
        )]
    );
    tx.send(Command::Shutdown).await.unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(player.take(), vec![Call::Stop]);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn real_worker_stop_and_pause_remain_immediate_during_the_timer() {
    for stop in [false, true] {
        let (tx, _, player, task) = run_worker();
        tx.send(Command::Next).await.unwrap();
        tx.send(if stop { Command::Stop } else { Command::Toggle })
            .await
            .unwrap();
        settled().await;
        assert_eq!(
            player.take(),
            vec![if stop { Call::Stop } else { Call::Pause }]
        );
        tokio::time::advance(LOAD_SETTLE_TIME).await;
        settled().await;
        if stop {
            assert!(player.take().is_empty());
        } else {
            assert!(matches!(
                player.take().as_slice(),
                [Call::Load(_, false, 0, _)]
            ));
        }
        tx.send(Command::Shutdown).await.unwrap();
        task.await.unwrap().unwrap();
    }
}
