//! Playlist creation, append and occurrence-specific removal using the existing session/HTTP
//! pool. No automatic mutation retries: a lost acknowledgement is ambiguous.
//! Wire schema: librespot-protocol 0.8 playlist4_external.proto (Spotify's
//! Windows client schema). Endpoint/ADD form also documented by the upstream
//! openweb Spotify adapter: https://github.com/imoonkey/openweb/blob/main/src/sites/spotify/adapters/spotify-pathfinder.ts
//! Creation/rootlist sequence verified against Spotify's public Web Player:
//! https://open.spotifycdn.com/cdn/build/web-player/web-player.f954508a.js
//! RootlistAPI.createPlaylist serializes its operations as OpList, then adds
//! the new URI to /playlist/v2/user/{username}/rootlist/changes (ListChanges).
//! Its removeByIndex emits REM(fromIndex, length) and requires baseRevision;
//! URI-keyed removal is deliberately avoided because it cannot target duplicates.
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use bytes::Bytes;
use http::{Method, Request};
use http_body_util::{BodyExt, Limited};
use librespot_core::{Session, SpotifyId};
use librespot_protocol::playlist4_external::{
    Add, ChangeInfo, CreateListReply, Delta, Item, ListAttributes, ListAttributesPartialState,
    ListChanges, Mov, Op, OpList, Rem, SelectedListContent, SourceInfo, UpdateListAttributes, op,
    source_info,
};
use protobuf::{Message, MessageField};

use crate::{
    localized_text::LocalizedText,
    model::{PlaylistId, TrackRow, normalized_playlist_name, reorder_indices},
};

pub const MAX_SELECTION: usize = 500;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);
const APPEND_VERIFY_ATTEMPTS: usize = 3;
const APPEND_VERIFY_BUDGET: Duration = Duration::from_secs(5);
const APPEND_VERIFY_READ_TIMEOUT: Duration = Duration::from_millis(1500);
const APPEND_VERIFY_BACKOFF: Duration = Duration::from_millis(250);
const UNCERTAIN: &str = "Ajout non confirme; verifiez la playlist avant de reessayer";
const VERIFY_FAILED: &str = "Impossible de verifier la playlist; aucun titre ajoute";
const CREATE_UNCERTAIN: &str = "Creation non confirmee; verifiez vos playlists avant de reessayer";
const ROOTLIST_UNCERTAIN: &str =
    "Ajout a votre bibliotheque non confirme; verifiez vos playlists avant de reessayer";
const ROOTLIST_VERIFY_FAILED: &str = "Impossible de verifier vos playlists; aucun titre ajoute";
const ROOTLIST_REFUSED: &str = "Spotify a refuse l'ajout a votre bibliotheque; aucun titre ajoute";
const REMOVE_UNCERTAIN: &str = "Retrait non confirme; verifiez la playlist avant de reessayer";
const REMOVE_VERIFY_FAILED: &str = "Impossible de verifier la playlist; aucun titre retire";
const REMOVE_OWNER_ONLY: &str = "Seul le proprietaire peut retirer des titres de cette playlist";
const REORDER_UNCERTAIN: &str = "Deplacement non confirme; actualisation de la playlist";
const REORDER_VERIFY_FAILED: &str = "Impossible de verifier la playlist; aucun titre deplace";
const REORDER_OWNER_ONLY: &str =
    "Seul le proprietaire peut deplacer des titres dans cette playlist";
const PLAYLIST_CHANGED: &str = "La playlist a change; actualisez avant de reessayer";

#[derive(Debug)]
pub struct Receipt {
    pub total: Option<usize>,
}

#[derive(Debug)]
pub struct ReorderReceipt {
    pub revision: Vec<u8>,
}

#[derive(Debug)]
pub struct CreatedPlaylist {
    pub id: PlaylistId,
}

pub fn valid_tracks(rows: &[TrackRow]) -> bool {
    !rows.is_empty()
        && rows.len() <= MAX_SELECTION
        && rows
            .iter()
            .all(|row| canonical_id(&row.uri, "spotify:track:").is_some())
}

fn canonical_id<'a>(uri: &'a str, prefix: &str) -> Option<&'a str> {
    let id = uri.strip_prefix(prefix)?;
    (id.len() == 22
        && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
        && SpotifyId::from_base62(id).is_ok())
    .then_some(id)
}

struct Reply {
    status: u16,
    body: Vec<u8>,
}

trait Transport {
    fn request(
        &self,
        method: Method,
        endpoint: &str,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<Reply, ()>>;
}

struct SessionTransport {
    session: Session,
    auth_epoch: Arc<AtomicU64>,
    generation: u64,
}
impl Transport for SessionTransport {
    async fn request(&self, method: Method, endpoint: &str, body: Vec<u8>) -> Result<Reply, ()> {
        let base = self.session.spclient().base_url().await.map_err(|_| ())?;
        let token = self.session.login5().auth_token().await.map_err(|_| ())?;
        let mut request = Request::builder()
            .method(method)
            .uri(format!("{base}{endpoint}"))
            .header(
                "authorization",
                format!("{} {}", token.token_type, token.access_token),
            )
            .header("accept", "application/x-protobuf")
            .header("content-type", "application/x-protobuf")
            .header("content-length", body.len());
        if let Ok(client_token) = self.session.spclient().client_token().await {
            request = request.header("client-token", client_token);
        }
        let request = request.body(Bytes::from(body)).map_err(|_| ())?;
        // Token/base-url resolution above may yield. Check the account again
        // at the final boundary, before handing a mutation to the transport.
        if self.auth_epoch.load(Ordering::Acquire) != self.generation {
            return Err(());
        }
        // request_fut sends once. SpClient::request retries connection errors;
        // HttpClient::request retries 429. Neither is appropriate for writes.
        let response = self
            .session
            .http_client()
            .request_fut(request)
            .map_err(|_| ())?
            .await
            .map_err(|_| ())?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Ok(Reply {
                status,
                body: Vec::new(),
            });
        }
        let body = Limited::new(response.into_body(), MAX_RESPONSE_BYTES)
            .collect()
            .await
            .map_err(|_| ())?
            .to_bytes()
            .to_vec();
        Ok(Reply { status, body })
    }
}

pub async fn append(
    session: Session,
    id: &PlaylistId,
    tracks: &[TrackRow],
    auth_epoch: Arc<AtomicU64>,
    generation: u64,
) -> Result<Receipt, LocalizedText> {
    let username = session.username();
    let transport = SessionTransport {
        session,
        auth_epoch: auth_epoch.clone(),
        generation,
    };
    append_with(&transport, id, tracks, &username, || {
        auth_epoch.load(Ordering::Acquire) == generation
    })
    .await
}

pub async fn create(
    session: Session,
    name: &str,
    auth_epoch: Arc<AtomicU64>,
    generation: u64,
) -> Result<CreatedPlaylist, LocalizedText> {
    let transport = SessionTransport {
        session,
        auth_epoch: auth_epoch.clone(),
        generation,
    };
    create_with(&transport, name, || {
        auth_epoch.load(Ordering::Acquire) == generation
    })
    .await
}

/// Remove the selected positions from a complete, unchanged displayed playlist.
/// The transport rechecks ownership and the ordered URI snapshot before issuing
/// one revision-guarded mutation. It never retries a possibly committed removal.
pub async fn remove(
    session: Session,
    id: &PlaylistId,
    expected_tracks: &[TrackRow],
    expected_revision: &[u8],
    indices: &[usize],
    auth_epoch: Arc<AtomicU64>,
    generation: u64,
) -> Result<Receipt, LocalizedText> {
    let username = session.username();
    let transport = SessionTransport {
        session,
        auth_epoch: auth_epoch.clone(),
        generation,
    };
    remove_with(
        &transport,
        id,
        expected_tracks,
        expected_revision,
        indices,
        &username,
        || auth_epoch.load(Ordering::Acquire) == generation,
    )
    .await
}

async fn remove_with(
    transport: &impl Transport,
    id: &PlaylistId,
    expected_tracks: &[TrackRow],
    expected_revision: &[u8],
    indices: &[usize],
    username: &str,
    still_connected: impl Fn() -> bool,
) -> Result<Receipt, LocalizedText> {
    if !still_connected() || username.is_empty() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    if indices.len() > MAX_SELECTION {
        return Err("Selection limitee a 500 titres".into());
    }
    if expected_revision.is_empty() {
        return Err(PLAYLIST_CHANGED.into());
    }
    if !valid_tracks(expected_tracks)
        || indices.is_empty()
        || indices.iter().any(|&i| i >= expected_tracks.len())
        || indices.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("Selection invalide; aucun titre retire".into());
    }
    let (endpoint, base) = verified_edit_base(
        transport,
        id,
        expected_tracks,
        expected_revision,
        username,
        REMOVE_VERIFY_FAILED,
        REMOVE_OWNER_ONLY,
    )
    .await?;
    let body = removal_changes(&base, indices)
        .write_to_bytes()
        .map_err(|_| LocalizedText::from(REMOVE_VERIFY_FAILED))?;
    if !still_connected() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    let result = tokio::time::timeout(
        TIMEOUT,
        transport.request(Method::POST, &format!("{endpoint}/changes"), body),
    )
    .await
    .map_err(|_| LocalizedText::from(REMOVE_UNCERTAIN))?
    .map_err(|_| LocalizedText::from(REMOVE_UNCERTAIN))?;
    removal_receipt(result, &base, expected_tracks.len() - indices.len())
}

async fn verified_edit_base(
    transport: &impl Transport,
    id: &PlaylistId,
    expected_tracks: &[TrackRow],
    expected_revision: &[u8],
    username: &str,
    verify_failed: &'static str,
    owner_only: &'static str,
) -> Result<(String, SelectedListContent), LocalizedText> {
    let PlaylistId::Spotify(uri) = id;
    let id =
        canonical_id(uri, "spotify:playlist:").ok_or_else(|| LocalizedText::from(verify_failed))?;
    let endpoint = format!("/playlist/v2/playlist/{id}");
    let read = format!(
        "{endpoint}?decorate=revision,attributes,length,owner,capabilities&from=0&length={}",
        expected_tracks.len()
    );
    let response = tokio::time::timeout(TIMEOUT, transport.request(Method::GET, &read, Vec::new()))
        .await
        .map_err(|_| LocalizedText::from(verify_failed))?
        .map_err(|_| LocalizedText::from(verify_failed))?;
    if !(200..300).contains(&response.status) {
        return Err(verify_failed.into());
    }
    let base = checked_content(&response.body).ok_or_else(|| LocalizedText::from(verify_failed))?;
    // Collaborative write permission is insufficient for this owner-only UI.
    if base.owner_username() != username
        || base.attributes.get_or_default().deleted_by_owner()
        || base
            .capabilities
            .as_ref()
            .is_some_and(|caps| caps.has_can_edit_items() && !caps.can_edit_items())
    {
        return Err(owner_only.into());
    }
    let same_items = base.contents.as_ref().is_some_and(|contents| {
        contents.pos() == 0
            && !contents.truncated()
            && contents.items.len() == expected_tracks.len()
            && contents
                .items
                .iter()
                .zip(expected_tracks)
                .all(|(actual, expected)| actual.uri() == expected.uri)
    });
    if !same_items
        || !base.has_length()
        || usize::try_from(base.length()).ok() != Some(expected_tracks.len())
        || base.revision() != expected_revision
        || base.multiple_heads()
        || base.changes_require_resync()
    {
        return Err(PLAYLIST_CHANGED.into());
    }
    Ok((endpoint, base))
}

/// A single revision-guarded delta contains all MOV ranges. Native moves retain
/// occurrence attributes (added-by, timestamps and item IDs), even for duplicates.
pub async fn reorder(
    session: Session,
    id: &PlaylistId,
    expected_tracks: &[TrackRow],
    expected_revision: &[u8],
    indices: &[usize],
    insert_before: usize,
    auth_epoch: Arc<AtomicU64>,
    generation: u64,
) -> Result<ReorderReceipt, LocalizedText> {
    let username = session.username();
    let transport = SessionTransport {
        session,
        auth_epoch: auth_epoch.clone(),
        generation,
    };
    reorder_with(
        &transport,
        id,
        expected_tracks,
        expected_revision,
        indices,
        insert_before,
        &username,
        || auth_epoch.load(Ordering::Acquire) == generation,
    )
    .await
}

async fn reorder_with(
    transport: &impl Transport,
    id: &PlaylistId,
    expected_tracks: &[TrackRow],
    expected_revision: &[u8],
    indices: &[usize],
    insert_before: usize,
    username: &str,
    still_connected: impl Fn() -> bool,
) -> Result<ReorderReceipt, LocalizedText> {
    if !still_connected() || username.is_empty() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    if !valid_tracks(expected_tracks) || expected_revision.is_empty() {
        return Err(REORDER_VERIFY_FAILED.into());
    }
    let order = reorder_indices(expected_tracks.len(), indices, insert_before)
        .ok_or_else(|| LocalizedText::from("Selection invalide; aucun titre deplace"))?;
    let (endpoint, base) = verified_edit_base(
        transport,
        id,
        expected_tracks,
        expected_revision,
        username,
        REORDER_VERIFY_FAILED,
        REORDER_OWNER_ONLY,
    )
    .await?;
    let body = reorder_changes(&base, &order)
        .write_to_bytes()
        .map_err(|_| LocalizedText::from(REORDER_VERIFY_FAILED))?;
    if !still_connected() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    let response = tokio::time::timeout(
        TIMEOUT,
        transport.request(Method::POST, &format!("{endpoint}/changes"), body),
    )
    .await
    .map_err(|_| LocalizedText::from(REORDER_UNCERTAIN))?
    .map_err(|_| LocalizedText::from(REORDER_UNCERTAIN))?;
    match response.status {
        401 | 403 => return Err(REORDER_OWNER_ONLY.into()),
        409 | 412 => return Err(PLAYLIST_CHANGED.into()),
        429 => return Err("Spotify limite les deplacements; patientez avant de reessayer".into()),
        400 | 404 | 405 | 413 | 422 => {
            return Err("Spotify a refuse le deplacement; aucun titre deplace".into());
        }
        200..=299 => {}
        _ => return Err(REORDER_UNCERTAIN.into()),
    }
    let result =
        checked_content(&response.body).ok_or_else(|| LocalizedText::from(REORDER_UNCERTAIN))?;
    let revision = if !result.revision().is_empty() && result.revision() != base.revision() {
        Some(result.revision())
    } else {
        result
            .resulting_revisions
            .iter()
            .rev()
            .find(|revision| !revision.is_empty() && revision.as_slice() != base.revision())
            .map(Vec::as_slice)
    };
    if result.multiple_heads()
        || result.changes_require_resync()
        || (result.has_length()
            && usize::try_from(result.length()).ok() != Some(expected_tracks.len()))
    {
        return Err(REORDER_UNCERTAIN.into());
    }
    Ok(ReorderReceipt {
        revision: revision
            .ok_or_else(|| LocalizedText::from(REORDER_UNCERTAIN))?
            .to_vec(),
    })
}

fn reorder_changes(base: &SelectedListContent, order: &[usize]) -> ListChanges {
    let mut current: Vec<usize> = (0..order.len()).collect();
    let mut ops = Vec::new();
    let mut to = 0;
    while to < order.len() {
        if current[to] == order[to] {
            to += 1;
            continue;
        }
        let from = current[to + 1..]
            .iter()
            .position(|&old| old == order[to])
            .unwrap()
            + to
            + 1;
        let mut length = 1;
        while from + length < current.len() && current[from + length] == order[to + length] {
            length += 1;
        }
        // Only upward moves are emitted: the destination gap has identical
        // coordinates before and after removing this range. Each next operation
        // uses the result of its predecessor in the same atomic delta.
        let mut mov = Mov::new();
        mov.set_from_index(from as i32);
        mov.set_to_index(to as i32);
        mov.set_length(length as i32);
        let mut op = Op::new();
        op.set_kind(op::Kind::MOV);
        op.mov = MessageField::some(mov);
        ops.push(op);
        current[to..from + length].rotate_right(length);
        to += length;
    }
    operations_changes(base, ops)
}

fn removal_changes(base: &SelectedListContent, indices: &[usize]) -> ListChanges {
    let mut ops = Vec::new();
    let mut remaining = indices.len();
    while remaining > 0 {
        let end = indices[remaining - 1];
        let mut start = end;
        remaining -= 1;
        while remaining > 0 && indices[remaining - 1] + 1 == start {
            start = indices[remaining - 1];
            remaining -= 1;
        }
        let mut rem = Rem::new();
        rem.set_from_index(start as i32);
        rem.set_length((end - start + 1) as i32);
        let mut op = Op::new();
        op.set_kind(op::Kind::REM);
        op.rem = MessageField::some(rem);
        ops.push(op);
    }
    // Higher positions are removed first, so earlier positions stay valid.
    operations_changes(base, ops)
}

fn removal_receipt(
    response: Reply,
    base: &SelectedListContent,
    expected_total: usize,
) -> Result<Receipt, LocalizedText> {
    match response.status {
        401 | 403 => return Err(REMOVE_OWNER_ONLY.into()),
        409 | 412 => return Err(PLAYLIST_CHANGED.into()),
        429 => return Err("Spotify limite les retraits; patientez avant de reessayer".into()),
        400 | 404 | 405 | 413 | 422 => {
            return Err("Spotify a refuse le retrait; aucun titre retire".into());
        }
        200..=299 => {}
        _ => return Err(REMOVE_UNCERTAIN.into()),
    }
    let result =
        checked_content(&response.body).ok_or_else(|| LocalizedText::from(REMOVE_UNCERTAIN))?;
    let acknowledged = !result.revision().is_empty() && result.revision() != base.revision()
        || result
            .resulting_revisions
            .iter()
            .any(|r| !r.is_empty() && r != base.revision());
    if !acknowledged
        || result.multiple_heads()
        || result.changes_require_resync()
        || (result.has_length() && usize::try_from(result.length()).ok() != Some(expected_total))
    {
        return Err(REMOVE_UNCERTAIN.into());
    }
    Ok(Receipt {
        total: Some(expected_total),
    })
}

async fn create_with(
    transport: &impl Transport,
    name: &str,
    still_connected: impl Fn() -> bool,
) -> Result<CreatedPlaylist, LocalizedText> {
    let name = normalized_playlist_name(name)
        .ok_or_else(|| LocalizedText::from("Nom de playlist invalide (1 a 100 caracteres)"))?;
    // The captured client operation creates a named empty list. Initial ADD
    // operations in this endpoint are not documented, so append only after
    // retaining the acknowledged URI. Never repeat creation automatically.
    let body = creation(&name)
        .write_to_bytes()
        .map_err(|_| LocalizedText::from("Spotify a refuse la creation de la playlist"))?;
    if !still_connected() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    let response = tokio::time::timeout(
        TIMEOUT,
        transport.request(Method::POST, "/playlist/v2/playlist", body),
    )
    .await
    .map_err(|_| LocalizedText::from(CREATE_UNCERTAIN))?
    .map_err(|_| LocalizedText::from(CREATE_UNCERTAIN))?;
    match response.status {
        400 | 401 | 403 | 404 | 405 | 409 | 412 | 413 | 422 => {
            return Err("Spotify a refuse la creation de la playlist".into());
        }
        429 => {
            return Err("Spotify limite les creations; patientez avant de reessayer".into());
        }
        200..=299 => {}
        _ => return Err(CREATE_UNCERTAIN.into()),
    }
    let result = (response.body.len() <= MAX_RESPONSE_BYTES)
        .then(|| CreateListReply::parse_from_bytes(&response.body).ok())
        .flatten()
        .filter(|reply| canonical_id(reply.uri(), "spotify:playlist:").is_some())
        .ok_or_else(|| LocalizedText::from(CREATE_UNCERTAIN))?;
    Ok(CreatedPlaylist {
        id: PlaylistId::Spotify(result.uri().to_owned()),
    })
}

fn creation(name: &str) -> OpList {
    let mut attributes = ListAttributes::new();
    attributes.set_name(name.to_owned());
    let mut state = ListAttributesPartialState::new();
    state.values = MessageField::some(attributes);
    let mut update = UpdateListAttributes::new();
    update.new_attributes = MessageField::some(state);
    let mut op = Op::new();
    op.set_kind(op::Kind::UPDATE_LIST_ATTRIBUTES);
    op.update_list_attributes = MessageField::some(update);
    let mut list = OpList::new();
    list.ops.push(op);
    list
}

/// Creating a playlist does not subscribe it to the current user's rootlist.
/// Spotify's own RootlistAPI performs this separate operation before returning
/// its created URI. Keep it separate here so an uncertain result never causes
/// a second creation or a second rootlist insertion.
pub async fn link_created_playlist(
    session: Session,
    id: &PlaylistId,
    auth_epoch: Arc<AtomicU64>,
    generation: u64,
) -> Result<(), LocalizedText> {
    let username = session.username();
    let transport = SessionTransport {
        session,
        auth_epoch: auth_epoch.clone(),
        generation,
    };
    link_created_with(&transport, id, &username, || {
        auth_epoch.load(Ordering::Acquire) == generation
    })
    .await
}

async fn link_created_with(
    transport: &impl Transport,
    id: &PlaylistId,
    username: &str,
    still_connected: impl Fn() -> bool,
) -> Result<(), LocalizedText> {
    let PlaylistId::Spotify(uri) = id;
    if canonical_id(uri, "spotify:playlist:").is_none()
        || username.is_empty()
        || username.len() > 512
    {
        return Err(ROOTLIST_VERIFY_FAILED.into());
    }
    if !still_connected() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    // This is a path segment, not a form field: spaces must be %20, not +.
    let username = url::form_urlencoded::byte_serialize(username.as_bytes())
        .collect::<String>()
        .replace('+', "%20");
    let endpoint = format!("/playlist/v2/user/{username}/rootlist");
    let read = format!("{endpoint}?decorate=revision,length&from=0&length=0");
    let response = tokio::time::timeout(TIMEOUT, transport.request(Method::GET, &read, Vec::new()))
        .await
        .map_err(|_| LocalizedText::from(ROOTLIST_VERIFY_FAILED))?
        .map_err(|_| LocalizedText::from(ROOTLIST_VERIFY_FAILED))?;
    if !(200..300).contains(&response.status) {
        return Err(ROOTLIST_VERIFY_FAILED.into());
    }
    let base = checked_content(&response.body)
        .ok_or_else(|| LocalizedText::from(ROOTLIST_VERIFY_FAILED))?;
    if base.revision().is_empty() || base.multiple_heads() || base.changes_require_resync() {
        return Err(ROOTLIST_VERIFY_FAILED.into());
    }
    let mut op = Op::new();
    op.set_kind(op::Kind::ADD);
    let mut add = Add::new();
    add.set_add_first(true);
    let mut item = Item::new();
    item.set_uri(uri.to_owned());
    add.items.push(item);
    op.add = MessageField::some(add);
    let body = operation_changes(&base, op)
        .write_to_bytes()
        .map_err(|_| LocalizedText::from(ROOTLIST_VERIFY_FAILED))?;
    if !still_connected() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    let response = tokio::time::timeout(
        TIMEOUT,
        transport.request(Method::POST, &format!("{endpoint}/changes"), body),
    )
    .await
    .map_err(|_| LocalizedText::from(ROOTLIST_UNCERTAIN))?
    .map_err(|_| LocalizedText::from(ROOTLIST_UNCERTAIN))?;
    match response.status {
        400 | 401 | 403 | 404 | 405 | 409 | 412 | 413 | 422 | 429 => {
            return Err(ROOTLIST_REFUSED.into());
        }
        200..=299 => {}
        _ => return Err(ROOTLIST_UNCERTAIN.into()),
    }
    receipt(response, &base, 1)
        .map(|_| ())
        .map_err(|_| LocalizedText::from(ROOTLIST_UNCERTAIN))
}

async fn append_with(
    transport: &impl Transport,
    id: &PlaylistId,
    tracks: &[TrackRow],
    username: &str,
    still_connected: impl Fn() -> bool,
) -> Result<Receipt, LocalizedText> {
    if !still_connected() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    if tracks.len() > MAX_SELECTION {
        return Err("Selection limitee a 500 titres".into());
    }
    if !valid_tracks(tracks) {
        return Err("Selection invalide; aucun titre ajoute".into());
    }
    let PlaylistId::Spotify(uri) = id;
    let id = canonical_id(uri, "spotify:playlist:").ok_or_else(|| {
        LocalizedText::from("La destination n'est plus disponible; actualisez les playlists")
    })?;
    let endpoint = format!("/playlist/v2/playlist/{id}");
    let read = format!(
        "{endpoint}?decorate=revision,attributes,length,owner,capabilities&from=0&length=0"
    );
    let response = tokio::time::timeout(TIMEOUT, transport.request(Method::GET, &read, Vec::new()))
        .await
        .map_err(|_| LocalizedText::from(VERIFY_FAILED))?
        .map_err(|_| LocalizedText::from(VERIFY_FAILED))?;
    if !still_connected() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    if !(200..300).contains(&response.status) {
        return Err(VERIFY_FAILED.into());
    }
    let base = checked_content(&response.body).ok_or_else(|| LocalizedText::from(VERIFY_FAILED))?;
    if !can_append(&base, username) {
        return Err("Cette playlist est en lecture seule; aucun titre ajoute".into());
    }
    if base.revision().is_empty() || base.multiple_heads() || base.changes_require_resync() {
        return Err("La playlist a change; actualisez avant de reessayer".into());
    }
    let body = changes(&base, tracks)
        .write_to_bytes()
        .map_err(|_| LocalizedText::from(VERIFY_FAILED))?;
    if !still_connected() {
        return Err("Connectez Spotify pour modifier une playlist".into());
    }
    // Once this request begins, a timeout or broken connection might mean the
    // server committed the append. Never retry or label it a definite failure.
    let response = tokio::time::timeout(
        TIMEOUT,
        transport.request(Method::POST, &format!("{endpoint}/changes"), body),
    )
    .await;
    if !still_connected() {
        return Err(UNCERTAIN.into());
    }
    let result = match response {
        Ok(Ok(response)) => receipt(response, &base, tracks.len()),
        _ => Err(UNCERTAIN.into()),
    };
    if result.as_ref().is_err_and(|message| *message == UNCERTAIN) {
        // A successful write may return an empty/incomplete acknowledgement,
        // or lose its response entirely. Only repeat reads, never the append.
        verify_appended_tracks(
            transport,
            &endpoint,
            &base,
            tracks,
            username,
            &still_connected,
        )
        .await
        .ok_or_else(|| LocalizedText::from(UNCERTAIN))
    } else {
        result
    }
}

fn can_append(content: &SelectedListContent, username: &str) -> bool {
    !content.attributes.get_or_default().deleted_by_owner()
        && content
            .capabilities
            .as_ref()
            .filter(|caps| caps.has_can_edit_items())
            .map(|caps| caps.can_edit_items())
            .unwrap_or_else(|| !username.is_empty() && content.owner_username() == username)
}

async fn verify_appended_tracks(
    transport: &impl Transport,
    endpoint: &str,
    base: &SelectedListContent,
    tracks: &[TrackRow],
    username: &str,
    still_connected: &impl Fn() -> bool,
) -> Option<Receipt> {
    let from = base
        .has_length()
        .then(|| usize::try_from(base.length()).ok())??;
    let expected_total = from.checked_add(tracks.len())?;
    i32::try_from(expected_total).ok()?;
    let read = format!(
        "{endpoint}?decorate=revision,attributes,length,owner,capabilities&from={from}&length={}",
        tracks.len()
    );
    let deadline = tokio::time::Instant::now() + APPEND_VERIFY_BUDGET;
    for attempt in 0..APPEND_VERIFY_ATTEMPTS {
        if !still_connected() || !can_append(base, username) {
            return None;
        }
        if attempt != 0 {
            tokio::time::timeout_at(
                deadline,
                tokio::time::sleep(APPEND_VERIFY_BACKOFF * attempt as u32),
            )
            .await
            .ok()?;
        }
        if !still_connected() || tokio::time::Instant::now() >= deadline {
            return None;
        }
        let read_deadline = deadline.min(tokio::time::Instant::now() + APPEND_VERIFY_READ_TIMEOUT);
        let response = tokio::time::timeout_at(
            read_deadline,
            transport.request(Method::GET, &read, Vec::new()),
        )
        .await;
        // Logout/account switches can occur while any request is in flight.
        if !still_connected() {
            return None;
        }
        let Ok(Ok(response)) = response else {
            continue;
        };
        if (400..500).contains(&response.status) {
            return None;
        }
        if !(200..300).contains(&response.status) {
            continue;
        }
        let Some(content) = checked_content(&response.body) else {
            continue;
        };
        if !can_append(&content, username) {
            return None;
        }
        // URI membership or a larger total is not proof: the selection can
        // already exist, contain duplicates, or have been added elsewhere.
        // Match every new occurrence at the old tail, in its original order.
        if !content.revision().is_empty()
            && content.revision() != base.revision()
            && !content.multiple_heads()
            && !content.changes_require_resync()
            && content.has_length()
            && usize::try_from(content.length()).ok() == Some(expected_total)
            && content.contents.as_ref().is_some_and(|items| {
                items.has_pos()
                    && usize::try_from(items.pos()).ok() == Some(from)
                    && !items.truncated()
                    && items.items.len() == tracks.len()
                    && items
                        .items
                        .iter()
                        .zip(tracks)
                        .all(|(item, row)| item.uri() == row.uri)
            })
        {
            return Some(Receipt {
                total: Some(expected_total),
            });
        }
    }
    None
}

fn checked_content(body: &[u8]) -> Option<SelectedListContent> {
    if body.len() > MAX_RESPONSE_BYTES {
        return None;
    }
    SelectedListContent::parse_from_bytes(body).ok()
}

fn changes(base: &SelectedListContent, tracks: &[TrackRow]) -> ListChanges {
    let mut add = Add::new();
    add.set_add_last(true);
    add.items = tracks
        .iter()
        .map(|row| {
            let mut item = Item::new();
            item.set_uri(row.uri.clone());
            item
        })
        .collect();
    let mut op = Op::new();
    op.set_kind(op::Kind::ADD);
    op.add = MessageField::some(add);
    operation_changes(base, op)
}

fn operation_changes(base: &SelectedListContent, op: Op) -> ListChanges {
    operations_changes(base, [op])
}

fn operations_changes(
    base: &SelectedListContent,
    ops: impl IntoIterator<Item = Op>,
) -> ListChanges {
    let mut source = SourceInfo::new();
    source.set_client(source_info::Client::CLIENT);
    source.set_app("Spotiamp".into());
    source.set_version(env!("CARGO_PKG_VERSION").into());
    let mut info = ChangeInfo::new();
    info.source = MessageField::some(source);
    let mut delta = Delta::new();
    delta.ops.extend(ops);
    delta.info = MessageField::some(info);
    let mut changes = ListChanges::new();
    changes.set_base_revision(base.revision().to_vec());
    changes.set_want_resulting_revisions(true);
    changes.deltas.push(delta);
    changes
}

fn receipt(
    response: Reply,
    base: &SelectedListContent,
    count: usize,
) -> Result<Receipt, LocalizedText> {
    match response.status {
        401 | 403 => return Err("Cette playlist est en lecture seule; aucun titre ajoute".into()),
        409 | 412 => return Err("La playlist a change; actualisez avant de reessayer".into()),
        429 => return Err("Spotify limite les ajouts; patientez avant de reessayer".into()),
        400 | 404 | 405 | 413 | 422 => {
            return Err("Spotify a refuse l'ajout; aucun titre ajoute".into());
        }
        200..=299 => {}
        _ => return Err(UNCERTAIN.into()),
    }
    let result = checked_content(&response.body).ok_or_else(|| LocalizedText::from(UNCERTAIN))?;
    let acknowledged = !result.revision().is_empty() && result.revision() != base.revision()
        || result
            .resulting_revisions
            .iter()
            .any(|r| !r.is_empty() && r != base.revision());
    if !acknowledged || result.multiple_heads() || result.changes_require_resync() {
        return Err(UNCERTAIN.into());
    }
    let total = if result.has_length() {
        usize::try_from(result.length()).ok()
    } else {
        base.has_length()
            .then(|| usize::try_from(base.length()).ok()?.checked_add(count))
            .flatten()
    };
    Ok(Receipt { total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use librespot_protocol::playlist_permission::Capabilities;
    use std::{cell::RefCell, collections::VecDeque};
    const URI: &str = "spotify:track:4uLU6hMCjMI75M1A2tKUQC";
    fn row() -> TrackRow {
        TrackRow {
            uri: URI.into(),
            ..Default::default()
        }
    }
    fn id() -> PlaylistId {
        PlaylistId::Spotify("spotify:playlist:4uLU6hMCjMI75M1A2tKUQC".into())
    }
    fn base() -> SelectedListContent {
        let mut p = SelectedListContent::new();
        p.set_revision(vec![1]);
        p.set_length(3);
        let mut c = Capabilities::new();
        c.set_can_edit_items(true);
        p.capabilities = MessageField::some(c);
        p
    }
    fn reply(p: SelectedListContent) -> Result<Reply, ()> {
        Ok(Reply {
            status: 200,
            body: p.write_to_bytes().unwrap(),
        })
    }
    fn removable_base(rows: &[TrackRow]) -> SelectedListContent {
        let mut p = base();
        p.set_owner_username("me".into());
        p.set_length(rows.len() as i32);
        let mut contents = librespot_protocol::playlist4_external::ListItems::new();
        contents.set_pos(0);
        contents.set_truncated(false);
        contents.items = rows
            .iter()
            .map(|r| {
                let mut item = Item::new();
                item.set_uri(r.uri.clone());
                item
            })
            .collect();
        p.contents = MessageField::some(contents);
        p
    }
    struct Fake {
        replies: RefCell<VecDeque<Result<Reply, ()>>>,
        requests: RefCell<Vec<(Method, String, Vec<u8>)>>,
    }
    impl Fake {
        fn new(replies: Vec<Result<Reply, ()>>) -> Self {
            Self {
                replies: RefCell::new(replies.into()),
                requests: RefCell::default(),
            }
        }
    }
    impl Transport for Fake {
        async fn request(
            &self,
            method: Method,
            endpoint: &str,
            body: Vec<u8>,
        ) -> Result<Reply, ()> {
            self.requests
                .borrow_mut()
                .push((method, endpoint.into(), body));
            self.replies.borrow_mut().pop_front().unwrap()
        }
    }
    fn created_reply(uri: &str) -> Result<Reply, ()> {
        let mut reply = CreateListReply::new();
        reply.set_uri(uri.into());
        reply.set_revision(vec![1]);
        Ok(Reply {
            status: 201,
            body: reply.write_to_bytes().unwrap(),
        })
    }

    #[tokio::test]
    async fn create_sends_only_a_unicode_name_and_retains_the_returned_identity() {
        let PlaylistId::Spotify(uri) = id();
        let transport = Fake::new(vec![created_reply(&uri)]);
        let created = create_with(&transport, "  Été 🎵  ", || true)
            .await
            .unwrap();
        assert_eq!(created.id, id());
        let requests = transport.requests.borrow();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0, Method::POST);
        assert_eq!(requests[0].1, "/playlist/v2/playlist");
        // The Web Player serializer is OpList (ops field 1), even though its
        // builder calls the intermediate JavaScript object a Delta.
        assert_eq!(requests[0].2[0], 0x0a);
        let ops = OpList::parse_from_bytes(&requests[0].2).unwrap();
        assert_eq!(ops.ops.len(), 1);
        assert_eq!(ops.ops[0].kind(), op::Kind::UPDATE_LIST_ATTRIBUTES);
        assert!(ops.ops[0].add.is_none());
        let attrs = ops.ops[0]
            .update_list_attributes
            .get_or_default()
            .new_attributes
            .get_or_default()
            .values
            .get_or_default();
        assert_eq!(attrs.name(), "Été 🎵");
        assert!(!attrs.has_collaborative());
    }

    #[tokio::test]
    async fn invalid_names_and_expired_account_never_create() {
        let transport = Fake::new(vec![]);
        for name in ["", " \t ", "two\nlines", &"é".repeat(101)] {
            assert!(create_with(&transport, name, || true).await.is_err());
        }
        assert!(create_with(&transport, "Name", || false).await.is_err());
        assert!(transport.requests.borrow().is_empty());
    }

    #[tokio::test]
    async fn ambiguous_creation_never_retries_or_manufactures_an_identity() {
        for response in [
            Err(()),
            Ok(Reply {
                status: 500,
                body: vec![],
            }),
            Ok(Reply {
                status: 204,
                body: vec![],
            }),
            Ok(Reply {
                status: 201,
                body: vec![0xff],
            }),
            Ok(Reply {
                status: 201,
                body: vec![0; MAX_RESPONSE_BYTES + 1],
            }),
            created_reply(URI),
            created_reply("spotify:playlist:invalid"),
            created_reply("spotify:playlist:4uLU6hMCjMI75M1A2tKUQC/changes"),
        ] {
            let transport = Fake::new(vec![response]);
            assert_eq!(
                create_with(&transport, "Name", || true).await.unwrap_err(),
                CREATE_UNCERTAIN
            );
            assert_eq!(transport.requests.borrow().len(), 1);
        }
    }

    #[tokio::test]
    async fn creation_refusal_and_rate_limit_are_definite_and_never_retried() {
        for status in [400, 401, 403, 404, 409, 412, 413, 422, 429] {
            let transport = Fake::new(vec![Ok(Reply {
                status,
                body: vec![],
            })]);
            let message = create_with(&transport, "Name", || true).await.unwrap_err();
            assert_ne!(message, CREATE_UNCERTAIN);
            assert_eq!(transport.requests.borrow().len(), 1);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_creation_is_uncertain_and_sent_once() {
        struct Slow(std::cell::Cell<usize>);
        impl Transport for Slow {
            async fn request(&self, _: Method, _: &str, _: Vec<u8>) -> Result<Reply, ()> {
                self.0.set(self.0.get() + 1);
                std::future::pending().await
            }
        }
        let transport = Slow(std::cell::Cell::new(0));
        assert_eq!(
            create_with(&transport, "Name", || true).await.unwrap_err(),
            CREATE_UNCERTAIN
        );
        assert_eq!(transport.0.get(), 1);
    }

    #[tokio::test]
    async fn rootlist_registration_uses_account_path_and_one_revision_guarded_playlist_add() {
        let mut after = base();
        after.set_revision(vec![2]);
        let transport = Fake::new(vec![reply(base()), reply(after)]);
        link_created_with(&transport, &id(), "a/b é+?#", || true)
            .await
            .unwrap();
        let requests = transport.requests.borrow();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].0, Method::GET);
        assert!(
            requests[0]
                .1
                .starts_with("/playlist/v2/user/a%2Fb%20%C3%A9%2B%3F%23/rootlist?")
        );
        assert_eq!(
            requests[1].1,
            "/playlist/v2/user/a%2Fb%20%C3%A9%2B%3F%23/rootlist/changes"
        );
        let changes = ListChanges::parse_from_bytes(&requests[1].2).unwrap();
        assert_eq!(changes.base_revision(), &[1]);
        assert_eq!(changes.deltas.len(), 1);
        assert_eq!(changes.deltas[0].ops.len(), 1);
        let op = &changes.deltas[0].ops[0];
        assert_eq!(op.kind(), op::Kind::ADD);
        let add = op.add.get_or_default();
        assert!(add.add_first());
        assert_eq!(add.items.len(), 1);
        assert_eq!(PlaylistId::Spotify(add.items[0].uri().into()), id());
    }

    #[tokio::test]
    async fn rootlist_failure_remains_distinct_from_creation_and_never_retries() {
        for (response, message) in [
            (Err(()), ROOTLIST_UNCERTAIN),
            (
                Ok(Reply {
                    status: 500,
                    body: vec![],
                }),
                ROOTLIST_UNCERTAIN,
            ),
            (
                Ok(Reply {
                    status: 403,
                    body: vec![],
                }),
                ROOTLIST_REFUSED,
            ),
            (
                Ok(Reply {
                    status: 409,
                    body: vec![],
                }),
                ROOTLIST_REFUSED,
            ),
            (
                Ok(Reply {
                    status: 429,
                    body: vec![],
                }),
                ROOTLIST_REFUSED,
            ),
            (reply(base()), ROOTLIST_UNCERTAIN),
        ] {
            let transport = Fake::new(vec![reply(base()), response]);
            assert_eq!(
                link_created_with(&transport, &id(), "me", || true)
                    .await
                    .unwrap_err(),
                message
            );
            assert_eq!(transport.requests.borrow().len(), 2);
        }
    }

    #[tokio::test]
    async fn expired_account_or_unusable_rootlist_never_posts() {
        let mut no_revision = base();
        no_revision.clear_revision();
        let mut conflicted = base();
        conflicted.set_changes_require_resync(true);
        for response in [Err(()), reply(no_revision), reply(conflicted)] {
            let transport = Fake::new(vec![response]);
            assert_eq!(
                link_created_with(&transport, &id(), "me", || true)
                    .await
                    .unwrap_err(),
                ROOTLIST_VERIFY_FAILED
            );
            assert_eq!(transport.requests.borrow().len(), 1);
        }
        let transport = Fake::new(vec![]);
        assert!(
            link_created_with(&transport, &id(), "me", || false)
                .await
                .is_err()
        );
        assert!(
            link_created_with(&transport, &id(), "", || true)
                .await
                .is_err()
        );
        assert!(transport.requests.borrow().is_empty());
        let transport = Fake::new(vec![reply(base())]);
        let calls = std::cell::Cell::new(0);
        assert!(
            link_created_with(&transport, &id(), "me", || {
                let call = calls.get();
                calls.set(call + 1);
                call == 0
            })
            .await
            .is_err()
        );
        assert_eq!(transport.requests.borrow().len(), 1);
    }

    #[tokio::test]
    async fn create_register_and_append_preserve_identity_and_track_order() {
        let PlaylistId::Spotify(uri) = id();
        let mut root_end = base();
        root_end.set_revision(vec![2]);
        let mut tracks_end = base();
        tracks_end.set_revision(vec![3]);
        tracks_end.set_length(3);
        let transport = Fake::new(vec![
            created_reply(&uri),
            reply(base()),
            reply(root_end),
            reply(base()),
            reply(tracks_end),
        ]);
        let created = create_with(&transport, "Name", || true).await.unwrap();
        link_created_with(&transport, &created.id, "me", || true)
            .await
            .unwrap();
        append_with(
            &transport,
            &created.id,
            &[row(), row(), row()],
            "me",
            || true,
        )
        .await
        .unwrap();
        let requests = transport.requests.borrow();
        let posts = requests
            .iter()
            .filter(|r| r.0 == Method::POST)
            .collect::<Vec<_>>();
        assert_eq!(posts.len(), 3);
        assert_eq!(posts[0].1, "/playlist/v2/playlist");
        assert_eq!(posts[1].1, "/playlist/v2/user/me/rootlist/changes");
        assert_eq!(
            posts[2].1,
            format!(
                "/playlist/v2/playlist/{}/changes",
                uri.strip_prefix("spotify:playlist:").unwrap()
            )
        );
        let changes = ListChanges::parse_from_bytes(&posts[2].2).unwrap();
        assert_eq!(changes.deltas[0].ops[0].add.get_or_default().items.len(), 3);
    }
    #[tokio::test]
    async fn append_sends_one_revision_guarded_add_preserving_order_and_duplicates() {
        let mut end = base();
        end.set_revision(vec![2]);
        end.set_length(6);
        let transport = Fake::new(vec![reply(base()), reply(end)]);
        let other = TrackRow {
            uri: "spotify:track:0000000000000000000001".into(),
            ..Default::default()
        };
        let tracks = [row(), other, row()];
        let result = append_with(&transport, &id(), &tracks, "", || true)
            .await
            .unwrap();
        assert_eq!(result.total, Some(6));
        let requests = transport.requests.borrow();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].0, Method::POST);
        let body = ListChanges::parse_from_bytes(&requests[1].2).unwrap();
        assert_eq!(body.base_revision(), &[1]);
        assert_eq!(body.deltas.len(), 1);
        assert_eq!(body.deltas[0].ops.len(), 1);
        let op = &body.deltas[0].ops[0];
        assert_eq!(op.kind(), op::Kind::ADD);
        assert!(op.add.get_or_default().add_last());
        assert_eq!(
            op.add
                .get_or_default()
                .items
                .iter()
                .map(|i| i.uri())
                .collect::<Vec<_>>(),
            tracks.iter().map(|t| t.uri.as_str()).collect::<Vec<_>>()
        );
    }
    #[tokio::test]
    async fn permissions_conflict_and_logout_prevent_any_post() {
        let mut denied = base();
        denied
            .capabilities
            .as_mut()
            .unwrap()
            .set_can_edit_items(false);
        denied.set_owner_username("me".into());
        let mut conflict = base();
        conflict.set_multiple_heads(true);
        for p in [denied, conflict] {
            let fake = Fake::new(vec![reply(p)]);
            assert!(
                append_with(&fake, &id(), &[row()], "me", || true)
                    .await
                    .is_err()
            );
            assert_eq!(fake.requests.borrow().len(), 1);
        }
        let fake = Fake::new(vec![reply(base())]);
        assert!(
            append_with(&fake, &id(), &[row()], "", || false)
                .await
                .is_err()
        );
        assert_eq!(fake.requests.borrow().len(), 0);
    }
    #[tokio::test(start_paused = true)]
    async fn uncertain_or_refused_writes_never_retry() {
        for response in [
            Err(()),
            Ok(Reply {
                status: 500,
                body: vec![],
            }),
            Ok(Reply {
                status: 200,
                body: vec![0xff],
            }),
            reply(base()),
        ] {
            let fake = Fake::new(vec![
                reply(base()),
                response,
                reply(base()),
                reply(base()),
                reply(base()),
            ]);
            assert_eq!(
                append_with(&fake, &id(), &[row()], "", || true)
                    .await
                    .unwrap_err(),
                UNCERTAIN
            );
            let requests = fake.requests.borrow();
            assert_eq!(requests.len(), 2 + APPEND_VERIFY_ATTEMPTS);
            assert_eq!(
                requests
                    .iter()
                    .filter(|(m, _, _)| *m == Method::POST)
                    .count(),
                1
            );
        }
        for status in [400, 401, 403, 404, 405, 409, 412, 413, 422, 429] {
            let fake = Fake::new(vec![
                reply(base()),
                Ok(Reply {
                    status,
                    body: vec![],
                }),
            ]);
            assert!(
                append_with(&fake, &id(), &[row()], "", || true)
                    .await
                    .is_err()
            );
            assert_eq!(fake.requests.borrow().len(), 2);
        }
    }
    #[tokio::test]
    async fn invalid_input_never_reaches_network() {
        for tracks in [
            vec![],
            vec![row(); 501],
            vec![TrackRow {
                uri: "spotify:episode:4uLU6hMCjMI75M1A2tKUQC".into(),
                ..Default::default()
            }],
        ] {
            let fake = Fake::new(vec![]);
            assert!(
                append_with(&fake, &id(), &tracks, "", || true)
                    .await
                    .is_err()
            );
            assert!(fake.requests.borrow().is_empty());
        }
    }
    #[tokio::test(start_paused = true)]
    async fn timeout_after_post_is_uncertain_and_is_never_retried() {
        struct Slow {
            calls: std::cell::Cell<usize>,
        }
        impl Transport for Slow {
            async fn request(&self, method: Method, _: &str, _: Vec<u8>) -> Result<Reply, ()> {
                self.calls.set(self.calls.get() + 1);
                if method == Method::GET {
                    reply(base())
                } else {
                    std::future::pending().await
                }
            }
        }
        let transport = Slow {
            calls: std::cell::Cell::new(0),
        };
        assert_eq!(
            append_with(&transport, &id(), &[row()], "", || true)
                .await
                .unwrap_err(),
            UNCERTAIN
        );
        assert_eq!(transport.calls.get(), 2 + APPEND_VERIFY_ATTEMPTS);
    }

    fn appended_segment(rows: &[TrackRow]) -> SelectedListContent {
        let mut content = removable_base(rows);
        content.set_revision(vec![2]);
        content.set_length(base().length() + rows.len() as i32);
        content.contents.as_mut().unwrap().set_pos(base().length());
        content
    }

    #[tokio::test(start_paused = true)]
    async fn append_lost_empty_or_unreadable_ack_is_confirmed_by_one_tail_read() {
        let tracks = [
            row(),
            TrackRow {
                uri: "spotify:track:0000000000000000000001".into(),
                ..Default::default()
            },
            row(),
        ];
        for response in [
            Err(()),
            Ok(Reply {
                status: 500,
                body: vec![],
            }),
            Ok(Reply {
                status: 200,
                body: vec![],
            }),
            Ok(Reply {
                status: 204,
                body: vec![],
            }),
            Ok(Reply {
                status: 200,
                body: vec![0xff],
            }),
            reply(base()),
        ] {
            let fake = Fake::new(vec![
                reply(base()),
                response,
                reply(appended_segment(&tracks)),
            ]);
            let result = append_with(&fake, &id(), &tracks, "", || true)
                .await
                .unwrap();
            assert_eq!(result.total, Some(6));
            let requests = fake.requests.borrow();
            assert_eq!(requests.len(), 3);
            assert_eq!(
                requests
                    .iter()
                    .filter(|(m, _, _)| *m == Method::POST)
                    .count(),
                1
            );
            assert_eq!(requests[2].0, Method::GET);
            assert!(requests[2].1.ends_with("&from=3&length=3"));
            assert!(requests[2].2.is_empty());
        }
    }

    #[tokio::test(start_paused = true)]
    async fn append_verification_waits_for_propagation_without_repeating_the_write() {
        let mut stale = appended_segment(&[row()]);
        stale.set_revision(base().revision().to_vec());
        let fake = Fake::new(vec![
            reply(base()),
            Err(()),
            reply(stale),
            Err(()),
            reply(appended_segment(&[row()])),
        ]);
        let started = tokio::time::Instant::now();
        assert_eq!(
            append_with(&fake, &id(), &[row()], "", || true)
                .await
                .unwrap()
                .total,
            Some(4)
        );
        assert_eq!(
            tokio::time::Instant::now() - started,
            APPEND_VERIFY_BACKOFF * 3
        );
        let requests = fake.requests.borrow();
        assert_eq!(requests.len(), 5);
        assert_eq!(
            requests
                .iter()
                .filter(|(m, _, _)| *m == Method::POST)
                .count(),
            1
        );
    }

    #[tokio::test(start_paused = true)]
    async fn append_verification_requires_new_revision_exact_positions_order_and_multiplicity() {
        let other = TrackRow {
            uri: "spotify:track:0000000000000000000001".into(),
            ..Default::default()
        };
        let tracks = [row(), other.clone(), row()];
        let good = appended_segment(&tracks);
        let mut variants = vec![
            appended_segment(&[other.clone(), row(), row()]),
            appended_segment(&[row(), other.clone()]),
            appended_segment(&[row(), other.clone(), other]),
        ];
        let mut p = good.clone();
        p.contents.as_mut().unwrap().set_pos(0);
        variants.push(p);
        let mut p = good.clone();
        p.contents.as_mut().unwrap().set_truncated(true);
        variants.push(p);
        let mut p = good.clone();
        p.contents = MessageField::none();
        variants.push(p);
        let mut p = good.clone();
        p.set_revision(base().revision().to_vec());
        variants.push(p);
        let mut p = good.clone();
        p.clear_revision();
        variants.push(p);
        let mut p = good.clone();
        p.set_multiple_heads(true);
        variants.push(p);
        let mut p = good.clone();
        p.set_changes_require_resync(true);
        variants.push(p);
        let mut p = good.clone();
        p.clear_length();
        variants.push(p);
        let mut p = good.clone();
        p.set_length(-1);
        variants.push(p);
        let mut p = good;
        p.set_length(7);
        variants.push(p);
        for content in variants {
            let fake = Fake::new(vec![
                reply(base()),
                Err(()),
                reply(content.clone()),
                reply(content.clone()),
                reply(content),
            ]);
            assert_eq!(
                append_with(&fake, &id(), &tracks, "", || true)
                    .await
                    .unwrap_err(),
                UNCERTAIN
            );
            let requests = fake.requests.borrow();
            assert_eq!(requests.len(), 5);
            assert_eq!(
                requests
                    .iter()
                    .filter(|(m, _, _)| *m == Method::POST)
                    .count(),
                1
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn append_verification_does_not_guess_a_missing_or_invalid_original_length() {
        for length in [None, Some(-1), Some(i32::MAX)] {
            let mut before = base();
            before.clear_length();
            if let Some(length) = length {
                before.set_length(length);
            }
            let fake = Fake::new(vec![reply(before), Err(())]);
            assert_eq!(
                append_with(&fake, &id(), &[row()], "", || true)
                    .await
                    .unwrap_err(),
                UNCERTAIN
            );
            assert_eq!(fake.requests.borrow().len(), 2);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn append_verification_rejects_unreadable_or_oversized_reads_with_bounded_attempts() {
        for body in [vec![], vec![0xff], vec![0; MAX_RESPONSE_BYTES + 1]] {
            let mut replies = vec![reply(base()), Err(())];
            replies.extend((0..APPEND_VERIFY_ATTEMPTS).map(|_| {
                Ok(Reply {
                    status: 200,
                    body: body.clone(),
                })
            }));
            let fake = Fake::new(replies);
            assert_eq!(
                append_with(&fake, &id(), &[row()], "", || true)
                    .await
                    .unwrap_err(),
                UNCERTAIN
            );
            let requests = fake.requests.borrow();
            // An empty protobuf has no read access information, so it stops
            // immediately; malformed/oversized replies can recover on a read.
            assert_eq!(requests.len(), if body.is_empty() { 3 } else { 5 });
            assert_eq!(
                requests
                    .iter()
                    .filter(|(m, _, _)| *m == Method::POST)
                    .count(),
                1
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn append_verification_supports_an_empty_destination_and_the_selection_limit() {
        let tracks = vec![row(); MAX_SELECTION];
        let mut before = base();
        before.set_length(0);
        let mut after = appended_segment(&tracks);
        after.set_length(MAX_SELECTION as i32);
        after.contents.as_mut().unwrap().set_pos(0);
        let fake = Fake::new(vec![reply(before), Err(()), reply(after)]);
        assert_eq!(
            append_with(&fake, &id(), &tracks, "", || true)
                .await
                .unwrap()
                .total,
            Some(MAX_SELECTION)
        );
        let requests = fake.requests.borrow();
        assert_eq!(requests.len(), 3);
        assert!(requests[2].1.ends_with("&from=0&length=500"));
    }

    #[tokio::test(start_paused = true)]
    async fn append_verification_stops_after_logout_or_revoked_access() {
        for stop_after in [1, 2, 3] {
            let fake = Fake::new(vec![
                reply(base()),
                Err(()),
                reply(appended_segment(&[row()])),
            ]);
            let error = append_with(&fake, &id(), &[row()], "", || {
                fake.requests.borrow().len() < stop_after
            })
            .await
            .unwrap_err();
            assert_eq!(fake.requests.borrow().len(), stop_after);
            assert_eq!(
                error,
                if stop_after == 1 {
                    "Connectez Spotify pour modifier une playlist"
                } else {
                    UNCERTAIN
                }
            );
        }
        let mut denied = appended_segment(&[row()]);
        denied
            .capabilities
            .as_mut()
            .unwrap()
            .set_can_edit_items(false);
        let mut deleted = appended_segment(&[row()]);
        let mut attrs = ListAttributes::new();
        attrs.set_deleted_by_owner(true);
        deleted.attributes = MessageField::some(attrs);
        for response in [
            reply(denied),
            reply(deleted),
            Ok(Reply {
                status: 403,
                body: vec![],
            }),
            Ok(Reply {
                status: 429,
                body: vec![],
            }),
        ] {
            let fake = Fake::new(vec![reply(base()), Err(()), response]);
            assert_eq!(
                append_with(&fake, &id(), &[row()], "", || true)
                    .await
                    .unwrap_err(),
                UNCERTAIN
            );
            assert_eq!(fake.requests.borrow().len(), 3);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn append_timeout_can_be_confirmed_and_verification_reads_have_one_shared_deadline() {
        struct Slow {
            calls: std::cell::Cell<usize>,
            confirm: bool,
        }
        impl Transport for Slow {
            async fn request(&self, _: Method, _: &str, _: Vec<u8>) -> Result<Reply, ()> {
                let call = self.calls.get();
                self.calls.set(call + 1);
                match call {
                    0 => reply(base()),
                    1 => std::future::pending().await,
                    _ if self.confirm => reply(appended_segment(&[row()])),
                    _ => std::future::pending().await,
                }
            }
        }
        for confirm in [true, false] {
            let fake = Slow {
                calls: std::cell::Cell::new(0),
                confirm,
            };
            let started = tokio::time::Instant::now();
            let result = append_with(&fake, &id(), &[row()], "", || true).await;
            let elapsed = tokio::time::Instant::now() - started;
            if confirm {
                assert_eq!(result.unwrap().total, Some(4));
                assert_eq!(fake.calls.get(), 3);
                assert_eq!(elapsed, TIMEOUT);
            } else {
                assert_eq!(result.unwrap_err(), UNCERTAIN);
                assert_eq!(fake.calls.get(), 2 + APPEND_VERIFY_ATTEMPTS);
                assert_eq!(elapsed, TIMEOUT + APPEND_VERIFY_BUDGET);
            }
        }
    }

    #[tokio::test]
    async fn missing_revision_unreadable_get_and_unknown_rights_never_post() {
        let mut no_revision = base();
        no_revision.clear_revision();
        let mut unknown = base();
        unknown.capabilities = MessageField::none();
        for response in [
            reply(no_revision),
            reply(unknown),
            Ok(Reply {
                status: 200,
                body: vec![0xff],
            }),
            Err(()),
        ] {
            let fake = Fake::new(vec![response]);
            assert!(
                append_with(&fake, &id(), &[row()], "someone", || true)
                    .await
                    .is_err()
            );
            assert_eq!(fake.requests.borrow().len(), 1);
        }
    }

    #[tokio::test]
    async fn removal_targets_only_selected_occurrences_in_descending_ranges() {
        let other = TrackRow {
            uri: "spotify:track:0000000000000000000001".into(),
            ..Default::default()
        };
        let tracks = [row(), other.clone(), row(), row(), other, row()];
        let mut after = base();
        after.set_revision(vec![2]);
        after.set_length(3);
        let fake = Fake::new(vec![reply(removable_base(&tracks)), reply(after)]);
        let result = remove_with(&fake, &id(), &tracks, &[1], &[1, 3, 4], "me", || true)
            .await
            .unwrap();
        assert_eq!(result.total, Some(3));
        let requests = fake.requests.borrow();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].0, Method::GET);
        assert!(requests[0].1.ends_with("&from=0&length=6"));
        assert_eq!(requests[1].0, Method::POST);
        assert_eq!(
            requests[1].1,
            "/playlist/v2/playlist/4uLU6hMCjMI75M1A2tKUQC/changes"
        );
        let changes = ListChanges::parse_from_bytes(&requests[1].2).unwrap();
        assert_eq!(changes.base_revision(), &[1]);
        assert!(changes.want_resulting_revisions());
        assert_eq!(changes.deltas.len(), 1);
        let ops = &changes.deltas[0].ops;
        assert_eq!(ops.len(), 2);
        let mut remaining = (0..tracks.len()).collect::<Vec<_>>();
        let mut ranges = Vec::new();
        for op in ops {
            assert_eq!(op.kind(), op::Kind::REM);
            assert!(op.add.is_none());
            let rem = op.rem.get_or_default();
            assert!(!rem.has_items_as_key());
            assert!(rem.items.is_empty());
            let start = rem.from_index() as usize;
            let length = rem.length() as usize;
            ranges.push((start, length));
            remaining.drain(start..start + length);
        }
        assert_eq!(ranges, [(3, 2), (1, 1)]);
        assert_eq!(remaining, [0, 2, 5]);
    }

    #[tokio::test]
    async fn removal_accepts_owner_without_capabilities_and_all_tracks_in_one_range() {
        let tracks = vec![row(); MAX_SELECTION];
        let mut before = removable_base(&tracks);
        before.capabilities = MessageField::none();
        let mut after = SelectedListContent::new();
        after.resulting_revisions.push(vec![2]);
        let fake = Fake::new(vec![reply(before), reply(after)]);
        let result = remove_with(
            &fake,
            &id(),
            &tracks,
            &[1],
            &(0..MAX_SELECTION).collect::<Vec<_>>(),
            "me",
            || true,
        )
        .await
        .unwrap();
        assert_eq!(result.total, Some(0));
        let requests = fake.requests.borrow();
        let changes = ListChanges::parse_from_bytes(&requests[1].2).unwrap();
        assert_eq!(changes.deltas[0].ops.len(), 1);
        let rem = changes.deltas[0].ops[0].rem.get_or_default();
        assert_eq!(rem.from_index(), 0);
        assert_eq!(rem.length(), MAX_SELECTION as i32);
    }

    #[tokio::test]
    async fn removal_invalid_snapshot_indices_identity_and_auth_never_reach_network() {
        let fake = Fake::new(vec![]);
        for indices in [
            vec![],
            vec![1],
            vec![0, 0],
            vec![1, 0],
            vec![usize::MAX],
            vec![0; MAX_SELECTION + 1],
        ] {
            assert!(
                remove_with(&fake, &id(), &[row()], &[1], &indices, "me", || true)
                    .await
                    .is_err()
            );
        }
        for tracks in [
            vec![],
            vec![row(); MAX_SELECTION + 1],
            vec![TrackRow {
                uri: "spotify:episode:4uLU6hMCjMI75M1A2tKUQC".into(),
                ..Default::default()
            }],
        ] {
            assert!(
                remove_with(&fake, &id(), &tracks, &[1], &[0], "me", || true)
                    .await
                    .is_err()
            );
        }
        assert_eq!(
            remove_with(&fake, &id(), &[row()], &[], &[0], "me", || true)
                .await
                .unwrap_err(),
            PLAYLIST_CHANGED
        );
        assert!(
            remove_with(
                &fake,
                &PlaylistId::Spotify("spotify:playlist:invalid/changes".into()),
                &[row()],
                &[1],
                &[0],
                "me",
                || true
            )
            .await
            .is_err()
        );
        assert!(
            remove_with(&fake, &id(), &[row()], &[1], &[0], "", || true)
                .await
                .is_err()
        );
        assert!(
            remove_with(&fake, &id(), &[row()], &[1], &[0], "me", || false)
                .await
                .is_err()
        );
        assert!(fake.requests.borrow().is_empty());
    }

    #[tokio::test]
    async fn removal_requires_exact_owner_even_with_collaborative_edit_permission() {
        let tracks = [row()];
        let mut variants = Vec::new();
        for owner in ["", "someone-else", "ME"] {
            let mut p = removable_base(&tracks);
            p.set_owner_username(owner.into());
            variants.push(p);
        }
        let mut denied = removable_base(&tracks);
        denied
            .capabilities
            .as_mut()
            .unwrap()
            .set_can_edit_items(false);
        variants.push(denied);
        let mut deleted = removable_base(&tracks);
        let mut attributes = ListAttributes::new();
        attributes.set_deleted_by_owner(true);
        deleted.attributes = MessageField::some(attributes);
        variants.push(deleted);
        for p in variants {
            let fake = Fake::new(vec![reply(p)]);
            assert_eq!(
                remove_with(&fake, &id(), &tracks, &[1], &[0], "me", || true)
                    .await
                    .unwrap_err(),
                REMOVE_OWNER_ONLY
            );
            assert_eq!(fake.requests.borrow().len(), 1);
        }
    }

    #[tokio::test]
    async fn removal_refuses_incomplete_changed_or_recreated_identical_uri_lists() {
        let tracks = [row(), row()];
        let mut variants = Vec::new();
        let original = removable_base(&tracks);
        let mut p = original.clone();
        p.contents = MessageField::none();
        variants.push(p);
        let mut p = original.clone();
        p.contents.as_mut().unwrap().set_pos(1);
        variants.push(p);
        let mut p = original.clone();
        p.contents.as_mut().unwrap().set_truncated(true);
        variants.push(p);
        let mut p = original.clone();
        p.contents.as_mut().unwrap().items.pop();
        variants.push(p);
        let mut p = original.clone();
        p.contents.as_mut().unwrap().items[0]
            .set_uri("spotify:track:0000000000000000000001".into());
        variants.push(p);
        let mut p = original.clone();
        p.clear_length();
        variants.push(p);
        let mut p = original.clone();
        p.set_length(-1);
        variants.push(p);
        let mut p = original.clone();
        p.set_length(3);
        variants.push(p);
        let mut p = original.clone();
        p.clear_revision();
        variants.push(p);
        let mut p = original.clone();
        p.set_revision(vec![2]);
        variants.push(p);
        let mut p = original.clone();
        p.set_multiple_heads(true);
        variants.push(p);
        let mut p = original;
        p.set_changes_require_resync(true);
        variants.push(p);
        for p in variants {
            let fake = Fake::new(vec![reply(p)]);
            assert_eq!(
                remove_with(&fake, &id(), &tracks, &[1], &[0], "me", || true)
                    .await
                    .unwrap_err(),
                PLAYLIST_CHANGED
            );
            assert_eq!(fake.requests.borrow().len(), 1);
        }
    }

    #[tokio::test]
    async fn removal_unreadable_get_or_logout_during_verification_never_posts() {
        for response in [
            Err(()),
            Ok(Reply {
                status: 403,
                body: vec![],
            }),
            Ok(Reply {
                status: 200,
                body: vec![0xff],
            }),
            Ok(Reply {
                status: 200,
                body: vec![0; MAX_RESPONSE_BYTES + 1],
            }),
        ] {
            let fake = Fake::new(vec![response]);
            assert_eq!(
                remove_with(&fake, &id(), &[row()], &[1], &[0], "me", || true)
                    .await
                    .unwrap_err(),
                REMOVE_VERIFY_FAILED
            );
            assert_eq!(fake.requests.borrow().len(), 1);
        }
        let fake = Fake::new(vec![reply(removable_base(&[row()]))]);
        let checks = std::cell::Cell::new(0);
        assert!(
            remove_with(&fake, &id(), &[row()], &[1], &[0], "me", || {
                let n = checks.get();
                checks.set(n + 1);
                n == 0
            })
            .await
            .is_err()
        );
        assert_eq!(fake.requests.borrow().len(), 1);
    }

    #[tokio::test]
    async fn removal_lost_or_inconsistent_acknowledgements_never_retry() {
        let mut no_ack = SelectedListContent::new();
        no_ack.set_revision(vec![1]);
        let mut wrong_length = SelectedListContent::new();
        wrong_length.set_revision(vec![2]);
        wrong_length.set_length(1);
        let mut negative_length = wrong_length.clone();
        negative_length.set_length(-1);
        let mut resync = SelectedListContent::new();
        resync.set_revision(vec![2]);
        resync.set_changes_require_resync(true);
        let mut heads = SelectedListContent::new();
        heads.set_revision(vec![2]);
        heads.set_multiple_heads(true);
        for response in [
            Err(()),
            Ok(Reply {
                status: 500,
                body: vec![],
            }),
            Ok(Reply {
                status: 204,
                body: vec![],
            }),
            Ok(Reply {
                status: 200,
                body: vec![0xff],
            }),
            Ok(Reply {
                status: 200,
                body: vec![0; MAX_RESPONSE_BYTES + 1],
            }),
            reply(no_ack),
            reply(wrong_length),
            reply(negative_length),
            reply(resync),
            reply(heads),
        ] {
            let fake = Fake::new(vec![reply(removable_base(&[row()])), response]);
            assert_eq!(
                remove_with(&fake, &id(), &[row()], &[1], &[0], "me", || true)
                    .await
                    .unwrap_err(),
                REMOVE_UNCERTAIN
            );
            assert_eq!(fake.requests.borrow().len(), 2);
        }
    }

    #[tokio::test]
    async fn removal_explicit_refusal_or_conflict_never_retries() {
        for status in [400, 401, 403, 404, 405, 409, 412, 413, 422, 429] {
            let fake = Fake::new(vec![
                reply(removable_base(&[row()])),
                Ok(Reply {
                    status,
                    body: vec![],
                }),
            ]);
            let error = remove_with(&fake, &id(), &[row()], &[1], &[0], "me", || true)
                .await
                .unwrap_err();
            assert_ne!(error, REMOVE_UNCERTAIN);
            if status == 409 || status == 412 {
                assert_eq!(error, PLAYLIST_CHANGED);
            }
            assert_eq!(fake.requests.borrow().len(), 2);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn removal_read_and_write_timeouts_are_distinguished_without_retries() {
        struct Slow {
            read: bool,
            calls: std::cell::Cell<usize>,
        }
        impl Transport for Slow {
            async fn request(&self, method: Method, _: &str, _: Vec<u8>) -> Result<Reply, ()> {
                self.calls.set(self.calls.get() + 1);
                if method == Method::GET && !self.read {
                    reply(removable_base(&[row()]))
                } else {
                    std::future::pending().await
                }
            }
        }
        for read in [true, false] {
            let fake = Slow {
                read,
                calls: std::cell::Cell::new(0),
            };
            assert_eq!(
                remove_with(&fake, &id(), &[row()], &[1], &[0], "me", || true)
                    .await
                    .unwrap_err(),
                if read {
                    REMOVE_VERIFY_FAILED
                } else {
                    REMOVE_UNCERTAIN
                }
            );
            assert_eq!(fake.calls.get(), if read { 1 } else { 2 });
        }
    }
    #[test]
    fn every_small_reorder_is_one_atomic_delta_of_occurrence_preserving_moves() {
        for len in 2..9 {
            for mask in 1..1usize << len {
                let indices = (0..len)
                    .filter(|i| mask & (1 << i) != 0)
                    .collect::<Vec<_>>();
                for gap in 0..=len {
                    let Some(order) = reorder_indices(len, &indices, gap) else {
                        continue;
                    };
                    let changes = reorder_changes(&base(), &order);
                    assert_eq!(changes.base_revision(), &[1]);
                    assert_eq!(changes.deltas.len(), 1);
                    let mut items = (0..len).collect::<Vec<_>>();
                    for op in &changes.deltas[0].ops {
                        assert_eq!(op.kind(), op::Kind::MOV);
                        assert!(op.add.is_none() && op.rem.is_none());
                        let mov = op.mov.as_ref().unwrap();
                        assert!(mov.items.is_empty());
                        let (from, to, length) = (
                            mov.from_index() as usize,
                            mov.to_index() as usize,
                            mov.length() as usize,
                        );
                        assert!(to < from && length > 0 && from + length <= len);
                        let moved = items.drain(from..from + length).collect::<Vec<_>>();
                        items.splice(to..to, moved);
                    }
                    assert_eq!(items, order);
                }
            }
        }
    }

    #[tokio::test]
    async fn reorder_posts_once_with_exact_revision_and_returns_confirmed_revision() {
        let tracks = vec![row(); 6];
        let mut after = base();
        after.set_revision(vec![9, 3]);
        after.set_length(6);
        let fake = Fake::new(vec![reply(removable_base(&tracks)), reply(after)]);
        let result = reorder_with(&fake, &id(), &tracks, &[1], &[1, 3], 6, "me", || true)
            .await
            .unwrap();
        assert_eq!(result.revision, [9, 3]);
        let requests = fake.requests.borrow();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].0, Method::GET);
        assert!(requests[0].1.ends_with("&from=0&length=6"));
        assert_eq!(requests[1].0, Method::POST);
        let changes = ListChanges::parse_from_bytes(&requests[1].2).unwrap();
        assert_eq!(changes.base_revision(), &[1]);
        assert!(changes.want_resulting_revisions());
        assert_eq!(changes.deltas.len(), 1);
        assert!(
            changes.deltas[0]
                .ops
                .iter()
                .all(|op| op.kind() == op::Kind::MOV)
        );
    }

    #[tokio::test]
    async fn reorder_rejects_invalid_noop_or_stale_snapshots_and_unauthorized_writes() {
        let tracks = vec![row(); 3];
        let fake = Fake::new(vec![]);
        for (indices, gap) in [
            (vec![], 0),
            (vec![0], 0),
            (vec![0], 1),
            (vec![3], 0),
            (vec![1, 1], 0),
            (vec![2, 1], 0),
            (vec![0], 4),
        ] {
            assert!(
                reorder_with(&fake, &id(), &tracks, &[1], &indices, gap, "me", || true)
                    .await
                    .is_err()
            );
        }
        assert!(fake.requests.borrow().is_empty());
        let mut variants = Vec::new();
        let mut before = removable_base(&tracks);
        before.set_owner_username("another-account".into());
        variants.push(before);
        let mut before = removable_base(&tracks);
        before.set_revision(vec![2]);
        variants.push(before);
        let mut before = removable_base(&tracks);
        before.contents.as_mut().unwrap().items[0]
            .set_uri("spotify:track:0000000000000000000001".into());
        variants.push(before);
        let mut before = removable_base(&tracks);
        before.contents.as_mut().unwrap().set_truncated(true);
        variants.push(before);
        let mut before = removable_base(&tracks);
        before
            .capabilities
            .as_mut()
            .unwrap()
            .set_can_edit_items(false);
        variants.push(before);
        for before in variants {
            let fake = Fake::new(vec![reply(before)]);
            assert!(
                reorder_with(&fake, &id(), &tracks, &[1], &[0], 3, "me", || true)
                    .await
                    .is_err()
            );
            assert_eq!(fake.requests.borrow().len(), 1);
        }
        let fake = Fake::new(vec![reply(removable_base(&tracks))]);
        let checks = std::cell::Cell::new(0);
        assert!(
            reorder_with(&fake, &id(), &tracks, &[1], &[0], 3, "me", || {
                let n = checks.get();
                checks.set(n + 1);
                n == 0
            })
            .await
            .is_err()
        );
        assert_eq!(fake.requests.borrow().len(), 1);
    }

    #[tokio::test]
    async fn reorder_never_retries_conflicts_or_ambiguous_acknowledgements() {
        let tracks = vec![row(); 2];
        let mut bad_length = base();
        bad_length.set_revision(vec![2]);
        bad_length.set_length(1);
        let mut resync = base();
        resync.set_revision(vec![2]);
        resync.set_changes_require_resync(true);
        for response in [
            Err(()),
            Ok(Reply {
                status: 500,
                body: vec![],
            }),
            Ok(Reply {
                status: 409,
                body: vec![],
            }),
            Ok(Reply {
                status: 429,
                body: vec![],
            }),
            Ok(Reply {
                status: 200,
                body: vec![0xff],
            }),
            reply(base()),
            reply(bad_length),
            reply(resync),
        ] {
            let fake = Fake::new(vec![reply(removable_base(&tracks)), response]);
            assert!(
                reorder_with(&fake, &id(), &tracks, &[1], &[1], 0, "me", || true)
                    .await
                    .is_err()
            );
            assert_eq!(fake.requests.borrow().len(), 2, "must never retry a move");
        }
    }
}
