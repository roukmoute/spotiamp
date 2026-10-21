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
        UpdateStatus::Failed("Échec réseau. ".repeat(1000)),
    ] {
        view.update = update;
        view.render(&state);
        assert_eq!(view.pixels.as_ptr(), pointer);
        assert_eq!(view.pixels.len(), WIDTH * HEIGHT);
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
    view.render(&state);
    view.save_ppm(directory.join("notification.ppm").to_str().unwrap())
        .unwrap();
}
