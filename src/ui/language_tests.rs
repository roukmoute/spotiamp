use super::*;
use minifb::Key;

#[test]
fn language_switch_reuses_pixels_and_preserves_metadata_playback_and_browsing() {
    for skin in [Skin::Classic, Skin::Modern] {
        let snapshot = Snapshot {
            connected: true,
            playing: true,
            current: Some(12),
            position_ms: 42_000,
            queue: (0..30)
                .map(|i| crate::model::TrackRow {
                    uri: format!("spotify:track:{i}"),
                    // These are real titles, even when equal to a UI message.
                    title: "Pause".into(),
                    artist: "Chargement...".into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let original = snapshot.clone();
        let mut view = View {
            skin,
            selected: Some(12),
            offset: 8,
            input: "https://open.spotify.com/playlist/unchanged".into(),
            ..Default::default()
        };
        let buffer = view.pixels.as_ptr();
        view.render(&snapshot);
        let french = view.pixels.clone();
        let fingerprint = view.fingerprint(&snapshot);
        view.set_language(
            LanguagePreference::Specific(Language::English),
            Language::English,
        );
        assert_ne!(fingerprint, view.fingerprint(&snapshot));
        view.render(&snapshot);
        assert_ne!(french, view.pixels);
        // Queue metadata and selection pixels have not changed with the chrome.
        for y in 221..413 {
            assert_eq!(
                &french[y * WIDTH + 15..y * WIDTH + 535],
                &view.pixels[y * WIDTH + 15..y * WIDTH + 535]
            );
        }
        assert_eq!(buffer, view.pixels.as_ptr());
        assert_eq!(view.selected, Some(12));
        assert_eq!(view.offset, 8);
        assert_eq!(view.input, "https://open.spotify.com/playlist/unchanged");
        assert_eq!(snapshot, original);
        view.set_language(LanguagePreference::Auto, Language::French);
        view.render(&snapshot);
        assert_eq!(french, view.pixels);
    }
}

#[test]
fn language_buttons_require_a_complete_click_and_keyboard_reaches_all_choices() {
    let mut view = View::default();
    let state = Snapshot::default();
    view.open_about();
    for (index, preference) in LANGUAGE_CHOICES.into_iter().enumerate() {
        let x = 180 + index * 112;
        assert!(view.pointer_down(x, 220, &state).is_none());
        assert!(view.pointer_up(Some((30, 30)), &state).is_none());
        view.pointer_down(x, 220, &state);
        assert!(
            matches!(view.pointer_up(Some((x, 220)), &state), Some(Action::SetLanguage(value)) if value == preference)
        );
        assert!(view.pointer_up(Some((x, 220)), &state).is_none());
    }
    view.open_about();
    for _ in 0..7 {
        view.about_key(Key::Tab, false);
    }
    assert!(matches!(
        view.about_key(Key::Enter, false),
        Some(Action::SetLanguage(LanguagePreference::Specific(
            Language::English
        )))
    ));
    view.about_key(Key::Tab, true);
    assert!(matches!(
        view.about_key(Key::Space, false),
        Some(Action::SetLanguage(LanguagePreference::Specific(
            Language::French
        )))
    ));
    view.about_key(Key::Tab, true);
    assert!(matches!(
        view.about_key(Key::Enter, false),
        Some(Action::SetLanguage(LanguagePreference::Auto))
    ));
    view.about = Some(AboutTab::Credits);
    assert!(view.click(400, 220, &state).is_none());
}

#[test]
fn existing_status_and_update_tooltip_change_language_without_new_events() {
    let mut view = View::default();
    let state = Snapshot::default();
    view.notice(LocalizedText::new("Information", "Information in English"));
    view.update = UpdateStatus::Available {
        version: "1.0.0".into(),
    };
    assert_eq!(view.about_tooltip(), "Mise à jour disponible (F2)");
    view.set_language(
        LanguagePreference::Specific(Language::English),
        Language::English,
    );
    assert_eq!(view.about_tooltip(), "Update available (F2)");
    assert_eq!(
        state.status.text(view.language),
        "Connect Spotify to get started."
    );
    assert_eq!(
        view.transient.as_ref().unwrap().0.text(view.language),
        "Information in English"
    );
}
