use super::*;
use crate::model::{LibraryCommand, PlaylistEntry, PlaylistId};

#[path = "load_tests.rs"]
mod load_tests;
#[path = "restore_tests.rs"]
mod restore_tests;

#[derive(Debug, PartialEq)]
enum Call {
    Load(SpotifyUri, bool, u32, Bitrate),
    Play,
    Pause,
    Stop,
    Seek(u32),
}

#[derive(Default)]
struct RecordingPlayer(Mutex<Vec<Call>>);
impl RecordingPlayer {
    fn take(&self) -> Vec<Call> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
    fn push(&self, call: Call) {
        self.0.lock().unwrap().push(call);
    }
}
impl PlayerTransport for RecordingPlayer {
    fn load_with_bitrate(&self, uri: SpotifyUri, play: bool, position: u32, bitrate: Bitrate) {
        self.push(Call::Load(uri, play, position, bitrate));
    }
    fn play(&self) {
        self.push(Call::Play);
    }
    fn pause(&self) {
        self.push(Call::Pause);
    }
    fn stop(&self) {
        self.push(Call::Stop);
    }
    fn seek(&self, position: u32) {
        self.push(Call::Seek(position));
    }
    fn is_invalid(&self) -> bool {
        false
    }
}

fn row(id: &str, name: &str) -> TrackRow {
    TrackRow {
        uri: format!("spotify:track:{id}"),
        title: name.into(),
        duration_ms: 120_000,
        ..TrackRow::default()
    }
}

fn worker(playing: bool) -> (Worker, Arc<RecordingPlayer>) {
    let mut worker = Worker::new(Arc::new(Mutex::new(Snapshot {
        connected: true,
        queue: vec![
            row("4uLU6hMCjMI75M1A2tKUQC", "Current"),
            row("0000000000000000000001", "Selected"),
        ],
        current: Some(0),
        playing,
        position_ms: 42_500,
        quality: AudioQuality::Kbps160,
        ..Snapshot::default()
    })));
    worker.stopped = false;
    worker.current_uri = Some(parse_input(&worker.snapshot().queue[0].uri).unwrap());
    worker.request_id = Some(7);
    let player = Arc::new(RecordingPlayer::default());
    worker.player = Some(player.clone());
    (worker, player)
}

fn send(worker: &mut Worker, command: Command) {
    let (updates, _) = mpsc::channel(1);
    worker.command(command, &updates);
    // These existing transport/order tests model separated user actions. The
    // burst/timer tests below leave the real debounce window unflushed.
    if let Some(load) = &worker.deferred_load {
        worker.dispatch_due_load(load.deadline);
    }
}

fn event(worker: &mut Worker, playing: bool) {
    let track_id = worker.current_uri.clone().unwrap();
    worker.player_event(if playing {
        PlayerEvent::Playing {
            track_id,
            play_request_id: 7,
            position_ms: 42_500,
        }
    } else {
        PlayerEvent::Paused {
            track_id,
            play_request_id: 7,
            position_ms: 42_500,
        }
    });
}

#[test]
fn play_resumes_current_paused_decoder_without_reloading_or_seeking() {
    let (mut worker, player) = worker(false);
    worker.set_quality(AudioQuality::Kbps320);
    send(&mut worker, Command::Play(1));
    assert_eq!(player.take(), vec![Call::Play]);
    assert_eq!(worker.snapshot().current, Some(0));
    assert_eq!(worker.snapshot().position_ms, 42_500);
    assert_eq!(worker.snapshot().playback_revision, 0);
    assert_eq!(worker.pending_loads, 0);
    assert_eq!(worker.request_id, Some(7));
    assert_eq!(worker.load_quality, AudioQuality::Kbps160);
    event(&mut worker, true);
    assert_eq!(worker.snapshot().position_ms, 42_500);
    assert!(worker.snapshot().status.contains("160 kb/s"));
}

#[test]
fn play_restarts_current_playing_decoder_and_keeps_its_quality() {
    let (mut worker, player) = worker(true);
    edit(&worker.state, |s| {
        s.playback_origin = PlaybackOrigin::Automatic
    });
    worker.set_quality(AudioQuality::Kbps96);
    send(&mut worker, Command::Play(1));
    assert_eq!(player.take(), vec![Call::Seek(0)]);
    assert_eq!(worker.snapshot().current, Some(0));
    assert_eq!(worker.load_quality, AudioQuality::Kbps160);
    assert_eq!(worker.snapshot().playback_revision, 1);
    assert_eq!(worker.snapshot().playback_origin, PlaybackOrigin::Explicit);
    assert_eq!(worker.pending_loads, 0);
    worker.player_event(PlayerEvent::Seeked {
        play_request_id: 7,
        track_id: worker.current_uri.clone().unwrap(),
        position_ms: 0,
    });
    assert_eq!(worker.snapshot().position_ms, 0);
    assert!(worker.snapshot().playing);
    assert_eq!(worker.snapshot().playback_revision, 1);
}

#[test]
fn rapid_pause_play_and_late_events_preserve_latest_transport_intent() {
    let (mut worker, player) = worker(true);
    send(&mut worker, Command::Toggle);
    // No Paused event has reached the worker yet.
    assert!(worker.snapshot().playing);
    send(&mut worker, Command::Play(1));
    assert_eq!(player.take(), vec![Call::Pause, Call::Play]);
    event(&mut worker, false);
    assert!(worker.wants_playback);
    send(&mut worker, Command::Toggle);
    // A delayed Playing event must not reverse that new pause request either.
    event(&mut worker, true);
    assert!(!worker.wants_playback);
    send(&mut worker, Command::Play(1));
    send(&mut worker, Command::Play(1));
    assert_eq!(player.take(), vec![Call::Pause, Call::Play, Call::Seek(0)]);
    assert_eq!(worker.snapshot().current, Some(0));
    assert_eq!(worker.pending_loads, 0);
}

#[test]
fn stopped_play_uses_selection_once_and_repeated_loading_play_does_not_reload() {
    let (mut worker, player) = worker(true);
    send(&mut worker, Command::Stop);
    worker.set_quality(AudioQuality::Kbps320);
    send(&mut worker, Command::Play(1));
    let selected = parse_input(&worker.snapshot().queue[1].uri).unwrap();
    assert_eq!(
        player.take(),
        vec![
            Call::Stop,
            Call::Load(selected.clone(), true, 0, Bitrate::Bitrate320)
        ]
    );
    assert_eq!(worker.snapshot().current, Some(1));
    assert_eq!(worker.snapshot().position_ms, 0);
    assert_eq!(worker.snapshot().playback_revision, 1);
    send(&mut worker, Command::Play(0));
    worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 8 });
    worker.player_event(PlayerEvent::Loading {
        play_request_id: 8,
        track_id: selected.clone(),
        position_ms: 0,
    });
    send(&mut worker, Command::Play(0));
    assert!(player.take().is_empty());
    assert_eq!(worker.snapshot().playback_revision, 1);
    send(&mut worker, Command::Toggle);
    send(&mut worker, Command::Play(0));
    assert_eq!(player.take(), vec![Call::Pause, Call::Play]);
    worker.player_event(PlayerEvent::Playing {
        play_request_id: 8,
        track_id: selected,
        position_ms: 250,
    });
    send(&mut worker, Command::Play(0));
    assert_eq!(player.take(), vec![Call::Seek(0)]);
    assert_eq!(worker.snapshot().current, Some(1));
    assert_eq!(worker.load_quality, AudioQuality::Kbps320);
    assert_eq!(worker.snapshot().playback_revision, 2);
}

#[test]
fn explicit_play_index_still_reloads_and_stopped_invalid_selection_is_safe() {
    let (mut worker, player) = worker(false);
    worker.set_quality(AudioQuality::Kbps96);
    send(&mut worker, Command::PlayIndex(1));
    assert_eq!(
        player.take(),
        vec![Call::Load(
            parse_input(&worker.snapshot().queue[1].uri).unwrap(),
            true,
            0,
            Bitrate::Bitrate96
        )]
    );
    send(&mut worker, Command::Stop);
    player.take();
    send(&mut worker, Command::Play(99));
    assert!(player.take().is_empty());
    assert!(worker.stopped);
}

#[test]
fn play_cancels_deferred_library_play_without_discarding_its_preview() {
    // The current-thread runtime is entered but never polled: the real library
    // command queues its request, and no Spotify network task can run.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    for cancel_with_play in [false, true] {
        let (mut worker, player) = worker(false);
        worker.session = Some(Session::new(SessionConfig::default(), None));
        worker.library.open = true;
        let id = PlaylistId::Spotify("spotify:playlist:4uLU6hMCjMI75M1A2tKUQC".into());
        worker.library.shared.lock().unwrap().entries = Arc::new(vec![PlaylistEntry {
            id: id.clone(),
            name: "Library".into(),
            track_count: Some(1),
            duration_ms: None,
            folder: String::new(),
            available: true,
        }]);
        send(
            &mut worker,
            Command::Library(LibraryCommand::Queue {
                id,
                replace: true,
                play: true,
                start: 0,
            }),
        );
        assert!(worker.library.shared.lock().unwrap().busy);
        if cancel_with_play {
            send(&mut worker, Command::Play(1));
        }
        worker.library_update(library::Update::Loaded {
            generation: 1,
            result: Ok((vec![row("0000000000000000000002", "Library track")], false)),
        });
        assert_eq!(
            worker.library.shared.lock().unwrap().tracks[0].title,
            "Library track"
        );
        if cancel_with_play {
            assert_eq!(worker.snapshot().queue[0].title, "Current");
            assert_eq!(player.take(), vec![Call::Play]);
        } else {
            assert_eq!(worker.snapshot().queue[0].title, "Library track");
            assert!(matches!(
                player.take().as_slice(),
                [Call::Stop, Call::Load(_, true, 0, _)]
            ));
        }
    }
}

fn queue(len: usize) -> (Worker, Arc<RecordingPlayer>) {
    let (worker, player) = worker(true);
    edit(&worker.state, |s| {
        // Duplicate URIs deliberately remain independent queue entries.
        s.queue = (0..len)
            .map(|index| row("4uLU6hMCjMI75M1A2tKUQC", &format!("Track {index}")))
            .collect();
        s.current = (len > 0).then_some(0);
        s.position_ms = 0;
    });
    (worker, player)
}

fn finish_current(worker: &mut Worker) {
    let id = worker.request_id.unwrap_or(0);
    let uri = worker.current_uri.clone().unwrap();
    worker.player_event(PlayerEvent::EndOfTrack {
        play_request_id: id,
        track_id: uri,
    });
    if worker.pending_loads > 0 {
        worker.player_event(PlayerEvent::PlayRequestIdChanged {
            play_request_id: id + 1,
        });
    }
}

fn prepare_library(worker: &mut Worker) -> Vec<PlaylistId> {
    worker.session = Some(Session::new(SessionConfig::default(), None));
    worker.library.open = true;
    let ids = ["0000000000000000000001", "0000000000000000000002"]
        .map(|id| PlaylistId::Spotify(format!("spotify:playlist:{id}")));
    worker.library.shared.lock().unwrap().entries = Arc::new(
        ids.iter()
            .map(|id| PlaylistEntry {
                id: id.clone(),
                name: "Library".into(),
                track_count: Some(20),
                duration_ms: None,
                folder: String::new(),
                available: true,
            })
            .collect(),
    );
    ids.into()
}

#[test]
fn library_ninth_track_replacement_and_same_track_replay_each_signal_once() {
    // Entering, without polling, allows the real asynchronous command flow to
    // be exercised with injected results and no Spotify network requests.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    let (mut worker, player) = worker(false);
    let ids = prepare_library(&mut worker);
    for (attempt, id) in [&ids[0], &ids[1], &ids[1]].into_iter().enumerate() {
        let rows: Vec<_> = (0..20)
            .map(|index| {
                row(
                    "4uLU6hMCjMI75M1A2tKUQC",
                    &format!("List {attempt}: {index}"),
                )
            })
            .collect();
        send(
            &mut worker,
            Command::Library(LibraryCommand::Queue {
                id: id.clone(),
                replace: true,
                play: true,
                start: 8,
            }),
        );
        if attempt < 2 {
            worker.library_update(library::Update::Loaded {
                generation: attempt as u64 + 1,
                result: Ok((rows.clone(), false)),
            });
            assert_eq!(worker.snapshot().queue, rows);
        }
        // The third command reuses the ready playlist, replaying the same row.
        let snapshot = worker.snapshot();
        assert_eq!(snapshot.current, Some(8));
        assert_eq!(snapshot.queue.len(), 20);
        assert_eq!(snapshot.playback_revision, attempt as u64 + 1);
        assert_eq!(snapshot.playback_origin, PlaybackOrigin::Explicit);
        assert_eq!(
            player.take(),
            vec![
                Call::Stop,
                Call::Load(
                    parse_input(&snapshot.queue[8].uri).unwrap(),
                    true,
                    0,
                    Bitrate::Bitrate160,
                ),
            ]
        );
    }
}

#[test]
fn failed_empty_stale_or_invalid_library_play_does_not_signal_or_replace() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    for failure in ["fetch", "empty", "stale", "index"] {
        let (mut worker, player) = worker(false);
        let id = prepare_library(&mut worker).remove(0);
        edit(&worker.state, |s| s.playback_revision = 41);
        let before = worker.snapshot();
        send(
            &mut worker,
            Command::Library(LibraryCommand::Queue {
                id,
                replace: true,
                play: true,
                start: 8,
            }),
        );
        worker.library_update(library::Update::Loaded {
            generation: 1 + u64::from(failure == "stale"),
            result: match failure {
                "fetch" => Err("Simulated network error".into()),
                "empty" => Ok((Vec::new(), false)),
                _ => Ok((vec![row("0000000000000000000001", "Only track")], false)),
            },
        });
        assert!(player.take().is_empty(), "{failure}");
        assert_eq!(worker.snapshot(), before, "{failure}");
    }
}

#[test]
fn progress_metadata_resume_and_generic_seek_do_not_request_reveal() {
    let (mut worker, player) = worker(false);
    edit(&worker.state, |s| s.playback_revision = 17);
    send(&mut worker, Command::Play(1));
    event(&mut worker, true);
    send(&mut worker, Command::Seek(0));
    worker.player_event(PlayerEvent::PositionChanged {
        play_request_id: 7,
        track_id: worker.current_uri.clone().unwrap(),
        position_ms: 500,
    });
    send(&mut worker, Command::Toggle);
    event(&mut worker, false);
    send(&mut worker, Command::Volume(20_000));
    worker.set_quality(AudioQuality::Kbps320);
    let (updates, _) = mpsc::channel(1);
    worker.update(
        Update::Metadata {
            generation: worker.generation,
            row: row("4uLU6hMCjMI75M1A2tKUQC", "Refreshed metadata"),
        },
        &updates,
    );
    assert_eq!(player.take(), vec![Call::Play, Call::Seek(0), Call::Pause]);
    assert_eq!(worker.snapshot().playback_revision, 17);
    send(&mut worker, Command::PlayIndex(99));
    edit(&worker.state, |s| s.queue[1].uri = "invalid".into());
    send(&mut worker, Command::PlayIndex(1));
    worker.player = None;
    send(&mut worker, Command::PlayIndex(0));
    assert!(player.take().is_empty());
    assert_eq!(worker.snapshot().playback_revision, 17);
    send(&mut worker, Command::Clear);
    assert_eq!(worker.snapshot().current, None);
    assert_eq!(worker.snapshot().playback_revision, 17);
}

#[test]
fn modes_default_off_and_toggles_never_change_audio_or_requested_quality() {
    let (mut worker, player) = worker(false);
    let before = worker.snapshot();
    assert!(!before.shuffle && !before.repeat);
    for command in [
        Command::ToggleShuffle,
        Command::ToggleRepeat,
        Command::ToggleShuffle,
        Command::ToggleRepeat,
    ] {
        send(&mut worker, command);
    }
    assert!(player.take().is_empty());
    assert_eq!(worker.snapshot(), before);
    assert_eq!(worker.load_quality, AudioQuality::Kbps160);
}

#[test]
fn accepted_end_of_track_marks_ninth_track_automatic_and_manual_navigation_resets_origin() {
    let (mut worker, player) = queue(20);
    send(&mut worker, Command::PlayIndex(7));
    assert_eq!(worker.snapshot().playback_origin, PlaybackOrigin::Explicit);
    worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 8 });
    player.take();
    // Real PlayerEvent dispatch, including the URI/request guards, takes the
    // eighth track to the ninth without a command from either window.
    finish_current(&mut worker);
    let snapshot = worker.snapshot();
    assert_eq!(snapshot.current, Some(8));
    assert_eq!(snapshot.playback_revision, 2);
    assert_eq!(snapshot.playback_origin, PlaybackOrigin::Automatic);
    assert!(matches!(
        player.take().as_slice(),
        [Call::Load(_, true, 0, _)]
    ));
    worker.player_event(PlayerEvent::EndOfTrack {
        play_request_id: 8,
        track_id: worker.current_uri.clone().unwrap(),
    });
    assert_eq!(
        worker.snapshot(),
        snapshot,
        "old end must not advance again"
    );
    assert!(player.take().is_empty());
    send(&mut worker, Command::Next);
    assert_eq!(worker.snapshot().current, Some(9));
    assert_eq!(worker.snapshot().playback_origin, PlaybackOrigin::Explicit);
    send(&mut worker, Command::Previous);
    assert_eq!(worker.snapshot().current, Some(8));
    assert_eq!(worker.snapshot().playback_origin, PlaybackOrigin::Explicit);
}

#[test]
fn natural_end_visits_each_row_then_stops_or_repeats_the_whole_queue() {
    for shuffle in [false, true] {
        for repeat in [false, true] {
            let (mut worker, player) = queue(8);
            if shuffle {
                send(&mut worker, Command::ToggleShuffle);
            }
            if repeat {
                send(&mut worker, Command::ToggleRepeat);
            }
            let mut visited = vec![0];
            for _ in 1..8 {
                finish_current(&mut worker);
                visited.push(worker.snapshot().current.unwrap());
                assert_eq!(
                    worker.snapshot().playback_revision,
                    visited.len() as u64 - 1
                );
                assert_eq!(worker.snapshot().playback_origin, PlaybackOrigin::Automatic);
                assert!(matches!(
                    player.take().as_slice(),
                    [Call::Load(_, true, 0, _)]
                ));
            }
            let last = *visited.last().unwrap();
            visited.sort_unstable();
            assert_eq!(visited, (0..8).collect::<Vec<_>>());
            finish_current(&mut worker);
            if repeat {
                let first = worker.snapshot().current.unwrap();
                assert_ne!(first, last);
                assert_eq!(worker.snapshot().playback_revision, 8);
                assert_eq!(worker.snapshot().playback_origin, PlaybackOrigin::Automatic);
                assert!(matches!(
                    player.take().as_slice(),
                    [Call::Load(_, true, 0, _)]
                ));
                let mut second_cycle = vec![first];
                for _ in 1..8 {
                    finish_current(&mut worker);
                    second_cycle.push(worker.snapshot().current.unwrap());
                }
                second_cycle.sort_unstable();
                assert_eq!(second_cycle, (0..8).collect::<Vec<_>>());
            } else {
                assert!(worker.stopped);
                assert_eq!(player.take(), vec![Call::Stop]);
                assert_eq!(worker.snapshot().status, "Fin de la file");
                assert_eq!(worker.snapshot().playback_revision, 7);
            }
        }
    }
}

#[test]
fn shuffled_previous_retraces_actual_order_including_repeat_boundary() {
    let (mut worker, player) = queue(6);
    send(&mut worker, Command::ToggleShuffle);
    send(&mut worker, Command::ToggleRepeat);
    let mut visited = vec![0];
    for _ in 0..7 {
        send(&mut worker, Command::Next);
        visited.push(worker.snapshot().current.unwrap());
    }
    player.take();
    for expected in visited.iter().rev().skip(1) {
        send(&mut worker, Command::Previous);
        assert_eq!(worker.snapshot().current, Some(*expected));
        assert!(matches!(
            player.take().as_slice(),
            [Call::Load(_, true, 0, _)]
        ));
    }
    send(&mut worker, Command::Next);
    assert_eq!(worker.snapshot().current, Some(visited[1]));
    edit(&worker.state, |s| s.position_ms = 3_001);
    edit(&worker.state, |s| {
        s.playback_origin = PlaybackOrigin::Automatic
    });
    player.take();
    let revision = worker.snapshot().playback_revision;
    send(&mut worker, Command::Previous);
    assert_eq!(player.take(), vec![Call::Seek(0)]);
    assert_eq!(worker.snapshot().current, Some(visited[1]));
    assert_eq!(worker.snapshot().playback_revision, revision + 1);
    assert_eq!(worker.snapshot().playback_origin, PlaybackOrigin::Explicit);
}

#[test]
fn single_empty_and_rapid_repeated_end_events_are_safe_in_all_modes() {
    for shuffle in [false, true] {
        for repeat in [false, true] {
            let (mut worker, player) = queue(1);
            if shuffle {
                send(&mut worker, Command::ToggleShuffle);
            }
            if repeat {
                send(&mut worker, Command::ToggleRepeat);
            }
            let old_id = worker.request_id.unwrap();
            let ended = || PlayerEvent::EndOfTrack {
                play_request_id: old_id,
                track_id: parse_input("spotify:track:4uLU6hMCjMI75M1A2tKUQC").unwrap(),
            };
            worker.player_event(ended());
            worker.player_event(ended());
            assert_eq!(worker.snapshot().playback_revision, u64::from(repeat));
            assert_eq!(
                worker.snapshot().playback_origin,
                if repeat {
                    PlaybackOrigin::Automatic
                } else {
                    PlaybackOrigin::Explicit
                }
            );
            let calls = player.take();
            if repeat {
                assert!(matches!(calls.as_slice(), [Call::Load(_, true, 0, _)]));
                worker.player_event(PlayerEvent::PlayRequestIdChanged {
                    play_request_id: old_id + 1,
                });
                worker.player_event(ended());
                assert!(player.take().is_empty());
                assert_eq!(worker.snapshot().playback_revision, 1);
            } else {
                assert_eq!(calls, vec![Call::Stop]);
            }
            send(&mut worker, Command::Clear);
            assert_eq!(worker.snapshot().shuffle, shuffle);
            assert_eq!(worker.snapshot().repeat, repeat);
            player.take();
            send(&mut worker, Command::Next);
            send(&mut worker, Command::Previous);
            send(&mut worker, Command::Play(0));
            assert!(
                !player
                    .take()
                    .iter()
                    .any(|call| matches!(call, Call::Load(..)))
            );
        }
    }
}

#[test]
fn explicit_selection_starts_a_fresh_shuffle_cycle_and_play_preserves_it() {
    let (mut worker, player) = queue(7);
    send(&mut worker, Command::ToggleShuffle);
    send(&mut worker, Command::Next);
    send(&mut worker, Command::PlayIndex(4));
    let mut visited = vec![4];
    // Loading Play leaves the selected stream and permutation intact.
    player.take();
    send(&mut worker, Command::Play(1));
    assert!(player.take().is_empty());
    for _ in 1..7 {
        send(&mut worker, Command::Next);
        visited.push(worker.snapshot().current.unwrap());
    }
    visited.sort_unstable();
    assert_eq!(visited, (0..7).collect::<Vec<_>>());
}

#[test]
fn import_append_and_removal_preserve_shuffle_history_and_remaining_entries() {
    let (mut worker, player) = queue(6);
    send(&mut worker, Command::ToggleShuffle);
    send(&mut worker, Command::Next);
    let second = worker.snapshot().current.unwrap();
    let (updates, _) = mpsc::channel(1);
    worker.update(
        Update::Rows {
            generation: worker.generation,
            rows: vec![row("0000000000000000000001", "Added")],
            notice: LocalizedText::default(),
        },
        &updates,
    );
    send(&mut worker, Command::Previous);
    assert_eq!(worker.snapshot().current, Some(0));
    send(&mut worker, Command::Next);
    assert_eq!(worker.snapshot().current, Some(second));
    // Removing an earlier index adjusts the current identity and history.
    send(&mut worker, Command::Remove(0));
    assert_eq!(worker.snapshot().current, Some(second - 1));
    let mut visited = vec![second - 1];
    for _ in 1..6 {
        send(&mut worker, Command::Next);
        visited.push(worker.snapshot().current.unwrap());
    }
    visited.sort_unstable();
    assert_eq!(visited, (0..6).collect::<Vec<_>>());
    send(&mut worker, Command::Clear);
    player.take();
    worker.update(
        Update::Rows {
            generation: worker.generation,
            rows: vec![row("0000000000000000000002", "After clear")],
            notice: LocalizedText::default(),
        },
        &updates,
    );
    send(&mut worker, Command::Next);
    assert_eq!(worker.snapshot().current, Some(0));
    assert!(matches!(
        player.take().as_slice(),
        [Call::Load(_, true, 0, _)]
    ));
}

#[test]
fn library_append_preserves_order_and_same_length_replacement_clears_old_history() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();
    for replace in [false, true] {
        let (mut worker, player) = queue(2);
        send(&mut worker, Command::ToggleShuffle);
        send(&mut worker, Command::ToggleRepeat);
        send(&mut worker, Command::Next);
        assert_eq!(worker.snapshot().current, Some(1));
        worker.session = Some(Session::new(SessionConfig::default(), None));
        worker.library.open = true;
        let id = PlaylistId::Spotify("spotify:playlist:4uLU6hMCjMI75M1A2tKUQC".into());
        worker.library.shared.lock().unwrap().entries = Arc::new(vec![PlaylistEntry {
            id: id.clone(),
            name: "Library".into(),
            track_count: Some(2),
            duration_ms: None,
            folder: String::new(),
            available: true,
        }]);
        send(
            &mut worker,
            Command::Library(LibraryCommand::Queue {
                id,
                replace,
                play: false,
                start: 0,
            }),
        );
        worker.library_update(library::Update::Loaded {
            generation: 1,
            result: Ok((
                vec![
                    row("0000000000000000000001", "New 0"),
                    row("0000000000000000000002", "New 1"),
                ],
                false,
            )),
        });
        assert!(worker.snapshot().shuffle && worker.snapshot().repeat);
        player.take();
        if replace {
            assert_eq!(worker.snapshot().current, None);
            send(&mut worker, Command::Next);
            let first = worker.snapshot().current;
            send(&mut worker, Command::Previous);
            assert_eq!(worker.snapshot().current, first); // no previous playlist history
            send(&mut worker, Command::Next);
            assert_ne!(worker.snapshot().current, first);
            assert_eq!(worker.snapshot().queue.len(), 2);
        } else {
            assert_eq!(worker.snapshot().current, Some(1));
            send(&mut worker, Command::Previous);
            assert_eq!(worker.snapshot().current, Some(0));
            send(&mut worker, Command::Next);
            assert_eq!(worker.snapshot().current, Some(1));
            let mut additions = Vec::new();
            for _ in 0..2 {
                send(&mut worker, Command::Next);
                additions.push(worker.snapshot().current.unwrap());
            }
            additions.sort_unstable();
            assert_eq!(additions, vec![2, 3]);
        }
        assert!(
            player
                .take()
                .iter()
                .all(|call| matches!(call, Call::Load(_, true, 0, _)))
        );
    }
}

#[test]
fn language_change_keeps_playback_and_pending_loads_untouched() {
    let (mut worker, player) = worker(true);
    event(&mut worker, true);
    let before = worker.snapshot();
    let (updates, _) = mpsc::channel(1);
    worker.command(Command::SetLanguage(Language::English), &updates);
    assert_eq!(worker.language, Language::English);
    assert_eq!(worker.snapshot(), before);
    assert!(player.take().is_empty());
    assert_eq!(
        before.status.text(Language::English),
        "Playing - requested quality 160 kb/s"
    );
    worker.command(Command::Next, &updates);
    let deadline = worker.deferred_load.as_ref().unwrap().deadline;
    let generation = worker.generation;
    worker.command(Command::SetLanguage(Language::French), &updates);
    assert_eq!(worker.deferred_load.as_ref().unwrap().deadline, deadline);
    assert_eq!(worker.generation, generation);
    assert!(player.take().is_empty());
}

#[test]
fn existing_playback_status_changes_language_without_reloading_or_reformatting() {
    let (mut worker, player) = worker(true);
    event(&mut worker, true);
    let before = worker.snapshot();
    let (updates, _) = mpsc::channel(1);
    for (language, expected) in Language::ALL.into_iter().zip([
        "Lecture - qualite demandee 160 kb/s",
        "Playing - requested quality 160 kb/s",
        "Wiedergabe - angeforderte Qualität 160 kb/s",
        "Reproduciendo - calidad solicitada 160 kb/s",
        "Riproduzione - qualità richiesta 160 kb/s",
        "A reproduzir - qualidade pedida 160 kb/s",
    ]) {
        worker.command(Command::SetLanguage(language), &updates);
        assert_eq!(before.status.text(language), expected);
        assert_eq!(worker.snapshot(), before);
        assert!(player.take().is_empty());
    }
    let named_like_ui = row("4uLU6hMCjMI75M1A2tKUQC", "Loading audio...");
    for language in Language::ALL {
        assert_eq!(named_like_ui.display_title(language), "Loading audio...");
    }
}
