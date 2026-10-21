use super::*;
use crate::model::{LibraryCommand, PlaylistEntry, PlaylistId};

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
    worker.set_quality(AudioQuality::Kbps96);
    send(&mut worker, Command::Play(1));
    assert_eq!(player.take(), vec![Call::Seek(0)]);
    assert_eq!(worker.snapshot().current, Some(0));
    assert_eq!(worker.load_quality, AudioQuality::Kbps160);
    assert_eq!(worker.pending_loads, 0);
    worker.player_event(PlayerEvent::Seeked {
        play_request_id: 7,
        track_id: worker.current_uri.clone().unwrap(),
        position_ms: 0,
    });
    assert_eq!(worker.snapshot().position_ms, 0);
    assert!(worker.snapshot().playing);
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
    send(&mut worker, Command::Play(0));
    worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 8 });
    worker.player_event(PlayerEvent::Loading {
        play_request_id: 8,
        track_id: selected.clone(),
        position_ms: 0,
    });
    send(&mut worker, Command::Play(0));
    assert!(player.take().is_empty());
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
