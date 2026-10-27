//! Physical X11 wheel events through the two real application frame paths.
use super::LibraryWindow;
use crate::{appearance::Skin, engine::Engine, library_ui, model, ui};
use minifb::{MouseButton, MouseMode, Scale, Window, WindowOptions};
use std::{
    cell::RefCell,
    io::Read,
    process::{Child, Command, Stdio},
    rc::Rc,
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
    // Drop the companion before the native parent, as the real application does.
    library: LibraryWindow,
    player: Window,
    view: ui::View,
    engine: Engine,
    commands: tokio::sync::mpsc::Receiver<model::Command>,
    snapshot: model::Snapshot,
    chars: Rc<RefCell<Vec<char>>>,
    keys: Rc<RefCell<Vec<(minifb::Key, bool, bool)>>>,
    mouse: Rc<RefCell<Vec<crate::MouseInput>>>,
    actions: Vec<ui::Action>,
    fingerprint: Option<u64>,
}

impl App {
    fn new(skin: Skin) -> Self {
        let rows: Vec<_> = (0..40)
            .map(|i| model::TrackRow {
                uri: format!("spotify:track:{i}"),
                title: format!("Titre {i:02}"),
                artist: "Artiste du test".into(),
                duration_ms: 180_000,
                ..Default::default()
            })
            .collect();
        let entries: Vec<_> = (0..30)
            .map(|i| model::PlaylistEntry {
                id: model::PlaylistId::Spotify(format!("playlist-{i}")),
                name: format!("Playlist {i:02}"),
                track_count: Some(rows.len()),
                duration_ms: None,
                folder: String::new(),
                available: true,
            })
            .collect();
        let snapshot = model::Snapshot {
            connected: true,
            current: Some(0),
            queue: rows.clone(),
            ..Default::default()
        };
        let library_snapshot = model::LibrarySnapshot {
            revision: 1,
            selected_id: Some(entries[0].id.clone()),
            entries: Arc::new(entries),
            tracks: Arc::new(rows),
            connected: true,
            queue_len: snapshot.queue.len(),
            ..Default::default()
        };
        let (tx, commands) = tokio::sync::mpsc::channel(8);
        // No Engine::spawn, runtime, authentication, account or network work.
        let engine = Engine {
            tx,
            state: Arc::new(Mutex::new(snapshot.clone())),
            library: Arc::new(Mutex::new(library_snapshot)),
            completion: None,
        };
        let mut player = Window::new(
            "Spotiamp physical wheel test",
            ui::WIDTH,
            ui::HEIGHT,
            WindowOptions {
                borderless: true,
                title: false,
                resize: false,
                ..Default::default()
            },
        )
        .unwrap();
        player.set_target_fps(0);
        player.set_position(0, 0);
        let chars = Rc::new(RefCell::new(Vec::new()));
        let keys = Rc::new(RefCell::new(Vec::new()));
        let mouse = Rc::new(RefCell::new(Vec::new()));
        player.set_input_callback(Box::new(crate::TextInput {
            chars: chars.clone(),
            keys: keys.clone(),
            mouse: mouse.clone(),
            held: [false; 512],
            modifiers: 0,
        }));
        let mut library = LibraryWindow::new_with_layout(Scale::X1, &mut player, None).unwrap();
        library.set_skin(skin);
        let mut view = ui::View::default();
        view.set_skin(skin);
        Self {
            library,
            player,
            view,
            engine,
            commands,
            snapshot,
            chars,
            keys,
            mouse,
            actions: Vec::new(),
            fingerprint: None,
        }
    }

    fn frame(&mut self) {
        {
            let mut chars = self.chars.borrow_mut();
            let mut keys = self.keys.borrow_mut();
            let mut mouse = self.mouse.borrow_mut();
            assert!(
                !mouse.iter().any(|e| matches!(
                    e,
                    crate::MouseInput::Button {
                        button: MouseButton::Left,
                        ..
                    }
                )),
                "wheel injection must never use a left click"
            );
            crate::process_frame_input(
                &mut self.view,
                &self.snapshot,
                crate::FrameInput {
                    keys: &keys,
                    chars: &chars,
                    mouse: &mouse,
                    pointer: self.player.get_mouse_pos(MouseMode::Pass),
                    mouse_down: self.player.get_mouse_down(MouseButton::Left),
                    active: self.player.is_active(),
                    wheel: self.player.get_scroll_wheel().map(|(_, y)| y),
                    ..Default::default()
                },
                &mut self.actions,
            );
            chars.clear();
            keys.clear();
            mouse.clear();
        }
        assert!(
            self.actions.is_empty(),
            "wheel must not trigger a player action"
        );
        // Preserve the production order: consume player input, pump player,
        // then run the real LibraryWindow::frame (which pumps its own window).
        let fingerprint = self.view.fingerprint(&self.snapshot);
        if self.fingerprint != Some(fingerprint) {
            self.view.render(&self.snapshot);
            self.player
                .update_with_buffer(&self.view.pixels, ui::WIDTH, ui::HEIGHT)
                .unwrap();
            self.fingerprint = Some(fingerprint);
        } else {
            self.player.update();
        }
        assert!(self.library.frame(&self.engine, &mut None).unwrap());
        assert!(
            matches!(
                self.commands.try_recv(),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty)
            ),
            "wheel must not select, queue or play a playlist"
        );
    }

    fn xdotool(&mut self, args: &[&str]) -> String {
        let mut process = Process(
            Command::new("xdotool")
                .args(args)
                .stdout(Stdio::piped())
                .spawn()
                .expect("xdotool is required for physical wheel injection"),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.frame();
            if let Some(status) = process.0.try_wait().unwrap() {
                assert!(status.success(), "xdotool failed: {args:?}");
                let mut output = String::new();
                process
                    .0
                    .stdout
                    .take()
                    .unwrap()
                    .read_to_string(&mut output)
                    .unwrap();
                return output;
            }
            assert!(Instant::now() < deadline, "xdotool timed out: {args:?}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn focus(&mut self, player: bool) {
        let id = if player {
            &self.player
        } else {
            &self.library.window
        }
        .get_window_handle() as usize;
        self.xdotool(&["windowfocus", "--sync", &id.to_string()]);
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.player.is_active() != player || self.library.window.is_active() == player {
            self.frame();
            assert!(Instant::now() < deadline, "native focus did not settle");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn wheel(&mut self, library: bool, position: (usize, usize), button: u8) {
        let id = if library {
            &self.library.window
        } else {
            &self.player
        }
        .get_window_handle() as usize;
        // --window applies only to mousemove. click uses XTest at the actual
        // pointer position, so delivery cannot bypass native focus routing.
        self.xdotool(&[
            "mousemove",
            "--window",
            &id.to_string(),
            &position.0.to_string(),
            &position.1.to_string(),
            "click",
            &button.to_string(),
        ]);
    }

    fn expect(&mut self, player: &ui::View, library: &library_ui::View, player_active: bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut matching_since = None;
        loop {
            self.frame();
            let matches =
                self.view.pixels == player.pixels && self.library.view.pixels == library.pixels;
            if matches {
                let since = matching_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_millis(100) {
                    break;
                }
            } else {
                matching_since = None;
            }
            assert!(
                Instant::now() < deadline,
                "native wheel did not match exactly one row of the reference render (player={}, library={})",
                self.view.pixels == player.pixels,
                self.library.view.pixels == library.pixels
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(self.player.is_active(), player_active);
        assert_eq!(self.library.window.is_active(), !player_active);
        let actual: usize = self.xdotool(&["getwindowfocus"]).trim().parse().unwrap();
        let expected = if player_active {
            &self.player
        } else {
            &self.library.window
        };
        assert_eq!(
            actual,
            expected.get_window_handle() as usize,
            "wheel stole native focus"
        );
    }
}

#[test]
#[ignore = "requires isolated Xvfb and xdotool"]
fn x11_physical_wheel_scrolls_real_inactive_library_panels_and_inactive_player() {
    for skin in [Skin::Classic, Skin::Modern] {
        let mut app = App::new(skin);
        let buffers = (app.view.pixels.as_ptr(), app.library.view.pixels.as_ptr());
        let capacities = (
            app.view.pixels.capacity(),
            app.library.view.pixels.capacity(),
        );
        let library_snapshot = app.engine.library.lock().unwrap().clone();
        let mut player_reference = ui::View::default();
        player_reference.set_skin(skin);
        let mut library_reference = library_ui::View::default();
        library_reference.set_skin(skin);
        app.focus(true);
        player_reference.set_focus(true);
        player_reference.render(&app.snapshot);
        library_reference.set_focus(false);
        library_reference.render(&library_snapshot);
        app.expect(&player_reference, &library_reference, true);

        for (position, button, delta) in [
            ((100, 150), 5, -1.0), // inactive playlist panel: down one row
            ((350, 150), 5, -1.0), // inactive track panel: down one row
            ((350, 150), 4, 1.0),
            ((100, 150), 4, 1.0),
        ] {
            library_reference.scroll(delta, Some(position), &library_snapshot);
            assert!(
                library_reference.render(&library_snapshot),
                "reference notch must scroll a row"
            );
            app.wheel(true, position, button);
            app.expect(&player_reference, &library_reference, true);
        }

        app.focus(false);
        player_reference.set_focus(false);
        player_reference.render(&app.snapshot);
        library_reference.set_focus(true);
        library_reference.render(&library_snapshot);
        app.expect(&player_reference, &library_reference, false);
        for (button, delta) in [(5, -1.0), (4, 1.0)] {
            let before = player_reference.fingerprint(&app.snapshot);
            player_reference.scroll(delta, app.snapshot.queue.len());
            assert_ne!(
                player_reference.fingerprint(&app.snapshot),
                before,
                "reference notch must scroll a row"
            );
            player_reference.render(&app.snapshot);
            app.wheel(false, (100, 250), button);
            app.expect(&player_reference, &library_reference, false);
        }
        assert_eq!(
            buffers,
            (app.view.pixels.as_ptr(), app.library.view.pixels.as_ptr())
        );
        assert_eq!(
            capacities,
            (
                app.view.pixels.capacity(),
                app.library.view.pixels.capacity()
            )
        );
        assert_eq!(*app.engine.state.lock().unwrap(), app.snapshot);
    }
}
