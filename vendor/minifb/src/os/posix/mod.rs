#![cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
// turn off a gazillion warnings about X keysym names
#![allow(non_upper_case_globals)]

mod common;

/// EGL/GLES2 presentation. EGL binds to X11 as well, but only the Wayland
/// backend calls into this so far, so it is gated with that backend rather
/// than built into x11-only configurations that cannot reach it.
#[cfg(feature = "wayland")]
mod gl;

#[cfg(feature = "wayland")]
mod wayland;
#[cfg(feature = "x11")]
mod x11;
#[cfg(feature = "wayland")]
mod xkb_ffi;
#[cfg(feature = "wayland")]
mod xkb_keysyms;

use crate::{
    icon::Icon, CursorStyle, InputCallback, Key, KeyRepeat, MenuHandle, MouseButton, MouseMode,
    Result, UnixMenu, WindowOptions,
};
pub use common::Menu;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::{ffi::c_void, time::Duration};

// Differentiate between Wayland and X11 at run-time
#[allow(clippy::large_enum_variant)]
pub enum Window {
    #[cfg(feature = "x11")]
    X11(x11::Window),
    #[cfg(feature = "wayland")]
    Wayland(wayland::Window),
}

impl Window {
    #[cfg(all(feature = "x11", feature = "wayland"))]
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        // Try to create Wayland display first
        let wl_window = wayland::Window::new(name, width, height, opts);
        match wl_window {
            Ok(w) => Ok(Window::Wayland(w)),
            Err(_) => {
                // Create X11 Window when Wayland fails
                let window = Window::X11(x11::Window::new(name, width, height, opts)?);
                Ok(window)
            }
        }
    }

    #[cfg(all(feature = "wayland", not(feature = "x11")))]
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let wl_window = wayland::Window::new(name, width, height, opts)?;
        Ok(Window::Wayland(wl_window))
    }

    #[cfg(all(feature = "x11", not(feature = "wayland")))]
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let window = Window::X11(x11::Window::new(name, width, height, opts)?);
        Ok(window)
    }

    pub fn set_title(&mut self, title: &str) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_title(title),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_title(title),
        }
    }

    pub fn set_icon(&mut self, icon: Icon) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_icon(icon),
            #[cfg(feature = "wayland")]
            Window::Wayland(_w) => {
                unimplemented!("Cannot set icons at runtime on Wayland, create a .desktop file!")
            }
        }
    }

    pub fn set_titlebar_drag_region(&mut self, region: Option<(usize, usize, usize, usize)>) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_titlebar_drag_region(region),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_titlebar_drag_region(region),
        }
    }

    pub fn set_docked_parent(
        &mut self,
        parent: Option<&mut Window>,
        offset_x: isize,
        offset_y: isize,
    ) -> Result<()> {
        match (self, parent) {
            #[cfg(feature = "x11")]
            (Window::X11(child), Some(Window::X11(parent))) => {
                child.set_docked_parent(Some(parent), offset_x, offset_y)
            }
            #[cfg(feature = "x11")]
            (Window::X11(child), None) => child.set_docked_parent(None, offset_x, offset_y),
            #[cfg(feature = "wayland")]
            (Window::Wayland(child), Some(Window::Wayland(parent))) => {
                child.set_docked_parent(Some(parent), offset_x, offset_y)
            }
            #[cfg(feature = "wayland")]
            (Window::Wayland(child), None) => child.set_docked_parent(None, offset_x, offset_y),
            #[cfg(all(feature = "wayland", feature = "x11"))]
            _ => Err(crate::Error::UpdateFailed(
                "Cannot dock windows from different backends".into(),
            )),
        }
    }

    pub fn set_dock_magnetic(&mut self, enabled: bool) -> Result<()> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(window) => window.set_dock_magnetic(enabled),
            #[cfg(feature = "wayland")]
            Window::Wayland(_) => {
                let _ = enabled;
                Err(crate::Error::UpdateFailed(
                    "Magnetic window docking is not supported on Wayland".into(),
                ))
            }
        }
    }

    pub fn get_work_area(&self) -> Option<(isize, isize, usize, usize)> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(window) => window.get_work_area(),
            #[cfg(feature = "wayland")]
            Window::Wayland(window) => window.get_work_area(),
        }
    }

    pub fn minimize(&mut self) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.minimize(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.minimize(),
        }
    }

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => {
                w.update_with_buffer_stride(buffer, buf_width, buf_height, buf_stride)
            }
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => {
                w.update_with_buffer_stride(buffer, buf_width, buf_height, buf_stride)
            }
        }
    }

    pub fn update(&mut self) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.update(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.update(),
        }
    }

    pub fn get_window_handle(&self) -> *mut c_void {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_window_handle(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_window_handle(),
        }
    }

    pub fn set_background_color(&mut self, bg_color: u32) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_background_color(bg_color),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_background_color(bg_color),
        }
    }

    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_cursor_visibility(visibility),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_cursor_visibility(visibility),
        }
    }

    pub fn set_position(&mut self, x: isize, y: isize) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_position(x, y),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_position(x, y),
        }
    }

    pub fn get_position(&self) -> (isize, isize) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_position(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_position(),
        }
    }

    pub fn topmost(&self, _topmost: bool) {
        // We will just do nothing until it is implemented so that nothing breaks
    }

    pub fn get_size(&self) -> (usize, usize) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_size(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_size(),
        }
    }

    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_mouse_pos(mode),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_mouse_pos(mode),
        }
    }

    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_unscaled_mouse_pos(mode),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_unscaled_mouse_pos(mode),
        }
    }

    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_mouse_down(button),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_mouse_down(button),
        }
    }

    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_scroll_wheel(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_scroll_wheel(),
        }
    }

    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_cursor_style(cursor),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_cursor_style(cursor),
        }
    }

    pub fn set_rate(&mut self, rate: Option<Duration>) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_rate(rate),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_rate(rate),
        }
    }

    pub fn get_delta_time(&self) -> Option<Duration> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_delta_time(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_delta_time(),
        }
    }

    pub fn update_rate(&mut self) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.update_rate(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.update_rate(),
        }
    }

    pub fn get_keys(&self) -> Vec<Key> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_keys(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_keys(),
        }
    }

    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Vec<Key> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_keys_pressed(repeat),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_keys_pressed(repeat),
        }
    }

    pub fn get_keys_released(&self) -> Vec<Key> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_keys_released(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_keys_released(),
        }
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.is_key_down(key),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.is_key_down(key),
        }
    }

    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_key_repeat_delay(delay),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_key_repeat_delay(delay),
        }
    }

    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_key_repeat_rate(rate),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_key_repeat_rate(rate),
        }
    }

    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.is_key_pressed(key, repeat),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.is_key_pressed(key, repeat),
        }
    }

    pub fn is_key_released(&self, key: Key) -> bool {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.is_key_released(key),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.is_key_released(key),
        }
    }

    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.set_input_callback(callback),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.set_input_callback(callback),
        }
    }

    pub fn is_open(&self) -> bool {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.is_open(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.is_open(),
        }
    }

    pub fn is_active(&self) -> bool {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.is_active(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.is_active(),
        }
    }

    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.add_menu(menu),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.add_menu(menu),
        }
    }

    pub fn get_posix_menus(&self) -> Option<&Vec<UnixMenu>> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.get_posix_menus(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.get_posix_menus(),
        }
    }

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.remove_menu(handle),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.remove_menu(handle),
        }
    }

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.is_menu_pressed(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.is_menu_pressed(),
        }
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.window_handle(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.window_handle(),
        }
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        match self {
            #[cfg(feature = "x11")]
            Window::X11(w) => w.display_handle(),
            #[cfg(feature = "wayland")]
            Window::Wayland(w) => w.display_handle(),
        }
    }
}
