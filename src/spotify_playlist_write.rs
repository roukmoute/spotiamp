//! Playlist creation and append-only writes using the existing session and HTTP connection
//! pool. No automatic mutation retries: a lost acknowledgement is ambiguous.
//! Wire schema: librespot-protocol 0.8 playlist4_external.proto (Spotify's
//! Windows client schema). Endpoint/ADD form also documented by the upstream
//! openweb Spotify adapter: https://github.com/imoonkey/openweb/blob/main/src/sites/spotify/adapters/spotify-pathfinder.ts
//! Creation/rootlist sequence verified against Spotify's public Web Player:
//! https://open.spotifycdn.com/cdn/build/web-player/web-player.f954508a.js
//! RootlistAPI.createPlaylist serializes its operations as OpList, then adds
//! the new URI to /playlist/v2/user/{username}/rootlist/changes (ListChanges).
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
    ListChanges, Op, OpList, SelectedListContent, SourceInfo, UpdateListAttributes, op,
    source_info,
};
use protobuf::{Message, MessageField};

use crate::{
    localized_text::LocalizedText,
    model::{PlaylistId, TrackRow, normalized_playlist_name},
};

pub const MAX_SELECTION: usize = 500;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);
const UNCERTAIN: &str = "Ajout non confirme; verifiez la playlist avant de reessayer";
const VERIFY_FAILED: &str = "Impossible de verifier la playlist; aucun titre ajoute";
const CREATE_UNCERTAIN: &str = "Creation non confirmee; verifiez vos playlists avant de reessayer";
const ROOTLIST_UNCERTAIN: &str =
    "Ajout a votre bibliotheque non confirme; verifiez vos playlists avant de reessayer";
const ROOTLIST_VERIFY_FAILED: &str = "Impossible de verifier vos playlists; aucun titre ajoute";
const ROOTLIST_REFUSED: &str = "Spotify a refuse l'ajout a votre bibliotheque; aucun titre ajoute";

#[derive(Debug)]
pub struct Receipt {
    pub total: Option<usize>,
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
    if !(200..300).contains(&response.status) {
        return Err(VERIFY_FAILED.into());
    }
    let base = checked_content(&response.body).ok_or_else(|| LocalizedText::from(VERIFY_FAILED))?;
    let allowed = base
        .capabilities
        .as_ref()
        .filter(|caps| caps.has_can_edit_items())
        .map(|caps| caps.can_edit_items())
        .unwrap_or_else(|| !username.is_empty() && base.owner_username() == username);
    if !allowed || base.attributes.get_or_default().deleted_by_owner() {
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
    let result = tokio::time::timeout(
        TIMEOUT,
        transport.request(Method::POST, &format!("{endpoint}/changes"), body),
    )
    .await
    .map_err(|_| LocalizedText::from(UNCERTAIN))?
    .map_err(|_| LocalizedText::from(UNCERTAIN))?;
    receipt(result, &base, tracks.len())
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
    let mut source = SourceInfo::new();
    source.set_client(source_info::Client::CLIENT);
    source.set_app("Spotiamp".into());
    source.set_version(env!("CARGO_PKG_VERSION").into());
    let mut info = ChangeInfo::new();
    info.source = MessageField::some(source);
    let mut delta = Delta::new();
    delta.ops.push(op);
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
    #[tokio::test]
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
            let fake = Fake::new(vec![reply(base()), response]);
            assert_eq!(
                append_with(&fake, &id(), &[row()], "", || true)
                    .await
                    .unwrap_err(),
                UNCERTAIN
            );
            assert_eq!(fake.requests.borrow().len(), 2);
        }
        for status in [403, 409, 429] {
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
        assert_eq!(transport.calls.get(), 2);
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
}
