//! Exercise the production loader bridge without Spotify, a network or a sink.
use super::*;
use std::sync::{atomic::AtomicBool, mpsc as std_mpsc};

const LIMIT: Duration = Duration::from_secs(5);

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}

struct DropSignal(std_mpsc::Sender<()>);
impl Drop for DropSignal {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

struct PendingLoad {
    started: std_mpsc::Receiver<()>,
    dropped: std_mpsc::Receiver<()>,
    // Failure cleanup can release a broken cancellation implementation, so a
    // failing test reports its assertion instead of hanging the whole suite.
    release: Option<oneshot::Sender<()>>,
}
impl PendingLoad {
    fn started(&self) {
        self.started
            .recv_timeout(LIMIT)
            .expect("loader did not start");
    }
    fn cancelled(&mut self) {
        if let Err(error) = self.dropped.recv_timeout(LIMIT) {
            self.release.take();
            panic!("abandoned loader future was not dropped: {error}");
        }
    }
}

fn pending<T: Send + 'static>(handles: &LoadHandles) -> (oneshot::Receiver<T>, PendingLoad) {
    let (started_tx, started) = std_mpsc::channel();
    let (dropped_tx, dropped) = std_mpsc::channel();
    let (release, release_rx) = oneshot::channel();
    let receiver = spawn_load(handles, async move {
        let _drop = DropSignal(dropped_tx);
        started_tx.send(()).unwrap();
        let _ = release_rx.await;
        None
    });
    (
        receiver,
        PendingLoad {
            started,
            dropped,
            release: Some(release),
        },
    )
}

fn empty(handles: &LoadHandles) {
    let deadline = Instant::now() + LIMIT;
    while !handles.lock().unwrap().is_empty() {
        assert!(
            Instant::now() < deadline,
            "finished loader handle remained registered"
        );
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn receiver_closed_before_poll_never_starts_load_work() {
    let runtime = runtime();
    let (sender, receiver) = oneshot::channel::<u32>();
    let polled = AtomicBool::new(false);
    drop(receiver);
    runtime.block_on(complete_load(sender, async {
        polled.store(true, Ordering::SeqCst);
        Some(123)
    }));
    assert!(!polled.load(Ordering::SeqCst));
}

#[test]
fn dropping_receiver_wakes_and_cancels_a_running_loader_thread() {
    let runtime = runtime();
    let _entered = runtime.enter();
    let handles = LoadHandles::default();
    let (receiver, mut work) = pending::<u32>(&handles);
    work.started();
    drop(receiver);
    work.cancelled();
    empty(&handles);
}

#[test]
fn completed_and_failed_loads_deliver_their_real_result_and_release_handles() {
    let runtime = runtime();
    let _entered = runtime.enter();
    let handles = LoadHandles::default();
    // Repeat immediate completion to exercise the spawn/register/cleanup race.
    for value in 0..100 {
        let receiver = spawn_load(&handles, async move { Some(value) });
        assert_eq!(runtime.block_on(receiver).unwrap(), value);
        empty(&handles);
    }
    let receiver = spawn_load::<u32>(&handles, async { None });
    assert!(
        runtime.block_on(receiver).is_err(),
        "a failed load is not success"
    );
    empty(&handles);
}

fn uri() -> SpotifyUri {
    SpotifyUri::from_uri("spotify:track:4uLU6hMCjMI75M1A2tKUQC").unwrap()
}

fn loading(receiver: oneshot::Receiver<PlayerLoadedTrackData>, id: u64) -> PlayerState {
    PlayerState::Loading {
        track_id: uri(),
        play_request_id: id,
        start_playback: false,
        loader: Box::pin(receiver.map_err(|_| ())),
    }
}

struct NoAudio;
impl Sink for NoAudio {
    fn write(&mut self, _: AudioPacket, _: &mut Converter) -> crate::audio_backend::SinkResult<()> {
        panic!("loader lifecycle tests must never produce audio")
    }
}

fn player() -> (
    PlayerInternal,
    mpsc::UnboundedSender<PlayerCommand>,
    mpsc::UnboundedReceiver<PlayerEvent>,
) {
    let (command_tx, commands) = mpsc::unbounded_channel();
    let (events, event_rx) = mpsc::unbounded_channel();
    let player = PlayerInternal {
        session: Session::new(crate::core::config::SessionConfig::default(), None),
        config: PlayerConfig::default(),
        commands,
        load_handles: LoadHandles::default(),
        state: PlayerState::Stopped,
        preload: PlayerPreload::None,
        sink: Box::new(NoAudio),
        sink_status: SinkStatus::Closed,
        sink_event_callback: None,
        volume_getter: Box::new(crate::mixer::NoOpVolume),
        event_senders: vec![events],
        converter: Converter::new(None),
        normalisation_peaks: [0.0; 2],
        normalisation_integrators: [0.0; 2],
        normalisation_channel: 0,
        normalisation_knee_factor: 1.0,
        auto_normalise_as_album: false,
        player_id: 0,
        play_request_id_generator: SeqGenerator::new(0),
        last_progress_update: Instant::now(),
        local_file_lookup: Arc::new(LocalFileLookup::default()),
    };
    (player, command_tx, event_rx)
}

#[test]
fn replacement_and_stop_cancel_only_the_abandoned_loading_state() {
    let runtime = runtime();
    let _entered = runtime.enter();
    let (mut player, _commands, mut events) = player();
    let (old_receiver, mut old) = pending(&player.load_handles);
    player.state = loading(old_receiver, 7);
    old.started();
    let (new_receiver, mut new) = pending(&player.load_handles);
    new.started();
    player.state = loading(new_receiver, 8);
    old.cancelled();
    assert!(matches!(
        new.dropped.try_recv(),
        Err(std_mpsc::TryRecvError::Empty)
    ));
    assert!(
        events.try_recv().is_err(),
        "superseded work must not emit Unavailable"
    );

    player.handle_command(PlayerCommand::Stop).unwrap();
    new.cancelled();
    assert!(matches!(player.state, PlayerState::Stopped));
    assert!(matches!(
        events.try_recv(),
        Ok(PlayerEvent::Stopped {
            play_request_id: 8,
            ..
        })
    ));
    assert!(events.try_recv().is_err());
    empty(&player.load_handles);
}

#[test]
fn actual_failed_loader_emits_unavailable_once_and_can_be_replaced() {
    let runtime = runtime();
    let _entered = runtime.enter();
    let (mut player, _commands, mut events) = player();
    let receiver = spawn_load(&player.load_handles, async { None });
    player.state = loading(receiver, 9);
    empty(&player.load_handles);
    let mut cx = Context::from_waker(futures_util::task::noop_waker_ref());
    assert!(Pin::new(&mut player).poll(&mut cx).is_pending());
    assert!(matches!(
        events.try_recv(),
        Ok(PlayerEvent::Unavailable {
            play_request_id: 9,
            ..
        })
    ));
    assert!(Pin::new(&mut player).poll(&mut cx).is_pending());
    assert!(
        events.try_recv().is_err(),
        "terminated loader must not repeat its failure"
    );

    let (receiver, mut retry) = pending(&player.load_handles);
    player.state = loading(receiver, 10);
    retry.started();
    assert!(Pin::new(&mut player).poll(&mut cx).is_pending());
    assert!(
        events.try_recv().is_err(),
        "old failure must not poison a new loader"
    );
    player.handle_command(PlayerCommand::Stop).unwrap();
    retry.cancelled();
    empty(&player.load_handles);
}

#[test]
fn player_drop_closes_loading_and_preload_receivers_before_joining_threads() {
    let (work_tx, work_rx) = std_mpsc::channel();
    let (finished_tx, finished_rx) = std_mpsc::channel();
    let thread = thread::spawn(move || {
        let runtime = runtime();
        let _entered = runtime.enter();
        let (mut player, _commands, _events) = player();
        let handles = player.load_handles.clone();
        let (receiver, loading_work) = pending(&handles);
        player.state = loading(receiver, 11);
        loading_work.started();
        let (receiver, preload_work) = pending::<PlayerLoadedTrackData>(&handles);
        player.preload = PlayerPreload::Loading {
            track_id: uri(),
            loader: Box::pin(receiver.map_err(|_| ())),
        };
        preload_work.started();
        work_tx
            .send((loading_work, preload_work, handles.clone()))
            .unwrap();
        drop(player);
        finished_tx.send(()).unwrap();
    });
    let (mut loading, mut preload, handles) = work_rx.recv_timeout(LIMIT).unwrap();
    let result = finished_rx.recv_timeout(LIMIT);
    if result.is_err() {
        // Release test futures even with a broken Drop ordering, then join so
        // the regression fails explicitly without leaving blocked test threads.
        loading.release.take();
        preload.release.take();
    }
    thread.join().unwrap();
    result.expect("PlayerInternal::drop kept a result receiver alive during join");
    loading.cancelled();
    preload.cancelled();
    assert!(handles.lock().unwrap().is_empty());
}
