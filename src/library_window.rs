//! Optional native window. Its framebuffer and input queues exist only while open.
use super::{MouseInput, TextInput, engine, library_ui, model::LibrarySnapshot};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};
use std::{cell::RefCell, rc::Rc};

pub struct LibraryWindow {
    window: Window,
    view: library_ui::View,
    snapshot: LibrarySnapshot,
    chars: Rc<RefCell<Vec<char>>>,
    keys: Rc<RefCell<Vec<(Key, bool, bool)>>>,
    mouse: Rc<RefCell<Vec<MouseInput>>>,
    actions: Vec<library_ui::Action>,
}

pub(super) fn inside(x: f32, y: f32) -> Option<(usize, usize)> {
    (x >= 0.0 && y >= 0.0 && x < library_ui::WIDTH as f32 && y < library_ui::HEIGHT as f32)
        .then_some((x as usize, y as usize))
}

impl LibraryWindow {
    pub fn new(scale: Scale, parent: &Window) -> Result<Self, minifb::Error> {
        let mut window = Window::new(
            "Spotiamp — Bibliothèque Spotify",
            library_ui::WIDTH,
            library_ui::HEIGHT,
            WindowOptions {
                scale,
                resize: false,
                borderless: true,
                title: false,
                ..WindowOptions::default()
            },
        )?;
        window.set_titlebar_drag_region(Some(library_ui::TITLEBAR_DRAG_REGION));
        // The player already paces the shared UI loop at 30 Hz.
        window.set_target_fps(0);
        let (x, y) = parent.get_position();
        window.set_position(x + 24, y + 24);
        let chars = Rc::new(RefCell::new(Vec::new()));
        let keys = Rc::new(RefCell::new(Vec::new()));
        let mouse = Rc::new(RefCell::new(Vec::with_capacity(16)));
        window.set_input_callback(Box::new(TextInput {
            chars: chars.clone(),
            keys: keys.clone(),
            mouse: mouse.clone(),
            held: [false; 512],
            modifiers: 0,
        }));
        Ok(Self {
            window,
            view: library_ui::View::default(),
            snapshot: LibrarySnapshot::default(),
            chars,
            keys,
            mouse,
            actions: Vec::with_capacity(8),
        })
    }

    /// Returns false after either native or in-skin Close. Playback is untouched.
    pub fn frame(&mut self, engine: &engine::Engine) -> Result<bool, minifb::Error> {
        if !self.window.is_open() {
            return Ok(false);
        }
        {
            let shared = engine
                .library
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if self.snapshot.revision != shared.revision {
                self.snapshot.clone_from(&shared);
            }
        }
        let active = self.window.is_active();
        self.view.set_focus(active);
        self.actions.clear();
        let pointer = self
            .window
            .get_mouse_pos(MouseMode::Pass)
            .and_then(|(x, y)| inside(x, y));
        {
            let keys = self.keys.borrow();
            if active {
                for &(key, control, shift) in keys.iter() {
                    if key == Key::F3 {
                        self.actions.push(library_ui::Action::Close);
                    } else if let Some(action) = self.view.key(key, control, shift, &self.snapshot)
                    {
                        self.actions.push(action);
                    }
                }
                for key in [Key::Up, Key::Down] {
                    if !keys.iter().any(|(candidate, _, _)| *candidate == key)
                        && self.window.is_key_pressed(key, KeyRepeat::Yes)
                        && let Some(action) = self.view.key(key, false, false, &self.snapshot)
                    {
                        self.actions.push(action);
                    }
                }
                for &event in self.mouse.borrow().iter() {
                    let action = match event {
                        MouseInput::Button {
                            button: MouseButton::Left,
                            down,
                            x,
                            y,
                        } => {
                            let position = inside(x, y);
                            if down {
                                position
                                    .and_then(|(x, y)| self.view.pointer_down(x, y, &self.snapshot))
                            } else {
                                self.view.pointer_up(position, &self.snapshot)
                            }
                        }
                        MouseInput::Cancel => {
                            self.view.cancel_pointer();
                            None
                        }
                        _ => None,
                    };
                    if let Some(action) = action {
                        self.actions.push(action);
                    }
                }
                self.view.pointer_move(pointer);
                if let Some((_, delta)) = self.window.get_scroll_wheel() {
                    self.view.scroll(delta, pointer, &self.snapshot);
                }
            }
        }
        self.keys.borrow_mut().clear();
        self.chars.borrow_mut().clear();
        self.mouse.borrow_mut().clear();
        for action in self.actions.drain(..) {
            match action {
                library_ui::Action::Close => return Ok(false),
                library_ui::Action::Minimize => {
                    self.window.minimize();
                    self.view.cancel_pointer();
                }
                library_ui::Action::Command(command) => {
                    if let Err(error) = engine.tx.try_send(super::model::Command::Library(command))
                    {
                        self.view.notice(&error.to_string());
                    }
                }
            }
        }
        if self.view.render(&self.snapshot) {
            self.window.update_with_buffer(
                &self.view.pixels,
                library_ui::WIDTH,
                library_ui::HEIGHT,
            )?;
        } else {
            self.window.update();
        }
        Ok(self.window.is_open())
    }
}
