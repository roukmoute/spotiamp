use super::*;

fn snapshot() -> Snapshot {
    Snapshot {
        connected: true,
        playing: true,
        position_ms: 31_000,
        duration_ms: 180_000,
        volume: 42_000,
        shuffle: true,
        repeat: true,
        current: Some(3),
        queue: (0..30)
            .map(|i| crate::model::TrackRow {
                uri: format!("spotify:track:{i}"),
                title: format!("Titre {i}"),
                artist: "Artiste".into(),
                duration_ms: 180_000,
            })
            .collect(),
        ..Snapshot::default()
    }
}

#[test]
fn switching_skins_reuses_pixels_and_preserves_player_and_view_state() {
    let state = snapshot();
    let mut view = View {
        input: "spotify:playlist:kept".into(),
        selected: Some(12),
        offset: 8,
        scroll_remainder: 0.4,
        ..View::default()
    };
    let pixels = view.pixels.as_ptr();
    let capacity = view.pixels.capacity();
    view.render(&state);
    let classic = view.pixels.clone();
    let classic_hash = view.fingerprint(&state);
    view.set_skin(Skin::Modern);
    assert_ne!(view.fingerprint(&state), classic_hash);
    view.render(&state);
    assert_ne!(
        view.pixels, classic,
        "Modern must be a visibly distinct skin"
    );
    let modern_hash = view.fingerprint(&state);
    view.set_skin(Skin::Modern);
    assert_eq!(view.fingerprint(&state), modern_hash, "no idle repaint");
    view.set_skin(Skin::Classic);
    view.render(&state);
    assert_eq!(view.pixels, classic, "Classic is restored exactly");
    assert_eq!(view.fingerprint(&state), classic_hash);
    assert_eq!(view.pixels.as_ptr(), pixels);
    assert_eq!(view.pixels.capacity(), capacity);
    assert_eq!(view.selected, Some(12));
    assert_eq!(view.offset, 8);
    assert_eq!(view.scroll_remainder, 0.4);
    assert_eq!(view.input, "spotify:playlist:kept");
    assert_eq!(state, snapshot());
}

#[test]
fn skin_buttons_require_matching_release_and_do_not_leak_old_captures() {
    let state = snapshot();
    let mut view = View::default();
    for (skin, x) in [(Skin::Modern, 150), (Skin::Classic, 80)] {
        view.pointer_down(x, 153, &state);
        assert!(matches!(view.pointer_up(Some((x, 153)), &state),
            Some(Action::SetSkin(actual)) if actual == skin));
        view.set_skin(skin);
        assert!(view.pointer_up(Some((x, 153)), &state).is_none());
        view.pointer_down(x, 153, &state);
        assert!(view.pointer_up(Some((210, 153)), &state).is_none());
        view.pointer_down(x, 153, &state);
        view.set_focus(false);
        view.set_focus(true);
        assert!(view.pointer_up(Some((x, 153)), &state).is_none());
    }
    view.pointer_down(70, 125, &state);
    view.set_skin(Skin::Modern);
    assert!(view.pointer_up(Some((70, 125)), &state).is_none());
    view.dragging = 1;
    view.last_click = Some((3, Instant::now()));
    view.set_skin(Skin::Classic);
    assert!(view.drag(400, 100, &state).is_none());
    assert!(view.last_click.is_none());
}

#[test]
fn both_skins_keep_the_same_transport_modes_sliders_and_quality_commands() {
    let state = snapshot();
    for skin in [Skin::Classic, Skin::Modern] {
        let mut view = View::default();
        view.set_skin(skin);
        view.render(&state);
        assert!(matches!(
            view.click(70, 125, &state),
            Some(Action::Send(Command::Play(3)))
        ));
        assert!(matches!(
            view.click(110, 125, &state),
            Some(Action::Send(Command::Toggle))
        ));
        assert!(matches!(
            view.click(150, 125, &state),
            Some(Action::Send(Command::Stop))
        ));
        assert!(matches!(
            view.click(245, 125, &state),
            Some(Action::Send(Command::ToggleShuffle))
        ));
        assert!(matches!(
            view.click(295, 125, &state),
            Some(Action::Send(Command::ToggleRepeat))
        ));
        assert!(matches!(
            view.click(280, 100, &state),
            Some(Action::Send(Command::Seek(90_000)))
        ));
        view.end_drag();
        assert!(matches!(
            view.click(543, 125, &state),
            Some(Action::Send(Command::Volume(65535)))
        ));
        view.end_drag();
        assert!(matches!(
            view.click(445, 425, &state),
            Some(Action::Send(Command::SetQuality(AudioQuality::Kbps160)))
        ));
        let pixels = view.pixels.clone();
        view.set_focus(false);
        view.render(&state);
        assert_ne!(pixels[..WIDTH * 20], view.pixels[..WIDTH * 20]);
    }
}

#[test]
fn skin_selector_is_not_clickable_through_about_or_help() {
    let state = snapshot();
    let mut view = View::default();
    for skin in [Skin::Classic, Skin::Modern] {
        view.set_skin(skin);
        view.open_about();
        view.pointer_down(150, 153, &state);
        assert!(view.pointer_up(Some((150, 153)), &state).is_none());
        assert!(view.about.is_some());
        assert_eq!(view.skin, skin);
        view.close_about();
        view.help = true;
        view.pointer_down(150, 153, &state);
        assert!(view.pointer_up(Some((150, 153)), &state).is_none());
        assert!(!view.help);
        assert_eq!(view.skin, skin);
    }
}
