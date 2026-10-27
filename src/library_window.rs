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
    drag: input::PointerDrag,
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
            drag: input::PointerDrag::default(),
        })
    }

    pub fn placement(&self) -> Option<minifb::RestorablePlacement> {
        self.window.get_restorable_placement()
    }

    pub fn set_skin(&mut self, skin: crate::appearance::Skin) {
        self.view.set_skin(skin);
    }

    pub fn drag_pointer_on(&self, player: &Window) -> Option<(f32, f32)> {
        self.drag
            .active
            .then(|| self.window.get_mouse_pos(MouseMode::Pass))
            .flatten()
            .filter(|&(x, y)| inside(x, y).is_none())
            .map(|point| self.point_on_player(player, point))
    }

    fn point_on_player(&self, player: &Window, point: (f32, f32)) -> (f32, f32) {
        translate_point(
            point,
            self.window.get_position(),
            self.window.get_size(),
            player.get_position(),
            player.get_size(),
        )
    }

    pub fn take_drop(
        &mut self,
        player: &Window,
    ) -> Option<(
        Vec<crate::model::TrackRow>,
        Option<crate::model::PlaylistId>,
        (f32, f32),
    )> {
        while !self.drag.completed.is_empty() {
            let (tracks, origin, point) = self.drag.completed.remove(0);
            if inside(point.0, point.1).is_none() {
                return Some((tracks, origin, self.point_on_player(player, point)));
            }
        }
        None
    }

    /// Consume captured library releases once, through the same path as buttons.
    pub fn drop_into_player(
        &mut self,
        player: &Window,
        view: &mut crate::ui::View,
        snapshot: &crate::model::Snapshot,
        engine: &engine::Engine,
    ) {
        view.show_queue_drop(self.drag_pointer_on(player), snapshot.queue.len());
        while let Some((tracks, origin, position)) = self.take_drop(player) {
            if let Some(index) = view.queue_drop_index(position, snapshot.queue.len()) {
                let command = crate::model::LibraryCommand::QueueTracks {
                    tracks,
                    origin,
                    expected_queue: Some(
                        snapshot.queue.iter().map(|row| row.uri.clone()).collect(),
                    ),
                    replace: false,
                    play: false,
                    insert_at: Some(index),
                };
                if let Err(error) = engine.tx.try_send(crate::model::Command::Library(command)) {
                    view.notice(engine::command_error(&error));
                }
            }
        }
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
            input::process_with_drag(
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
                &mut self.drag,
            );
            if active {
                self.drag
                    .moved(self.window.get_mouse_pos(MouseMode::Pass), &mut self.view);
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
                    dispatch_command(&mut self.view, &engine.tx, command);
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

#[cfg(feature = "spotify")]
type LibraryCommandSender = tokio::sync::mpsc::Sender<super::model::Command>;
#[cfg(not(feature = "spotify"))]
type LibraryCommandSender = engine::Sender;

// Keep acknowledgement adjacent to the actual channel send, after the whole
// native input batch has been processed. No command or track clone is needed.
fn dispatch_command(
    view: &mut library_ui::View,
    sender: &LibraryCommandSender,
    command: super::model::LibraryCommand,
) {
    let creation = matches!(command, super::model::LibraryCommand::CreatePlaylist { .. });
    match sender.try_send(super::model::Command::Library(command)) {
        Ok(()) => {
            if creation {
                view.creation_dispatched();
            }
        }
        Err(error) => {
            view.notice(engine::command_error(&error));
            if let super::model::Command::Library(command) = error.into_inner() {
                view.command_rejected(&command);
            }
        }
    }
}

// Both minifb positions and sizes use the same desktop units on each platform.
// Input callbacks use framebuffer pixels, including outside-window releases.
fn translate_point(
    point: (f32, f32),
    source: (isize, isize),
    source_size: (usize, usize),
    destination: (isize, isize),
    destination_size: (usize, usize),
) -> (f32, f32) {
    (
        (source.0 - destination.0) as f32 * crate::ui::WIDTH as f32
            / destination_size.0.max(1) as f32
            + point.0 * source_size.0 as f32 / library_ui::WIDTH as f32 * crate::ui::WIDTH as f32
                / destination_size.0.max(1) as f32,
        (source.1 - destination.1) as f32 * crate::ui::HEIGHT as f32
            / destination_size.1.max(1) as f32
            + point.1 * source_size.1 as f32 / library_ui::HEIGHT as f32 * crate::ui::HEIGHT as f32
                / destination_size.1.max(1) as f32,
    )
}

#[cfg(test)]
mod drop_coordinate_tests {
    use super::translate_point;
    #[test]
    fn scaled_and_negative_desktop_positions_translate_without_clamping() {
        assert_eq!(
            translate_point((-360.0, 269.0), (560, 0), (760, 470), (0, 0), (560, 470)),
            (200.0, 269.0)
        );
        assert_eq!(
            translate_point(
                (-360.0, 269.0),
                (-880, -200),
                (1520, 940),
                (-2000, -200),
                (1120, 940)
            ),
            (200.0, 269.0)
        );
        assert_eq!(
            translate_point((200.0, -201.0), (0, 470), (760, 470), (0, 0), (560, 470)),
            (200.0, 269.0)
        );
    }
}
