//! Docked native library. Its framebuffer and input queues exist only while open.
use super::{MouseInput, engine, library_ui, model::LibrarySnapshot, repeat_only};
use minifb::{Key, MouseButton, MouseMode, Scale, Window, WindowOptions};
use std::{cell::RefCell, rc::Rc};

pub struct LibraryWindow {
    window: Window,
    #[cfg(target_os = "windows")]
    _icons: Option<crate::windows_icon::WindowIcons>,
    view: library_ui::View,
    snapshot: LibrarySnapshot,
    input: Rc<RefCell<Vec<input::Input>>>,
    actions: Vec<library_ui::Action>,
}

#[cfg(all(test, target_os = "linux", feature = "spotify"))]
#[path = "wheel_native_x11_tests.rs"]
mod wheel_native_x11_tests;

#[cfg(all(test, target_os = "linux", feature = "spotify"))]
#[path = "library_search_native_tests.rs"]
mod search_native_tests;

#[path = "library_input.rs"]
mod input;

#[cfg(test)]
#[path = "library_keyboard_tests.rs"]
mod keyboard_tests;

pub(super) fn inside(x: f32, y: f32) -> Option<(usize, usize)> {
    (x >= 0.0 && y >= 0.0 && x < library_ui::WIDTH as f32 && y < library_ui::HEIGHT as f32)
        .then_some((x as usize, y as usize))
}

impl LibraryWindow {
    pub fn set_language(&mut self, language: crate::locale::Language) {
        self.view.set_language(language);
        self.window.set_title(language.text(
            "Spotiamp — Bibliothèque Spotify",
            "Spotiamp — Spotify Library",
        ));
    }

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
        let input = Rc::new(RefCell::new(Vec::with_capacity(16)));
        window.set_input_callback(Box::new(input::Callback::new(input.clone())));
        Ok(Self {
            window,
            #[cfg(target_os = "windows")]
            _icons: icons,
            view: library_ui::View::default(),
            snapshot: LibrarySnapshot::default(),
            input,
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
    pub fn frame(
        &mut self,
        engine: &engine::Engine,
        clipboard: &mut Option<arboard::Clipboard>,
    ) -> Result<bool, minifb::Error> {
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
            let repeat = |key| {
                repeat_only(self.window.is_key_down(key), |mode| {
                    self.window.is_key_pressed(key, mode)
                })
            };
            let control = [
                Key::LeftCtrl,
                Key::RightCtrl,
                Key::LeftSuper,
                Key::RightSuper,
            ]
            .into_iter()
            .any(|key| self.window.is_key_down(key));
            let shift =
                self.window.is_key_down(Key::LeftShift) || self.window.is_key_down(Key::RightShift);
            let repeats = [
                Key::Up,
                Key::Down,
                Key::Backspace,
                Key::Delete,
                Key::Left,
                Key::Right,
            ]
            .map(|key| (key, repeat(key), control, shift));
            input::process(
                &mut self.view,
                &self.snapshot,
                active,
                &self.input.borrow(),
                &repeats,
                &mut |text| {
                    if clipboard.is_none() {
                        *clipboard = arboard::Clipboard::new().ok();
                    }
                    let clipboard = clipboard.as_mut().ok_or(())?;
                    match text {
                        Some(text) => clipboard
                            .set_text(text)
                            .map(|()| String::new())
                            .map_err(|_| ()),
                        None => clipboard.get_text().map_err(|_| ()),
                    }
                },
                &mut self.actions,
            );
            if active {
                self.view.pointer_move(pointer);
            }
        }
        // Scrolling does not activate this window or change keyboard focus.
        if let Some((_, delta)) = self.window.get_scroll_wheel() {
            self.view.scroll(delta, pointer, &self.snapshot);
        }
        self.input.borrow_mut().clear();
        for action in self.actions.drain(..) {
            match action {
                library_ui::Action::Close => return Ok(false),
                library_ui::Action::Minimize => {
                    self.window.minimize();
                    self.view.cancel_pointer();
                }
                library_ui::Action::Paste | library_ui::Action::Copy(_) => {
                    unreachable!("clipboard actions are resolved in input order")
                }
                library_ui::Action::Command(command) => {
                    if let Err(error) = engine.tx.try_send(super::model::Command::Library(command))
                    {
                        self.view.command_rejected();
                        self.view.notice(engine::command_error(&error));
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
