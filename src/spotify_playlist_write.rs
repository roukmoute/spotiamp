//! Append-only playlist writes using the existing session and HTTP connection
//! pool. No automatic mutation retries: a lost acknowledgement is ambiguous.
//! Wire schema: librespot-protocol 0.8 playlist4_external.proto (Spotify's
//! Windows client schema). Endpoint/ADD form also documented by the upstream
//! openweb Spotify adapter: https://github.com/imoonkey/openweb/blob/main/src/sites/spotify/adapters/spotify-pathfinder.ts
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
    Add, ChangeInfo, Delta, Item, ListChanges, Op, SelectedListContent, SourceInfo, op, source_info,
};
use protobuf::{Message, MessageField};

use crate::{
    localized_text::LocalizedText,
    model::{PlaylistId, TrackRow},
};

pub const MAX_SELECTION: usize = 500;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);
const UNCERTAIN: &str = "Ajout non confirme; verifiez la playlist avant de reessayer";
const VERIFY_FAILED: &str = "Impossible de verifier la playlist; aucun titre ajoute";

#[derive(Debug)]
pub struct Receipt {
    pub total: Option<usize>,
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

struct SessionTransport(Session);
impl Transport for SessionTransport {
    async fn request(&self, method: Method, endpoint: &str, body: Vec<u8>) -> Result<Reply, ()> {
        let base = self.0.spclient().base_url().await.map_err(|_| ())?;
        let token = self.0.login5().auth_token().await.map_err(|_| ())?;
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
        if let Ok(client_token) = self.0.spclient().client_token().await {
            request = request.header("client-token", client_token);
        }
        let request = request.body(Bytes::from(body)).map_err(|_| ())?;
        // request_fut sends once. SpClient::request retries connection errors;
        // HttpClient::request retries 429. Neither is appropriate for writes.
        let response = self
            .0
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
    append_with(&SessionTransport(session), id, tracks, &username, || {
        auth_epoch.load(Ordering::Acquire) == generation
    })
    .await
}

async fn append_with(
    transport: &impl Transport,
    id: &PlaylistId,
    tracks: &[TrackRow],
    username: &str,
    still_connected: impl Fn() -> bool,
) -> Result<Receipt, LocalizedText> {
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
        assert_eq!(fake.requests.borrow().len(), 1);
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
