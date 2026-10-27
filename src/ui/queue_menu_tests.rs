use super::*;
use crate::model::PlaybackOrigin;
use minifb::Key;

fn queue() -> Snapshot {
    Snapshot {
        connected: true,
        queue: (0..12)
            .map(|index| TrackRow {
                uri: format!("spotify:track:{index}"),
                title: format!("Song {}", index + 1),
                artist: "Spotiamp".into(),
                duration_ms: 180_000,
                ..Default::default()
            })
            .collect(),
        current: Some(0),
        playing: true,
        queue_reorder_request: 17,
        position_ms: 42_000,
        ..Default::default()
    }
}

fn row_y(index: usize) -> usize {
    221 + index * 24 + 12
}

fn group(view: &mut View, s: &Snapshot) {
    view.choose_queue_row(1, false, false, s);
    view.choose_queue_row(3, true, false, s);
    view.choose_queue_row(5, true, false, s);
}

fn item(view: &View, index: usize) -> (usize, usize) {
    let rect = view.queue_menu.as_ref().unwrap().rect;
    (rect.0 + 12, rect.1 + 4 + index * 28 + 12)
}

fn activate(view: &mut View, index: usize, s: &Snapshot) -> Option<Action> {
    let point = item(view, index);
    assert!(view.pointer_down(point.0, point.1, s).is_none());
    view.pointer_up(Some(point), s)
}

#[test]
fn queue_context_click_preserves_chosen_group_and_selects_unchosen_occurrence_in_both_skins() {
    for skin in [Skin::Classic, Skin::Modern] {
        let s = queue();
        let mut view = View {
            skin,
            ..Default::default()
        };
        group(&mut view, &s);
        view.last_click = Some((3, Instant::now()));
        view.context_menu_at(80, row_y(3), &s);
        assert!(view.has_queue_menu());
        assert_eq!(view.selected_queue_indices(&s), [1, 3, 5]);
        assert_eq!(view.selected, Some(3));
        assert!(view.queue_drag.is_none());
        assert!(view.last_click.is_none());
        // A native right release must never be reinterpreted as a left click.
        assert!(view.pointer_up(Some(item(&view, 0)), &s).is_none());
        assert!(view.has_queue_menu());
        view.close_queue_menu();
        view.context_menu_at(80, row_y(2), &s);
        assert_eq!(view.selected_queue_indices(&s), [2]);
        assert_eq!(view.selected, Some(2));
        let Some(Action::Send(Command::PlayQueueIndex {
            expected_queue,
            expected_reorder_request,
            index,
        })) = activate(&mut view, 0, &s)
        else {
            panic!("expected explicit captured play")
        };
        assert_eq!(index, 2);
        assert_eq!(expected_reorder_request, 17);
        assert_eq!(
            expected_queue,
            s.queue
                .iter()
                .map(|row| row.uri.clone())
                .collect::<Vec<_>>()
        );
        assert!(!view.has_queue_menu());
    }
}

#[test]
fn queue_menu_group_actions_capture_exact_occurrences_and_metadata_in_queue_order() {
    let mut s = queue();
    s.queue[5].uri = s.queue[1].uri.clone();
    for action in 1..5 {
        let mut view = View::default();
        group(&mut view, &s);
        view.context_menu_at(80, row_y(3), &s);
        let original = s.queue.clone();
        s.queue[1].title = format!("Changed metadata {action}");
        let result = activate(&mut view, action, &s).unwrap();
        match result {
            Action::QueueToPlaylist { tracks, create } => {
                assert!(action == 1 || action == 2);
                assert_eq!(create, action == 2);
                assert_eq!(
                    tracks,
                    [
                        original[1].clone(),
                        original[3].clone(),
                        original[5].clone()
                    ]
                );
            }
            Action::Send(Command::EditQueue {
                expected_queue,
                expected_reorder_request,
                indices,
                keep_selected,
            }) => {
                assert!(action == 3 || action == 4);
                assert_eq!(indices, [1, 3, 5]);
                assert_eq!(keep_selected, action == 4);
                assert_eq!(expected_reorder_request, 17);
                assert_eq!(
                    expected_queue,
                    original
                        .iter()
                        .map(|row| row.uri.clone())
                        .collect::<Vec<_>>()
                );
            }
            _ => panic!("unexpected queue action"),
        }
        assert!(!view.has_queue_menu());
        assert!(view.pointer_up(Some((95, 280)), &s).is_none());
    }
}

#[test]
fn queue_menu_disabled_actions_are_inert_offline_busy_and_when_crop_has_nothing_to_remove() {
    for connected in [false, true] {
        for busy in [false, true] {
            let s = Snapshot {
                connected,
                busy,
                ..queue()
            };
            for action in 0..5 {
                let mut view = View::default();
                group(&mut view, &s);
                view.context_menu_at(80, row_y(3), &s);
                let enabled = !busy && (connected || action >= 3);
                assert_eq!(activate(&mut view, action, &s).is_some(), enabled);
                assert_eq!(view.has_queue_menu(), !enabled);
            }
        }
    }
    let s = queue();
    let mut view = View::default();
    view.select_all_queue(&s);
    view.context_menu_at(80, row_y(3), &s);
    assert!(activate(&mut view, 4, &s).is_none());
    assert!(view.has_queue_menu());
}

#[test]
fn queue_context_menu_ignores_blank_rows_chrome_help_about_and_unfocused_windows() {
    let s = Snapshot {
        queue: queue().queue[..2].to_vec(),
        ..queue()
    };
    for (x, y) in [(80, row_y(2)), (80, 195), (8, 245), (540, 245), (80, 425)] {
        let mut view = View::default();
        view.context_menu_at(x, y, &s);
        assert!(!view.has_queue_menu());
        assert_eq!(view.selected, None);
    }
    for mode in 0..3 {
        let mut view = View::default();
        match mode {
            0 => view.help = true,
            1 => view.open_about(),
            _ => view.set_focus(false),
        }
        view.context_menu_at(80, row_y(0), &s);
        assert!(!view.has_queue_menu());
    }
}

#[test]
fn queue_menu_clamps_to_all_edges_and_uses_visible_item_geometry() {
    let s = queue();
    for skin in [Skin::Classic, Skin::Modern] {
        for (x, y) in [(14, 221), (534, 221), (14, 412), (534, 412)] {
            let mut view = View {
                skin,
                ..Default::default()
            };
            view.context_menu_at(x, y, &s);
            let rect = view.queue_menu.as_ref().unwrap().rect;
            assert!(rect.0 >= 4 && rect.1 >= 4);
            assert!(rect.0 + rect.2 + 3 < WIDTH);
            assert!(rect.1 + rect.3 + 3 < HEIGHT);
            assert!(matches!(
                activate(&mut view, 3, &s),
                Some(Action::Send(Command::EditQueue {
                    keep_selected: false,
                    ..
                }))
            ));
        }
    }
}

#[test]
fn queue_menu_press_release_identity_and_dismissal_never_click_through() {
    let s = queue();
    let mut view = View::default();
    group(&mut view, &s);
    view.context_menu_at(80, row_y(3), &s);
    let first = item(&view, 0);
    let second = item(&view, 1);
    view.pointer_down(first.0, first.1, &s);
    assert!(view.pointer_up(Some(second), &s).is_none());
    assert!(view.pointer_up(Some(first), &s).is_none());
    view.pointer_down(first.0, first.1, &s);
    view.context_menu_at(second.0, second.1, &s);
    assert!(
        view.pointer_up(Some(first), &s).is_none(),
        "secondary press cancels a left press"
    );
    // Outside press dismisses; its release cannot select/play the covered row.
    view.pointer_down(500, row_y(0), &s);
    assert!(!view.has_queue_menu());
    assert!(view.pointer_up(Some((500, row_y(0))), &s).is_none());
    assert_eq!(view.selected_queue_indices(&s), [1, 3, 5]);
    assert_eq!(view.selected, Some(3));
    view.context_menu_at(80, row_y(3), &s);
    view.context_menu_at(500, row_y(0), &s);
    assert!(!view.has_queue_menu());
    assert_eq!(view.selected_queue_indices(&s), [1, 3, 5]);
}

#[test]
fn queue_menu_rejects_structural_changes_and_duplicate_only_reorder_ack() {
    for change in 0..4 {
        let mut s = queue();
        let mut view = View::default();
        group(&mut view, &s);
        view.context_menu_at(80, row_y(3), &s);
        let point = item(&view, 3);
        view.pointer_down(point.0, point.1, &s);
        match change {
            0 => s.queue.swap(8, 9),
            1 => {
                s.queue.pop();
            }
            2 => s.queue.push(s.queue[0].clone()),
            _ => s.queue_reorder_request += 1,
        }
        assert!(view.pointer_up(Some(point), &s).is_none());
        assert!(!view.has_queue_menu());
        assert!(view.pointer_up(Some(point), &s).is_none());
    }
}

#[test]
fn queue_menu_survives_metadata_progress_and_automatic_playback_without_retargeting() {
    let mut s = queue();
    let mut view = View::default();
    group(&mut view, &s);
    view.context_menu_at(80, row_y(3), &s);
    let rect = view.queue_menu.as_ref().unwrap().rect;
    s.queue[3].title = "Loaded metadata".into();
    s.current = Some(10);
    s.playback_revision = 42;
    s.playback_origin = PlaybackOrigin::Automatic;
    s.position_ms = 1500;
    view.sync_playback(&s);
    view.render(&s);
    assert!(view.has_queue_menu());
    assert_eq!(view.queue_menu.as_ref().unwrap().rect, rect);
    assert_eq!(view.offset, 0);
    assert_eq!(view.selected, Some(3));
    assert_eq!(view.selected_queue_indices(&s), [1, 3, 5]);
    assert!(matches!(
        activate(&mut view, 0, &s),
        Some(Action::Send(Command::PlayQueueIndex { index: 3, .. }))
    ));
}

#[test]
fn queue_menu_cancels_on_focus_loss_modal_skin_language_or_pointer_cancellation() {
    let s = queue();
    for change in 0..7 {
        let mut view = View::default();
        group(&mut view, &s);
        view.context_menu_at(80, row_y(3), &s);
        let point = item(&view, 3);
        view.pointer_down(point.0, point.1, &s);
        match change {
            0 => view.set_focus(false),
            1 => view.open_about(),
            2 => view.set_skin(Skin::Modern),
            3 => view.set_language(
                LanguagePreference::Specific(Language::English),
                Language::English,
            ),
            4 => view.cancel_pointer(),
            5 => {
                view.help = true;
                view.render(&s);
            }
            _ => {
                view.about = Some(AboutTab::Player);
                view.sync_playback(&s);
            }
        }
        assert!(!view.has_queue_menu());
        assert!(view.pointer_up(Some(point), &s).is_none());
    }
}

#[test]
fn queue_menu_keyboard_open_navigation_and_activation_are_isolated_from_background() {
    let s = queue();
    let mut view = View::default();
    group(&mut view, &s);
    view.queue_menu_key(Key::Menu, false, false, &s);
    assert!(view.has_queue_menu());
    view.queue_menu_key(Key::Down, false, false, &s);
    assert_eq!(view.queue_menu.as_ref().unwrap().focus, 1);
    view.queue_menu_key(Key::End, false, false, &s);
    assert_eq!(view.queue_menu.as_ref().unwrap().focus, 4);
    assert!(matches!(
        view.queue_menu_key(Key::Enter, false, false, &s),
        Some(Action::Send(Command::EditQueue {
            keep_selected: true,
            ..
        }))
    ));
    assert!(!view.has_queue_menu());
    view.queue_menu_key(Key::F10, false, true, &s);
    assert!(view.has_queue_menu());
    view.queue_menu_key(Key::Escape, false, false, &s);
    assert!(!view.has_queue_menu());
    view.input_active = true;
    view.queue_menu_key(Key::Menu, false, false, &s);
    assert!(!view.has_queue_menu());
    view.input_active = false;
    let offline = Snapshot {
        connected: false,
        ..s.clone()
    };
    view.queue_menu_key(Key::Menu, false, false, &offline);
    assert_eq!(view.queue_menu.as_ref().unwrap().focus, 3);
    view.queue_menu_key(Key::Up, false, false, &offline);
    assert_eq!(view.queue_menu.as_ref().unwrap().focus, 4);
    view.queue_menu_key(Key::Down, false, false, &offline);
    assert_eq!(view.queue_menu.as_ref().unwrap().focus, 3);
    view.queue_menu_key(Key::Home, false, false, &offline);
    assert_eq!(view.queue_menu.as_ref().unwrap().focus, 3);
}

#[test]
fn queue_menu_keyboard_uses_a_chosen_occurrence_and_cancels_pending_drag() {
    let s = queue();
    let mut view = View::default();
    group(&mut view, &s);
    view.selected = Some(4); // Ctrl navigation moved focus outside chosen group.
    view.queue_menu_key(Key::Menu, false, false, &s);
    assert_eq!(view.selected, Some(1));
    assert_eq!(view.selected_queue_indices(&s), [1, 3, 5]);
    view.close_queue_menu();
    view.pointer_down(80, row_y(3), &s);
    view.queue_menu_key(Key::F10, false, true, &s);
    assert!(view.queue_drag.is_none());
    assert!(view.pointer_up(Some((80, 410)), &s).is_none());
    assert!(view.has_queue_menu());
    view.close_queue_menu();
    view.selected = Some(11);
    view.queue_choices = None;
    view.queue_menu_key(Key::Menu, false, false, &s);
    assert_eq!(view.offset, 4);
    assert_eq!(view.selected, Some(11));
}

#[test]
fn queue_menu_blocks_scrolling_typing_and_background_selection_while_open() {
    let s = queue();
    let mut view = View::default();
    group(&mut view, &s);
    view.input = "unchanged".into();
    view.context_menu_at(80, row_y(3), &s);
    view.scroll(-4.0, s.queue.len());
    view.insert_char('x');
    view.move_queue_selection(1, false, true, &s);
    view.select_all_queue(&s);
    assert!(view.click(80, row_y(0), &s).is_none());
    assert_eq!(view.queue_drop_index((80.0, 390.0), s.queue.len()), None);
    assert_eq!(view.offset, 0);
    assert_eq!(view.input, "unchanged");
    assert_eq!(view.selected_queue_indices(&s), [1, 3, 5]);
    assert!(view.has_queue_menu());
}

#[test]
fn queue_menu_pointer_poll_does_not_override_keyboard_focus_or_redraw_idle_frames() {
    let s = queue();
    for skin in [Skin::Classic, Skin::Modern] {
        let mut view = View {
            skin,
            ..Default::default()
        };
        group(&mut view, &s);
        view.context_menu_at(80, row_y(3), &s);
        view.queue_menu_key(Key::Down, false, false, &s);
        let fingerprint = view.fingerprint(&s);
        view.pointer_move(Some((80, row_y(3))));
        assert_eq!(view.queue_menu.as_ref().unwrap().focus, 1);
        assert_eq!(view.fingerprint(&s), fingerprint);
        let third = item(&view, 2);
        view.pointer_move(Some(third));
        assert_eq!(view.queue_menu.as_ref().unwrap().focus, 2);
        assert_ne!(view.fingerprint(&s), fingerprint);
    }
}

#[test]
fn queue_menu_renders_all_languages_and_both_skins_into_the_existing_framebuffer() {
    let s = queue();
    for skin in [Skin::Classic, Skin::Modern] {
        let mut view = View {
            skin,
            ..Default::default()
        };
        let buffer = view.pixels.as_ptr();
        let capacity = view.pixels.capacity();
        for language in [
            Language::French,
            Language::English,
            Language::German,
            Language::Spanish,
            Language::Italian,
            Language::Portuguese,
        ] {
            view.set_language(LanguagePreference::Specific(language), language);
            group(&mut view, &s);
            view.context_menu_at(210, row_y(3), &s);
            view.render(&s);
            let fingerprint = view.fingerprint(&s);
            let pixels = view.pixels.clone();
            view.render(&s);
            assert_eq!(view.pixels, pixels);
            assert_eq!(view.fingerprint(&s), fingerprint);
            assert_eq!(view.pixels.as_ptr(), buffer);
            assert_eq!(view.pixels.capacity(), capacity);
            if language == Language::French {
                if let Ok(prefix) = std::env::var("SPOTIAMP_QUEUE_MENU_PREVIEW") {
                    view.save_ppm(&format!(
                        "{prefix}-{}.ppm",
                        if skin == Skin::Classic {
                            "classic"
                        } else {
                            "modern"
                        }
                    ))
                    .unwrap();
                }
            }
            view.close_queue_menu();
        }
    }
}
