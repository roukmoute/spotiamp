//! Bounded music search through the authenticated session's context resolver.
//! No web API registration, artwork, extra HTTP client, or persistent cache.
use std::{collections::HashSet, time::Duration};

use librespot_core::{Session, SpotifyId, SpotifyUri};
use librespot_protocol::context::Context;
use protobuf::Message;

use crate::{
    localized_text::LocalizedText,
    model::{SEARCH_QUERY_MAX_BYTES, SEARCH_QUERY_MAX_CHARS, TrackRow, TrackTitleKind},
};

pub const MAX_RESULTS: usize = 50;
const MAX_CONTEXT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_CONTEXT_PAGES: usize = 32;
const MAX_RAW_TRACKS: usize = 1000;

pub fn normalize_query(query: &str) -> Result<String, LocalizedText> {
    if query.len() > SEARCH_QUERY_MAX_BYTES || query.chars().count() > SEARCH_QUERY_MAX_CHARS {
        return Err("Recherche limitee a 256 caracteres".into());
    }
    Ok(query
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .to_owned())
}

fn search_uri(query: &str) -> String {
    // Spaces use '+' as required by librespot's search context. Percent-encode
    // every other delimiter: the query is a URI path component, not a URL.
    format!(
        "spotify:search:{}",
        url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>()
    )
}

pub async fn fetch(session: &Session, query: &str) -> Result<(Vec<TrackRow>, bool), LocalizedText> {
    // get_context deliberately omits SpClient's metrics/salt query parameters,
    // which change search semantics. Use its existing authentication/transport.
    let context = tokio::time::timeout(
        Duration::from_secs(20),
        session.spclient().get_context(&search_uri(query)),
    )
    .await
    .map_err(|_| LocalizedText::from("Spotify ne repond pas a la recherche; reessayez"))?
    .map_err(|_| {
        LocalizedText::from("Recherche Spotify indisponible; reessayez dans un instant")
    })?;
    parse_context(context)
}

fn parse_context(context: Context) -> Result<(Vec<TrackRow>, bool), LocalizedText> {
    // The existing transport/parser has already buffered this response. This
    // bounds retained/processed data, not that transport's peak allocation.
    if context.compute_size() > MAX_CONTEXT_BYTES
        || context.pages.len() > MAX_CONTEXT_PAGES
        || context
            .pages
            .iter()
            .map(|page| page.tracks.len())
            .sum::<usize>()
            > MAX_RAW_TRACKS
    {
        return Err("Reponse de recherche Spotify trop volumineuse".into());
    }
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    let mut truncated = context.loading();
    for page in context.pages {
        truncated |= page.loading()
            || !page.next_page_url().is_empty()
            || (page.tracks.is_empty() && !page.page_url().is_empty());
        for track in page.tracks {
            let uri = if track.uri().is_empty() {
                SpotifyId::from_raw(track.gid())
                    .ok()
                    .map(|id| SpotifyUri::Track { id })
            } else {
                let text = track.uri();
                // A context can mix podcasts, local files or malformed URIs.
                text.strip_prefix("spotify:track:")
                    .filter(|id| id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric()))
                    .and_then(|id| SpotifyId::from_base62(id).ok())
                    .map(|id| SpotifyUri::Track { id })
            };
            let Some(uri) = uri.and_then(|uri| uri.to_uri().ok()) else {
                continue;
            };
            if !seen.insert(uri.clone()) {
                continue;
            }
            if rows.len() == MAX_RESULTS {
                truncated = true;
                break;
            }
            let title = display_text(
                track
                    .metadata
                    .get("title")
                    .map(String::as_str)
                    .unwrap_or(""),
            );
            let artist = display_text(
                track
                    .metadata
                    .get("artist_name")
                    .map(String::as_str)
                    .unwrap_or(""),
            );
            let duration_ms = track
                .metadata
                .get("duration")
                .or_else(|| track.metadata.get("duration_ms"))
                .and_then(|duration| duration.parse::<u32>().ok())
                .unwrap_or(0);
            rows.push(TrackRow {
                title_kind: if title.is_empty() {
                    TrackTitleKind::Loading
                } else {
                    TrackTitleKind::Metadata
                },
                uri,
                title,
                artist,
                duration_ms,
            });
        }
    }
    Ok((rows, truncated))
}

pub fn display_text(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(256)
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use librespot_protocol::{context_page::ContextPage, context_track::ContextTrack};

    fn context(uris: &[&str]) -> Context {
        let mut context = Context::new();
        let mut page = ContextPage::new();
        page.tracks = uris
            .iter()
            .map(|uri| {
                let mut track = ContextTrack::new();
                track.set_uri((*uri).into());
                track
            })
            .collect();
        context.pages.push(page);
        context
    }

    #[test]
    fn query_is_bounded_normalized_and_encoded_as_one_path_component() {
        let query = normalize_query("  été\nAC/DC + ?#&% ").unwrap();
        assert_eq!(query, "été AC/DC + ?#&%");
        assert_eq!(
            search_uri(&query),
            "spotify:search:%C3%A9t%C3%A9+AC%2FDC+%2B+%3F%23%26%25"
        );
        assert!(normalize_query(&"é".repeat(257)).is_err());
        assert!(normalize_query(&"😀".repeat(256)).is_ok());
        assert_eq!(normalize_query(" \n\t ").unwrap(), "");
    }

    #[test]
    fn results_keep_order_deduplicate_and_reject_non_music_or_injected_uris() {
        let mut data = context(&[
            "spotify:track:0000000000000000000002",
            "spotify:episode:0000000000000000000001",
            "spotify:track:0000000000000000000001",
            "spotify:track:0000000000000000000002",
            "spotify:track:0000000000000000000003?query=bad",
            "spotify:track:bad",
        ]);
        let first = &mut data.pages[0].tracks[0];
        first
            .metadata
            .insert("title".into(), "  Vrai\ntitre été ".into());
        first
            .metadata
            .insert("artist_name".into(), "Artiste".into());
        first.metadata.insert("duration".into(), "123456".into());
        let (rows, truncated) = parse_context(data).unwrap();
        assert!(!truncated);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].title, "Vrai titre été");
        assert_eq!(rows[0].artist, "Artiste");
        assert_eq!(rows[0].duration_ms, 123456);
        assert_eq!(rows[1].title_kind, TrackTitleKind::Loading);
        assert_eq!(rows[1].uri, "spotify:track:0000000000000000000001");
    }

    #[test]
    fn gid_only_results_and_unloaded_pages_are_supported_without_following_urls() {
        let mut data = context(&[""]);
        data.pages[0].tracks[0].set_gid(vec![0; 16]);
        data.pages[0].set_next_page_url("https://untrusted.invalid/ignored".into());
        let (rows, truncated) = parse_context(data).unwrap();
        assert!(truncated);
        assert_eq!(rows[0].uri, "spotify:track:0000000000000000000000");
    }

    #[test]
    fn result_count_and_decoded_response_size_are_bounded() {
        let ids: Vec<_> = (0..60)
            .map(|id| format!("spotify:track:{}", SpotifyId { id }.to_base62().unwrap()))
            .collect();
        let (rows, truncated) =
            parse_context(context(&ids.iter().map(String::as_str).collect::<Vec<_>>())).unwrap();
        assert_eq!(rows.len(), MAX_RESULTS);
        assert!(truncated);
        assert_eq!(rows[49].uri, ids[49]);
        let mut data = context(&[]);
        data.metadata
            .insert("oversize".into(), "x".repeat(MAX_CONTEXT_BYTES as usize));
        assert!(parse_context(data).is_err());
        assert_eq!(parse_context(context(&[])).unwrap(), (vec![], false));
    }
}
