//! Read-only playlist catalogue over the existing Spotify session. Retain only
//! small display rows; tracks and artwork are deliberately not fetched here.
use std::{collections::HashSet, time::Duration};

use librespot_core::{Session, SpotifyId};
use librespot_protocol::playlist4_external::SelectedListContent;
use protobuf::Message;

use crate::{
    localized_text::LocalizedText,
    model::{PlaylistEntry, PlaylistId},
};

const PAGE_SIZE: usize = 120;
const MAX_PLAYLISTS: usize = 1000;
const MAX_RAW_ITEMS: usize = 2000;
const MAX_PAGE_BYTES: usize = 2 * 1024 * 1024;
const MAX_FOLDER_DEPTH: usize = 32;
const MAX_FOLDER_PATH_CHARS: usize = 256;
const PLAYLIST_NAME_MAX_CHARS: usize = 100;
const PAGE_TIMEOUT: Duration = Duration::from_secs(20);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn fetch(session: Session) -> Result<Vec<PlaylistEntry>, LocalizedText> {
    tokio::time::timeout(TOTAL_TIMEOUT, async {
        let mut catalogue = Catalogue::default();
        loop {
            let bytes = tokio::time::timeout(
                PAGE_TIMEOUT,
                session
                    .spclient()
                    .get_rootlist(catalogue.offset, Some(PAGE_SIZE)),
            )
            .await
            .map_err(|_| LocalizedText::from("Spotify ne repond pas au chargement des playlists"))?
            .map_err(|_| LocalizedText::from("Impossible de recuperer vos playlists Spotify"))?;
            if catalogue.push_bytes(&bytes)? {
                return Ok(catalogue.entries);
            }
        }
    })
    .await
    .map_err(|_| LocalizedText::from("Le chargement des playlists Spotify a depasse une minute"))?
}

#[derive(Default)]
struct Catalogue {
    entries: Vec<PlaylistEntry>,
    seen: HashSet<String>,
    folders: Vec<String>,
    offset: usize,
    revision: Option<Vec<u8>>,
    length: Option<usize>,
}

impl Catalogue {
    /// Returns true only when the complete advertised list has been consumed.
    fn push_bytes(&mut self, bytes: &[u8]) -> Result<bool, LocalizedText> {
        // SpClient already buffered the HTTP body. This bounds decoding and
        // retained data, not the transport's peak allocation for a bad response.
        if bytes.len() > MAX_PAGE_BYTES {
            return Err("Reponse Spotify trop volumineuse pour la bibliotheque".into());
        }
        let page = SelectedListContent::parse_from_bytes(bytes)
            .map_err(|_| LocalizedText::from("Reponse de bibliotheque Spotify illisible"))?;
        let revision = page.revision();
        if let Some(expected) = &self.revision {
            if revision != expected {
                return Err("Vos playlists ont change pendant le chargement; actualisez".into());
            }
        } else if !revision.is_empty() {
            self.revision = Some(revision.to_vec());
        }

        if page.has_length() {
            let length = usize::try_from(page.length())
                .map_err(|_| LocalizedText::from("Longueur de bibliotheque Spotify invalide"))?;
            if self.length.is_some_and(|previous| previous != length) {
                return Err("Vos playlists ont change pendant le chargement; actualisez".into());
            }
            if length > MAX_RAW_ITEMS {
                return Err("Bibliotheque Spotify limitee a 2000 entrees, dossiers compris".into());
            }
            self.length = Some(length);
        }

        let Some(contents) = page.contents.as_ref() else {
            if self.offset == 0 && self.length.unwrap_or(0) == 0 {
                return Ok(true);
            }
            return Err("Spotify a renvoye une bibliotheque incomplete; actualisez".into());
        };
        if usize::try_from(contents.pos()).ok() != Some(self.offset) {
            return Err("Spotify a renvoye une page de playlists desordonnee".into());
        }
        if contents.items.len() != contents.meta_items.len() {
            return Err("Les noms des playlists Spotify ne correspondent pas aux entrees".into());
        }
        let next = self
            .offset
            .checked_add(contents.items.len())
            .filter(|next| *next <= MAX_RAW_ITEMS)
            .ok_or("Bibliotheque Spotify limitee a 2000 entrees, dossiers compris")?;
        if self.length.is_some_and(|length| next > length) {
            return Err("Longueur de bibliotheque Spotify incoherente".into());
        }
        if contents.truncated() {
            if next == self.offset || self.length.is_some_and(|length| next >= length) {
                return Err("Spotify a renvoye une page incomplete sans progression".into());
            }
            if self.revision.is_none() {
                return Err("Spotify ne fournit pas la revision de vos playlists".into());
            }
            if next == MAX_RAW_ITEMS {
                return Err("Bibliotheque Spotify limitee a 2000 entrees, dossiers compris".into());
            }
        } else if self.length.is_some_and(|length| next < length) {
            return Err("Spotify a renvoye une bibliotheque incomplete; actualisez".into());
        }

        for (item, metadata) in contents.items.iter().zip(&contents.meta_items) {
            let uri = item.uri();
            if let Some(marker) = uri.strip_prefix("spotify:start-group:") {
                let (id, name) = marker.split_once(':').unwrap_or((marker, ""));
                if id.is_empty() {
                    return Err("Dossier Spotify sans identifiant".into());
                }
                if self.folders.len() == MAX_FOLDER_DEPTH {
                    return Err(
                        "La bibliotheque Spotify contient trop de dossiers imbriques".into(),
                    );
                }
                let name = display_text(&decode_folder_name(name), PLAYLIST_NAME_MAX_CHARS);
                self.folders
                    .push(if name.is_empty() { "(?)".into() } else { name });
                continue;
            }
            if uri.starts_with("spotify:end-group:") {
                // An orphan closing marker must not underflow. An unclosed
                // final folder still gives its preceding rows the right path.
                self.folders.pop();
                continue;
            }
            let Some(uri) = canonical_playlist_uri(uri)? else {
                continue;
            };
            if !self.seen.insert(uri.clone()) {
                continue;
            }
            if self.entries.len() == MAX_PLAYLISTS {
                return Err("Bibliotheque Spotify limitee a 1000 playlists".into());
            }
            let attributes = metadata.attributes.get_or_default();
            let name = display_text(attributes.name(), PLAYLIST_NAME_MAX_CHARS);
            self.entries.push(PlaylistEntry {
                name: if name.is_empty() {
                    format!("Playlist {}", uri.trim_start_matches("spotify:playlist:"))
                } else {
                    name
                },
                id: PlaylistId::Spotify(uri),
                track_count: metadata
                    .has_length()
                    .then(|| usize::try_from(metadata.length()).ok())
                    .flatten(),
                duration_ms: None,
                folder: display_text(&self.folders.join(" / "), MAX_FOLDER_PATH_CHARS),
                available: matches!(metadata.status_code(), 0 | 200)
                    && !attributes.deleted_by_owner(),
            });
        }
        self.offset = next;
        Ok(!contents.truncated())
    }
}

fn canonical_playlist_uri(uri: &str) -> Result<Option<String>, LocalizedText> {
    let id = if let Some(id) = uri.strip_prefix("spotify:playlist:") {
        id
    } else if let Some(named) = uri.strip_prefix("spotify:user:") {
        let Some((owner, tail)) = named.split_once(':') else {
            return Ok(None);
        };
        let Some(id) = tail.strip_prefix("playlist:") else {
            return Ok(None);
        };
        if owner.is_empty() {
            return Err("Identifiant de playlist Spotify invalide".into());
        }
        id
    } else {
        return Ok(None);
    };
    if id.len() != 22 || !id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Err("Identifiant de playlist Spotify invalide".into());
    }
    SpotifyId::from_base62(id)
        .map_err(|_| LocalizedText::from("Identifiant de playlist Spotify invalide"))?;
    Ok(Some(format!("spotify:playlist:{id}")))
}

fn display_text(text: &str, limit: usize) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(limit)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn decode_folder_name(encoded: &str) -> String {
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut bytes = encoded.bytes();
    while let Some(byte) = bytes.next() {
        match byte {
            b'+' => decoded.push(b' '),
            b'%' => {
                let high = bytes.next();
                let low = bytes.next();
                let hex = |byte: u8| char::from(byte).to_digit(16);
                match (high.and_then(hex), low.and_then(hex)) {
                    (Some(high), Some(low)) => decoded.push(((high << 4) | low) as u8),
                    _ => {
                        decoded.push(b'%');
                        decoded.extend(high);
                        decoded.extend(low);
                    }
                }
            }
            byte => decoded.push(byte),
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use librespot_protocol::playlist4_external::{Item, ListAttributes, ListItems, MetaItem};

    const ID: &str = "37i9dQZF1DXcBWIGoYBM5M";
    const OTHER: &str = "37i9dQZF1DWSw8liJZcPOI";

    fn page(
        position: i32,
        total: Option<i32>,
        more: bool,
        rows: &[(&str, &str)],
    ) -> SelectedListContent {
        let mut page = SelectedListContent::new();
        page.set_revision(vec![7; 20]);
        if let Some(total) = total {
            page.set_length(total);
        }
        let mut contents = ListItems::new();
        contents.set_pos(position);
        contents.set_truncated(more);
        for (uri, name) in rows {
            let mut item = Item::new();
            item.set_uri((*uri).into());
            contents.items.push(item);
            let mut meta = MetaItem::new();
            let mut attributes = ListAttributes::new();
            attributes.set_name((*name).into());
            meta.attributes = protobuf::MessageField::some(attributes);
            contents.meta_items.push(meta);
        }
        page.contents = protobuf::MessageField::some(contents);
        page
    }

    fn push(catalogue: &mut Catalogue, page: &SelectedListContent) -> Result<bool, LocalizedText> {
        // Exercise the production protobuf decoder, including required fields.
        catalogue.push_bytes(&page.write_to_bytes().unwrap())
    }

    #[test]
    fn modern_and_legacy_playlists_are_canonical_and_do_not_fetch_tracks() {
        let mut data = page(
            0,
            Some(3),
            false,
            &[
                (&format!("spotify:playlist:{ID}"), " Ma playlist "),
                (&format!("spotify:user:alice:playlist:{OTHER}"), "Autre"),
                (&format!("spotify:user:alice:playlist:{ID}"), "Duplicate"),
            ],
        );
        data.contents.as_mut().unwrap().meta_items[0].set_length(42);
        let mut catalogue = Catalogue::default();
        assert!(push(&mut catalogue, &data).unwrap());
        assert_eq!(catalogue.offset, 3);
        assert_eq!(catalogue.entries.len(), 2);
        assert_eq!(catalogue.entries[0].name, "Ma playlist");
        assert_eq!(catalogue.entries[0].track_count, Some(42));
        assert_eq!(catalogue.entries[0].duration_ms, None);
        assert_eq!(
            catalogue.entries[1].id,
            PlaylistId::Spotify(format!("spotify:playlist:{OTHER}"))
        );
    }

    #[test]
    fn folders_and_offsets_survive_page_boundaries_and_encoded_unicode() {
        let mut catalogue = Catalogue::default();
        assert!(
            !push(
                &mut catalogue,
                &page(
                    0,
                    Some(6),
                    true,
                    &[
                        ("spotify:start-group:a:Fran%C3%A7ais+%26+jazz", ""),
                        ("spotify:start-group:b:%E9%9B%AA%3A%C3%A9t%C3%A9", ""),
                        (&format!("spotify:playlist:{ID}"), "Premier"),
                    ]
                )
            )
            .unwrap()
        );
        assert_eq!(catalogue.offset, 3);
        assert!(
            push(
                &mut catalogue,
                &page(
                    3,
                    Some(6),
                    false,
                    &[
                        ("spotify:end-group:b", ""),
                        (&format!("spotify:playlist:{OTHER}"), "Second"),
                        ("spotify:end-group:a", ""),
                    ]
                )
            )
            .unwrap()
        );
        assert_eq!(catalogue.entries[0].folder, "Français & jazz / 雪:été");
        assert_eq!(catalogue.entries[1].folder, "Français & jazz");
        assert!(catalogue.folders.is_empty());
    }

    #[test]
    fn several_full_pages_preserve_order_and_stop_after_the_last_partial_page() {
        let uris: Vec<_> = (1..=255)
            .map(|id| format!("spotify:playlist:{}", SpotifyId { id }.to_base62().unwrap()))
            .collect();
        let mut catalogue = Catalogue::default();
        for (index, chunk) in uris.chunks(PAGE_SIZE).enumerate() {
            let rows: Vec<_> = chunk.iter().map(|uri| (uri.as_str(), "Liste")).collect();
            let last = index == 2;
            assert_eq!(
                push(
                    &mut catalogue,
                    &page((index * PAGE_SIZE) as i32, Some(255), !last, &rows),
                )
                .unwrap(),
                last
            );
        }
        assert_eq!(catalogue.offset, 255);
        assert_eq!(catalogue.entries.len(), 255);
        for (entry, uri) in catalogue.entries.iter().zip(uris) {
            assert_eq!(entry.id, PlaylistId::Spotify(uri));
        }
    }

    #[test]
    fn unavailable_and_deleted_playlists_remain_visible_with_unknown_counts() {
        for status in [403, 404, 410, 500] {
            let mut data = page(
                0,
                Some(1),
                false,
                &[(&format!("spotify:playlist:{ID}"), "")],
            );
            let meta = &mut data.contents.as_mut().unwrap().meta_items[0];
            meta.set_status_code(status);
            meta.set_length(-1);
            let mut catalogue = Catalogue::default();
            push(&mut catalogue, &data).unwrap();
            assert!(!catalogue.entries[0].available);
            assert_eq!(catalogue.entries[0].track_count, None);
            assert!(catalogue.entries[0].name.starts_with("Playlist "));
        }
        let mut data = page(
            0,
            Some(1),
            false,
            &[(&format!("spotify:playlist:{ID}"), "Supprimee")],
        );
        data.contents.as_mut().unwrap().meta_items[0]
            .attributes
            .as_mut()
            .unwrap()
            .set_deleted_by_owner(true);
        let mut catalogue = Catalogue::default();
        push(&mut catalogue, &data).unwrap();
        assert!(!catalogue.entries[0].available);
    }

    #[test]
    fn missing_metadata_never_attaches_the_next_playlist_name() {
        let mut data = page(
            0,
            Some(2),
            false,
            &[
                (&format!("spotify:playlist:{ID}"), "A"),
                (&format!("spotify:playlist:{OTHER}"), "B"),
            ],
        );
        data.contents.as_mut().unwrap().meta_items.remove(0);
        assert!(push(&mut Catalogue::default(), &data).is_err());
    }

    #[test]
    fn revision_and_total_changes_reject_mixed_snapshots() {
        let first = page(
            0,
            Some(2),
            true,
            &[(&format!("spotify:playlist:{ID}"), "A")],
        );
        for changed_revision in [true, false] {
            let mut catalogue = Catalogue::default();
            push(&mut catalogue, &first).unwrap();
            let mut second = page(
                1,
                Some(2),
                false,
                &[(&format!("spotify:playlist:{OTHER}"), "B")],
            );
            if changed_revision {
                second.set_revision(vec![8; 20]);
            } else {
                second.set_length(3);
            }
            assert!(push(&mut catalogue, &second).is_err());
        }
        let mut no_revision = first;
        no_revision.clear_revision();
        assert!(push(&mut Catalogue::default(), &no_revision).is_err());

        let mut catalogue = Catalogue::default();
        push(
            &mut catalogue,
            &page(
                0,
                Some(2),
                true,
                &[(&format!("spotify:playlist:{ID}"), "A")],
            ),
        )
        .unwrap();
        let mut last = page(
            1,
            Some(2),
            false,
            &[(&format!("spotify:playlist:{OTHER}"), "B")],
        );
        last.clear_revision();
        assert!(push(&mut catalogue, &last).is_err());
    }

    #[test]
    fn missing_repeated_and_nonprogressing_pages_are_errors() {
        let first = page(
            0,
            Some(2),
            true,
            &[(&format!("spotify:playlist:{ID}"), "A")],
        );
        for second in [
            page(
                0,
                Some(2),
                false,
                &[(&format!("spotify:playlist:{OTHER}"), "B")],
            ),
            page(1, Some(2), true, &[]),
            page(1, Some(2), false, &[]),
        ] {
            let mut catalogue = Catalogue::default();
            push(&mut catalogue, &first).unwrap();
            assert!(push(&mut catalogue, &second).is_err());
        }
        let mut missing = page(0, Some(1), false, &[]);
        missing.contents = protobuf::MessageField::none();
        assert!(push(&mut Catalogue::default(), &missing).is_err());
    }

    #[test]
    fn empty_library_and_unadvertised_length_are_supported() {
        assert!(push(&mut Catalogue::default(), &page(0, Some(0), false, &[])).unwrap());
        let mut empty = SelectedListContent::new();
        empty.set_length(0);
        assert!(push(&mut Catalogue::default(), &empty).unwrap());
        assert!(
            push(
                &mut Catalogue::default(),
                &page(0, None, false, &[(&format!("spotify:playlist:{ID}"), "A")])
            )
            .unwrap()
        );
    }

    #[test]
    fn corrupt_protobuf_invalid_lengths_uris_and_bounds_fail_explicitly() {
        assert!(Catalogue::default().push_bytes(&[0xff]).is_err());
        assert!(
            Catalogue::default()
                .push_bytes(&vec![0; MAX_PAGE_BYTES + 1])
                .is_err()
        );
        for total in [-1, 2001] {
            assert!(push(&mut Catalogue::default(), &page(0, Some(total), false, &[])).is_err());
        }
        for uri in [
            "spotify:playlist:short",
            "spotify:playlist:ZZZZZZZZZZZZZZZZZZZZZZ",
            "spotify:user::playlist:37i9dQZF1DXcBWIGoYBM5M",
            "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M:extra",
        ] {
            assert!(
                push(
                    &mut Catalogue::default(),
                    &page(0, Some(1), false, &[(uri, "A")])
                )
                .is_err(),
                "{uri}"
            );
        }
        assert_eq!(
            canonical_playlist_uri("spotify:track:37i9dQZF1DXcBWIGoYBM5M").unwrap(),
            None
        );
    }

    #[test]
    fn playlist_and_raw_item_caps_do_not_return_silent_partial_success() {
        let mut catalogue = Catalogue {
            offset: MAX_RAW_ITEMS,
            ..Catalogue::default()
        };
        assert!(
            push(
                &mut catalogue,
                &page(
                    MAX_RAW_ITEMS as i32,
                    None,
                    false,
                    &[("spotify:end-group:a", "")]
                )
            )
            .is_err()
        );
        let mut catalogue = Catalogue {
            entries: (0..MAX_PLAYLISTS)
                .map(|_| PlaylistEntry {
                    id: PlaylistId::Spotify("already present".into()),
                    name: String::new(),
                    track_count: None,
                    duration_ms: None,
                    folder: String::new(),
                    available: true,
                })
                .collect(),
            ..Catalogue::default()
        };
        assert!(
            push(
                &mut catalogue,
                &page(
                    0,
                    Some(1),
                    false,
                    &[(&format!("spotify:playlist:{ID}"), "A")]
                )
            )
            .is_err()
        );
    }

    #[test]
    fn text_and_folder_nesting_are_bounded_without_utf8_panics() {
        assert_eq!(decode_folder_name("x%é+%FF%4"), "x%é �%4");
        assert_eq!(display_text("a\nb\0c", 5), "a b c");
        assert_eq!(display_text(&"é".repeat(500), 100).chars().count(), 100);
        let rows = vec![("spotify:start-group:a:folder", ""); MAX_FOLDER_DEPTH + 1];
        assert!(
            push(
                &mut Catalogue::default(),
                &page(0, Some(rows.len() as i32), false, &rows)
            )
            .is_err()
        );
        assert!(
            push(
                &mut Catalogue::default(),
                &page(0, Some(1), false, &[("spotify:end-group:orphan", "")])
            )
            .unwrap()
        );
    }
}
