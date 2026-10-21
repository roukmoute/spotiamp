//! Catalogue and one lazy playlist preview; independent of audio snapshots.
use super::*;
use crate::model::{LibraryCommand, LibrarySnapshot, PlaylistEntry, PlaylistId};

#[derive(Default)]
pub(super) struct State {
    pub(super) shared: Arc<Mutex<LibrarySnapshot>>,
    pub(super) open: bool,
    generation: u64,
    task: Option<JoinHandle<()>>,
    ready: Option<(PlaylistId, bool)>,
    pending_queue: Option<QueueRequest>,
}

#[derive(Clone, Copy)]
struct QueueRequest {
    replace: bool,
    play: bool,
    start: usize,
    audio_generation: u64,
}

pub(super) enum Update {
    Catalogue {
        generation: u64,
        result: Result<Vec<PlaylistEntry>, String>,
    },
    Loaded {
        generation: u64,
        result: Result<(Vec<TrackRow>, bool), String>,
    },
    Metadata {
        generation: u64,
        rows: Vec<TrackRow>,
    },
    Finished {
        generation: u64,
    },
}

impl State {
    fn edit(&self, change: impl FnOnce(&mut LibrarySnapshot)) {
        if let Ok(mut snapshot) = self.shared.lock() {
            change(&mut snapshot);
            snapshot.revision = snapshot.revision.wrapping_add(1);
        }
    }

    fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.pending_queue = None;
        self.ready = None;
    }
}

impl Worker {
    pub(super) fn sync_library_summary(&self) {
        // No queue clone or library hash on a position/volume update.
        if let Ok(audio) = self.state.lock()
            && let Ok(mut library) = self.library.shared.lock()
            && (library.connected != audio.connected || library.queue_len != audio.queue.len())
        {
            library.connected = audio.connected;
            library.queue_len = audio.queue.len();
            library.revision = library.revision.wrapping_add(1);
        }
    }

    pub(super) fn reset_library(&mut self, message: &str) {
        self.library.cancel();
        self.library.edit(|s| {
            s.entries = Arc::default();
            s.tracks = Arc::default();
            s.selected_id = None;
            s.loading = false;
            s.busy = false;
            s.status = message.into();
        });
    }

    pub(super) fn cancel_library_queue(&mut self) {
        // Clear is authoritative even if a playlist response is already queued.
        if self.library.pending_queue.take().is_some() {
            self.library.edit(|s| s.busy = false);
        }
    }

    pub(super) fn library_command(
        &mut self,
        command: LibraryCommand,
        updates: &mpsc::Sender<super::Update>,
    ) {
        match command {
            LibraryCommand::Open if !self.library.open => {
                self.library.open = true;
                self.refresh_library(updates);
            }
            LibraryCommand::Close => {
                self.library.open = false;
                self.reset_library("");
            }
            LibraryCommand::Refresh if self.library.open => self.refresh_library(updates),
            LibraryCommand::Select(id) if self.library.open => {
                if self
                    .library
                    .ready
                    .as_ref()
                    .is_some_and(|(ready, _)| ready == &id)
                {
                    return;
                }
                self.load_library_playlist(id, None, updates);
            }
            LibraryCommand::Queue {
                id,
                replace,
                play,
                start,
            } if self.library.open => {
                if self.import_task.is_some() {
                    self.library.edit(|s| {
                        s.status =
                            "Import de liens en cours; attendez sa fin ou videz la file".into()
                    });
                    return;
                }
                if self.session.is_none() || (play && self.player.is_none()) {
                    self.library.edit(|s| {
                        s.status = "Connectez Spotify pour charger cette playlist".into()
                    });
                    return;
                }
                let request = QueueRequest {
                    replace,
                    play,
                    start,
                    audio_generation: self.generation,
                };
                if self
                    .library
                    .ready
                    .as_ref()
                    .is_some_and(|(ready, _)| ready == &id)
                {
                    let rows = self
                        .library
                        .shared
                        .lock()
                        .map(|s| s.tracks.clone())
                        .unwrap_or_default();
                    self.apply_library_queue(&rows, request);
                } else {
                    self.load_library_playlist(id, Some(request), updates);
                }
            }
            _ => {}
        }
    }

    pub(super) fn refresh_library(&mut self, updates: &mpsc::Sender<super::Update>) {
        self.reset_library("Chargement de vos playlists Spotify...");
        let Some(session) = self.session.clone() else {
            self.library
                .edit(|s| s.status = "Connectez Spotify pour afficher vos playlists".into());
            return;
        };
        self.library.edit(|s| s.loading = true);
        let generation = self.library.generation;
        let updates = updates.clone();
        self.library.task = Some(tokio::spawn(async move {
            let result = crate::spotify_library::fetch(session).await;
            let _ = updates
                .send(super::Update::Library(Update::Catalogue {
                    generation,
                    result,
                }))
                .await;
        }));
    }

    fn load_library_playlist(
        &mut self,
        id: PlaylistId,
        request: Option<QueueRequest>,
        updates: &mpsc::Sender<super::Update>,
    ) {
        let allowed = self
            .library
            .shared
            .lock()
            .is_ok_and(|s| s.entries.iter().any(|e| e.id == id && e.available));
        if !allowed {
            self.library.edit(|s| {
                s.status = "Cette playlist est indisponible; actualisez la bibliotheque".into()
            });
            return;
        }
        let Some(session) = self.session.clone() else {
            self.library
                .edit(|s| s.status = "Connectez Spotify pour lire cette playlist".into());
            return;
        };
        let PlaylistId::Spotify(ref uri) = id;
        let uri = match parse_input(uri) {
            Ok(uri @ SpotifyUri::Playlist { .. }) => uri,
            _ => {
                self.library
                    .edit(|s| s.status = "Identifiant de playlist Spotify invalide".into());
                return;
            }
        };
        self.library.cancel();
        self.library.pending_queue = request;
        self.library.edit(|s| {
            s.selected_id = Some(id);
            s.tracks = Arc::default();
            s.loading = true;
            s.busy = request.is_some();
            s.status = "Chargement de la playlist...".into();
        });
        let generation = self.library.generation;
        let updates = updates.clone();
        self.library.task = Some(tokio::spawn(load_playlist(
            session, uri, generation, updates,
        )));
    }

    pub(super) fn library_update(&mut self, update: Update) {
        let generation = match &update {
            Update::Catalogue { generation, .. }
            | Update::Loaded { generation, .. }
            | Update::Metadata { generation, .. }
            | Update::Finished { generation } => *generation,
        };
        if !self.library.open || generation != self.library.generation {
            return;
        }
        match update {
            Update::Catalogue { result, .. } => {
                self.library.task = None;
                self.library.edit(|s| {
                    s.loading = false;
                    match result {
                        Ok(entries) => {
                            s.status = format!(
                                "{} playlist(s) Spotify. Selectionnez une playlist.",
                                entries.len()
                            );
                            s.entries = Arc::new(entries);
                        }
                        Err(message) => s.status = message,
                    }
                });
            }
            Update::Loaded { result, .. } => match result {
                Ok((rows, truncated)) => {
                    let rows = Arc::new(rows);
                    let selected = self
                        .library
                        .shared
                        .lock()
                        .ok()
                        .and_then(|s| s.selected_id.clone());
                    self.library.ready = selected.map(|id| (id, truncated));
                    self.library.edit(|s| {
                        s.loading = false;
                        s.busy = false;
                        s.tracks = rows.clone();
                        s.status = playlist_notice(s.tracks.len(), truncated);
                    });
                    if let Some(request) = self.library.pending_queue.take() {
                        self.apply_library_queue(&rows, request);
                    }
                }
                Err(message) => {
                    self.library.task = None;
                    self.library.pending_queue = None;
                    self.library.edit(|s| {
                        s.loading = false;
                        s.busy = false;
                        s.status = message;
                    });
                }
            },
            Update::Metadata { rows, .. } => {
                self.library.edit(|s| {
                    let tracks = Arc::make_mut(&mut s.tracks);
                    for row in &rows {
                        for existing in tracks.iter_mut().filter(|existing| existing.uri == row.uri)
                        {
                            *existing = row.clone();
                        }
                    }
                });
                // A user may queue the URI rows while names are still loading.
                // Update matching rows without touching playback or queue order.
                edit(&self.state, |s| {
                    for row in rows {
                        for existing in s
                            .queue
                            .iter_mut()
                            .filter(|existing| existing.uri == row.uri)
                        {
                            *existing = row.clone();
                        }
                    }
                });
            }
            Update::Finished { .. } => self.library.task = None,
        }
    }

    fn apply_library_queue(&mut self, rows: &[TrackRow], request: QueueRequest) {
        if request.audio_generation != self.generation {
            return;
        }
        let error = if self.import_task.is_some() {
            Some("Import de liens en cours; la file a ete conservee")
        } else if request.play && self.player.is_none() {
            Some("Connectez Spotify pour lire cette playlist; la file a ete conservee")
        } else if rows.is_empty() {
            Some("Aucun titre musical accessible; la file a ete conservee")
        } else if request.start >= rows.len() {
            Some("Ce titre ne figure plus dans la playlist; selectionnez-le a nouveau")
        } else {
            None
        };
        if let Some(message) = error {
            self.library.edit(|s| s.status = message.into());
            return;
        }
        let old_len = self
            .state
            .lock()
            .map(|s| s.queue.len())
            .unwrap_or(MAX_QUEUE);
        let capacity = if request.replace {
            MAX_QUEUE
        } else {
            MAX_QUEUE.saturating_sub(old_len)
        };
        let count = rows.len().min(capacity);
        if count == 0 || (request.play && request.start >= count) {
            self.library.edit(|s| {
                s.status = "File pleine : 500 titres maximum; la file a ete conservee".into()
            });
            return;
        }
        // All validation and network work has succeeded before stopping or
        // replacing anything. Incoming old import events cannot refill it.
        if request.replace {
            self.cancel_import();
            self.stop();
            self.current_uri = None;
        }
        edit(&self.state, |s| {
            if request.replace {
                s.queue.clear();
                s.current = None;
                s.duration_ms = 0;
            }
            s.queue.extend(rows.iter().take(count).cloned());
            s.status = format!("{count} titre(s) de la playlist ajoutes");
        });
        if request.replace {
            self.reset_order(None);
        } else {
            self.queue_appended();
        }
        self.library.edit(|s| {
            s.status = if count < rows.len() {
                format!("{count} titres ajoutes; file limitee a {MAX_QUEUE}")
            } else {
                format!("{count} titre(s) ajoutes a la file")
            }
        });
        if request.play {
            self.play(if request.replace {
                request.start
            } else {
                old_len + request.start
            });
        }
        self.sync_library_summary();
    }
}

fn playlist_notice(count: usize, truncated: bool) -> String {
    if truncated {
        format!("{count} titres; apercu limite a 500 ou tronque par Spotify")
    } else {
        format!("{count} titre(s). Ajouter a la file ou Lire.")
    }
}

async fn load_playlist(
    session: Session,
    uri: SpotifyUri,
    generation: u64,
    updates: mpsc::Sender<super::Update>,
) {
    let result = resolve_ids(&session, &uri, MAX_QUEUE).await;
    let (ids, truncated) = match result {
        Ok(resolved) => resolved,
        Err(message) => {
            let _ = updates
                .send(super::Update::Library(Update::Loaded {
                    generation,
                    result: Err(message),
                }))
                .await;
            return;
        }
    };
    let rows = ids
        .iter()
        .filter_map(|uri| uri.to_uri().ok())
        .map(|uri| TrackRow {
            uri,
            title: "Chargement du titre...".into(),
            ..TrackRow::default()
        })
        .collect();
    if updates
        .send(super::Update::Library(Update::Loaded {
            generation,
            result: Ok((rows, truncated)),
        }))
        .await
        .is_err()
    {
        return;
    }
    let mut pending: VecDeque<_> = ids.into();
    let mut requests = JoinSet::new();
    let mut batch = Vec::with_capacity(8);
    loop {
        while requests.len() < 2 {
            let Some(uri) = pending.pop_front() else {
                break;
            };
            let session = session.clone();
            requests.spawn(async move { track_metadata(&session, &uri).await });
        }
        match requests.join_next().await {
            Some(Ok(row)) => batch.push(row),
            Some(Err(_)) => {}
            None => break,
        }
        if batch.len() >= 8
            && updates
                .send(super::Update::Library(Update::Metadata {
                    generation,
                    rows: std::mem::take(&mut batch),
                }))
                .await
                .is_err()
        {
            return;
        }
    }
    if !batch.is_empty() {
        let _ = updates
            .send(super::Update::Library(Update::Metadata {
                generation,
                rows: batch,
            }))
            .await;
    }
    let _ = updates
        .send(super::Update::Library(Update::Finished { generation }))
        .await;
}

#[cfg(test)]
#[path = "library/tests.rs"]
mod tests;
