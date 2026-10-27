//! Bounded native player. No web view, artwork, discovery, PCM preloading or
//! persistent audio cache. Spotify protocol work stays off the UI thread.
use std::{
    collections::VecDeque,
    hash::{Hash, Hasher},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use librespot_core::{Session, SpotifyUri, authentication::Credentials, config::SessionConfig};
use librespot_metadata::{Album, Metadata, Playlist, Track};
use librespot_playback::{
    audio_backend::{Sink, SinkError, SinkResult},
    config::{Bitrate, PlayerConfig, VolumeCtrl},
    convert::Converter,
    decoder::AudioPacket,
    mixer::{Mixer, MixerConfig, softmixer::SoftMixer},
    player::{Player, PlayerEvent, PlayerEventChannel},
};
use librespot_protocol::authentication::AuthenticationType;
use tokio::{
    sync::mpsc,
    task::{JoinHandle, JoinSet},
};

pub use crate::model::{Command, Snapshot, TrackRow};
use crate::{
    credentials,
    locale::Language,
    localized_text::LocalizedText,
    model::{AudioQuality, LibrarySnapshot, PlaybackOrigin, PlaylistId, TrackTitleKind},
    playback_settings::{self, PlaybackSession},
    settings,
};

#[path = "engine/auth.rs"]
mod auth;
#[path = "engine/library.rs"]
mod library;
#[path = "engine/order.rs"]
mod order;
#[cfg(test)]
#[path = "engine/transport_tests.rs"]
mod transport_tests;

pub const MAX_QUEUE: usize = 500;
const NETWORK_TIMEOUT: Duration = Duration::from_secs(20);
const LOAD_SETTLE_TIME: Duration = Duration::from_millis(200);

// Keep transport decisions testable at the actual player-call boundary without
// opening audio, authenticating, or adding another runtime to the application.
trait PlayerTransport: Send + Sync {
    fn load_with_bitrate(&self, uri: SpotifyUri, play: bool, position: u32, bitrate: Bitrate);
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
    fn seek(&self, position: u32);
    fn is_invalid(&self) -> bool;
}

impl PlayerTransport for Player {
    fn load_with_bitrate(&self, uri: SpotifyUri, play: bool, position: u32, bitrate: Bitrate) {
        Player::load_with_bitrate(self, uri, play, position, bitrate);
    }
    fn play(&self) {
        Player::play(self);
    }
    fn pause(&self) {
        Player::pause(self);
    }
    fn stop(&self) {
        Player::stop(self);
    }
    fn seek(&self, position: u32) {
        Player::seek(self, position);
    }
    fn is_invalid(&self) -> bool {
        Player::is_invalid(self)
    }
}

pub struct Engine {
    pub tx: mpsc::Sender<Command>,
    pub state: Arc<Mutex<Snapshot>>,
    pub library: Arc<Mutex<LibrarySnapshot>>,
    pub(crate) completion: Option<std::sync::mpsc::Receiver<Result<(), LocalizedText>>>,
}

/// Localized command-channel errors without formatting a command payload.
pub fn command_error(error: &mpsc::error::TrySendError<Command>) -> LocalizedText {
    match error {
        mpsc::error::TrySendError::Full(_) => LocalizedText::new(
            "Le lecteur est occupe; reessayez dans un instant.",
            "The player is busy; try again in a moment.",
        ),
        mpsc::error::TrySendError::Closed(_) => LocalizedText::new(
            "Le moteur audio est arrete; redemarrez Spotiamp.",
            "The audio engine has stopped; restart Spotiamp.",
        ),
    }
}

impl Engine {
    /// A full queue defers this cosmetic setting to the existing UI loop.
    /// A closed engine cannot consume it, so it must not cause endless retries.
    pub fn sync_language(&self, language: Language) -> bool {
        !matches!(
            self.tx.try_send(Command::SetLanguage(language)),
            Err(mpsc::error::TrySendError::Full(_))
        )
    }

    // Probes deliberately avoid reading or writing playback-session preferences.
    #[allow(dead_code)]
    pub fn spawn() -> Self {
        Self::spawn_config(Language::French, false)
    }

    pub fn spawn_with_language(language: Language) -> Self {
        Self::spawn_config(language, true)
    }

    /// Dropping the final command sender drains queued actions before saving.
    /// This also works when the bounded command queue is completely full.
    pub fn shutdown(self) -> Result<(), LocalizedText> {
        // Normal FIFO delivery also handles a diagnostic sender clone. If
        // full, dropping the application's sender still drains the queue.
        let _ = self.tx.try_send(Command::Shutdown);
        drop(self.tx);
        let Some(completion) = self.completion else {
            return Ok(());
        };
        completion
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| {
                LocalizedText::new(
                    "La sauvegarde de la lecture n'a pas pu etre confirmee avant la fermeture.",
                    "Playback could not be confirmed saved before closing.",
                )
            })?
    }

    fn spawn_config(language: Language, persist_playback: bool) -> Self {
        let state = Arc::new(Mutex::new(Snapshot {
            volume: 45_875,
            quality: settings::load(),
            status: "Connectez Spotify Premium pour commencer".into(),
            ..Snapshot::default()
        }));
        let (tx, rx) = mpsc::channel(32);
        let (finished, completion) = std::sync::mpsc::channel();
        let library = Arc::new(Mutex::new(LibrarySnapshot::default()));
        let shared_library = library.clone();
        let shared = state.clone();
        let result = std::thread::Builder::new()
            .name("spotiamp-engine".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .max_blocking_threads(2)
                    .build();
                let outcome = match runtime {
                    Ok(runtime) => {
                        let mut worker = Worker::new(shared);
                        worker.language = language;
                        worker.library.shared = shared_library;
                        worker.persist_playback = persist_playback;
                        // Bounded disk parsing stays off the UI thread.
                        let saved = persist_playback.then(playback_settings::load).flatten();
                        if let Some(saved) = saved {
                            worker.restore_session(saved);
                        }
                        worker.saved_fingerprint = worker.playback_fingerprint();
                        let outcome = runtime.block_on(worker.run(rx));
                        // Never hold up shutdown on an OAuth listener or decoder.
                        runtime.shutdown_timeout(Duration::from_millis(200));
                        outcome
                    }
                    Err(_) => {
                        edit(&shared, |s| {
                            s.status = "Impossible de demarrer le moteur audio".into()
                        });
                        Err("Impossible de demarrer le moteur audio".into())
                    }
                };
                let _ = finished.send(outcome);
            });
        if result.is_err() {
            edit(&state, |s| {
                s.status = "Impossible de creer le thread audio".into()
            });
        }
        Self {
            tx,
            state,
            library,
            completion: Some(completion),
        }
    }
}

fn edit(state: &Arc<Mutex<Snapshot>>, f: impl FnOnce(&mut Snapshot)) {
    if let Ok(mut snapshot) = state.lock() {
        f(&mut snapshot);
    }
}

fn restored_notice() -> LocalizedText {
    LocalizedText::new(
        "Lecture restauree en pause. Appuyez sur Lecture pour reprendre.",
        "Playback restored paused. Press Play to resume.",
    )
}

fn requested_bitrate(quality: AudioQuality) -> Bitrate {
    match quality {
        AudioQuality::Kbps96 => Bitrate::Bitrate96,
        AudioQuality::Kbps160 => Bitrate::Bitrate160,
        AudioQuality::Kbps320 => Bitrate::Bitrate320,
    }
}

fn player_config(quality: AudioQuality) -> PlayerConfig {
    PlayerConfig {
        bitrate: requested_bitrate(quality),
        gapless: false,
        ditherer: None,
        position_update_interval: Some(Duration::from_millis(500)),
        ..PlayerConfig::default()
    }
}

enum Update {
    Library(library::Update),
    CredentialCleared {
        generation: u64,
        result: Result<(), &'static str>,
    },
    Connected {
        generation: u64,
        result: Result<auth::Connected, LocalizedText>,
    },
    Rows {
        generation: u64,
        rows: Vec<TrackRow>,
        notice: LocalizedText,
    },
    Metadata {
        generation: u64,
        row: TrackRow,
    },
    Finished {
        generation: u64,
        result: Result<(), LocalizedText>,
    },
}

struct DeferredLoad {
    uri: SpotifyUri,
    quality: AudioQuality,
    position_ms: u32,
    deadline: tokio::time::Instant,
}

struct Worker {
    persist_playback: bool,
    saved_fingerprint: Option<u64>,
    last_checkpoint: std::time::Instant,
    resume_pending: bool,
    queue_playlist: Option<PlaylistId>,
    import_playlist: Option<PlaylistId>,
    restore_library_selection: Option<PlaylistId>,
    language: Language,
    state: Arc<Mutex<Snapshot>>,
    library: library::State,
    session: Option<Session>,
    player: Option<Arc<dyn PlayerTransport>>,
    mixer: Option<SoftMixer>,
    events: Option<PlayerEventChannel>,
    connect_task: Option<JoinHandle<()>>,
    login_attempt: Option<Arc<credentials::LoginAttempt>>,
    import_task: Option<JoinHandle<()>>,
    generation: u64,
    auth_generation: u64,
    auth_epoch: Arc<AtomicU64>,
    auth_backend: Arc<dyn auth::Backend>,
    clearing_credentials: bool,
    current_uri: Option<SpotifyUri>,
    stopped: bool,
    // Transport commands precede their asynchronous player events. Never let
    // a delayed Playing/Paused event reverse a newer button press's intent.
    wants_playback: bool,
    loading_audio: bool,
    deferred_load: Option<DeferredLoad>,
    order: order::Order,
    pending_loads: usize,
    request_id: Option<u64>,
    // The preference can change during playback. Keep the last load's request
    // separate; a subsequent Playing event must not claim the new preference.
    load_quality: AudioQuality,
    audio_error: Arc<Mutex<Option<LocalizedText>>>,
}

impl Worker {
    fn new(state: Arc<Mutex<Snapshot>>) -> Self {
        let load_quality = state.lock().map(|s| s.quality).unwrap_or_default();
        let wants_playback = state.lock().is_ok_and(|s| s.playing);
        let mut worker = Self {
            persist_playback: false,
            saved_fingerprint: None,
            last_checkpoint: std::time::Instant::now(),
            resume_pending: false,
            queue_playlist: None,
            import_playlist: None,
            restore_library_selection: None,
            language: Language::French,
            state,
            library: library::State::default(),
            session: None,
            player: None,
            mixer: None,
            events: None,
            connect_task: None,
            login_attempt: None,
            import_task: None,
            generation: 0,
            auth_generation: 0,
            auth_epoch: Arc::new(AtomicU64::new(0)),
            auth_backend: Arc::new(auth::Native),
            clearing_credentials: false,
            current_uri: None,
            stopped: true,
            wants_playback,
            loading_audio: false,
            deferred_load: None,
            order: order::Order::default(),
            pending_loads: 0,
            request_id: None,
            load_quality,
            audio_error: Arc::new(Mutex::new(None)),
        };
        let current = worker.snapshot().current;
        worker.reset_order(current);
        worker
    }

    async fn run(mut self, mut commands: mpsc::Receiver<Command>) -> Result<(), LocalizedText> {
        let (updates, mut update_rx) = mpsc::channel(16);
        self.start_connection(auth::Mode::Automatic, &updates);
        let mut health = tokio::time::interval(Duration::from_secs(2));
        health.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                // Drain already-queued input before an expired debounce timer.
                biased;
                command = commands.recv() => {
                    match command {
                        None | Some(Command::Shutdown) => break,
                        Some(command) => self.command(command, &updates),
                    }
                }
                Some(update) = update_rx.recv() => self.update(update, &updates),
                event = receive_event(&mut self.events) => {
                    if let Some(event) = event { self.player_event(event); }
                    else { self.events = None; }
                }
                _ = wait_for_load(self.deferred_load.as_ref().map(|load| load.deadline)) => {
                    self.dispatch_due_load(tokio::time::Instant::now());
                }
                _ = health.tick() => {
                    if self.last_checkpoint.elapsed() >= Duration::from_secs(15) {
                        self.last_checkpoint = std::time::Instant::now();
                        if let Err(message) = self.save_checkpoint() {
                            edit(&self.state, |s| s.status = message);
                        }
                    }
                    let audio_error = self.audio_error.lock().ok().and_then(|mut error| error.take());
                    if let Some(message) = audio_error {
                        self.stop();
                        edit(&self.state, |s| s.status = message);
                    }
                    let lost = self.session.as_ref().is_some_and(Session::is_invalid);
                    let failed = self.player.as_ref().is_some_and(|p| p.is_invalid());
                    if lost || failed {
                        self.disconnect();
                        edit(&self.state, |s| s.status = if lost {
                            "Connexion perdue. Cliquez sur Connecter pour reprendre."
                        } else { "Sortie audio indisponible. Verifiez le peripherique puis reconnectez-vous." }.into());
                    }
                }
            }
            self.sync_library_summary();
            self.restore_library_preview(&updates);
        }
        let saved = self.save_checkpoint();
        self.disconnect();
        saved
    }

    fn restore_session(&mut self, mut saved: PlaybackSession) {
        // A prior process may have quit while metadata was still arriving.
        // No request is running now; the real title will refresh on playback.
        for row in &mut saved.queue {
            if row.title_kind == TrackTitleKind::Loading {
                row.title_kind = TrackTitleKind::Unavailable;
            }
        }
        self.queue_playlist = saved.playlist.clone();
        self.restore_library_selection = saved.playlist;
        edit(&self.state, |s| {
            s.queue = saved.queue;
            s.current = saved.current.filter(|&index| index < s.queue.len());
            s.duration_ms = s.current.map_or(0, |index| s.queue[index].duration_ms);
            s.position_ms = if s.current.is_some() {
                saved.position_ms
            } else {
                0
            };
            s.volume = saved.volume;
            s.shuffle = saved.shuffle;
            s.repeat = saved.repeat;
            s.playing = false;
            if s.current.is_some() {
                s.playback_revision = 1;
                s.playback_origin = PlaybackOrigin::Explicit;
                s.status = restored_notice();
            }
        });
        let current = self.state.lock().ok().and_then(|state| state.current);
        self.resume_pending = current.is_some();
        self.stopped = true;
        self.wants_playback = false;
        self.reset_order(current);
    }

    fn playback_fingerprint(&self) -> Option<u64> {
        let state = self.state.lock().ok()?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        state.queue.hash(&mut hasher);
        state.current.hash(&mut hasher);
        state.position_ms.hash(&mut hasher);
        state.volume.hash(&mut hasher);
        state.shuffle.hash(&mut hasher);
        state.repeat.hash(&mut hasher);
        self.queue_playlist.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn save_checkpoint(&mut self) -> Result<(), LocalizedText> {
        if !self.persist_playback {
            return Ok(());
        }
        let fingerprint = self.playback_fingerprint();
        if fingerprint.is_some() && fingerprint == self.saved_fingerprint {
            return Ok(());
        }
        // Clone only at a changed checkpoint, not on progress ticks or paints.
        let saved = self
            .state
            .lock()
            .ok()
            .map(|state| playback_settings::from_snapshot(&state, self.queue_playlist.clone()))
            .ok_or_else(|| {
                LocalizedText::new(
                    "Etat de lecture indisponible pour la sauvegarde.",
                    "Playback state unavailable for saving.",
                )
            })?;
        playback_settings::save(&saved).map_err(|error| {
            LocalizedText::with_languages(
                format!("Lecture non memorisee : {error}"),
                format!("Playback could not be saved: {error}"),
                format!("Wiedergabe konnte nicht gespeichert werden: {error}"),
                format!("No se pudo guardar la reproducción: {error}"),
                format!("Impossibile salvare la riproduzione: {error}"),
                format!("Não foi possível guardar a reprodução: {error}"),
            )
        })?;
        self.saved_fingerprint = fingerprint;
        Ok(())
    }

    fn merge_queue_origin(&mut self, playlist: Option<PlaylistId>, replace: bool) {
        let previous = self.queue_playlist.clone();
        if replace || self.state.lock().is_ok_and(|s| s.queue.is_empty()) {
            self.queue_playlist = playlist;
        } else if self.queue_playlist != playlist {
            self.queue_playlist = None;
        }
        if self.queue_playlist != previous {
            self.restore_library_selection = None;
        }
    }

    fn snapshot(&self) -> Snapshot {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn start_connection(&mut self, mode: auth::Mode, updates: &mpsc::Sender<Update>) {
        if self.clearing_credentials {
            edit(&self.state, |s| {
                s.status = "Effacement du trousseau en cours...".into()
            });
            return;
        }
        // Coalesce repeated Connect presses. In particular, a startup attempt
        // cannot silently acquire permission to open a browser later on.
        if self.session.is_some() || self.connect_task.is_some() {
            return;
        }
        edit(&self.state, |s| {
            s.busy = true;
            s.status = match mode {
                auth::Mode::Automatic => "Reconnexion a la session Spotify enregistree...",
                auth::Mode::Manual => {
                    "Connexion Spotify : autorisez l'acces dans votre navigateur si demande"
                }
            }
            .into();
        });
        self.auth_generation = self.auth_generation.wrapping_add(1);
        self.auth_epoch
            .store(self.auth_generation, Ordering::Release);
        let generation = self.auth_generation;
        let epoch = self.auth_epoch.clone();
        let attempt =
            (mode == auth::Mode::Manual).then(|| Arc::new(credentials::LoginAttempt::default()));
        self.login_attempt = attempt.clone();
        let backend = self.auth_backend.clone();
        let language = self.language;
        let updates = updates.clone();
        self.connect_task = Some(tokio::spawn(async move {
            let result =
                auth::connect(backend.as_ref(), mode, attempt, epoch, generation, language).await;
            // Connected owns the session until Worker accepts it. A closed
            // update channel or cancelled/stale result safely shuts it down.
            let _ = updates.send(Update::Connected { generation, result }).await;
        }));
    }

    fn command(&mut self, command: Command, updates: &mpsc::Sender<Update>) {
        // A more recent transport/queue action supersedes a library Play
        // waiting on the network; its preview may still finish normally.
        if matches!(
            command,
            Command::Add(_)
                | Command::Play(_)
                | Command::PlayIndex(_)
                | Command::Toggle
                | Command::Stop
                | Command::Next
                | Command::Previous
                | Command::Remove(_)
                | Command::ReorderQueue { .. }
                | Command::Clear
        ) {
            self.cancel_library_queue();
        }
        match command {
            Command::Connect => self.start_connection(auth::Mode::Manual, updates),
            Command::CancelConnect => {
                // A release captured while connecting may arrive after success.
                // It must never turn into Logout or erase the saved session.
                if self.connect_task.is_some()
                    && self.session.is_none()
                    && !self.clearing_credentials
                {
                    self.disconnect();
                    edit(&self.state, |s| {
                        s.status = "Connexion annulee; session enregistree conservee".into()
                    });
                }
            }
            Command::Logout => {
                if self.clearing_credentials {
                    return;
                }
                self.disconnect();
                self.clearing_credentials = true;
                edit(&self.state, |s| {
                    s.busy = true;
                    s.status = "Deconnecte; effacement du trousseau en cours...".into();
                });
                let generation = self.auth_generation;
                let updates = updates.clone();
                let backend = self.auth_backend.clone();
                tokio::spawn(async move {
                    let result = backend.clear().await;
                    let _ = updates
                        .send(Update::CredentialCleared { generation, result })
                        .await;
                });
            }
            Command::Add(input) => {
                let uri = match parse_input(&input) {
                    Ok(uri) => uri,
                    Err(message) => {
                        edit(&self.state, |s| s.status = message.into());
                        return;
                    }
                };
                let Some(session) = self.session.clone() else {
                    edit(&self.state, |s| {
                        s.status = "Connectez Spotify avant d'ajouter un lien".into()
                    });
                    return;
                };
                if self.import_task.is_some() {
                    edit(&self.state, |s| {
                        s.status = "Import en cours; Vider permet de l'annuler".into()
                    });
                    return;
                }
                let capacity = MAX_QUEUE.saturating_sub(self.snapshot().queue.len());
                if capacity == 0 {
                    edit(&self.state, |s| {
                        s.status = "File pleine : 500 titres maximum".into()
                    });
                    return;
                }
                self.import_playlist = match &uri {
                    SpotifyUri::Playlist { .. } => uri.to_uri().ok().map(PlaylistId::Spotify),
                    _ => None,
                };
                let generation = self.generation;
                let updates = updates.clone();
                edit(&self.state, |s| {
                    s.busy = true;
                    s.status = "Chargement des titres Spotify...".into();
                });
                self.import_task = Some(tokio::spawn(async move {
                    let result = import(session, uri, capacity, generation, &updates).await;
                    let _ = updates.send(Update::Finished { generation, result }).await;
                }));
            }
            Command::Play(index) => self.press_play(index),
            Command::PlayIndex(index) => self.play(index),
            Command::Toggle => {
                let snapshot = self.snapshot();
                if let Some(player) = &self.player {
                    if !self.stopped && self.wants_playback {
                        player.pause();
                        self.wants_playback = false;
                    } else if self.stopped {
                        self.press_play(snapshot.current.unwrap_or(0));
                    } else {
                        // A queued selection has no decoder yet. Remember the
                        // intent without resuming the previous track instead.
                        if self.deferred_load.is_none() {
                            player.play();
                        }
                        self.wants_playback = true;
                    }
                } else {
                    edit(&self.state, |s| {
                        s.status = "Connectez Spotify pour lire la file".into()
                    });
                }
            }
            Command::ToggleShuffle => {
                edit(&self.state, |s| s.shuffle = !s.shuffle);
                self.reset_order(self.snapshot().current);
            }
            Command::ToggleRepeat => edit(&self.state, |s| s.repeat = !s.repeat),
            Command::Stop => self.stop(),
            Command::Next => self.next(PlaybackOrigin::Explicit),
            Command::Previous => self.previous(),
            Command::Seek(position) => {
                let position = position.min(self.snapshot().duration_ms);
                if self.resume_pending {
                    edit(&self.state, |s| s.position_ms = position);
                } else if let Some(load) = &mut self.deferred_load {
                    load.position_ms = position;
                    edit(&self.state, |s| s.position_ms = position);
                } else if let Some(player) = &self.player {
                    player.seek(position);
                    // Save the accepted intent even if the application closes
                    // before the asynchronous Seeked event reaches this worker.
                    edit(&self.state, |s| s.position_ms = position);
                }
            }
            Command::Volume(volume) => {
                if let Some(mixer) = &self.mixer {
                    mixer.set_volume(volume);
                }
                edit(&self.state, |s| s.volume = volume);
            }
            Command::SetLanguage(language) => self.language = language,
            Command::SetQuality(quality) => {
                self.set_quality(quality);
                if let Err(message) = settings::save(quality) {
                    edit(&self.state, |s| s.status = message.into());
                }
            }
            Command::Remove(index) => {
                let snapshot = self.snapshot();
                if index >= snapshot.queue.len() {
                    return;
                }
                let was_current = snapshot.current == Some(index);
                if was_current {
                    self.stop();
                    self.current_uri = None;
                }
                if snapshot.shuffle {
                    self.order.remove(index);
                }
                edit(&self.state, |s| remove_row(s, index));
                if self.state.lock().is_ok_and(|s| s.queue.is_empty()) {
                    self.queue_playlist = None;
                    self.restore_library_selection = None;
                }
            }
            Command::ReorderQueue {
                request_id,
                expected_queue,
                indices,
                insert_before,
            } => {
                let before = self.snapshot();
                if self.import_task.is_some()
                    || expected_queue.len() != before.queue.len()
                    || expected_queue
                        .iter()
                        .zip(&before.queue)
                        .any(|(uri, row)| uri != &row.uri)
                {
                    edit(&self.state, |s| {
                        s.status = "La file a change; aucun titre deplace".into()
                    });
                    return;
                }
                let Some(order) =
                    crate::model::reorder_indices(before.queue.len(), &indices, insert_before)
                else {
                    return;
                };
                let mut old_to_new = vec![0; order.len()];
                for (new, &old) in order.iter().enumerate() {
                    old_to_new[old] = new;
                }
                // The decoder, pending load and shuffle history keep identifying
                // the same occurrences; only their display positions change.
                edit(&self.state, |s| {
                    let mut rows = std::mem::take(&mut s.queue)
                        .into_iter()
                        .map(Some)
                        .collect::<Vec<_>>();
                    s.queue = order.iter().map(|&old| rows[old].take().unwrap()).collect();
                    s.current = s.current.map(|old| old_to_new[old]);
                    s.queue_reorder_request = request_id;
                });
                if before.shuffle {
                    self.order.reorder(&old_to_new);
                }
            }
            Command::Clear => {
                self.queue_playlist = None;
                self.restore_library_selection = None;
                self.cancel_import();
                self.stop();
                self.current_uri = None;
                self.order.clear();
                edit(&self.state, |s| {
                    s.queue.clear();
                    s.current = None;
                    s.duration_ms = 0;
                    s.status = "File vide".into();
                });
            }
            Command::Library(command) => self.library_command(command, updates),
            Command::Shutdown => {}
        }
    }

    fn update(&mut self, update: Update, updates: &mpsc::Sender<Update>) {
        match update {
            Update::Library(update) => self.library_update(update),
            Update::Connected { generation, result } => {
                // A result can already be queued when Logout cancels the task.
                if generation != self.auth_generation || self.connect_task.is_none() {
                    return;
                }
                self.connect_task = None;
                self.login_attempt = None;
                match result {
                    Ok(mut connection) => {
                        let session = connection.take_session();
                        let config = MixerConfig {
                            volume_ctrl: VolumeCtrl::Linear,
                            ..MixerConfig::default()
                        };
                        let mixer = match SoftMixer::open(config) {
                            Ok(mixer) => mixer,
                            Err(_) => {
                                session.shutdown();
                                edit(&self.state, |s| {
                                    s.busy = false;
                                    s.status = "Impossible d'initialiser le volume audio".into();
                                });
                                return;
                            }
                        };
                        mixer.set_volume(self.snapshot().volume);
                        let config = player_config(self.snapshot().quality);
                        let audio_error = self.audio_error.clone();
                        let player = Player::new(
                            config,
                            session.clone(),
                            mixer.get_soft_volume(),
                            move || Box::new(NativeSink::new(audio_error)),
                        );
                        self.events = Some(player.get_player_event_channel());
                        self.player = Some(player);
                        self.session = Some(session);
                        self.mixer = Some(mixer);
                        edit(&self.state, |s| {
                            s.connected = true;
                            s.busy = false;
                            s.status = connection.notice.take().unwrap_or_else(|| {
                                if self.resume_pending {
                                    restored_notice()
                                } else {
                                    "Connecte. Collez un lien de titre, album ou playlist.".into()
                                }
                            });
                        });
                        if self.library.open {
                            self.refresh_library(updates);
                        }
                    }
                    Err(message) => edit(&self.state, |s| {
                        s.busy = false;
                        s.status = message;
                    }),
                }
            }
            Update::Rows {
                generation,
                rows,
                notice,
            } if generation == self.generation => {
                let origin = self.import_playlist.take();
                self.merge_queue_origin(origin, false);
                edit(&self.state, |s| {
                    s.queue.extend(
                        rows.into_iter()
                            .take(MAX_QUEUE.saturating_sub(s.queue.len())),
                    );
                    s.status = notice;
                });
                self.queue_appended();
            }
            Update::Metadata { generation, row } if generation == self.generation => {
                edit(&self.state, |s| {
                    for existing in &mut s.queue {
                        if existing.uri == row.uri {
                            *existing = row.clone();
                        }
                    }
                    if let Some(index) = s.current
                        && let Some(current) = s.queue.get(index)
                    {
                        s.duration_ms = current.duration_ms;
                    }
                });
            }
            Update::Finished { generation, result } if generation == self.generation => {
                self.import_task = None;
                edit(&self.state, |s| {
                    s.busy = false;
                    if let Err(message) = result {
                        s.status = message;
                    }
                });
            }
            Update::CredentialCleared { generation, result }
                if generation == self.auth_generation =>
            {
                self.clearing_credentials = false;
                edit(&self.state, |s| {
                    s.busy = false;
                    s.status = match result {
                        Ok(()) => "Deconnecte; identifiants effaces du trousseau",
                        Err(message) => message,
                    }
                    .into();
                });
            }
            _ => {}
        }
    }

    fn set_quality(&mut self, quality: AudioQuality) {
        // This deliberately sends no command to the player: even a paused or
        // still-loading track retains its original requested stream quality.
        edit(&self.state, |s| {
            s.quality = quality;
            s.status = LocalizedText::with_languages(
                format!("Prochain chargement : {} kb/s demandes", quality.kbps()),
                format!("Next load: {} kb/s requested", quality.kbps()),
                format!("Nächster Ladevorgang: {} kb/s angefordert", quality.kbps()),
                format!("Próxima carga: {} kb/s solicitados", quality.kbps()),
                format!("Prossimo caricamento: {} kb/s richiesti", quality.kbps()),
                format!("Próximo carregamento: {} kb/s pedidos", quality.kbps()),
            );
        });
    }

    fn play(&mut self, index: usize) {
        if self.load(index, PlaybackOrigin::Explicit) {
            // Explicit selection starts a new cycle anchored at that row.
            self.reset_order(Some(index));
        }
    }

    fn load(&mut self, index: usize, origin: PlaybackOrigin) -> bool {
        self.load_at(index, origin, 0)
    }

    fn load_at(&mut self, index: usize, origin: PlaybackOrigin, position_ms: u32) -> bool {
        let snapshot = self.snapshot();
        let Some(row) = snapshot.queue.get(index) else {
            return false;
        };
        if self.player.is_none() {
            edit(&self.state, |s| {
                s.status = "Connectez Spotify pour lire la file".into()
            });
            return false;
        }
        if let Ok(uri) = SpotifyUri::from_uri(&row.uri) {
            self.resume_pending = false;
            let immediate =
                self.stopped || self.current_uri.is_none() || origin == PlaybackOrigin::Automatic;
            let now = tokio::time::Instant::now();
            // Navigation remains immediate in the queue, while only the last
            // selection in a burst reaches the network/audio loader.
            self.deferred_load = Some(DeferredLoad {
                uri: uri.clone(),
                quality: snapshot.quality,
                position_ms,
                deadline: now
                    + if immediate {
                        Duration::ZERO
                    } else {
                        LOAD_SETTLE_TIME
                    },
            });
            self.current_uri = Some(uri);
            self.stopped = false;
            self.wants_playback = true;
            self.loading_audio = true;
            edit(&self.state, |s| {
                s.current = Some(index);
                s.playback_revision = s.playback_revision.wrapping_add(1);
                s.playback_origin = origin;
                s.position_ms = position_ms;
                s.duration_ms = row.duration_ms;
                s.playing = false;
                s.status = "Chargement audio...".into();
            });
            self.dispatch_due_load(now);
            return true;
        }
        false
    }

    fn dispatch_due_load(&mut self, now: tokio::time::Instant) {
        if !self
            .deferred_load
            .as_ref()
            .is_some_and(|load| load.deadline <= now)
        {
            return;
        }
        let load = self.deferred_load.take().unwrap();
        if let Some(player) = &self.player {
            // Count only actual loads: coalesced selections produce no ACK.
            // Older events stay blocked until every dispatched Load is ACKed,
            // including duplicates of the same Spotify URI.
            self.pending_loads += 1;
            self.load_quality = load.quality;
            player.load_with_bitrate(
                load.uri,
                self.wants_playback,
                load.position_ms,
                requested_bitrate(load.quality),
            );
        }
    }

    fn press_play(&mut self, start_index: usize) {
        if self.resume_pending {
            let resume = self
                .state
                .lock()
                .ok()
                .and_then(|state| state.current.map(|index| (index, state.position_ms)));
            if let Some((index, position_ms)) = resume {
                if self.load_at(index, PlaybackOrigin::Explicit, position_ms) {
                    self.reset_order(Some(index));
                }
                return;
            }
        }
        if self.stopped || self.current_uri.is_none() {
            self.play(start_index);
        } else if let Some(player) = &self.player {
            if !self.wants_playback {
                if self.deferred_load.is_none() {
                    player.play();
                }
                self.wants_playback = true;
            } else if !self.loading_audio {
                // Seeking keeps the loaded decoder and its requested quality.
                // Repeated Play while Loading must not restart network work.
                player.seek(0);
                edit(&self.state, |s| {
                    s.position_ms = 0;
                    s.playback_revision = s.playback_revision.wrapping_add(1);
                    s.playback_origin = PlaybackOrigin::Explicit;
                });
            }
        }
    }

    fn next(&mut self, origin: PlaybackOrigin) {
        let snapshot = self.snapshot();
        if self.player.is_none() {
            edit(&self.state, |s| {
                s.status = "Connectez Spotify pour lire la file".into()
            });
            return;
        }
        let next = if snapshot.shuffle {
            self.order.next(snapshot.repeat, &mut rand::rng())
        } else {
            let next = snapshot.current.map_or(0, |index| index + 1);
            if next < snapshot.queue.len() {
                Some(next)
            } else if snapshot.repeat && !snapshot.queue.is_empty() {
                Some(0)
            } else {
                None
            }
        };
        if let Some(index) = next {
            self.load(index, origin);
        } else {
            self.stop();
            edit(&self.state, |s| s.status = "Fin de la file".into());
        }
    }

    fn previous(&mut self) {
        let snapshot = self.snapshot();
        let Some(player) = &self.player else {
            edit(&self.state, |s| {
                s.status = "Connectez Spotify pour lire la file".into()
            });
            return;
        };
        if snapshot.position_ms > 3_000 && !self.stopped {
            if let Some(load) = &mut self.deferred_load {
                load.position_ms = 0;
                edit(&self.state, |s| s.position_ms = 0);
            } else {
                player.seek(0);
            }
            edit(&self.state, |s| {
                s.position_ms = 0;
                s.playback_revision = s.playback_revision.wrapping_add(1);
                s.playback_origin = PlaybackOrigin::Explicit;
            });
            return;
        }
        let previous = if snapshot.shuffle {
            self.order.previous()
        } else if snapshot.queue.is_empty() {
            None
        } else if snapshot.current == Some(0) && snapshot.repeat {
            Some(snapshot.queue.len() - 1)
        } else {
            Some(snapshot.current.unwrap_or(0).saturating_sub(1))
        };
        if let Some(index) = previous {
            self.load(index, PlaybackOrigin::Explicit);
        }
    }

    fn reset_order(&mut self, current: Option<usize>) {
        if let Ok(snapshot) = self.state.lock() {
            if snapshot.shuffle {
                self.order
                    .reset(snapshot.queue.len(), current, &mut rand::rng());
            } else {
                self.order.clear();
            }
        }
    }

    fn queue_appended(&mut self) {
        if let Ok(snapshot) = self.state.lock()
            && snapshot.shuffle
        {
            self.order.append(snapshot.queue.len(), &mut rand::rng());
        }
    }

    fn stop(&mut self) {
        self.resume_pending = false;
        self.deferred_load = None;
        self.stopped = true;
        self.wants_playback = false;
        self.loading_audio = false;
        if let Some(player) = &self.player {
            player.stop();
        }
        edit(&self.state, |s| {
            s.playing = false;
            s.position_ms = 0;
            s.status = "Arrete".into();
        });
    }

    fn player_event(&mut self, event: PlayerEvent) {
        use PlayerEvent::*;
        if let PlayRequestIdChanged { play_request_id } = event {
            // Seeking while Loading re-emits the SAME request ID upstream.
            // It must not acknowledge a different Load that is still queued.
            if self.request_id != Some(play_request_id) {
                self.pending_loads = self.pending_loads.saturating_sub(1);
            }
            self.request_id = Some(play_request_id);
            return;
        }
        let event_request = match &event {
            PositionChanged {
                play_request_id, ..
            } => Some(*play_request_id),
            _ => event.get_play_request_id(),
        };
        let active = |uri: &SpotifyUri| {
            self.current_uri.as_ref() == Some(uri)
                && !self.stopped
                && self.deferred_load.is_none()
                && self.pending_loads == 0
                && (event_request.is_none() || event_request == self.request_id)
        };
        match event {
            Playing {
                track_id,
                position_ms,
                ..
            } if active(&track_id) => {
                self.loading_audio = false;
                edit(&self.state, |s| {
                    s.playing = true;
                    s.position_ms = position_ms;
                    s.status = LocalizedText::with_languages(
                        format!(
                            "Lecture - qualite demandee {} kb/s",
                            self.load_quality.kbps()
                        ),
                        format!(
                            "Playing - requested quality {} kb/s",
                            self.load_quality.kbps()
                        ),
                        format!(
                            "Wiedergabe - angeforderte Qualität {} kb/s",
                            self.load_quality.kbps()
                        ),
                        format!(
                            "Reproduciendo - calidad solicitada {} kb/s",
                            self.load_quality.kbps()
                        ),
                        format!(
                            "Riproduzione - qualità richiesta {} kb/s",
                            self.load_quality.kbps()
                        ),
                        format!(
                            "A reproduzir - qualidade pedida {} kb/s",
                            self.load_quality.kbps()
                        ),
                    );
                });
            }
            Paused {
                track_id,
                position_ms,
                ..
            } if active(&track_id) => {
                self.loading_audio = false;
                edit(&self.state, |s| {
                    s.playing = false;
                    s.position_ms = position_ms;
                    s.status = "Pause".into();
                });
            }
            Loading {
                track_id,
                position_ms,
                ..
            } if active(&track_id) => {
                self.loading_audio = true;
                edit(&self.state, |s| {
                    s.playing = false;
                    s.position_ms = position_ms;
                    s.status = "Chargement audio...".into();
                });
            }
            PositionChanged {
                track_id,
                position_ms,
                ..
            }
            | PositionCorrection {
                track_id,
                position_ms,
                ..
            }
            | Seeked {
                track_id,
                position_ms,
                ..
            } if active(&track_id) => {
                edit(&self.state, |s| s.position_ms = position_ms);
            }
            TrackChanged { audio_item } if active(&audio_item.track_id) => {
                edit(&self.state, |s| {
                    s.duration_ms = audio_item.duration_ms;
                    if let Some(row) = s.current.and_then(|i| s.queue.get_mut(i)) {
                        row.title = audio_item.name;
                        row.title_kind = TrackTitleKind::Metadata;
                        row.duration_ms = audio_item.duration_ms;
                        if let librespot_metadata::audio::item::UniqueFields::Track {
                            artists,
                            ..
                        } = audio_item.unique_fields
                        {
                            row.artist = artists
                                .iter()
                                .map(|a| a.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                        }
                    }
                });
            }
            EndOfTrack { track_id, .. } if active(&track_id) => {
                self.next(PlaybackOrigin::Automatic)
            }
            Unavailable { track_id, .. } if active(&track_id) => {
                self.stopped = true;
                self.wants_playback = false;
                self.loading_audio = false;
                edit(&self.state, |s| {
                    s.playing = false;
                    s.status =
                        "Titre indisponible, acces refuse ou erreur audio. Essayez Suivant.".into();
                });
            }
            _ => {}
        }
    }

    fn cancel_import(&mut self) {
        self.import_playlist = None;
        self.generation = self.generation.wrapping_add(1);
        if let Some(task) = self.import_task.take() {
            task.abort();
        }
        edit(&self.state, |s| s.busy = self.connect_task.is_some());
    }

    fn disconnect(&mut self) {
        // A lost session releases audio but must not erase the resume point.
        let resume = if self.resume_pending || !self.stopped {
            self.state
                .lock()
                .ok()
                .and_then(|s| s.current.map(|_| s.position_ms))
        } else {
            None
        };
        self.reset_library("Connectez Spotify pour afficher vos playlists");
        self.auth_generation = self.auth_generation.wrapping_add(1);
        self.auth_epoch
            .store(self.auth_generation, Ordering::Release);
        if let Some(task) = self.connect_task.take() {
            task.abort();
            if let Some(attempt) = self.login_attempt.take() {
                attempt.cancel();
            }
        }
        self.cancel_import();
        self.stop();
        if let Some(position_ms) = resume {
            self.resume_pending = true;
            edit(&self.state, |s| s.position_ms = position_ms);
        }
        self.current_uri = None;
        if let Some(session) = self.session.take() {
            session.shutdown();
        }
        self.events = None;
        if let Some(player) = self.player.take() {
            // Upstream Drop joins pending loader threads. Keep this runtime
            // available to finish network cancellation while they wind down.
            tokio::task::spawn_blocking(move || drop(player));
        }
        self.mixer = None;
        self.pending_loads = 0;
        self.request_id = None;
        edit(&self.state, |s| {
            s.connected = false;
            s.busy = false;
        });
    }
}

async fn receive_event(events: &mut Option<PlayerEventChannel>) -> Option<PlayerEvent> {
    match events {
        Some(events) => events.recv().await,
        None => std::future::pending().await,
    }
}

async fn wait_for_load(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

/// A small fallible Rodio sink. Upstream's rodio constructor unwraps device
/// errors; here a missing output device leaves the GUI alive with a useful error.
/// The device opens on Play, and its resources are released on Stop/Pause.
pub(crate) struct NativeSink {
    output: Option<(rodio::Sink, rodio::OutputStream)>,
    error: Arc<Mutex<Option<LocalizedText>>>,
}

impl NativeSink {
    pub(crate) fn new(error: Arc<Mutex<Option<LocalizedText>>>) -> Self {
        Self {
            output: None,
            error,
        }
    }
}

impl Sink for NativeSink {
    fn start(&mut self) -> SinkResult<()> {
        if self.output.is_none() {
            let mut stream =
                rodio::OutputStreamBuilder::open_default_stream().map_err(|error| {
                    let message = LocalizedText::with_languages(
                        format!("Sortie audio indisponible : {error}"),
                        format!("Audio output unavailable: {error}"),
                        format!("Audioausgabe nicht verfügbar: {error}"),
                        format!("Salida de audio no disponible: {error}"),
                        format!("Uscita audio non disponibile: {error}"),
                        format!("Saída de áudio indisponível: {error}"),
                    );
                    if let Ok(mut shared) = self.error.lock() {
                        *shared = Some(message.clone());
                    }
                    SinkError::ConnectionRefused(message.to_string())
                })?;
            stream.log_on_drop(false);
            let sink = rodio::Sink::connect_new(stream.mixer());
            self.output = Some((sink, stream));
        }
        if let Some((sink, _)) = &self.output {
            sink.play();
        }
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        // Upstream treats a stop error as fatal. Always finish successfully,
        // including after a device is unplugged; never wait forever on a sink.
        if let Some((sink, _stream)) = self.output.take() {
            // Drain the small queue before pausing so resuming does not skip
            // already decoded samples. Bound the wait for a removed device.
            let started = std::time::Instant::now();
            while !sink.empty() && started.elapsed() < Duration::from_millis(300) {
                std::thread::sleep(Duration::from_millis(5));
            }
            sink.stop();
        }
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        let Some((sink, _)) = &self.output else {
            return Err(SinkError::NotConnected("Sortie audio fermee".into()));
        };
        let samples = packet
            .samples()
            .map_err(|error| SinkError::OnWrite(error.to_string()))?;
        let samples = converter.f64_to_f32(samples);
        sink.append(rodio::buffer::SamplesBuffer::new(2, 44_100, samples));
        // Bounded short PCM queue, not a decoded-song cache. Give the driver up
        // to one second to make progress; do not deadlock on device removal.
        let started = std::time::Instant::now();
        while sink.len() > 8 {
            if started.elapsed() > Duration::from_secs(1) {
                let message = "La sortie audio ne repond plus; verifiez votre peripherique";
                if let Ok(mut shared) = self.error.lock() {
                    *shared = Some(message.into());
                }
                return Err(SinkError::OnWrite(message.into()));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}

fn remove_row(snapshot: &mut Snapshot, index: usize) {
    if index >= snapshot.queue.len() {
        return;
    }
    snapshot.queue.remove(index);
    snapshot.current = match snapshot.current {
        Some(current) if current == index => {
            snapshot.duration_ms = 0;
            snapshot.position_ms = 0;
            snapshot.playing = false;
            None
        }
        Some(current) if current > index => Some(current - 1),
        other => other,
    };
}

/// Accept only explicit Spotify music resources. No arbitrary URL fetches,
/// local-file access, embedded credentials, shortened links or extra URI parts.
fn parse_input(input: &str) -> Result<SpotifyUri, &'static str> {
    let input = input.trim();
    if input.len() > 2048 {
        return Err("Lien trop long");
    }
    let canonical = if input.starts_with("spotify:") {
        let parts: Vec<_> = input.split(':').collect();
        if parts.len() != 3 {
            return Err("URI attendue : spotify:track|album|playlist:identifiant");
        }
        input.to_owned()
    } else {
        let url = url::Url::parse(input)
            .map_err(|_| "Collez un lien Spotify de titre, album ou playlist")?;
        if url.scheme() != "https"
            || url.host_str() != Some("open.spotify.com")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
        {
            return Err("Lien attendu : https://open.spotify.com/track|album|playlist/identifiant");
        }
        let mut parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        if parts.first().is_some_and(|part| part.starts_with("intl-")) {
            parts.remove(0);
        }
        if parts.len() != 2 {
            return Err("Ce lien Spotify n'est pas un titre, album ou playlist");
        }
        format!("spotify:{}:{}", parts[0], parts[1])
    };
    let parts: Vec<_> = canonical.split(':').collect();
    if !matches!(parts[1], "track" | "album" | "playlist")
        || parts[2].len() != 22
        || !parts[2].bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err("Seuls les liens Spotify de titre, album et playlist sont pris en charge");
    }
    SpotifyUri::from_uri(&canonical).map_err(|_| "Identifiant Spotify invalide")
}

async fn resolve_ids(
    session: &Session,
    uri: &SpotifyUri,
    capacity: usize,
) -> Result<(Vec<SpotifyUri>, bool), LocalizedText> {
    tokio::time::timeout(NETWORK_TIMEOUT, async {
        match uri {
            SpotifyUri::Track { .. } => Ok((vec![uri.clone()], false)),
            SpotifyUri::Album { .. } => {
                let album = Album::get(session, uri).await?;
                let truncated = album.tracks().count() > capacity;
                Ok((album.tracks().take(capacity).cloned().collect(), truncated))
            }
            SpotifyUri::Playlist { .. } => {
                let playlist = Playlist::get(session, uri).await?;
                let truncated =
                    playlist.contents.is_truncated || playlist.length as usize > capacity;
                let tracks = playlist
                    .tracks()
                    .filter(|id| matches!(id, SpotifyUri::Track { .. }))
                    .take(capacity)
                    .cloned()
                    .collect();
                Ok((tracks, truncated))
            }
            _ => unreachable!("input validated before network request"),
        }
    })
    .await
    .map_err(|_| LocalizedText::from("Spotify ne repond pas; reessayez"))?
    .map_err(|_: librespot_core::Error| {
        LocalizedText::from("Impossible de lire ce lien Spotify; verifiez son acces")
    })
}

async fn track_metadata(session: &Session, uri: &SpotifyUri) -> TrackRow {
    let result = tokio::time::timeout(Duration::from_secs(8), Track::get(session, uri)).await;
    let uri = uri.to_uri().unwrap_or_default();
    match result {
        Ok(Ok(track)) => TrackRow {
            title_kind: TrackTitleKind::Metadata,
            uri,
            title: track.name,
            artist: track
                .artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            duration_ms: track.duration.max(0) as u32,
        },
        _ => TrackRow {
            title_kind: TrackTitleKind::Unavailable,
            uri,
            ..TrackRow::default()
        },
    }
}

async fn import(
    session: Session,
    uri: SpotifyUri,
    capacity: usize,
    generation: u64,
    updates: &mpsc::Sender<Update>,
) -> Result<(), LocalizedText> {
    let (ids, truncated) = resolve_ids(&session, &uri, capacity).await?;
    if ids.is_empty() {
        return Err("Aucun titre musical accessible dans ce lien".into());
    }
    let rows: Vec<_> = ids
        .iter()
        .filter_map(|id| {
            id.to_uri().ok().map(|uri| TrackRow {
                title_kind: TrackTitleKind::Loading,
                uri,
                ..TrackRow::default()
            })
        })
        .collect();
    let count = rows.len();
    let notice = if truncated {
        LocalizedText::with_languages(
            format!("{count} titres ajoutes; liste limitee a {MAX_QUEUE} ou tronquee par Spotify"),
            format!("{count} tracks added; list limited to {MAX_QUEUE} or truncated by Spotify"),
            format!(
                "{count} Titel hinzugefügt; Liste auf {MAX_QUEUE} begrenzt oder von Spotify gekürzt"
            ),
            format!(
                "{count} canciones añadidas; lista limitada a {MAX_QUEUE} o recortada por Spotify"
            ),
            format!("{count} brani aggiunti; lista limitata a {MAX_QUEUE} o troncata da Spotify"),
            format!(
                "{count} faixas adicionadas; lista limitada a {MAX_QUEUE} ou truncada pelo Spotify"
            ),
        )
    } else {
        LocalizedText::with_languages(
            format!("{count} titre(s) ajoute(s). Double-cliquez pour lire."),
            format!("{count} track(s) added. Double-click to play."),
            format!("{count} Titel hinzugefügt. Doppelklick zum Abspielen."),
            format!("{count} canciones añadidas. Doble clic para reproducir."),
            format!("{count} brani aggiunti. Doppio clic per riprodurre."),
            format!("{count} faixas adicionadas. Duplo clique para reproduzir."),
        )
    };
    updates
        .send(Update::Rows {
            generation,
            rows,
            notice,
        })
        .await
        .map_err(|_| "Application fermee")?;

    // Two metadata requests at a time keep imports usable without retaining
    // hundreds of full Track/Album protobufs, covers or artist biographies.
    let mut pending: VecDeque<_> = ids.into();
    let mut requests = JoinSet::new();
    loop {
        while requests.len() < 2 {
            let Some(uri) = pending.pop_front() else {
                break;
            };
            let session = session.clone();
            requests.spawn(async move { track_metadata(&session, &uri).await });
        }
        match requests.join_next().await {
            Some(Ok(row)) => {
                updates
                    .send(Update::Metadata { generation, row })
                    .await
                    .map_err(|_| "Application fermee")?;
            }
            Some(Err(_)) => {}
            None => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "4uLU6hMCjMI75M1A2tKUQC";

    #[test]
    fn language_delivery_retries_after_full_queue_and_stops_after_close() {
        let (tx, mut rx) = mpsc::channel(1);
        let state = Arc::new(Mutex::new(Snapshot::default()));
        let engine = Engine {
            tx,
            state: state.clone(),
            library: Arc::new(Mutex::new(LibrarySnapshot::default())),
            completion: None,
        };
        let before = state.lock().unwrap().clone();
        engine.tx.try_send(Command::Next).unwrap();
        assert!(!engine.sync_language(Language::English));
        assert!(matches!(rx.try_recv(), Ok(Command::Next)));
        assert!(engine.sync_language(Language::English));
        assert!(matches!(
            rx.try_recv(),
            Ok(Command::SetLanguage(Language::English))
        ));
        assert!(rx.try_recv().is_err());
        assert_eq!(*state.lock().unwrap(), before);
        drop(rx);
        assert!(engine.sync_language(Language::French));
    }

    #[test]
    fn graceful_shutdown_drains_a_full_queue_before_acknowledging_completion() {
        let (tx, rx) = mpsc::channel(2);
        let (finished, completion) = std::sync::mpsc::channel();
        let state = Arc::new(Mutex::new(Snapshot::default()));
        let shared = state.clone();
        let library = Arc::new(Mutex::new(LibrarySnapshot::default()));
        tx.try_send(Command::Volume(123)).unwrap();
        tx.try_send(Command::Volume(456)).unwrap();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let outcome = runtime.block_on(async move {
                let mut worker = Worker::new(shared);
                // Synthetic session prevents startup auth; no network or OS store.
                worker.session = Some(Session::new(SessionConfig::default(), None));
                worker.run(rx).await
            });
            let _ = finished.send(outcome);
        });
        let engine = Engine {
            tx,
            state: state.clone(),
            library,
            completion: Some(completion),
        };
        engine.shutdown().unwrap();
        assert_eq!(state.lock().unwrap().volume, 456);
        thread.join().unwrap();
    }

    #[test]
    fn quality_preferences_map_to_requested_stream_bitrates() {
        for (quality, bitrate) in [
            (AudioQuality::Kbps96, Bitrate::Bitrate96),
            (AudioQuality::Kbps160, Bitrate::Bitrate160),
            (AudioQuality::Kbps320, Bitrate::Bitrate320),
        ] {
            assert_eq!(player_config(quality).bitrate, bitrate);
            assert!(!player_config(quality).gapless);
        }
        assert_eq!(
            player_config(AudioQuality::default()).bitrate,
            Bitrate::Bitrate320
        );
    }

    #[test]
    fn changing_quality_preserves_current_playback_and_its_requested_quality() {
        let state = Arc::new(Mutex::new(Snapshot {
            queue: vec![TrackRow::default()],
            current: Some(0),
            connected: true,
            playing: true,
            position_ms: 12_345,
            quality: AudioQuality::Kbps160,
            ..Snapshot::default()
        }));
        let mut worker = Worker::new(state.clone());
        let uri = parse_input(&format!("spotify:track:{ID}")).unwrap();
        worker.current_uri = Some(uri.clone());
        worker.stopped = false;
        worker.request_id = Some(7);
        worker.set_quality(AudioQuality::Kbps320);

        let snapshot = worker.snapshot();
        assert_eq!(snapshot.quality, AudioQuality::Kbps320);
        assert_eq!(snapshot.current, Some(0));
        assert!(snapshot.playing);
        assert_eq!(snapshot.position_ms, 12_345);
        assert_eq!(worker.pending_loads, 0);
        assert_eq!(worker.request_id, Some(7));
        assert_eq!(worker.load_quality, AudioQuality::Kbps160);

        // Resuming the existing stream must still describe its original
        // request, even though the next load will use the new preference.
        worker.player_event(PlayerEvent::Playing {
            play_request_id: 7,
            track_id: uri,
            position_ms: 12_345,
        });
        assert_eq!(
            worker.snapshot().status,
            "Lecture - qualite demandee 160 kb/s"
        );
        assert_eq!(worker.snapshot().quality, AudioQuality::Kbps320);
    }

    #[test]
    fn queued_load_events_cannot_report_a_later_unloaded_quality_preference() {
        let state = Arc::new(Mutex::new(Snapshot {
            current: Some(0),
            quality: AudioQuality::Kbps320,
            ..Snapshot::default()
        }));
        let mut worker = Worker::new(state);
        let uri = parse_input(&format!("spotify:track:{ID}")).unwrap();
        worker.current_uri = Some(uri.clone());
        worker.stopped = false;
        worker.pending_loads = 2;
        worker.request_id = Some(1);
        // Two loads are already queued; the newest requested 320. Changing
        // the preference again must not relabel either of those requests.
        worker.set_quality(AudioQuality::Kbps96);
        let preference_notice = worker.snapshot().status;
        worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 2 });
        worker.player_event(PlayerEvent::Playing {
            play_request_id: 2,
            track_id: uri.clone(),
            position_ms: 0,
        });
        assert_eq!(worker.snapshot().status, preference_notice);
        worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 3 });
        worker.player_event(PlayerEvent::Playing {
            play_request_id: 2,
            track_id: uri.clone(),
            position_ms: 0,
        });
        assert_eq!(worker.snapshot().status, preference_notice);
        worker.player_event(PlayerEvent::Playing {
            play_request_id: 3,
            track_id: uri,
            position_ms: 0,
        });
        assert_eq!(
            worker.snapshot().status,
            "Lecture - qualite demandee 320 kb/s"
        );
        assert_eq!(worker.snapshot().quality, AudioQuality::Kbps96);
    }

    #[test]
    fn spotify_links_normalize_without_tracking_query() {
        for input in [
            format!(" spotify:track:{ID} "),
            format!("https://open.spotify.com/track/{ID}?si=ignored"),
            format!("https://open.spotify.com/intl-fr/track/{ID}/"),
        ] {
            assert_eq!(
                parse_input(&input).unwrap().to_uri().unwrap(),
                format!("spotify:track:{ID}")
            );
        }
        assert!(matches!(
            parse_input(&format!("spotify:album:{ID}")),
            Ok(SpotifyUri::Album { .. })
        ));
        assert!(matches!(
            parse_input(&format!("spotify:playlist:{ID}")),
            Ok(SpotifyUri::Playlist { .. })
        ));
    }

    #[test]
    fn malformed_or_untrusted_links_are_rejected_before_network_access() {
        for input in [
            format!("https://open.spotify.com.evil.example/track/{ID}"),
            format!("https://open.spotify.com@evil.example/track/{ID}"),
            format!("https://user@open.spotify.com/track/{ID}"),
            format!("http://open.spotify.com/track/{ID}"),
            format!("spotify:track:{ID}:extra"),
            format!("spotify:artist:{ID}"),
            "spotify:track:abc".into(),
            format!("https://open.spotify.com/track/{ID}/extra"),
            "file:///etc/passwd".into(),
            "spotify:track:zzzzzzzzzzzzzzzzzzzzzz".into(),
        ] {
            assert!(parse_input(&input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn removing_before_current_preserves_playback_selection() {
        let mut state = Snapshot {
            queue: vec![TrackRow::default(); 4],
            current: Some(2),
            playing: true,
            position_ms: 12_000,
            ..Snapshot::default()
        };
        remove_row(&mut state, 0);
        assert_eq!(state.current, Some(1));
        assert!(state.playing);
        assert_eq!(state.position_ms, 12_000);
        remove_row(&mut state, 1);
        assert_eq!(state.current, None);
        assert!(!state.playing);
        assert_eq!(state.position_ms, 0);
        remove_row(&mut state, 100);
        assert_eq!(state.queue.len(), 2);
    }

    #[test]
    fn stale_events_cannot_change_a_new_load_of_the_same_track() {
        let state = Arc::new(Mutex::new(Snapshot {
            queue: vec![TrackRow::default()],
            current: Some(0),
            playing: true,
            position_ms: 123,
            ..Snapshot::default()
        }));
        let mut worker = Worker::new(state.clone());
        let uri = parse_input(&format!("spotify:track:{ID}")).unwrap();
        worker.current_uri = Some(uri.clone());
        worker.stopped = false;
        worker.pending_loads = 1;
        worker.request_id = Some(1);
        worker.player_event(PlayerEvent::EndOfTrack {
            play_request_id: 1,
            track_id: uri.clone(),
        });
        assert!(state.lock().unwrap().playing);
        worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 2 });
        worker.player_event(PlayerEvent::PositionChanged {
            play_request_id: 1,
            track_id: uri.clone(),
            position_ms: 9_000,
        });
        assert_eq!(state.lock().unwrap().position_ms, 123);
        worker.player_event(PlayerEvent::PositionChanged {
            play_request_id: 2,
            track_id: uri,
            position_ms: 2_000,
        });
        assert_eq!(state.lock().unwrap().position_ms, 2_000);
    }

    #[test]
    fn duplicate_ack_from_seek_during_loading_does_not_acknowledge_next_load() {
        let mut worker = Worker::new(Arc::new(Mutex::new(Snapshot::default())));
        worker.pending_loads = 2;
        worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 0 });
        assert_eq!(worker.pending_loads, 1);
        worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 0 });
        assert_eq!(worker.pending_loads, 1);
        worker.player_event(PlayerEvent::PlayRequestIdChanged { play_request_id: 2 });
        assert_eq!(worker.pending_loads, 0);
    }
}
