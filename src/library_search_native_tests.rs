//! XTest input through the real library frame. Synthetic rows, no account/network.
use super::*;
use crate::{appearance::Skin, model};
use std::{
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct App {
    library: LibraryWindow,
    player: Window,
    view: crate::ui::View,
    engine: engine::Engine,
    receiver: tokio::sync::mpsc::Receiver<model::Command>,
    commands: Vec<model::LibraryCommand>,
    clipboard: Option<arboard::Clipboard>,
}

impl App {
    fn new(skin: Skin) -> Self {
        let (tx, receiver) = tokio::sync::mpsc::channel(16);
        let engine = engine::Engine {
            tx,
            state: Default::default(),
            library: Arc::new(Mutex::new(LibrarySnapshot {
                connected: true,
                revision: 1,
                ..Default::default()
            })),
            completion: None,
        };
        let mut player = Window::new(
            "Spotiamp search test",
            crate::ui::WIDTH,
            crate::ui::HEIGHT,
            WindowOptions {
                borderless: true,
                title: false,
                ..Default::default()
            },
        )
        .unwrap();
        player.set_target_fps(0);
        player.set_position(0, 0);
        let mut library = LibraryWindow::new_with_layout(Scale::X1, &mut player, None).unwrap();
        library.set_skin(skin);
        Self {
            library,
            player,
            view: crate::ui::View::default(),
            engine,
            receiver,
            commands: Vec::new(),
            clipboard: None,
        }
    }

    fn frame(&mut self) {
        self.player.update();
        assert!(
            self.library
                .frame(&self.engine, &mut self.clipboard)
                .unwrap()
        );
        self.library.drop_into_player(
            &self.player,
            &mut self.view,
            &self.engine.state.lock().unwrap(),
            &self.engine,
        );
        while let Ok(command) = self.receiver.try_recv() {
            match command {
                model::Command::Library(command) => self.commands.push(command),
                other => panic!("unexpected player command {other:?}"),
            }
        }
    }

    fn xdotool(&mut self, args: &[&str]) {
        let mut child = Process(
            Command::new("xdotool")
                .args(args)
                .stdout(Stdio::null())
                .spawn()
                .expect("xdotool is required"),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.frame();
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < deadline, "xdotool timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        // Event callbacks are consumed in the next frame, after the native pump.
        for _ in 0..8 {
            self.frame();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn click(&mut self, x: usize, y: usize, double: bool) {
        let id = (self.library.window.get_window_handle() as usize).to_string();
        self.xdotool(&[
            "mousemove",
            "--window",
            &id,
            &x.to_string(),
            &y.to_string(),
            "click",
            "--repeat",
            if double { "2" } else { "1" },
            "--delay",
            "80",
            "1",
        ]);
    }
}

#[test]
#[ignore = "requires isolated Xvfb and xdotool"]
fn x11_search_typing_submit_result_double_click_and_append_use_real_library_frame() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut app = App::new(skin);
        app.frame();
        let id = (app.library.window.get_window_handle() as usize).to_string();
        app.xdotool(&[
            "windowfocus",
            "--sync",
            &id,
            "key",
            "--clearmodifiers",
            "ctrl+f",
        ]);
        app.xdotool(&["type", "--clearmodifiers", "--delay", "0", "Daft Punk"]);
        assert!(
            app.commands.is_empty(),
            "Typing must never request the network"
        );
        app.xdotool(&["key", "Return"]);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::Search(query)] if query == "Daft Punk")
        );
        app.commands.clear();
        {
            let mut snapshot = app.engine.library.lock().unwrap();
            snapshot.revision += 1;
            snapshot.search = model::SearchSnapshot {
                revision: 1,
                query: "Daft Punk".into(),
                tracks: Arc::new(vec![model::TrackRow {
                    uri: "spotify:track:0000000000000000000001".into(),
                    title: "Synthetic search result".into(),
                    artist: "Test artist".into(),
                    duration_ms: 120_000,
                    ..Default::default()
                }]),
                ..Default::default()
            };
        }
        app.frame();
        app.click(320, 132, true);
        assert!(matches!(
            app.commands.as_slice(),
            [model::LibraryCommand::QueueTracks {
                tracks, origin: _, expected_queue: _, replace: true, play: true, insert_at: None,
            }] if tracks.len() == 1 && tracks[0].uri.ends_with('1')
        ));
        app.commands.clear();
        app.click(330, 425, false);
        app.click(390, 310, false);
        assert!(matches!(
            app.commands.as_slice(),
            [model::LibraryCommand::QueueTracks {
                tracks, origin: _, expected_queue: _, replace: false, play: false, insert_at: None,
            }] if tracks.len() == 1 && tracks[0].uri.ends_with('1')
        ));
        app.commands.clear();
        app.click(705, 40, false);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::Search(query)] if query.is_empty())
        );
    }
}

#[test]
#[ignore = "requires isolated Xvfb and xdotool"]
fn x11_control_selection_drag_inserts_captured_tracks_in_player_without_playing() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut app = App::new(skin);
        let rows: Vec<_> = (0..8)
            .map(|i| model::TrackRow {
                uri: format!("spotify:track:{i:022}"),
                title: format!("Track {i}"),
                ..Default::default()
            })
            .collect();
        {
            let mut state = app.engine.library.lock().unwrap();
            state.revision += 1;
            state.search.revision = 1;
            state.search.tracks = Arc::new(rows.clone());
        }
        app.engine.state.lock().unwrap().queue = rows.clone();
        app.frame();
        let source = (app.library.window.get_window_handle() as usize).to_string();
        let target = (app.player.get_window_handle() as usize).to_string();
        app.xdotool(&["windowfocus", "--sync", &source, "key", "ctrl+f"]);
        app.click(320, 132, false);
        app.xdotool(&["keydown", "ctrl"]);
        app.click(320, 180, false);
        app.xdotool(&["keyup", "ctrl"]);
        app.commands.clear();
        app.xdotool(&[
            "mousemove",
            "--window",
            &source,
            "320",
            "132",
            "mousedown",
            "1",
        ]);
        app.xdotool(&["mousemove", "--window", &target, "200", "269"]);
        app.xdotool(&["mouseup", "1"]);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::QueueTracks {
            tracks, origin: _, expected_queue: _, replace: false, play: false, insert_at: Some(2)
        }] if tracks.iter().map(|row| &row.uri).collect::<Vec<_>>() == vec![&rows[0].uri, &rows[2].uri]),
            "actual native drag commands: {:?}",
            app.commands
        );
        // No duplicate command on subsequent idle event pumps.
        app.commands.clear();
        for _ in 0..5 {
            app.frame();
        }
        assert!(app.commands.is_empty());
    }
}

#[test]
#[ignore = "requires isolated Xvfb and xdotool"]
fn x11_create_playlist_uses_control_selection_unicode_clipboard_and_enter_in_both_skins() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut app = App::new(skin);
        let rows: Vec<_> = (0..3)
            .map(|index| model::TrackRow {
                uri: format!("spotify:track:{index:022}"),
                title: format!("Selected track {index}"),
                artist: "Synthetic artist".into(),
                ..Default::default()
            })
            .collect();
        {
            let mut snapshot = app.engine.library.lock().unwrap();
            snapshot.revision += 1;
            snapshot.search.revision = 1;
            snapshot.search.query = "jazz".into();
            snapshot.search.tracks = Arc::new(rows.clone());
        }
        app.frame();
        let id = (app.library.window.get_window_handle() as usize).to_string();
        app.xdotool(&["windowfocus", "--sync", &id, "key", "ctrl+f"]);
        app.click(320, 132, false);
        app.xdotool(&["keydown", "ctrl"]);
        app.click(320, 180, false);
        app.xdotool(&["keyup", "ctrl"]);
        app.click(330, 425, false);
        app.click(390, 366, false);
        assert!(
            app.commands.is_empty(),
            "Opening the name editor must not write a playlist or play audio"
        );

        // Use the actual shared application clipboard: native Ctrl+V must route
        // its Unicode text to the name editor, leaving the search field intact.
        app.clipboard
            .get_or_insert_with(|| arboard::Clipboard::new().unwrap())
            .set_text("  Été à São Paulo  ")
            .unwrap();
        app.xdotool(&["key", "ctrl+v"]);
        assert!(app.commands.is_empty(), "Pasting a name must not submit it");
        app.xdotool(&["key", "Return"]);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::CreatePlaylist { name, tracks }]
                if name == "Été à São Paulo" && tracks == &vec![rows[0].clone(), rows[2].clone()]),
            "actual native creation commands for {skin:?}: {:?}",
            app.commands
        );
        assert!(app.engine.state.lock().unwrap().queue.is_empty());
        // The held-key pump and subsequent idle frames must not create again.
        app.commands.clear();
        for _ in 0..8 {
            app.frame();
        }
        assert!(app.commands.is_empty());
    }
}

#[test]
#[ignore = "requires isolated Xvfb and xdotool"]
fn x11_send_menu_preserves_selection_dismisses_on_focus_loss_and_dispatches_once() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut app = App::new(skin);
        let id = model::PlaylistId::Spotify("owned-menu-test-playlist".into());
        let rows: Vec<_> = (0..4)
            .map(|index| model::TrackRow {
                uri: format!("spotify:track:{index:022}"),
                title: format!("Track {index}"),
                ..Default::default()
            })
            .collect();
        {
            let mut snapshot = app.engine.library.lock().unwrap();
            snapshot.revision += 1;
            snapshot.selected_id = Some(id.clone());
            snapshot.selected_owned = true;
            snapshot.selected_playlist_revision = Arc::from([1, 2, 3]);
            snapshot.entries = Arc::new(vec![model::PlaylistEntry {
                id: id.clone(),
                name: "Owned menu fixture".into(),
                track_count: Some(rows.len()),
                duration_ms: None,
                folder: String::new(),
                available: true,
            }]);
            snapshot.tracks = Arc::new(rows.clone());
        }
        app.engine.state.lock().unwrap().queue = rows.clone();
        app.frame();
        let library = (app.library.window.get_window_handle() as usize).to_string();
        let player = (app.player.get_window_handle() as usize).to_string();
        app.xdotool(&["windowfocus", "--sync", &library]);
        app.click(320, 132, false);
        app.xdotool(&["keydown", "ctrl"]);
        app.click(320, 180, false);
        app.xdotool(&["keyup", "ctrl"]);

        // Clicking the covered list dismisses the menu without changing the
        // explicitly selected rows or starting a drag from the background.
        app.click(330, 425, false);
        assert!(app.library.view.has_overlay());
        app.click(320, 156, false);
        assert!(!app.library.view.has_overlay());
        assert!(app.commands.is_empty());
        assert_eq!(
            app.library
                .view
                .selected_tracks(&app.engine.library.lock().unwrap()),
            vec![rows[0].clone(), rows[2].clone()]
        );

        // Native Shift+F10 opens the same menu and its append item captures
        // both rows, without sending a playback or replace operation.
        app.xdotool(&["key", "shift+F10"]);
        assert!(app.library.view.has_overlay());
        app.xdotool(&["mousemove", "--window", &library, "390", "282"]);
        app.xdotool(&["key", "Down"]);
        // Several native frames with the mouse resting over Play must keep
        // the newer keyboard choice (Append), rather than reset its focus.
        app.xdotool(&["key", "Return"]);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::QueueTracks {
            tracks, origin: _, expected_queue: _, replace: false, play: false, insert_at: None,
        }] if tracks == &vec![rows[0].clone(), rows[2].clone()])
        );
        app.commands.clear();

        app.click(330, 425, false);
        app.xdotool(&["windowfocus", "--sync", &player]);
        assert!(!app.library.view.has_overlay());
        assert!(app.commands.is_empty());
        app.xdotool(&["windowfocus", "--sync", &library]);
        app.click(330, 425, false);
        app.click(390, 394, false);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::RemoveFromPlaylist {
            id: requested_id, indices, expected_revision, expected_tracks,
        }] if requested_id == &id && indices == &[0, 2]
            && expected_revision.as_ref() == [1, 2, 3] && expected_tracks.as_ref() == &rows)
        );
        assert_eq!(app.engine.state.lock().unwrap().queue, rows);
        app.commands.clear();
        for _ in 0..8 {
            app.frame();
        }
        assert!(app.commands.is_empty());
    }
}

#[test]
#[ignore = "requires isolated Xvfb and xdotool"]
fn x11_playlist_delete_held_across_completion_never_repeats_or_changes_playback() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut app = App::new(skin);
        let id = model::PlaylistId::Spotify("owned-test-playlist".into());
        let rows: Vec<_> = (0..4)
            .map(|index| model::TrackRow {
                uri: format!("spotify:track:{index:022}"),
                title: format!("Track {index}"),
                ..Default::default()
            })
            .collect();
        {
            let mut snapshot = app.engine.library.lock().unwrap();
            snapshot.revision += 1;
            snapshot.selected_id = Some(id.clone());
            snapshot.selected_owned = true;
            snapshot.selected_playlist_revision = Arc::from([1, 2, 3]);
            snapshot.entries = Arc::new(vec![model::PlaylistEntry {
                id: id.clone(),
                name: "Owned fixture".into(),
                track_count: Some(rows.len()),
                duration_ms: None,
                folder: String::new(),
                available: true,
            }]);
            snapshot.tracks = Arc::new(rows.clone());
        }
        app.engine.state.lock().unwrap().queue = rows.clone();
        app.frame();
        let window = (app.library.window.get_window_handle() as usize).to_string();
        app.xdotool(&["windowfocus", "--sync", &window]);
        app.click(320, 132, false);
        app.xdotool(&["keydown", "ctrl"]);
        app.click(320, 180, false);
        app.xdotool(&["keyup", "ctrl"]);
        app.xdotool(&["keydown", "Delete"]);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::RemoveFromPlaylist {
            id: requested_id, indices, expected_revision, expected_tracks,
        }] if requested_id == &id && indices == &[0, 2] && expected_revision.as_ref() == [1, 2, 3]
            && expected_tracks.as_ref() == &rows)
        );
        app.commands.clear();
        {
            let mut snapshot = app.engine.library.lock().unwrap();
            snapshot.revision += 1;
            snapshot.playlist_remove_revision += 1;
            snapshot.selected_playlist_revision = Arc::from([4, 5, 6]);
            snapshot.tracks = Arc::new(vec![rows[1].clone(), rows[3].clone()]);
        }
        app.frame();
        // Select a remaining row without releasing Delete, then let native
        // repeat events run well past the initial repeat delay.
        app.click(320, 132, false);
        app.xdotool(&["sleep", "1"]);
        assert!(
            app.commands.is_empty(),
            "held Delete emitted a second mutation: {:?}",
            app.commands
        );
        app.xdotool(&["keyup", "Delete"]);
        app.xdotool(&["key", "Delete"]);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::RemoveFromPlaylist { indices, .. }] if indices == &[0])
        );
        assert_eq!(app.engine.state.lock().unwrap().queue, rows);
    }
}
