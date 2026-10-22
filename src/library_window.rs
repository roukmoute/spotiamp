//! Docked native library. Its framebuffer and input queues exist only while open.
use super::{MouseInput, TextInput, engine, library_ui, model::LibrarySnapshot, repeat_only};
use minifb::{Key, MouseButton, MouseMode, Scale, Window, WindowOptions};
use std::{cell::RefCell, rc::Rc};

pub struct LibraryWindow {
    window: Window,
    #[cfg(target_os = "windows")]
    _icons: Option<crate::windows_icon::WindowIcons>,
    view: library_ui::View,
    snapshot: LibrarySnapshot,
    chars: Rc<RefCell<Vec<char>>>,
    keys: Rc<RefCell<Vec<(Key, bool, bool)>>>,
    mouse: Rc<RefCell<Vec<MouseInput>>>,
    actions: Vec<library_ui::Action>,
}

#[cfg(all(test, target_os = "linux", feature = "spotify"))]
#[path = "wheel_native_x11_tests.rs"]
mod wheel_native_x11_tests;

#[cfg(test)]
#[path = "library_keyboard_tests.rs"]
mod keyboard_tests;

fn process_keyboard(
    view: &mut library_ui::View,
    snapshot: &LibrarySnapshot,
    active: bool,
    keys: &[(Key, bool, bool)],
    repeat_up: bool,
    repeat_down: bool,
    actions: &mut Vec<library_ui::Action>,
) {
    if !active {
        return;
    }
    for &(key, control, shift) in keys {
        if key == Key::F3 {
            actions.push(library_ui::Action::Close);
        } else if let Some(action) = view.key(key, control, shift, snapshot) {
            actions.push(action);
        }
    }
    for (key, repeated) in [(Key::Up, repeat_up), (Key::Down, repeat_down)] {
        if repeated
            && !keys.iter().any(|(candidate, _, _)| *candidate == key)
            && let Some(action) = view.key(key, false, false, snapshot)
        {
            actions.push(action);
        }
    }
}

pub(super) fn inside(x: f32, y: f32) -> Option<(usize, usize)> {
    (x >= 0.0 && y >= 0.0 && x < library_ui::WIDTH as f32 && y < library_ui::HEIGHT as f32)
        .then_some((x as usize, y as usize))
}

impl LibraryWindow {
    pub fn new_with_layout(
        scale: Scale,
        parent: &mut Window,
        saved: Option<crate::window_settings::SavedLayout>,
    ) -> Result<Self, minifb::Error> {
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
        #[cfg(target_os = "windows")]
        let icons = crate::windows_icon::WindowIcons::attach(&window).ok();
        window.set_titlebar_drag_region(Some(library_ui::TITLEBAR_DRAG_REGION));
        // The player already paces the shared UI loop at 30 Hz.
        window.set_target_fps(0);
        if let Some(saved) = saved {
            window.set_docked_parent(
                Some(parent),
                saved.library.0 - saved.player.0,
                saved.library.1 - saved.player.1,
            )?;
            window.set_dock_magnetic(true)?;
            if !saved.attached {
                window.set_dock_attached(false)?;
            }
            // Attach/free first, then move. This also handles asynchronous X11
            // placement replies without accidentally pulling a separated peer.
            parent.set_position(saved.player.0, saved.player.1);
            if !saved.attached {
                window.set_position(saved.library.0, saved.library.1);
            }
        } else {
            let placement = crate::window_layout::place(
                parent.get_position(),
                parent.get_size(),
                window.get_size(),
                parent.get_work_area(),
            );
            if parent.get_position() != placement.player {
                parent.set_position(placement.player.0, placement.player.1);
            }
            window.set_docked_parent(Some(parent), placement.offset.0, placement.offset.1)?;
            window.set_dock_magnetic(true)?;
        }
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
            #[cfg(target_os = "windows")]
            _icons: icons,
            view: library_ui::View::default(),
            snapshot: LibrarySnapshot::default(),
            chars,
            keys,
            mouse,
            actions: Vec::with_capacity(8),
        })
    }

    pub fn placement(&self) -> Option<minifb::RestorablePlacement> {
        self.window.get_restorable_placement()
    }

    pub fn set_skin(&mut self, skin: crate::appearance::Skin) {
        self.view.set_skin(skin);
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
            let repeat = |key| {
                repeat_only(self.window.is_key_down(key), |mode| {
                    self.window.is_key_pressed(key, mode)
                })
            };
            process_keyboard(
                &mut self.view,
                &self.snapshot,
                active,
                &keys,
                repeat(Key::Up),
                repeat(Key::Down),
                &mut self.actions,
            );
            if active {
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
            }
        }
        // Scrolling does not activate this window or change keyboard focus.
        if let Some((_, delta)) = self.window.get_scroll_wheel() {
            self.view.scroll(delta, pointer, &self.snapshot);
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
