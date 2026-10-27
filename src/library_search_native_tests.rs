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
            [model::LibraryCommand::QueueSearch {
                revision: 1,
                index: 0,
                play: true,
            }]
        ));
        app.commands.clear();
        app.click(420, 425, false);
        assert!(matches!(
            app.commands.as_slice(),
            [model::LibraryCommand::QueueSearch {
                revision: 1,
                index: 0,
                play: false,
            }]
        ));
        app.commands.clear();
        app.click(705, 40, false);
        assert!(
            matches!(app.commands.as_slice(), [model::LibraryCommand::Search(query)] if query.is_empty())
        );
    }
}
