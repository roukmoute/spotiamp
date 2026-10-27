use super::*;
use crate::{
    keyboard_input_tests::{Keyboard, Polled},
    model::{LibraryCommand, PlaylistEntry, PlaylistId, TrackRow},
};
use std::sync::Arc;

fn state() -> LibrarySnapshot {
    let entries: Vec<_> = (0..10)
        .map(|i| PlaylistEntry {
            id: PlaylistId::Spotify(format!("spotify:playlist:{i:022}")),
            name: format!("Playlist {i}"),
            track_count: Some(20),
            duration_ms: None,
            folder: String::new(),
            available: true,
        })
        .collect();
    LibrarySnapshot {
        connected: true,
        revision: 1,
        selected_id: Some(entries[0].id.clone()),
        entries: Arc::new(entries),
        tracks: Arc::new(
            (0..20)
                .map(|i| TrackRow {
                    uri: format!("spotify:track:{i:022}"),
                    ..Default::default()
                })
                .collect(),
        ),
        ..Default::default()
    }
}

fn frame(
    view: &mut library_ui::View,
    keys: &Keyboard,
    state: &LibrarySnapshot,
    active: bool,
    polled: Option<Polled>,
) -> Vec<library_ui::Action> {
    let mut actions = Vec::new();
    view.set_focus(active);
    let events: Vec<_> = keys
        .callback
        .keys
        .borrow()
        .iter()
        .map(|&(key, control, shift)| input::Input::Key(key, control, shift))
        .collect();
    input::process(
        view,
        state,
        active,
        &events,
        &[
            (Key::Up, keys.repeated(Key::Up, polled), false, false),
            (Key::Down, keys.repeated(Key::Down, polled), false, false),
        ],
        &mut |_| Err(()),
        &mut actions,
    );
    keys.clear();
    actions
}

fn selected(actions: &[library_ui::Action], state: &LibrarySnapshot) -> Vec<usize> {
    actions
        .iter()
        .filter_map(|action| match action {
            library_ui::Action::Command(LibraryCommand::Select(id)) => {
                state.entries.iter().position(|entry| &entry.id == id)
            }
            _ => panic!("unexpected library action: {action:?}"),
        })
        .collect()
}

#[test]
fn library_arrow_callback_then_delayed_native_edge_selects_only_one_playlist() {
    for skin in [
        crate::appearance::Skin::Classic,
        crate::appearance::Skin::Modern,
    ] {
        let state = state();
        let mut view = library_ui::View::default();
        view.set_skin(skin);
        let mut keys = Keyboard::new();
        keys.down(Key::Down);
        assert_eq!(
            selected(&frame(&mut view, &keys, &state, true, None), &state),
            vec![1]
        );
        assert!(
            frame(
                &mut view,
                &keys,
                &state,
                true,
                Some(Polled::initial(Key::Down))
            )
            .is_empty()
        );
        keys.down(Key::Down);
        assert_eq!(
            selected(
                &frame(
                    &mut view,
                    &keys,
                    &state,
                    true,
                    Some(Polled::repeat(Key::Down))
                ),
                &state
            ),
            vec![2]
        );
        keys.up(Key::Down);
        assert!(
            frame(
                &mut view,
                &keys,
                &state,
                true,
                Some(Polled::repeat(Key::Down))
            )
            .is_empty()
        );
        keys.tap(Key::Up);
        keys.tap(Key::Up);
        assert_eq!(
            selected(&frame(&mut view, &keys, &state, true, None), &state),
            vec![1, 0]
        );
    }
}

#[test]
fn library_track_taps_and_focus_preserve_control_enter_append() {
    let state = state();
    let mut view = library_ui::View::default();
    let mut keys = Keyboard::new();
    keys.tap(Key::Right);
    keys.tap(Key::Home);
    keys.tap(Key::Down);
    keys.tap(Key::Down);
    assert!(frame(&mut view, &keys, &state, true, None).is_empty());
    keys.tap(Key::Enter);
    assert!(matches!(
        frame(&mut view, &keys, &state, true, None).as_slice(),
        [library_ui::Action::Command(LibraryCommand::QueueTracks {
            tracks, origin: _, expected_queue: _, replace: true, play: true, insert_at: None,
        })] if tracks.len() == 1 && tracks[0] == state.tracks[2]
    ));
    keys.down(Key::Down);
    assert!(frame(&mut view, &keys, &state, false, None).is_empty());
    keys.up(Key::Down);
    assert!(
        frame(
            &mut view,
            &keys,
            &state,
            true,
            Some(Polled::initial(Key::Down))
        )
        .is_empty()
    );
    keys.down(Key::LeftCtrl);
    keys.tap(Key::Enter);
    keys.up(Key::LeftCtrl);
    assert!(matches!(
        frame(&mut view, &keys, &state, true, None).as_slice(),
        [library_ui::Action::Command(LibraryCommand::QueueTracks {
            tracks, origin: _, expected_queue: _, replace: false, play: false, insert_at: None,
        })] if tracks.len() == 1 && tracks[0] == state.tracks[2]
    ));
}
