use super::*;

fn reorder_command(worker: &Worker, indices: Vec<usize>, insert_before: usize) -> Command {
    Command::ReorderQueue {
        request_id: 13,
        expected_queue: worker
            .snapshot()
            .queue
            .iter()
            .map(|row| row.uri.clone())
            .collect(),
        indices,
        insert_before,
    }
}

#[test]
fn queue_reorder_preserves_exact_duplicate_decoder_and_playback_state() {
    for playing in [true, false] {
        let (mut worker, player) = worker(playing);
        edit(&worker.state, |s| {
            s.queue = vec![
                row("4uLU6hMCjMI75M1A2tKUQC", "First duplicate"),
                row("0000000000000000000001", "Middle"),
                row("4uLU6hMCjMI75M1A2tKUQC", "Current duplicate"),
            ];
            s.current = Some(2);
        });
        let previous = worker.snapshot();
        let fingerprint = worker.playback_fingerprint();
        let (updates, _) = mpsc::channel(1);
        let command = reorder_command(&worker, vec![0, 2], 0);
        worker.command(command, &updates);
        let after = worker.snapshot();
        assert_eq!(
            after
                .queue
                .iter()
                .map(|r| r.title.as_str())
                .collect::<Vec<_>>(),
            ["First duplicate", "Current duplicate", "Middle"]
        );
        assert_eq!(after.current, Some(1));
        assert_eq!(after.position_ms, previous.position_ms);
        assert_eq!(after.playback_revision, previous.playback_revision);
        assert_eq!(after.playing, previous.playing);
        assert_eq!(worker.request_id, Some(7));
        assert_eq!(worker.pending_loads, 0);
        assert_eq!(
            worker.current_uri.as_ref().unwrap().to_uri().unwrap(),
            after.queue[1].uri
        );
        assert_ne!(
            worker.playback_fingerprint(),
            fingerprint,
            "new order must be saved"
        );
        assert!(player.take().is_empty(), "dragging never calls the decoder");
    }
}

#[test]
fn queue_reorder_keeps_deferred_load_and_rejects_stale_or_invalid_drops() {
    let (mut worker, player) = worker(true);
    let (updates, _) = mpsc::channel(1);
    worker.command(Command::PlayIndex(1), &updates);
    player.take();
    let pending = worker.deferred_load.as_ref().unwrap();
    let (uri, deadline, position) = (pending.uri.clone(), pending.deadline, pending.position_ms);
    let command = reorder_command(&worker, vec![1], 0);
    worker.command(command, &updates);
    assert_eq!(worker.snapshot().current, Some(0));
    let pending = worker.deferred_load.as_ref().unwrap();
    assert_eq!(
        (&pending.uri, pending.deadline, pending.position_ms),
        (&uri, deadline, position)
    );
    assert!(player.take().is_empty());
    let before = worker.snapshot();
    for command in [
        Command::ReorderQueue {
            request_id: 13,
            expected_queue: vec!["stale".into()],
            indices: vec![0],
            insert_before: 2,
        },
        reorder_command(&worker, vec![usize::MAX], 0),
        reorder_command(&worker, vec![0], 1),
    ] {
        worker.command(command, &updates);
        let after = worker.snapshot();
        assert_eq!(after.queue, before.queue);
        assert_eq!(after.current, before.current);
        assert_eq!(after.position_ms, before.position_ms);
        assert!(player.take().is_empty());
    }
    worker.dispatch_due_load(deadline);
    assert!(matches!(player.take().as_slice(), [Call::Load(track, true, _, _)] if track == &uri));
}
