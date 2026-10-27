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
    catalogue_loaded: bool,
    search_generation: u64,
    search_task: Option<JoinHandle<()>>,
    write_generation: u64,
    write_task: Option<JoinHandle<()>>,
    write_refresh: Option<PlaylistId>,
    write_creating: bool,
    write_removing: bool,
    // Only protects creations acknowledged while a catalogue read was in flight.
    // A later explicit refresh remains authoritative.
    catalogue_created: Vec<PlaylistEntry>,
}

#[derive(Clone, Copy)]
struct QueueRequest {
    replace: bool,
    play: bool,
    start: usize,
    audio_generation: u64,
}

pub(super) enum Update {
    Removed {
        generation: u64,
        auth_generation: u64,
        id: PlaylistId,
        count: usize,
        result: Result<crate::spotify_playlist_write::Receipt, LocalizedText>,
    },
    PlaylistAccess {
        generation: u64,
        revision: Vec<u8>,
        owned: bool,
    },
    Creation {
        generation: u64,
        auth_generation: u64,
        name: String,
        step: CreationUpdate,
    },
    Written {
        generation: u64,
        auth_generation: u64,
        id: PlaylistId,
        count: usize,
        result: Result<crate::spotify_playlist_write::Receipt, LocalizedText>,
    },
    Search {
        generation: u64,
        update: SearchUpdate,
    },
    Catalogue {
        generation: u64,
        result: Result<Vec<PlaylistEntry>, LocalizedText>,
    },
    Loaded {
        generation: u64,
        result: Result<(Vec<TrackRow>, bool), LocalizedText>,
    },
    Metadata {
        generation: u64,
        rows: Vec<TrackRow>,
    },
    Finished {
        generation: u64,
    },
}

pub(super) enum CreationUpdate {
    Created(PlaylistId),
    Finished {
        id: Option<PlaylistId>,
        count: usize,
        result: Result<crate::spotify_playlist_write::Receipt, LocalizedText>,
    },
}

pub(super) enum SearchUpdate {
    Loaded(Result<(Vec<TrackRow>, bool), LocalizedText>),
    Metadata(Vec<TrackRow>),
    Finished { timed_out: bool },
}

impl State {
    fn is_loading(&self, id: &PlaylistId) -> bool {
        self.task.is_some()
            && self
                .shared
                .lock()
                .is_ok_and(|s| s.loading && s.selected_id.as_ref() == Some(id))
    }

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

    fn cancel_search(&mut self) {
        self.replace_search("", false, LocalizedText::default());
    }

    fn replace_search(&mut self, query: &str, loading: bool, status: LocalizedText) {
        self.search_generation = self.search_generation.wrapping_add(1);
        if let Some(task) = self.search_task.take() {
            task.abort();
        }
        self.edit(|s| {
            s.search = crate::model::SearchSnapshot {
                revision: self.search_generation,
                query: query.into(),
                loading,
                status,
                ..Default::default()
            };
        });
    }
}

impl Worker {
    /// Restore the source playlist once its catalogue exists. A deliberate
    /// selection cancels this pending startup action; browsing never changes
    /// the remembered origin of the actual playback queue.
    pub(super) fn restore_library_preview(&mut self, updates: &mpsc::Sender<super::Update>) {
        if !self.library.open || !self.library.catalogue_loaded || self.session.is_none() {
            return;
        }
        if let Some(id) = self.library.write_refresh.take()
            && self
                .library
                .shared
                .lock()
                .is_ok_and(|s| s.selected_id.as_ref() == Some(&id))
        {
            self.load_library_playlist(id, None, updates);
            return;
        }
        if let Some(id) = self.take_restored_playlist() {
            self.load_library_playlist(id, None, updates);
        }
    }

    fn take_restored_playlist(&mut self) -> Option<PlaylistId> {
        let id = self.restore_library_selection.take()?;
        self.library
            .shared
            .lock()
            .ok()
            .filter(|s| {
                s.entries
                    .iter()
                    .any(|entry| entry.id == id && entry.available)
            })
            .map(|_| id)
    }

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

    pub(super) fn reset_library(&mut self, message: &'static str) {
        self.finish_search_metadata();
        self.library.cancel_search();
        if !message.is_empty() {
            self.library.write_generation = self.library.write_generation.wrapping_add(1);
            if let Some(task) = self.library.write_task.take() {
                task.abort();
                self.library.edit(|s| {
                    s.playlist_write_busy = false;
                    s.playlist_write_status = if self.library.write_creating {
                        "Creation ou ajout non confirme; verifiez vos playlists avant de reessayer"
                    } else if self.library.write_removing {
                        "Retrait non confirme; verifiez la playlist avant de reessayer"
                    } else {
                        "Ajout non confirme; verifiez la playlist avant de reessayer"
                    }
                    .into();
                });
            } else {
                // A finished result belongs to the previous authenticated
                // session; do not let it hide the reconnect notice.
                self.library.edit(|s| {
                    s.playlist_write_busy = false;
                    s.playlist_write_status = LocalizedText::default();
                });
            }
            self.library.write_refresh = None;
            self.library.write_creating = false;
            self.library.write_removing = false;
            self.library.edit(|s| {
                s.playlist_remove_revision = s.playlist_remove_revision.wrapping_add(1);
            });
        }
        // Hiding the library does not cancel an already submitted mutation.
        self.reset_playlist(message);
    }

    fn reset_playlist(&mut self, message: &'static str) {
        self.library.cancel();
        self.library.catalogue_created.clear();
        self.library.catalogue_loaded = false;
        self.library.edit(|s| {
            s.entries = Arc::default();
            s.tracks = Arc::default();
            s.selected_id = None;
            s.selected_owned = false;
            s.selected_playlist_revision = Arc::default();
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
        if matches!(
            &command,
            LibraryCommand::Search(_)
                | LibraryCommand::Select(_)
                | LibraryCommand::QueueSearch { .. }
                | LibraryCommand::QueueTracks { .. }
                | LibraryCommand::Queue { .. }
                | LibraryCommand::Refresh
        ) {
            // Preserve progress while a write is in flight. Once finished,
            // the next explicit action owns the status bar again, including
            // errors from searching or attempting to fill a full queue.
            if let Ok(mut snapshot) = self.library.shared.lock()
                && !snapshot.playlist_write_busy
                && !snapshot.playlist_write_status.is_empty()
            {
                snapshot.playlist_write_status = LocalizedText::default();
                snapshot.revision = snapshot.revision.wrapping_add(1);
            }
        }
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
            LibraryCommand::Search(query) if self.library.open => {
                self.search_library(query, updates)
            }
            LibraryCommand::QueueTracks {
                tracks,
                origin,
                expected_queue,
                replace,
                play,
                insert_at,
            } => {
                self.queue_selected_tracks(
                    tracks,
                    replace,
                    play,
                    insert_at,
                    origin,
                    expected_queue,
                );
            }
            LibraryCommand::AddToPlaylist { id, tracks } if self.library.open => {
                self.add_selection_to_playlist(id, tracks, updates);
            }
            LibraryCommand::CreatePlaylist { name, tracks } if self.library.open => {
                self.create_playlist_from_selection(name, tracks, updates);
            }
            LibraryCommand::RemoveFromPlaylist {
                id,
                expected_tracks,
                expected_revision,
                indices,
            } if self.library.open => {
                self.remove_playlist_selection(
                    id,
                    expected_tracks,
                    expected_revision,
                    indices,
                    updates,
                );
            }
            LibraryCommand::QueueSearch {
                revision,
                index,
                play,
            } if self.library.open => {
                self.queue_search_result(revision, index, play);
            }
            LibraryCommand::Select(id) if self.library.open => {
                self.restore_library_selection = None;
                if self
                    .library
                    .ready
                    .as_ref()
                    .is_some_and(|(ready, _)| ready == &id)
                    || self.library.is_loading(&id)
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
                self.restore_library_selection = None;
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
                } else if self.library.is_loading(&id) {
                    // A playlist double-click follows its first Select. Keep
                    // that read (even if its response is already queued) and
                    // attach the latest queue/play intent to the same result.
                    self.library.pending_queue = Some(request);
                    self.library.edit(|s| s.busy = true);
                } else {
                    self.load_library_playlist(id, Some(request), updates);
                }
            }
            _ => {}
        }
    }

    pub(super) fn refresh_library(&mut self, updates: &mpsc::Sender<super::Update>) {
        self.reset_playlist("Chargement de vos playlists Spotify...");
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
            s.selected_owned = false;
            s.selected_playlist_revision = Arc::default();
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
        if let Update::Removed {
            generation,
            auth_generation,
            id,
            count,
            result,
        } = update
        {
            if generation == self.library.write_generation
                && auth_generation == self.auth_generation
            {
                self.library.write_task = None;
                self.library.write_removing = false;
                if result.is_ok() {
                    self.library.write_refresh = Some(id.clone());
                }
                self.library.edit(|s| {
                    if s.selected_id.as_ref() == Some(&id) {
                        // Do not let a second Delete reuse the old occurrence indices.
                        // A fresh playlist read restores editability after success;
                        // an uncertain failure requires an explicit refresh.
                        s.selected_owned = false;
                    }
                    if let Ok(receipt) = &result
                        && let Some(entry) = Arc::make_mut(&mut s.entries)
                            .iter_mut()
                            .find(|entry| entry.id == id)
                    {
                        entry.track_count = receipt.total;
                        entry.duration_ms = None;
                    }
                    s.playlist_write_busy = false;
                    s.playlist_remove_revision = s.playlist_remove_revision.wrapping_add(1);
                    s.playlist_write_status = match result {
                        Ok(_) => playlist_remove_success(count),
                        Err(message) => message,
                    };
                });
            }
            return;
        }
        if let Update::Creation {
            generation,
            auth_generation,
            name,
            step,
        } = update
        {
            if generation == self.library.write_generation
                && auth_generation == self.auth_generation
            {
                self.creation_update(name, step);
            }
            return;
        }
        if let Update::Written {
            generation,
            auth_generation,
            id,
            count,
            result,
        } = update
        {
            if generation == self.library.write_generation
                && auth_generation == self.auth_generation
            {
                self.library.write_task = None;
                if let Ok(receipt) = &result {
                    self.library.edit(|s| {
                        if let Some(entry) = Arc::make_mut(&mut s.entries)
                            .iter_mut()
                            .find(|entry| entry.id == id)
                        {
                            entry.track_count = receipt.total;
                            entry.duration_ms = None;
                        }
                    });
                    // Refresh only the destination still being browsed.
                    self.library.write_refresh = Some(id);
                }
                self.library.edit(|s| {
                    s.playlist_write_busy = false;
                    s.playlist_write_status = match result {
                        Ok(_) => playlist_write_success(count),
                        Err(message) => message,
                    };
                });
            }
            return;
        }
        if let Update::Search { generation, update } = update {
            if self.library.open && generation == self.library.search_generation {
                self.search_update(update);
            }
            return;
        }
        let generation = match &update {
            Update::Catalogue { generation, .. }
            | Update::PlaylistAccess { generation, .. }
            | Update::Loaded { generation, .. }
            | Update::Metadata { generation, .. }
            | Update::Finished { generation } => *generation,
            Update::Search { .. }
            | Update::Written { .. }
            | Update::Creation { .. }
            | Update::Removed { .. } => {
                unreachable!()
            }
        };
        if !self.library.open || generation != self.library.generation {
            return;
        }
        match update {
            Update::Search { .. }
            | Update::Written { .. }
            | Update::Creation { .. }
            | Update::Removed { .. } => {
                unreachable!()
            }
            Update::PlaylistAccess {
                revision, owned, ..
            } => self.library.edit(|s| {
                s.selected_owned = owned;
                s.selected_playlist_revision = revision.into();
            }),
            Update::Catalogue { result, .. } => {
                self.library.task = None;
                self.library.catalogue_loaded = result.is_ok();
                let created = std::mem::take(&mut self.library.catalogue_created);
                self.library.edit(|s| {
                    s.loading = false;
                    match result {
                        Ok(mut entries) => {
                            for entry in created {
                                if !entries.iter().any(|known| known.id == entry.id) {
                                    entries.push(entry);
                                }
                            }
                            s.status = LocalizedText::with_languages(
                                format!(
                                    "{} playlist(s) Spotify. Selectionnez une playlist.",
                                    entries.len()
                                ),
                                format!(
                                    "{} Spotify playlist(s). Select a playlist.",
                                    entries.len()
                                ),
                                format!("{} Spotify-Playlists. Playlist auswählen.", entries.len()),
                                format!(
                                    "{} playlists de Spotify. Selecciona una playlist.",
                                    entries.len()
                                ),
                                format!(
                                    "{} playlist Spotify. Seleziona una playlist.",
                                    entries.len()
                                ),
                                format!(
                                    "{} playlists do Spotify. Seleciona uma playlist.",
                                    entries.len()
                                ),
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
                        s.selected_owned = false;
                        s.selected_playlist_revision = Arc::default();
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
                            merge_search_metadata(existing, &row);
                        }
                    }
                });
            }
            Update::Finished { .. } => self.library.task = None,
        }
    }

    fn remove_playlist_selection(
        &mut self,
        id: PlaylistId,
        expected_tracks: Arc<Vec<TrackRow>>,
        expected_revision: Arc<[u8]>,
        indices: Vec<usize>,
        updates: &mpsc::Sender<super::Update>,
    ) {
        let reject = if self.library.write_task.is_some() {
            Some("Modification de playlist en cours; attendez sa confirmation")
        } else if self.session.is_none() {
            Some("Connectez Spotify pour modifier une playlist")
        } else if indices.is_empty()
            || indices.len() > crate::spotify_playlist_write::MAX_SELECTION
            || indices.windows(2).any(|pair| pair[0] >= pair[1])
            || indices
                .last()
                .is_some_and(|index| *index >= expected_tracks.len())
            || expected_revision.is_empty()
        {
            Some("Selection invalide; aucun titre retire")
        } else if !self.library.shared.lock().is_ok_and(|s| {
            s.selected_id.as_ref() == Some(&id)
                && s.selected_owned
                && !s.loading
                && !s.busy
                && s.selected_playlist_revision == expected_revision
                && s.tracks.len() == expected_tracks.len()
                && s.tracks
                    .iter()
                    .zip(expected_tracks.iter())
                    .all(|(a, b)| a.uri == b.uri)
                && s.entries
                    .iter()
                    .any(|entry| entry.id == id && entry.available)
        }) {
            Some("Impossible de verifier la playlist; aucun titre retire")
        } else {
            None
        };
        if let Some(message) = reject {
            self.library.edit(|s| {
                s.playlist_remove_revision = s.playlist_remove_revision.wrapping_add(1);
                s.playlist_write_status = message.into();
            });
            return;
        }
        let session = self.session.clone().unwrap();
        self.library.write_generation = self.library.write_generation.wrapping_add(1);
        self.library.write_creating = false;
        self.library.write_removing = true;
        let generation = self.library.write_generation;
        let auth_generation = self.auth_generation;
        let epoch = self.auth_epoch.clone();
        let updates = updates.clone();
        let count = indices.len();
        self.library.edit(|s| {
            s.playlist_write_busy = true;
            s.playlist_write_status = "Retrait de la playlist Spotify en cours...".into();
        });
        self.library.write_task = Some(tokio::spawn(async move {
            let result = crate::spotify_playlist_write::remove(
                session,
                &id,
                &expected_tracks,
                &expected_revision,
                &indices,
                epoch,
                auth_generation,
            )
            .await;
            let _ = updates
                .send(super::Update::Library(Update::Removed {
                    generation,
                    auth_generation,
                    id,
                    count,
                    result,
                }))
                .await;
        }));
    }

    fn add_selection_to_playlist(
        &mut self,
        id: PlaylistId,
        tracks: Vec<TrackRow>,
        updates: &mpsc::Sender<super::Update>,
    ) {
        let reject = if self.library.write_task.is_some() {
            Some("Ajout en cours; attendez sa confirmation")
        } else if tracks.len() > crate::spotify_playlist_write::MAX_SELECTION {
            Some("Selection limitee a 500 titres")
        } else if !crate::spotify_playlist_write::valid_tracks(&tracks) {
            Some("Selection invalide; aucun titre ajoute")
        } else if self.session.is_none() {
            Some("Connectez Spotify pour modifier une playlist")
        } else if !self.library.shared.lock().is_ok_and(|s| {
            s.entries
                .iter()
                .any(|entry| entry.id == id && entry.available)
        }) {
            Some("La destination n'est plus disponible; actualisez les playlists")
        } else {
            None
        };
        if let Some(message) = reject {
            self.library
                .edit(|s| s.playlist_write_status = message.into());
            return;
        }
        let session = self.session.clone().unwrap();
        self.library.write_generation = self.library.write_generation.wrapping_add(1);
        let generation = self.library.write_generation;
        let auth_generation = self.auth_generation;
        let epoch = self.auth_epoch.clone();
        let count = tracks.len();
        let updates = updates.clone();
        self.library.write_creating = false;
        self.library.write_removing = false;
        self.library.edit(|s| {
            s.playlist_write_busy = true;
            s.playlist_write_status = "Ajout a la playlist Spotify en cours...".into();
        });
        self.library.write_task = Some(tokio::spawn(async move {
            let result = crate::spotify_playlist_write::append(
                session,
                &id,
                &tracks,
                epoch,
                auth_generation,
            )
            .await;
            let _ = updates
                .send(super::Update::Library(Update::Written {
                    generation,
                    auth_generation,
                    id,
                    count,
                    result,
                }))
                .await;
        }));
    }

    fn create_playlist_from_selection(
        &mut self,
        name: String,
        tracks: Vec<TrackRow>,
        updates: &mpsc::Sender<super::Update>,
    ) {
        let name = crate::model::normalized_playlist_name(&name);
        let reject = if self.library.write_task.is_some() {
            Some("Ajout en cours; attendez sa confirmation")
        } else if name.is_none() {
            Some("Nom de playlist invalide (1 a 100 caracteres)")
        } else if tracks.len() > crate::spotify_playlist_write::MAX_SELECTION {
            Some("Selection limitee a 500 titres")
        } else if !crate::spotify_playlist_write::valid_tracks(&tracks) {
            Some("Selection invalide; aucun titre ajoute")
        } else if self.session.is_none() {
            Some("Connectez Spotify pour modifier une playlist")
        } else {
            None
        };
        if let Some(message) = reject {
            self.library
                .edit(|s| s.playlist_write_status = message.into());
            return;
        }
        let name = name.unwrap().to_owned();
        let session = self.session.clone().unwrap();
        self.library.write_generation = self.library.write_generation.wrapping_add(1);
        self.library.write_creating = true;
        self.library.write_removing = false;
        let generation = self.library.write_generation;
        let auth_generation = self.auth_generation;
        let epoch = self.auth_epoch.clone();
        let updates = updates.clone();
        self.library.edit(|s| {
            s.playlist_write_busy = true;
            s.playlist_write_status = "Creation de la playlist Spotify en cours...".into();
        });
        self.library.write_task = Some(tokio::spawn(async move {
            let created = crate::spotify_playlist_write::create(
                session.clone(),
                &name,
                epoch.clone(),
                auth_generation,
            )
            .await;
            let (id, result) = match created {
                Ok(created) => {
                    // Publish the identity before attempting a second mutation.
                    // A refused/uncertain append leaves this destination usable.
                    if updates
                        .send(super::Update::Library(Update::Creation {
                            generation,
                            auth_generation,
                            name: name.clone(),
                            step: CreationUpdate::Created(created.id.clone()),
                        }))
                        .await
                        .is_err()
                    {
                        return;
                    }
                    let result = match crate::spotify_playlist_write::link_created_playlist(
                        session.clone(),
                        &created.id,
                        epoch.clone(),
                        auth_generation,
                    )
                    .await
                    {
                        Ok(()) => {
                            crate::spotify_playlist_write::append(
                                session,
                                &created.id,
                                &tracks,
                                epoch,
                                auth_generation,
                            )
                            .await
                        }
                        Err(message) => Err(message),
                    };
                    (Some(created.id), result)
                }
                Err(message) => (None, Err(message)),
            };
            let _ = updates
                .send(super::Update::Library(Update::Creation {
                    generation,
                    auth_generation,
                    name,
                    step: CreationUpdate::Finished {
                        id,
                        count: tracks.len(),
                        result,
                    },
                }))
                .await;
        }));
    }

    fn creation_update(&mut self, name: String, step: CreationUpdate) {
        match step {
            CreationUpdate::Created(id) => {
                let entry = PlaylistEntry {
                    id,
                    name: name.clone(),
                    track_count: Some(0),
                    duration_ms: None,
                    folder: String::new(),
                    available: true,
                };
                if self
                    .library
                    .shared
                    .lock()
                    .is_ok_and(|s| s.loading && s.selected_id.is_none())
                {
                    self.library.catalogue_created.push(entry.clone());
                }
                self.library.edit(|s| {
                    if !s.entries.iter().any(|known| known.id == entry.id) {
                        Arc::make_mut(&mut s.entries).push(entry);
                    }
                    s.playlist_write_status = playlist_created_adding(&name);
                });
            }
            CreationUpdate::Finished { id, count, result } => {
                self.library.write_task = None;
                self.library.write_creating = false;
                if let Some(id) = &id {
                    let total = result.as_ref().ok().and_then(|receipt| receipt.total);
                    self.library.edit(|s| {
                        if let Some(entry) = Arc::make_mut(&mut s.entries)
                            .iter_mut()
                            .find(|entry| &entry.id == id)
                        {
                            // An uncertain append must not display a definite zero.
                            entry.track_count = total;
                        }
                    });
                    for entry in self
                        .library
                        .catalogue_created
                        .iter_mut()
                        .filter(|entry| &entry.id == id)
                    {
                        entry.track_count = total;
                    }
                    self.library.write_refresh = Some(id.clone());
                }
                self.library.edit(|s| {
                    s.playlist_write_busy = false;
                    s.playlist_write_status = match (id, result) {
                        (Some(_), Ok(_)) => playlist_created_success(&name, count),
                        (Some(_), Err(error)) => playlist_created_error(&name, &error),
                        (None, Err(error)) => error,
                        (None, Ok(_)) => unreachable!(),
                    };
                });
            }
        }
    }

    fn queue_selected_tracks(
        &mut self,
        rows: Vec<TrackRow>,
        replace: bool,
        play: bool,
        insert_at: Option<usize>,
        origin: Option<PlaylistId>,
        expected_queue: Option<Vec<String>>,
    ) {
        let before = self.snapshot();
        let index = if replace {
            0
        } else {
            insert_at.unwrap_or(before.queue.len())
        };
        let error = if rows.len() > crate::spotify_playlist_write::MAX_SELECTION {
            Some("Selection limitee a 500 titres")
        } else if !crate::spotify_playlist_write::valid_tracks(&rows) {
            Some("Selection invalide; aucun titre ajoute")
        } else if self.import_task.is_some() {
            Some("Import de liens en cours; la file a ete conservee")
        } else if play && self.player.is_none() {
            Some("Connectez Spotify pour lire cette playlist; la file a ete conservee")
        } else if (replace && insert_at.is_some())
            || index > before.queue.len()
            || expected_queue.as_ref().is_some_and(|expected| {
                expected.len() != before.queue.len()
                    || expected
                        .iter()
                        .zip(&before.queue)
                        .any(|(uri, row)| uri != &row.uri)
            })
        {
            Some("Position d'insertion invalide; la file a ete conservee")
        } else if !replace && before.queue.len().saturating_add(rows.len()) > MAX_QUEUE {
            Some("File pleine : 500 titres maximum; la file a ete conservee")
        } else {
            None
        };
        if let Some(message) = error {
            self.library.edit(|s| {
                s.status = message.into();
                s.search.status = message.into();
            });
            return;
        }
        // One transaction independent of current catalogue/search revisions.
        self.cancel_library_queue();
        self.merge_queue_origin(origin, replace);
        let count = rows.len();
        if replace {
            self.cancel_import();
            self.stop();
            self.current_uri = None;
        }
        edit(&self.state, |s| {
            if replace {
                s.queue = rows;
                s.current = None;
                s.duration_ms = 0;
            } else {
                s.queue.splice(index..index, rows);
                if let Some(current) = &mut s.current
                    && *current >= index
                {
                    *current += count;
                }
            }
        });
        if replace {
            self.reset_order(None);
        } else if before.shuffle {
            self.order.insert(index, count, &mut rand::rng());
        }
        let notice = LocalizedText::with_languages(
            format!("{count} titre(s) ajoutes a la file"),
            format!("{count} track(s) added to the queue"),
            format!("{count} Titel zur Warteschlange hinzugefügt"),
            format!("{count} canciones añadidas a la cola"),
            format!("{count} brani aggiunti alla coda"),
            format!("{count} faixas adicionadas à fila"),
        );
        self.library.edit(|s| {
            s.status = notice.clone();
            s.search.status = notice;
        });
        if play {
            self.play(index);
        }
        self.sync_library_summary();
    }

    fn apply_library_queue(&mut self, rows: &[TrackRow], request: QueueRequest) {
        let origin = self.library.ready.as_ref().map(|(id, _)| id.clone());
        self.apply_tracks_queue(rows, request, origin, false);
    }

    fn queue_status(&self, message: LocalizedText, search: bool) {
        self.library.edit(|s| {
            if search {
                s.search.status = message;
            } else {
                s.status = message;
            }
        });
    }

    fn apply_tracks_queue(
        &mut self,
        rows: &[TrackRow],
        request: QueueRequest,
        origin: Option<PlaylistId>,
        search: bool,
    ) {
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
            self.queue_status(message.into(), search);
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
            self.queue_status(
                "File pleine : 500 titres maximum; la file a ete conservee".into(),
                search,
            );
            return;
        }
        self.merge_queue_origin(origin, request.replace);
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
            s.status = if search {
                "Titre ajoute depuis la recherche".into()
            } else {
                LocalizedText::with_languages(
                    format!("{count} titre(s) de la playlist ajoutes"),
                    format!("{count} playlist track(s) added"),
                    format!("{count} Playlist-Titel hinzugefügt"),
                    format!("{count} canciones de la playlist añadidas"),
                    format!("{count} brani della playlist aggiunti"),
                    format!("{count} faixas da playlist adicionadas"),
                )
            };
        });
        if request.replace {
            self.reset_order(None);
        } else {
            self.queue_appended();
        }
        self.queue_status(
            if count < rows.len() {
                LocalizedText::with_languages(
                    format!("{count} titres ajoutes; file limitee a {MAX_QUEUE}"),
                    format!("{count} tracks added; queue limited to {MAX_QUEUE}"),
                    format!("{count} Titel hinzugefügt; Warteschlange auf {MAX_QUEUE} begrenzt"),
                    format!("{count} canciones añadidas; cola limitada a {MAX_QUEUE}"),
                    format!("{count} brani aggiunti; coda limitata a {MAX_QUEUE}"),
                    format!("{count} faixas adicionadas; fila limitada a {MAX_QUEUE}"),
                )
            } else {
                LocalizedText::with_languages(
                    format!("{count} titre(s) ajoutes a la file"),
                    format!("{count} track(s) added to the queue"),
                    format!("{count} Titel zur Warteschlange hinzugefügt"),
                    format!("{count} canciones añadidas a la cola"),
                    format!("{count} brani aggiunti alla coda"),
                    format!("{count} faixas adicionadas à fila"),
                )
            },
            search,
        );
        if request.play {
            self.play(if request.replace {
                request.start
            } else {
                old_len + request.start
            });
        }
        self.sync_library_summary();
    }

    fn search_library(&mut self, query: String, updates: &mpsc::Sender<super::Update>) {
        let query = crate::spotify_search::normalize_query(&query);
        // Repeated Enter while this exact request is in flight reuses its job.
        if self.library.search_task.is_some()
            && query.as_ref().is_ok_and(|query| {
                !query.is_empty()
                    && self.library.shared.lock().is_ok_and(|s| {
                        s.search.loading && s.search.query.as_ref() == query.as_str()
                    })
            })
        {
            return;
        }
        self.finish_search_metadata();
        let query = match query {
            Ok(query) => query,
            Err(message) => {
                self.library.replace_search("", false, message);
                return;
            }
        };
        if query.is_empty() {
            self.library.cancel_search();
            return;
        }
        let Some(session) = self.session.clone() else {
            self.library.replace_search(
                &query,
                false,
                "Connectez Spotify pour rechercher des titres".into(),
            );
            return;
        };
        // Publish query + revision + loading together so UI acknowledgements
        // cannot observe the new request with a temporarily empty query.
        self.library
            .replace_search(&query, true, "Recherche Spotify en cours...".into());
        let generation = self.library.search_generation;
        let updates = updates.clone();
        self.library.search_task = Some(tokio::spawn(search_tracks(
            session, query, generation, updates,
        )));
    }

    fn search_update(&mut self, update: SearchUpdate) {
        match update {
            SearchUpdate::Loaded(result) => {
                self.library.edit(|s| {
                    s.search.loading = false;
                    match result {
                        Ok((rows, truncated)) => {
                            s.search.status = search_notice(rows.len(), truncated);
                            s.search.tracks = Arc::new(rows);
                        }
                        Err(message) => s.search.status = message,
                    }
                });
            }
            SearchUpdate::Metadata(rows) => {
                self.library.edit(|s| {
                    let tracks = Arc::make_mut(&mut s.search.tracks);
                    for row in &rows {
                        if let Some(existing) =
                            tracks.iter_mut().find(|existing| existing.uri == row.uri)
                        {
                            merge_search_metadata(existing, row);
                        }
                    }
                });
                // A result can be queued before its name is resolved.
                edit(&self.state, |s| {
                    for row in rows {
                        for existing in s
                            .queue
                            .iter_mut()
                            .filter(|existing| existing.uri == row.uri)
                        {
                            merge_search_metadata(existing, &row);
                        }
                    }
                });
            }
            SearchUpdate::Finished { timed_out } => {
                self.library.search_task = None;
                if timed_out {
                    self.finish_search_metadata();
                    self.library.edit(|s| {
                        s.search.loading = false;
                        s.search.status =
                            "Recherche incomplete; relancez la recherche pour reessayer".into();
                        for row in Arc::make_mut(&mut s.search.tracks) {
                            if row.title_kind == TrackTitleKind::Loading {
                                row.title_kind = TrackTitleKind::Unavailable;
                            }
                        }
                    });
                }
            }
        }
    }

    fn finish_search_metadata(&self) {
        let unfinished = self
            .library
            .shared
            .lock()
            .map(|s| s.search.tracks.clone())
            .unwrap_or_default();
        if unfinished.is_empty() {
            return;
        }
        edit(&self.state, |s| {
            for row in &mut s.queue {
                if row.title_kind == TrackTitleKind::Loading
                    && unfinished.iter().any(|result| result.uri == row.uri)
                {
                    row.title_kind = TrackTitleKind::Unavailable;
                }
            }
        });
    }

    fn queue_search_result(&mut self, revision: u64, index: usize, play: bool) {
        let row = self.library.shared.lock().ok().and_then(|s| {
            (s.search.revision == revision && !s.search.loading)
                .then(|| s.search.tracks.get(index).cloned())
                .flatten()
        });
        let Some(row) = row else {
            self.queue_status(
                "Ce resultat a change; selectionnez-le a nouveau".into(),
                true,
            );
            return;
        };
        if self.session.is_none() || (play && self.player.is_none()) {
            self.queue_status("Connectez Spotify pour lire ce titre".into(), true);
            return;
        }
        self.cancel_library_queue();
        // Search is not a playlist: only the chosen result enters the queue.
        // Reuse the playlist transaction's validation, capacity, shuffle and
        // playback rules, with no playlist origin attached to these results.
        self.apply_tracks_queue(
            &[row],
            QueueRequest {
                replace: play,
                play,
                start: 0,
                audio_generation: self.generation,
            },
            None,
            true,
        );
    }
}

fn merge_search_metadata(existing: &mut TrackRow, row: &TrackRow) {
    // A failed supplementary read must not erase names already present in the
    // search context (or known metadata on a matching playback queue entry).
    if row.title_kind == TrackTitleKind::Metadata || existing.title_kind != TrackTitleKind::Metadata
    {
        *existing = row.clone();
    }
}

fn search_notice(count: usize, truncated: bool) -> LocalizedText {
    if count == 0 {
        return if truncated {
            "Aucun titre charge; essayez une recherche plus precise".into()
        } else {
            "Aucun titre trouve. Essayez un autre nom ou artiste.".into()
        };
    }
    LocalizedText::with_languages(
        format!(
            "{count} resultat(s){}. Double-clic : lire.",
            if truncated { " (liste limitee)" } else { "" }
        ),
        format!(
            "{count} result(s){}. Double-click to play.",
            if truncated { " (limited list)" } else { "" }
        ),
        format!(
            "{count} Ergebnisse{}. Doppelklick zum Abspielen.",
            if truncated { " (begrenzte Liste)" } else { "" }
        ),
        format!(
            "{count} resultados{}. Doble clic para reproducir.",
            if truncated { " (lista limitada)" } else { "" }
        ),
        format!(
            "{count} risultati{}. Doppio clic per riprodurre.",
            if truncated { " (elenco limitato)" } else { "" }
        ),
        format!(
            "{count} resultados{}. Duplo clique para reproduzir.",
            if truncated { " (lista limitada)" } else { "" }
        ),
    )
}

async fn search_tracks(
    session: Session,
    query: String,
    generation: u64,
    updates: mpsc::Sender<super::Update>,
) {
    let send = |update| {
        updates.send(super::Update::Library(Update::Search {
            generation,
            update,
        }))
    };
    let result = tokio::time::timeout(Duration::from_secs(60), async {
        // Coalesce rapid explicit submissions before any HTTP work begins.
        tokio::time::sleep(Duration::from_millis(200)).await;
        let (rows, truncated) = match crate::spotify_search::fetch(&session, &query).await {
            Ok(result) => result,
            Err(message) => {
                let _ = send(SearchUpdate::Loaded(Err(message))).await;
                return;
            }
        };
        let mut pending: VecDeque<_> = rows
            .iter()
            .filter(|row| {
                row.title_kind != TrackTitleKind::Metadata
                    || row.artist.is_empty()
                    || row.duration_ms == 0
            })
            .filter_map(|row| parse_input(&row.uri).ok())
            .collect();
        if send(SearchUpdate::Loaded(Ok((rows, truncated))))
            .await
            .is_err()
        {
            return;
        }
        let mut requests = JoinSet::new();
        while !pending.is_empty() || !requests.is_empty() {
            while requests.len() < 2 {
                let Some(uri) = pending.pop_front() else {
                    break;
                };
                let session = session.clone();
                requests.spawn(async move {
                    let mut row = track_metadata(&session, &uri).await;
                    row.title = crate::spotify_search::display_text(&row.title);
                    row.artist = crate::spotify_search::display_text(&row.artist);
                    row
                });
            }
            if let Some(Ok(row)) = requests.join_next().await
                && send(SearchUpdate::Metadata(vec![row])).await.is_err()
            {
                return;
            }
        }
    })
    .await;
    let _ = send(SearchUpdate::Finished {
        timed_out: result.is_err(),
    })
    .await;
}

fn playlist_notice(count: usize, truncated: bool) -> LocalizedText {
    if truncated {
        LocalizedText::with_languages(
            format!("{count} titres; apercu limite a 500 ou tronque par Spotify"),
            format!("{count} tracks; preview limited to 500 or truncated by Spotify"),
            format!("{count} Titel; Vorschau auf 500 begrenzt oder von Spotify gekürzt"),
            format!("{count} canciones; vista limitada a 500 o recortada por Spotify"),
            format!("{count} brani; anteprima limitata a 500 o troncata da Spotify"),
            format!("{count} faixas; prévia limitada a 500 ou truncada pelo Spotify"),
        )
    } else {
        LocalizedText::with_languages(
            format!("{count} titre(s). Ajouter a la file ou Lire."),
            format!("{count} track(s). Add to queue or Play."),
            format!("{count} Titel. Zur Warteschlange hinzufügen oder abspielen."),
            format!("{count} canciones. Añadir a la cola o reproducir."),
            format!("{count} brani. Aggiungi alla coda o riproduci."),
            format!("{count} faixas. Adicionar à fila ou reproduzir."),
        )
    }
}

async fn load_playlist(
    session: Session,
    uri: SpotifyUri,
    generation: u64,
    updates: mpsc::Sender<super::Update>,
) {
    // Reuse the playlist response already needed for the preview. Its owner
    // and opaque revision are retained without another request or cache.
    let result = tokio::time::timeout(NETWORK_TIMEOUT, Playlist::get(&session, &uri))
        .await
        .map_err(|_| LocalizedText::from("Spotify ne repond pas; reessayez"))
        .and_then(|result| {
            result.map_err(|_| {
                LocalizedText::from("Impossible de lire ce lien Spotify; verifiez son acces")
            })
        });
    let (ids, truncated, revision, owned) = match result {
        Ok(playlist) => playlist_preview(playlist, &session.username()),
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
    if updates
        .send(super::Update::Library(Update::PlaylistAccess {
            generation,
            revision,
            owned,
        }))
        .await
        .is_err()
    {
        return;
    }
    let rows = ids
        .iter()
        .filter_map(|uri| uri.to_uri().ok())
        .map(|uri| TrackRow {
            uri,
            title_kind: TrackTitleKind::Loading,
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

fn playlist_preview(playlist: Playlist, username: &str) -> (Vec<SpotifyUri>, bool, Vec<u8>, bool) {
    let truncated = playlist.contents.is_truncated || playlist.length as usize > MAX_QUEUE;
    let tracks: Vec<_> = playlist
        .tracks()
        .filter(|uri| matches!(uri, SpotifyUri::Track { .. }))
        .take(MAX_QUEUE)
        .cloned()
        .collect();
    let owned = !username.is_empty()
        && matches!(&playlist.id, SpotifyUri::Playlist { user: Some(owner), .. } if owner == username)
        && !truncated
        && !playlist.has_multiple_heads
        && !playlist.attributes.is_deleted_by_owner
        && playlist.contents.position == 0
        && usize::try_from(playlist.length).ok() == Some(tracks.len())
        && playlist.contents.items.len() == tracks.len()
        && !playlist.revision.is_empty();
    (tracks, truncated, playlist.revision, owned)
}

#[cfg(test)]
#[path = "library/tests.rs"]
mod tests;

fn playlist_write_success(count: usize) -> LocalizedText {
    LocalizedText::with_languages(
        format!("{count} titre(s) ajoutes a la playlist Spotify"),
        format!("{count} track(s) added to the Spotify playlist"),
        format!("{count} Titel zur Spotify-Playlist hinzugefügt"),
        format!("{count} canciones añadidas a la playlist de Spotify"),
        format!("{count} brani aggiunti alla playlist Spotify"),
        format!("{count} faixas adicionadas à playlist do Spotify"),
    )
}

fn playlist_remove_success(count: usize) -> LocalizedText {
    LocalizedText::with_languages(
        format!("{count} titre(s) retires de la playlist Spotify"),
        format!("{count} track(s) removed from the Spotify playlist"),
        format!("{count} Titel aus der Spotify-Playlist entfernt"),
        format!("{count} canciones eliminadas de la playlist de Spotify"),
        format!("{count} brani rimossi dalla playlist Spotify"),
        format!("{count} faixas removidas da playlist do Spotify"),
    )
}

fn playlist_created_adding(name: &str) -> LocalizedText {
    let name = playlist_status_name(name);
    LocalizedText::with_languages(
        format!("Playlist « {name} » creee; ajout des titres en cours..."),
        format!("Playlist \"{name}\" created; adding tracks..."),
        format!("Playlist \"{name}\" erstellt; Titel werden hinzugefügt..."),
        format!("Playlist «{name}» creada; añadiendo canciones..."),
        format!("Playlist «{name}» creata; aggiunta dei brani..."),
        format!("Playlist «{name}» criada; a adicionar faixas..."),
    )
}

fn playlist_created_success(name: &str, count: usize) -> LocalizedText {
    let name = playlist_status_name(name);
    LocalizedText::with_languages(
        format!("Playlist « {name} » creee : {count} titre(s) ajoutes"),
        format!("Playlist \"{name}\" created: {count} track(s) added"),
        format!("Playlist \"{name}\" erstellt: {count} Titel hinzugefügt"),
        format!("Playlist «{name}» creada: {count} canciones añadidas"),
        format!("Playlist «{name}» creata: {count} brani aggiunti"),
        format!("Playlist «{name}» criada: {count} faixas adicionadas"),
    )
}

fn playlist_created_error(name: &str, error: &LocalizedText) -> LocalizedText {
    use crate::locale::Language;
    let name = playlist_status_name(name);
    LocalizedText::with_languages(
        format!(
            "Playlist « {name} » creee. {}",
            error.text(Language::French)
        ),
        format!(
            "Playlist \"{name}\" created. {}",
            error.text(Language::English)
        ),
        format!(
            "Playlist \"{name}\" erstellt. {}",
            error.text(Language::German)
        ),
        format!(
            "Playlist «{name}» creada. {}",
            error.text(Language::Spanish)
        ),
        format!(
            "Playlist «{name}» creata. {}",
            error.text(Language::Italian)
        ),
        format!(
            "Playlist «{name}» criada. {}",
            error.text(Language::Portuguese)
        ),
    )
}

/// Leave room for the outcome and recovery advice in the two-line status bar.
/// The full name still goes to Spotify and remains in the catalogue.
fn playlist_status_name(name: &str) -> std::borrow::Cow<'_, str> {
    if name.chars().nth(40).is_none() {
        return name.into();
    }
    let end = name.char_indices().nth(37).map_or(name.len(), |(i, _)| i);
    format!("{}...", &name[..end]).into()
}
