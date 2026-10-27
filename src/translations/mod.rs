//! Compiled, immutable catalogues. Looking up a translated label borrows static
//! text and requires no parser, allocation, filesystem access or locale lock.
use crate::locale::Language;

mod messages;
mod ui;
mod updater;

type Entry = (&'static str, [&'static str; 4]);

pub fn lookup(english: &str, language: Language) -> Option<&'static str> {
    let column = match language {
        Language::German => 0,
        Language::Spanish => 1,
        Language::Italian => 2,
        Language::Portuguese => 3,
        Language::French | Language::English => return None,
    };
    for entries in [ui::ENTRIES, messages::ENTRIES, updater::ENTRIES] {
        if let Ok(index) = entries.binary_search_by_key(&english, |entry| entry.0) {
            return Some(entries[index].1[column]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalogues_are_sorted_unique_complete_and_resolve_every_language() {
        let mut seen = BTreeSet::new();
        for entries in [ui::ENTRIES, messages::ENTRIES, updater::ENTRIES] {
            for pair in entries.windows(2) {
                assert!(
                    pair[0].0 < pair[1].0,
                    "Unsorted or duplicated key: {}",
                    pair[1].0
                );
            }
            for (key, translations) in entries {
                assert!(seen.insert(key), "Duplicated catalogue key: {key}");
                for (language, translation) in Language::ALL[2..].iter().zip(translations) {
                    assert!(
                        !translation.trim().is_empty(),
                        "Empty {language:?} translation: {key}"
                    );
                    assert_eq!(lookup(key, *language), Some(*translation));
                }
            }
        }
    }

    #[test]
    fn unknown_keys_fall_back_without_changing_existing_french_and_english_text() {
        let french = String::from("texte local inconnu");
        let english = String::from("unknown local text");
        for language in Language::ALL {
            let result = language.text(&french, &english);
            assert_eq!(
                result,
                if language == Language::French {
                    &french
                } else {
                    &english
                }
            );
            assert!(std::ptr::eq(
                result.as_ptr(),
                if language == Language::French {
                    french.as_ptr()
                } else {
                    english.as_ptr()
                }
            ));
        }
    }

    #[test]
    fn bitmap_labels_fit_fixed_controls_and_supported_glyphs() {
        // The native window title is drawn by the OS; all other catalogue UI
        // labels use the existing BASIC_FONTS + LATIN_FONTS bitmap renderer.
        for (key, translations) in ui::ENTRIES {
            if *key == "Spotiamp — Spotify Library" {
                continue;
            }
            for text in translations {
                assert!(
                    text.chars()
                        .all(|c| (' '..='~').contains(&c) || ('\u{a0}'..='\u{ff}').contains(&c)),
                    "Unsupported bitmap glyph in {key}: {text}"
                );
            }
        }
        for (key, max_chars) in [
            ("Credits", 11),
            ("Shortcuts", 11),
            ("0.2.17 / RIGHT-CLICK MENU", 56),
            ("0.2.18 / REORDER TRACKS", 56),
            ("0.2.19 / QUEUE MULTI-SELECTION", 56),
            ("Select several queue tracks and drag them together.", 56),
            ("Ctrl/Cmd-click, Shift-click and Ctrl/Cmd+A.", 56),
            ("Drag tracks to reorder the queue or your playlists.", 56),
            ("Playback continues; drop at the insertion marker.", 56),
            ("Right-click a track to open its actions.", 56),
            ("Selection preserved when clicking a selected track.", 56),
            ("0.2.16 / SEND TO MENU", 56),
            ("Play, queue or manage your playlists.", 56),
            ("One menu for the selected tracks.", 56),
            ("0.2.15 / REMOVE PLAYLIST TRACKS", 56),
            ("Delete removes selected tracks from your playlist.", 56),
            ("Playback and queue are preserved.", 56),
            ("0.2.14 / IMMEDIATE UPDATE CHECK", 56),
            ("Opening Updates checks for a new version immediately.", 56),
            ("Downloads and ready-to-install updates are preserved.", 56),
            ("0.2.13 / NEW PLAYLISTS", 56),
            ("Create a Spotify playlist from To playlist.", 56),
            (
                "Name and selected tracks, without interrupting playback.",
                56,
            ),
            ("History", 11),
            ("Updates", 11),
            ("+ ADD", 6),
            ("PASTE", 5),
            ("LENGTH", 7),
            ("CLOSE", 11),
            ("COPY INFO", 20),
            ("GITHUB PROJECT", 22),
            ("INSTALL / RESTART", 24),
            ("Play / pause (outside text input)", 35),
            ("Current version:", 40),
            ("SEND TO...", 19),
            ("Play selection", 38),
            ("Add to queue", 38),
            ("To a playlist...", 38),
            ("New playlist...", 38),
            ("Remove from this playlist", 38),
            ("TO PLAYLIST...", 19),
            ("ADD TO A PLAYLIST", 46),
            ("selected tracks", 30),
            ("No playlists available.", 44),
            ("NEW...", 14),
            ("NEW PLAYLIST", 46),
            ("Playlist name", 46),
            ("100 characters maximum.", 46),
            ("CREATE", 11),
            ("BACK", 11),
            ("ADD", 12),
            ("CANCEL", 12),
            ("Search", 24),
            ("SEARCH", 9),
            ("CLEAR", 10),
            ("Spotify search", 60),
            ("Enter a song or artist.", 58),
            ("Enter: search    Ctrl+F: search", 58),
            ("Searching...", 58),
            ("No results.", 58),
            ("results", 24),
        ] {
            for language in Language::ALL[2..].iter().copied() {
                let text = lookup(key, language).unwrap();
                assert!(
                    text.chars().count() <= max_chars,
                    "{language:?} {key} exceeds {max_chars} characters: {text}"
                );
            }
        }
        for key in [
            "TITLE / ARTIST",
            "TIME",
            "TRACKS",
            "PLAY QUEUE",
            "VOLUME",
            "CLASSIC",
        ] {
            for language in Language::ALL[2..].iter().copied() {
                let text = lookup(key, language).unwrap();
                assert!(
                    text.chars()
                        .all(|c| c.is_ascii_uppercase() || " /".contains(c)),
                    "Unsupported tiny header glyph: {text}"
                );
            }
        }
    }

    // These sources contain presentation text only. Checking literal .text()
    // pairs prevents newly added controls from silently retaining English in
    // one of the other languages. Bitmap drawing calls begin with coordinates
    // and are deliberately ignored; real metadata never enters this catalogue.
    #[test]
    fn every_literal_ui_label_has_all_four_additional_translations() {
        let sources = [
            include_str!("../main.rs"),
            include_str!("../ui.rs"),
            include_str!("../ui/skin.rs"),
            include_str!("../ui/modern_skin.rs"),
            include_str!("../ui/about_skin.rs"),
            include_str!("../library_ui.rs"),
            include_str!("../library_ui/picker.rs"),
            include_str!("../library_ui/selection.rs"),
            include_str!("../library_ui/send_menu.rs"),
            include_str!("../library_window.rs"),
            include_str!("../model.rs"),
            include_str!("../about_actions.rs"),
        ];
        let mut count = 0;
        for source in sources {
            for call in source.split(".text(").skip(1) {
                let Some((_, after_french)) = literal(call) else {
                    continue;
                };
                let Some(after_comma) = after_french.trim_start().strip_prefix(',') else {
                    continue;
                };
                let Some((english, _)) = literal(after_comma) else {
                    continue;
                };
                count += 1;
                for language in Language::ALL[2..].iter().copied() {
                    assert!(
                        lookup(english, language).is_some(),
                        "Missing {language:?} UI label: {english}"
                    );
                }
            }
        }
        assert!(count >= 170, "UI catalogue coverage unexpectedly shrank");
    }

    fn literal(value: &str) -> Option<(&str, &str)> {
        let text = value.trim_start().strip_prefix('"')?;
        let mut escaped = false;
        for (index, character) in text.char_indices() {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                return Some((&text[..index], &text[index + 1..]));
            }
        }
        None
    }
}
