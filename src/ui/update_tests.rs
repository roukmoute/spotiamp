use super::*;
use minifb::Key;

fn release(view: &mut View, x: usize, y: usize, state: &Snapshot) -> Option<Action> {
    assert!(view.pointer_down(x, y, state).is_none());
    view.pointer_up(Some((x, y)), state)
}

fn states() -> Vec<UpdateStatus> {
    vec![
        UpdateStatus::Disabled,
        UpdateStatus::Idle,
        UpdateStatus::Checking,
        UpdateStatus::Current,
        UpdateStatus::Available {
            version: "0.1.0-alpha.7".into(),
        },
        UpdateStatus::Downloading { percent: 42 },
        UpdateStatus::Ready {
            version: "0.1.0-alpha.7".into(),
        },
        UpdateStatus::Installing,
        UpdateStatus::Failed("Connexion interrompue".into()),
    ]
}

#[test]
fn about_opens_updates_when_attention_or_download_is_pending() {
    for update in states() {
        let expected = if matches!(
            update,
            UpdateStatus::Available { .. }
                | UpdateStatus::Downloading { .. }
                | UpdateStatus::Ready { .. }
                | UpdateStatus::Failed(_)
        ) {
            AboutTab::Updates
        } else {
            AboutTab::Player
        };
        let mut view = View {
            update,
            ..View::default()
        };
        view.open_about();
        assert_eq!(view.about, Some(expected));
        assert_eq!(view.about_focus, expected as usize);
    }
}

#[test]
fn update_actions_follow_state_for_mouse_and_keyboard() {
    let state = Snapshot::default();
    for update in states() {
        let can_check = matches!(
            update,
            UpdateStatus::Idle
                | UpdateStatus::Current
                | UpdateStatus::Available { .. }
                | UpdateStatus::Ready { .. }
                | UpdateStatus::Failed(_)
        );
        let mut view = View {
            update: update.clone(),
            about: Some(AboutTab::Updates),
            ..View::default()
        };
        assert_eq!(
            matches!(
                release(&mut view, 100, 325, &state),
                Some(Action::CheckUpdates)
            ),
            can_check
        );
        view.about_focus = 8;
        assert_eq!(
            matches!(
                view.about_key(Key::Enter, false),
                Some(Action::CheckUpdates)
            ),
            can_check
        );

        for keyboard in [false, true] {
            let action = if keyboard {
                view.about_focus = 9;
                view.about_key(Key::Space, false)
            } else {
                release(&mut view, 400, 325, &state)
            };
            match update {
                UpdateStatus::Available { .. } => {
                    assert!(matches!(action, Some(Action::DownloadUpdate)))
                }
                UpdateStatus::Ready { .. } => {
                    assert!(matches!(action, Some(Action::InstallUpdate)))
                }
                _ => assert!(action.is_none()),
            }
        }
        assert_eq!(view.about, Some(AboutTab::Updates));
        assert_eq!(view.update, update); // UI requests work; it never invents success.
    }
}

#[test]
fn update_click_capture_cancels_on_exit_focus_loss_or_changed_action() {
    let state = Snapshot::default();
    let mut view = View {
        about: Some(AboutTab::Updates),
        update: UpdateStatus::Available {
            version: "0.1.0-alpha.7".into(),
        },
        ..View::default()
    };
    view.pointer_down(400, 325, &state);
    assert!(view.button_down(286, 315));
    assert!(view.pointer_up(Some((10, 10)), &state).is_none());
    view.pointer_down(400, 325, &state);
    view.set_focus(false);
    view.set_focus(true);
    assert!(view.pointer_up(Some((400, 325)), &state).is_none());
    view.pointer_down(400, 325, &state);
    view.update = UpdateStatus::Ready {
        version: "0.1.0-alpha.7".into(),
    };
    assert!(view.pointer_up(Some((400, 325)), &state).is_none());
    assert!(matches!(
        release(&mut view, 400, 325, &state),
        Some(Action::InstallUpdate)
    ));
    view.pointer_down(100, 325, &state);
    view.update = UpdateStatus::Checking;
    assert!(view.pointer_up(Some((100, 325)), &state).is_none());
    view.update = UpdateStatus::Available {
        version: "0.1.0-alpha.7".into(),
    };
    view.pointer_down(400, 325, &state);
    view.close_about();
    assert!(view.pointer_up(Some((400, 325)), &state).is_none());
}

#[test]
fn update_buttons_exist_only_on_update_tab_and_focus_cycle_includes_them() {
    let state = Snapshot::default();
    let mut view = View {
        update: UpdateStatus::Ready {
            version: "0.1.0-alpha.7".into(),
        },
        ..View::default()
    };
    for tab in AboutTab::ALL {
        if tab == AboutTab::Updates {
            continue;
        }
        view.about = Some(tab);
        assert!(release(&mut view, 100, 325, &state).is_none());
        assert!(release(&mut view, 400, 325, &state).is_none());
        view.about_focus = 7;
        view.about_key(Key::Tab, false);
        assert_eq!(view.about_focus, 0);
    }
    view.about = Some(AboutTab::Updates);
    view.about_focus = 7;
    view.about_key(Key::Tab, false);
    assert_eq!(view.about_focus, 8);
    view.about_key(Key::Tab, false);
    assert_eq!(view.about_focus, 9);
    view.about_key(Key::Tab, false);
    assert_eq!(view.about_focus, 0);
    view.about_key(Key::Tab, true);
    assert_eq!(view.about_focus, 9);
    view.about_key(Key::Left, false);
    assert_eq!(view.about, Some(AboutTab::History));
    assert_eq!(view.about_focus, 3);
}

#[test]
fn update_progress_redraws_existing_buffer_and_leaves_audio_status_intact() {
    let state = Snapshot {
        status: "Lecture en cours".into(),
        ..Snapshot::default()
    };
    let mut view = View {
        update: UpdateStatus::Idle,
        ..View::default()
    };
    view.render(&state);
    let pointer = view.pixels.as_ptr();
    let audio_status = view.pixels[WIDTH * 443..].to_vec();
    let mut fingerprint = view.fingerprint(&state);
    for update in states() {
        view.update = update;
        assert_ne!(fingerprint, view.fingerprint(&state));
        fingerprint = view.fingerprint(&state);
        view.render(&state);
        assert_eq!(&view.pixels[WIDTH * 443..], audio_status.as_slice());
        view.about = Some(AboutTab::Updates);
        view.render(&state);
        assert_eq!(view.pixels.as_ptr(), pointer);
        view.about = None;
    }
    // Defensive rendering bounds for an invalid percentage or long Unicode error.
    view.about = Some(AboutTab::Updates);
    for update in [
        UpdateStatus::Downloading { percent: 255 },
        UpdateStatus::Failed("Échec réseau. ".repeat(1000).into()),
    ] {
        view.update = update;
        view.render(&state);
        assert_eq!(view.pixels.as_ptr(), pointer);
        assert_eq!(view.pixels.len(), WIDTH * HEIGHT);
    }
}

fn attention_states() -> [UpdateStatus; 2] {
    [
        UpdateStatus::Available {
            version: "9.9.9".into(),
        },
        UpdateStatus::Ready {
            version: "9.9.9".into(),
        },
    ]
}

fn player_fixture() -> Snapshot {
    Snapshot {
        connected: true,
        playing: true,
        current: Some(0),
        position_ms: 74_000,
        duration_ms: 241_000,
        status: "Lecture en cours : le statut audio reste visible.".into(),
        queue: vec![crate::model::TrackRow {
            title: "Un titre dans votre file".into(),
            artist: "Aperçu de démonstration".into(),
            duration_ms: 241_000,
            ..crate::model::TrackRow::default()
        }],
        ..Snapshot::default()
    }
}

fn assert_unchanged_outside_about_button(before: &[u32], after: &[u32]) {
    let (left, top, width, height) = Button::About.rect();
    assert_eq!(before.len(), after.len());
    for (index, (&old, &new)) in before.iter().zip(after).enumerate() {
        let (x, y) = (index % WIDTH, index / WIDTH);
        if !(left..left + width).contains(&x) || !(top..top + height).contains(&y) {
            assert_eq!(old, new, "animation changed another control at ({x},{y})");
        }
    }
}

#[test]
fn update_indicator_changes_only_at_half_cycles_and_only_inside_about_button() {
    let state = player_fixture();
    for update in attention_states() {
        let mut view = View {
            update: update.clone(),
            ..View::default()
        };
        let buffer = view.pixels.as_ptr();
        view.tick_update_indicator(Duration::ZERO);
        view.render(&state);
        let green_pixels = view.pixels.clone();
        let green_fingerprint = view.fingerprint(&state);
        view.tick_update_indicator(Duration::from_millis(750));
        view.render(&state);
        let gold_pixels = view.pixels.clone();
        let gold_fingerprint = view.fingerprint(&state);
        assert_ne!(green_pixels, gold_pixels);
        assert_ne!(green_fingerprint, gold_fingerprint);
        assert_unchanged_outside_about_button(&green_pixels, &gold_pixels);

        // Absolute timestamps include both sides of thresholds and a skipped
        // whole cycle: elapsed time alone must not invalidate the framebuffer.
        for (millis, lit) in [
            (0, true),
            (1, true),
            (749, true),
            (750, false),
            (751, false),
            (1499, false),
            (1500, true),
            (2249, true),
            (2250, false),
            (3000, true),
            (3750, false),
            (60_000, true),
            (60_001, true),
        ] {
            view.tick_update_indicator(Duration::from_millis(millis));
            assert_eq!(
                view.fingerprint(&state),
                if lit {
                    green_fingerprint
                } else {
                    gold_fingerprint
                }
            );
            view.render(&state);
            assert_eq!(&view.pixels, if lit { &green_pixels } else { &gold_pixels });
            assert_eq!(
                view.pixels[121 * WIDTH + 318],
                if lit { GREEN } else { 0xe7c969 }
            );
            assert_eq!(view.pixels.as_ptr(), buffer);
            assert_eq!(view.update, update);
        }
    }
}

#[test]
fn update_indicator_stops_for_non_actionable_states_and_has_no_idle_redraws() {
    let state = player_fixture();
    for update in states().into_iter().filter(|status| {
        !matches!(
            status,
            UpdateStatus::Available { .. } | UpdateStatus::Ready { .. }
        )
    }) {
        let mut view = View {
            update: attention_states()[0].clone(),
            ..View::default()
        };
        view.tick_update_indicator(Duration::ZERO);
        view.render(&state);
        assert_eq!(view.pixels[121 * WIDTH + 318], GREEN);
        view.update = update;
        view.tick_update_indicator(Duration::ZERO);
        view.render(&state);
        let pixels = view.pixels.clone();
        let fingerprint = view.fingerprint(&state);
        let buffer = view.pixels.as_ptr();
        assert_eq!(view.pixels[121 * WIDTH + 318], 0xe7c969);
        for millis in [1, 749, 750, 1499, 1500, 60_000] {
            view.tick_update_indicator(Duration::from_millis(millis));
            assert_eq!(view.fingerprint(&state), fingerprint);
            view.render(&state);
            assert_eq!(view.pixels, pixels);
            assert_eq!(view.pixels.as_ptr(), buffer);
        }
    }
}

#[test]
fn update_indicator_and_tooltip_are_suspended_behind_about_and_help() {
    let state = player_fixture();
    for update in attention_states() {
        for about in [false, true] {
            let mut view = View {
                update: update.clone(),
                ..View::default()
            };
            view.tick_update_indicator(Duration::ZERO);
            if about {
                view.open_about();
            } else {
                view.help = true;
            }
            view.render(&state);
            let pixels = view.pixels.clone();
            let fingerprint = view.fingerprint(&state);
            let buffer = view.pixels.as_ptr();
            view.pointer_move(Some((318, 126)));
            for millis in [0, 749, 750, 1499, 1500, 60_000] {
                view.tick_update_indicator(Duration::from_millis(millis));
                assert_eq!(view.fingerprint(&state), fingerprint);
                view.render(&state);
                assert_eq!(view.pixels, pixels);
                assert_eq!(view.pixels.as_ptr(), buffer);
            }
            if about {
                view.close_about();
            } else {
                view.help = false;
            }
            view.pointer_move(None);
            view.tick_update_indicator(Duration::from_millis(60_000));
            view.render(&state);
            assert_eq!(view.pixels[121 * WIDTH + 318], GREEN);
            view.tick_update_indicator(Duration::from_millis(60_750));
            view.render(&state);
            assert_eq!(view.pixels[121 * WIDTH + 318], 0xe7c969);
        }
    }
}

#[test]
fn update_tooltip_is_explicit_fits_and_click_opens_the_matching_update_actions() {
    let state = player_fixture();
    for (update, text, install) in [
        (
            attention_states()[0].clone(),
            "Mise à jour disponible (F2)",
            false,
        ),
        (
            attention_states()[1].clone(),
            "Mise à jour à installer (F2)",
            true,
        ),
    ] {
        let mut view = View {
            update,
            ..View::default()
        };
        view.tick_update_indicator(Duration::ZERO);
        view.render(&state);
        let idle_fingerprint = view.fingerprint(&state);
        assert_eq!(view.about_tooltip(), text);
        assert!(text.chars().count() <= 31);
        view.pointer_move(Some((318, 126)));
        let hovered = view.fingerprint(&state);
        assert_ne!(hovered, idle_fingerprint);
        view.render(&state);
        // Independently draw the expected text and compare the complete glyph
        // strip: a correct helper alone does not prove the tooltip is visible.
        let mut expected = View::default();
        expected.rect(292, 146, 248, 8, 0xefe7bc);
        expected.text(292, 146, text, SHADOW, 31);
        for y in 146..154 {
            assert_eq!(
                &view.pixels[y * WIDTH + 292..y * WIDTH + 540],
                &expected.pixels[y * WIDTH + 292..y * WIDTH + 540]
            );
        }
        view.pointer_move(Some((319, 127)));
        assert_eq!(view.fingerprint(&state), hovered);
        view.pointer_move(None);
        assert_eq!(view.fingerprint(&state), idle_fingerprint);
        assert!(release(&mut view, 318, 126, &state).is_none());
        assert_eq!(view.about, Some(AboutTab::Updates));
        let action = release(&mut view, 400, 325, &state);
        assert!(matches!(
            (install, action),
            (false, Some(Action::DownloadUpdate)) | (true, Some(Action::InstallUpdate))
        ));
    }
}

#[test]
fn blinking_about_button_keeps_pressed_relief_and_capture_cancellation() {
    let state = player_fixture();
    for update in attention_states() {
        for millis in [0, 750] {
            let mut view = View {
                update: update.clone(),
                ..View::default()
            };
            view.tick_update_indicator(Duration::from_millis(millis));
            view.render(&state);
            let unpressed = view.pixels.clone();
            assert!(view.pointer_down(318, 126, &state).is_none());
            assert!(view.button_down(308, 116));
            view.render(&state);
            // Compare only the metal edge so tooltip/bolt color cannot make
            // this pass if the button loses its visible pressed bevel.
            assert_ne!(
                &view.pixels[117 * WIDTH + 309..117 * WIDTH + 329],
                &unpressed[117 * WIDTH + 309..117 * WIDTH + 329]
            );
            view.tick_update_indicator(Duration::from_millis(millis + 750));
            view.render(&state);
            assert!(view.button_down(308, 116));
            assert!(view.pointer_up(Some((318, 126)), &state).is_none());
            assert_eq!(view.about, Some(AboutTab::Updates));
            view.close_about();
            view.pointer_down(318, 126, &state);
            view.pointer_up(Some((300, 126)), &state);
            assert!(view.about.is_none());
            view.pointer_down(318, 126, &state);
            view.set_focus(false);
            view.set_focus(true);
            view.pointer_up(Some((318, 126)), &state);
            assert!(view.about.is_none());
        }
    }
}

#[test]
#[ignore = "Exports update state previews only when SPOTIAMP_UI_PREVIEW_DIR is set"]
fn export_update_previews() {
    let directory = std::env::var_os("SPOTIAMP_UI_PREVIEW_DIR").expect("preview output directory");
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let state = Snapshot {
        status: "Lecture en cours : le statut audio reste visible.".into(),
        ..Snapshot::default()
    };
    let mut view = View::default();
    for (name, update) in [
        ("available", UpdateStatus::Available { version: "0.1.0-alpha.7".into() }),
        ("downloading", UpdateStatus::Downloading { percent: 42 }),
        ("ready", UpdateStatus::Ready { version: "0.1.0-alpha.7".into() }),
        ("failed", UpdateStatus::Failed("Échec du téléchargement : connexion interrompue. Réessayez après avoir vérifié votre accès réseau.".into())),
    ] {
        view.update = update;
        view.about = Some(AboutTab::Updates);
        view.about_focus = AboutTab::Updates as usize;
        view.render(&state);
        view.save_ppm(directory.join(format!("{name}.ppm")).to_str().unwrap()).unwrap();
    }
    view.update = UpdateStatus::Available {
        version: "0.1.0-alpha.7".into(),
    };
    view.about = None;
    view.tick_update_indicator(Duration::ZERO);
    view.render(&state);
    view.save_ppm(directory.join("notification.ppm").to_str().unwrap())
        .unwrap();

    let state = player_fixture();
    for (name, update) in ["available", "ready"].into_iter().zip(attention_states()) {
        for (variant, millis) in [
            ("green", 0),
            ("gold", 750),
            ("tooltip", 0),
            ("pressed", 750),
        ] {
            let mut view = View {
                update: update.clone(),
                ..View::default()
            };
            view.tick_update_indicator(Duration::from_millis(millis));
            if variant == "tooltip" {
                view.pointer_move(Some((318, 126)));
            }
            if variant == "pressed" {
                view.pointer_down(318, 126, &state);
            }
            view.render(&state);
            view.save_ppm(
                directory
                    .join(format!("indicator-{name}-{variant}.ppm"))
                    .to_str()
                    .unwrap(),
            )
            .unwrap();
        }
    }
}
