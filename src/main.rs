#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(feature = "spotify")]
mod credentials;
#[cfg(feature = "spotify")]
mod engine;
#[cfg(not(feature = "spotify"))]
#[path = "engine_stub.rs"]
mod engine;
mod model;
#[cfg(feature = "spotify")]
mod settings;
mod ui;

use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};
use model::Command;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

struct TextInput {
    chars: Rc<RefCell<Vec<char>>>,
    keys: Rc<RefCell<Vec<(Key, bool)>>>,
    mouse: Rc<RefCell<Vec<MouseInput>>>,
    held: [bool; 512],
    modifiers: u8,
}

#[derive(Clone, Copy, Debug)]
enum MouseInput {
    Button {
        button: MouseButton,
        down: bool,
        x: f32,
        y: f32,
    },
    Cancel,
}

fn inside_window(x: f32, y: f32) -> Option<(usize, usize)> {
    (x >= 0.0 && y >= 0.0 && x < ui::WIDTH as f32 && y < ui::HEIGHT as f32)
        .then_some((x as usize, y as usize))
}

impl TextInput {
    fn mouse_event(&mut self, event: MouseInput) {
        let mut events = self.mouse.borrow_mut();
        if events.len() >= 128 {
            // Never keep an unmatched press after overflowing the bounded queue.
            events.clear();
            events.push(MouseInput::Cancel);
        } else {
            events.push(event);
        }
    }
}

impl InputCallback for TextInput {
    fn set_mouse_button_state(&mut self, button: MouseButton, down: bool, x: f32, y: f32) {
        self.mouse_event(MouseInput::Button { button, down, x, y });
    }

    fn mouse_input_cancelled(&mut self) {
        self.mouse_event(MouseInput::Cancel);
    }

    fn add_char(&mut self, code: u32) {
        if let Some(c) = char::from_u32(code).filter(|c| !c.is_control()) {
            if self.modifiers != 0 {
                return;
            }
            let mut pending = self.chars.borrow_mut();
            if pending.len() < 2048 {
                pending.push(c);
            }
        }
    }
    fn set_key_state(&mut self, key: Key, down: bool) {
        let bit = match key {
            Key::LeftCtrl => 1,
            Key::RightCtrl => 2,
            Key::LeftSuper => 4,
            Key::RightSuper => 8,
            _ => 0,
        };
        if down {
            self.modifiers |= bit;
        } else {
            self.modifiers &= !bit;
        }
        let index = key as usize;
        if down && !self.held[index] {
            let mut pending = self.keys.borrow_mut();
            if pending.len() < 128 {
                pending.push((key, self.modifiers != 0));
            }
        }
        self.held[index] = down;
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Spotiamp: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!(concat!(
            "Spotiamp ",
            env!("CARGO_PKG_VERSION"),
            " experimental\n--scale 2        double-size bitmap UI\n--screenshot FILE.ppm   render the disconnected UI without opening a window\n--quit-after SECONDS    automatic exit for reproducible idle measurements\nSpace: play/pause; Ctrl+V: paste URL; Enter: add/play; Delete: remove; F1: help"
        ));
        return Ok(());
    }
    let mut view = ui::View::default();
    if let Some(i) = args.iter().position(|a| a == "--screenshot") {
        let path = args.get(i + 1).ok_or("--screenshot requires a path")?;
        view.render(&model::Snapshot::default());
        view.save_ppm(path)?;
        return Ok(());
    }
    let quit_after = args
        .iter()
        .position(|a| a == "--quit-after")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<u64>().ok());
    let scale = if args.windows(2).any(|a| a == ["--scale", "2"]) {
        Scale::X2
    } else {
        Scale::X1
    };
    let mut window = Window::new(
        &format!(
            "Spotiamp {} — lecteur Spotify expérimental",
            env!("CARGO_PKG_VERSION")
        ),
        ui::WIDTH,
        ui::HEIGHT,
        WindowOptions {
            scale,
            resize: false,
            ..WindowOptions::default()
        },
    )?;
    // Poll input at 30 Hz, but repaint only when visible state changes.
    window.set_target_fps(30);
    let input = Rc::new(RefCell::new(Vec::new()));
    let key_events = Rc::new(RefCell::new(Vec::new()));
    let mouse_events = Rc::new(RefCell::new(Vec::with_capacity(16)));
    window.set_input_callback(Box::new(TextInput {
        chars: input.clone(),
        keys: key_events.clone(),
        mouse: mouse_events.clone(),
        held: [false; 512],
        modifiers: 0,
    }));
    let engine = engine::Engine::spawn();
    let mut clipboard = None;
    let mut actions = Vec::with_capacity(8);
    let mut fingerprint = None;
    let mut snapshot = model::Snapshot::default();
    let start = Instant::now();
    while window.is_open() {
        if quit_after.is_some_and(|s| start.elapsed() >= Duration::from_secs(s)) {
            break;
        }
        {
            let shared = engine.state.lock().unwrap_or_else(|e| e.into_inner());
            if *shared != snapshot {
                snapshot.clone_from(&shared);
            }
        }
        // Keep key-down edges even when key-up arrives in the same 30 Hz poll.
        let keys = std::mem::take(&mut *key_events.borrow_mut());
        let pressed = |key| keys.iter().any(|(k, _)| *k == key);
        let shortcut = |key| keys.iter().any(|(k, control)| *k == key && *control);
        let mut action = None;
        view.set_focus(window.is_active());
        actions.clear();
        if pressed(Key::F1) {
            view.cancel_pointer();
            view.help = !view.help;
        }
        if pressed(Key::Escape) {
            view.cancel_pointer();
            view.help = false;
            view.input_active = false;
        }
        if pressed(Key::Tab) {
            view.input_active = !view.input_active;
        }
        // Process native edges in order: a press and release within one 30 Hz
        // poll must still activate once and must not leave the button stuck.
        for event in mouse_events.borrow_mut().drain(..) {
            let next = match event {
                MouseInput::Button {
                    button: MouseButton::Left,
                    down,
                    x,
                    y,
                } => {
                    let position = inside_window(x, y);
                    if down {
                        position.and_then(|(x, y)| view.pointer_down(x, y, &snapshot))
                    } else {
                        view.pointer_up(position, &snapshot)
                    }
                }
                MouseInput::Cancel => {
                    view.cancel_pointer();
                    None
                }
                _ => None,
            };
            if let Some(next) = next {
                actions.push(next);
            }
        }
        let pointer = window.get_mouse_pos(MouseMode::Pass);
        view.pointer_move(pointer.and_then(|(x, y)| inside_window(x, y)));
        if window.is_active() && window.get_mouse_down(MouseButton::Left) {
            if let Some((x, y)) = pointer {
                action = view.drag(x as usize, y as usize, &snapshot);
            }
        }
        if let Some((_, y)) = window.get_scroll_wheel() {
            view.scroll(y, snapshot.queue.len());
        }
        if shortcut(Key::V) {
            action = Some(ui::Action::Paste);
        }
        if view.input_active {
            if shortcut(Key::A) {
                view.select_all = true;
            }
            if pressed(Key::Backspace) || window.is_key_pressed(Key::Backspace, KeyRepeat::Yes) {
                if view.select_all {
                    view.input.clear();
                    view.select_all = false;
                } else {
                    view.input.pop();
                }
            }
            for c in input.borrow_mut().drain(..) {
                view.insert_char(c);
            }
            if pressed(Key::Enter) {
                action = Some(ui::Action::Submit);
            }
        } else if !view.help {
            if pressed(Key::Space) {
                action = Some(ui::Action::Send(Command::Toggle));
            }
            if pressed(Key::Enter) {
                if let Some(i) = view.selected {
                    action = Some(ui::Action::Send(Command::PlayIndex(i)));
                }
            }
            if pressed(Key::Delete) {
                if let Some(i) = view.selected {
                    action = Some(ui::Action::Send(Command::Remove(i)));
                }
            }
            if pressed(Key::Down) || window.is_key_pressed(Key::Down, KeyRepeat::Yes) {
                view.move_selection(1, snapshot.queue.len());
            }
            if pressed(Key::Up) || window.is_key_pressed(Key::Up, KeyRepeat::Yes) {
                view.move_selection(-1, snapshot.queue.len());
            }
        }
        input.borrow_mut().clear();
        if let Some(action) = action {
            actions.push(action);
        }
        for action in actions.drain(..) {
            let command = match action {
                ui::Action::Send(c) => Some(c),
                ui::Action::Submit => {
                    let text = view.input.trim().to_owned();
                    if text.is_empty() {
                        None
                    } else {
                        Some(Command::Add(text))
                    }
                }
                ui::Action::Paste => {
                    if clipboard.is_none() {
                        clipboard = arboard::Clipboard::new().ok();
                    }
                    match clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                        Some(s) => {
                            view.input = s.chars().filter(|c| !c.is_control()).take(2048).collect();
                            view.input_active = true;
                            view.select_all = true;
                        }
                        None => view.notice("Presse-papiers texte indisponible."),
                    }
                    None
                }
            };
            if let Some(c) = command {
                if let Err(e) = engine.tx.try_send(c) {
                    view.notice(&e.to_string());
                } else {
                    view.input_active = false;
                }
            }
        }
        let next_fingerprint = view.fingerprint(&snapshot);
        if fingerprint != Some(next_fingerprint) {
            view.render(&snapshot);
            window.update_with_buffer(&view.pixels, ui::WIDTH, ui::HEIGHT)?;
            fingerprint = Some(next_fingerprint);
        } else {
            window.update();
        }
    }
    let _ = engine.tx.try_send(Command::Shutdown);
    Ok(())
}

#[cfg(test)]
mod pointer_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_key_press_survives_release_before_the_next_frame() {
        let keys = Rc::new(RefCell::new(Vec::new()));
        let chars = Rc::new(RefCell::new(Vec::new()));
        let mut input = TextInput {
            chars: chars.clone(),
            keys: keys.clone(),
            mouse: Rc::new(RefCell::new(Vec::new())),
            held: [false; 512],
            modifiers: 0,
        };
        input.set_key_state(Key::F1, true);
        input.set_key_state(Key::F1, false);
        assert_eq!(&*keys.borrow(), &[(Key::F1, false)]);
        input.set_key_state(Key::LeftCtrl, true);
        input.set_key_state(Key::V, true);
        input.add_char('v' as u32);
        input.set_key_state(Key::V, false);
        input.set_key_state(Key::LeftCtrl, false);
        assert!(keys.borrow().contains(&(Key::V, true)));
        assert!(chars.borrow().is_empty());
    }
}
