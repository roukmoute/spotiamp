use super::*;

#[test]
fn language_choice_is_modal_and_does_not_trigger_playback_or_edit_the_url() {
    let mut view = ui::View::default();
    view.input = "keep this URL".into();
    view.open_about();
    let events = [
        MouseInput::Button {
            button: MouseButton::Left,
            down: true,
            x: 420.,
            y: 220.,
        },
        MouseInput::Button {
            button: MouseButton::Left,
            down: false,
            x: 420.,
            y: 220.,
        },
    ];
    let mut actions = Vec::new();
    process_frame_input(
        &mut view,
        &model::Snapshot {
            connected: true,
            playing: true,
            ..Default::default()
        },
        FrameInput {
            active: true,
            mouse: &events,
            chars: &['x'],
            ..Default::default()
        },
        &mut actions,
    );
    assert!(matches!(
        actions.as_slice(),
        [ui::Action::SetLanguage(LanguagePreference::Specific(
            Language::English
        ))]
    ));
    assert_eq!(view.input, "keep this URL");
    assert_eq!(view.about, Some(ui::AboutTab::Player));
}

#[test]
fn command_line_language_override_is_optional_and_rejects_invalid_values() {
    assert_eq!(requested_language(&[]), Ok(None));
    for (value, preference) in [
        ("auto", LanguagePreference::Auto),
        ("fr", LanguagePreference::Specific(Language::French)),
        ("en", LanguagePreference::Specific(Language::English)),
    ] {
        assert_eq!(
            requested_language(&["--language".into(), value.into()]),
            Ok(Some(preference))
        );
    }
    for args in [
        vec!["--language".into()],
        vec!["--language".into(), "unknown".into()],
    ] {
        assert!(requested_language(&args).is_err());
    }
    assert!(player_title(Language::English).ends_with(" — Spotify player"));
    assert!(player_title(Language::French).ends_with(" — lecteur Spotify"));
}
