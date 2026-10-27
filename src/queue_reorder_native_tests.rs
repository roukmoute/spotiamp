//! Real XTest left-button capture through the same player input pipeline.
use super::*;
use std::process::{Child, Command as ProcessCommand, Stdio};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Player {
    window: Window,
    view: ui::View,
    state: model::Snapshot,
    keys: Rc<RefCell<Vec<(Key, bool, bool)>>>,
    chars: Rc<RefCell<Vec<char>>>,
    mouse: Rc<RefCell<Vec<MouseInput>>>,
    commands: Vec<Command>,
}

impl Player {
    fn new(skin: appearance::Skin, scale: Scale) -> Self {
        let mut window = Window::new(
            "Spotiamp queue reorder test",
            ui::WIDTH,
            ui::HEIGHT,
            WindowOptions {
                borderless: true,
                title: false,
                scale,
                ..Default::default()
            },
        )
        .unwrap();
        window.set_position(0, 0);
        window.set_target_fps(0);
        let keys = Rc::default();
        let chars = Rc::default();
        let mouse = Rc::default();
        window.set_input_callback(Box::new(TextInput {
            keys: Rc::clone(&keys),
            chars: Rc::clone(&chars),
            mouse: Rc::clone(&mouse),
            held: [false; 512],
            modifiers: 0,
        }));
        let mut view = ui::View::default();
        view.set_skin(skin);
        let mut state = model::Snapshot {
            connected: true,
            playing: true,
            current: Some(2),
            queue: (0..20)
                .map(|i| model::TrackRow {
                    uri: format!("spotify:track:{i}"),
                    title: format!("Track {i}"),
                    ..Default::default()
                })
                .collect(),
            position_ms: 42_000,
            ..Default::default()
        };
        state.queue[4].uri = state.queue[2].uri.clone();
        Self {
            window,
            view,
            state,
            keys,
            chars,
            mouse,
            commands: Vec::new(),
        }
    }

    fn frame(&mut self) {
        self.window.update();
        let mut keys = self.keys.borrow_mut();
        let mut chars = self.chars.borrow_mut();
        let mut mouse = self.mouse.borrow_mut();
        let mut actions = Vec::new();
        process_frame_input(
            &mut self.view,
            &self.state,
            FrameInput {
                active: self.window.is_active(),
                keys: &keys,
                chars: &chars,
                mouse: &mouse,
                pointer: self.window.get_mouse_pos(MouseMode::Pass),
                mouse_down: self.window.get_mouse_down(MouseButton::Left),
                ..Default::default()
            },
            &mut actions,
        );
        keys.clear();
        chars.clear();
        mouse.clear();
        drop((keys, chars, mouse));
        for action in actions {
            match action {
                ui::Action::Send(command) => self.commands.push(command),
                _ => panic!("unexpected non-command action"),
            }
        }
        self.view.show_queue_drop(None, self.state.queue.len());
        self.view.render(&self.state);
        self.window
            .update_with_buffer(&self.view.pixels, ui::WIDTH, ui::HEIGHT)
            .unwrap();
    }

    fn xdotool(&mut self, args: &[&str]) {
        let mut child = Process(
            ProcessCommand::new("xdotool")
                .args(args)
                .stdout(Stdio::null())
                .spawn()
                .expect("xdotool required"),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.frame();
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(status.success(), "xdotool {args:?}");
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        for _ in 0..8 {
            self.frame();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn point(&mut self, x: usize, y: usize, factor: usize) {
        let id = (self.window.get_window_handle() as usize).to_string();
        self.xdotool(&[
            "mousemove",
            "--window",
            &id,
            &(x * factor).to_string(),
            &(y * factor).to_string(),
        ]);
    }
}

#[test]
#[ignore = "requires isolated Xvfb and xdotool"]
fn x11_left_drag_reorders_queue_without_restarting_at_both_scales_and_skins() {
    for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
        for (scale, factor) in [(Scale::X1, 1), (Scale::X2, 2)] {
            let mut player = Player::new(skin, scale);
            player.frame();
            let id = (player.window.get_window_handle() as usize).to_string();
            player.xdotool(&["windowfocus", "--sync", &id]);
            player.point(80, 281, factor);
            player.xdotool(&["mousedown", "1"]);
            player.point(80, 368, factor);
            assert!(
                player.commands.is_empty(),
                "holding a row never starts playback or writes an order"
            );
            assert_eq!(
                player.view.pixels[365 * ui::WIDTH + 80],
                if skin == appearance::Skin::Modern {
                    0x55e4ff
                } else {
                    0x5bff35
                }
            );
            player.xdotool(&["mouseup", "1"]);
            assert_eq!(player.commands.len(), 1);
            let Command::ReorderQueue {
                request_id,
                expected_queue,
                indices,
                insert_before,
            } = player.commands.remove(0)
            else {
                panic!("expected reorder, never playback")
            };
            assert_eq!(indices, [2]);
            assert_eq!(insert_before, 6);
            assert_eq!(
                expected_queue,
                player
                    .state
                    .queue
                    .iter()
                    .map(|row| row.uri.clone())
                    .collect::<Vec<_>>()
            );
            assert_eq!(player.view.selected, Some(2));
            let order =
                model::reorder_indices(player.state.queue.len(), &indices, insert_before).unwrap();
            player.state.queue = order
                .iter()
                .map(|&i| player.state.queue[i].clone())
                .collect();
            player.state.current = Some(5);
            player.state.queue_reorder_request = request_id;
            player.frame();
            assert_eq!(player.view.selected, Some(5));
            assert_eq!(player.state.position_ms, 42_000);
            assert!(player.commands.is_empty());
            // A native Escape consumes a captured drop, with no later release action.
            player.point(80, 353, factor);
            player.xdotool(&["mousedown", "1"]);
            player.point(80, 233, factor);
            player.xdotool(&["key", "Escape", "mouseup", "1"]);
            assert!(player.commands.is_empty());
        }
    }
}
