//! Native two-window regressions; macOS must run `smoke` on the main thread.
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

struct Observed {
    chars: Rc<RefCell<Vec<char>>>,
    keys: Rc<RefCell<Vec<(Key, bool, bool)>>>,
}

fn observe(window: &mut Window) -> Observed {
    let chars = Rc::new(RefCell::new(Vec::new()));
    let keys = Rc::new(RefCell::new(Vec::new()));
    window.set_input_callback(Box::new(super::TextInput {
        chars: chars.clone(),
        keys: keys.clone(),
        mouse: Rc::default(),
        held: [false; 512],
        modifiers: 0,
    }));
    Observed { chars, keys }
}

fn window(name: &str) -> Result<Window, String> {
    let mut window = Window::new(
        name,
        200,
        120,
        WindowOptions {
            borderless: true,
            title: false,
            resize: false,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| e.to_string())?;
    window.set_target_fps(0);
    Ok(window)
}

fn pump(a: &mut Window, b: &mut Window) {
    let until = Instant::now() + Duration::from_millis(100);
    while Instant::now() < until {
        a.update();
        b.update();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn check_focus_loss(a: &Window) -> Result<(), String> {
    for key in [Key::LeftCtrl, Key::F3, Key::Up] {
        if a.is_key_down(key) || a.is_key_pressed(key, KeyRepeat::Yes) {
            return Err(format!("{key:?} remains down/repeating after focus loss"));
        }
    }
    Ok(())
}

fn check_fresh_input(input: &Observed) -> Result<(), String> {
    if !input.chars.borrow().contains(&'a') {
        return Err("stale Ctrl suppresses text after focus returns".into());
    }
    let keys = input.keys.borrow();
    for key in [Key::F3, Key::Up] {
        if !keys.contains(&(key, false, false)) {
            return Err(format!(
                "{key:?} is blocked or keeps stale modifiers after focus returns"
            ));
        }
    }
    Ok(())
}

#[cfg(all(test, target_os = "windows"))]
mod windows {
    use super::*;
    use std::ffi::c_void;
    type Hwnd = *mut c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetFocus(window: Hwnd) -> Hwnd;
        fn GetFocus() -> Hwnd;
        fn SendMessageW(window: Hwnd, message: u32, wp: usize, lp: isize) -> isize;
        fn IsWindow(window: Hwnd) -> i32;
        fn GetGuiResources(process: Hwnd, flags: u32) -> u32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> Hwnd;
    }
    fn key(window: &Window, scan: usize, down: bool) {
        unsafe {
            SendMessageW(
                window.get_window_handle(),
                if down { 0x100 } else { 0x101 },
                0,
                (scan << 16) as isize,
            );
        }
    }
    fn focus(window: &Window) {
        unsafe {
            SetFocus(window.get_window_handle());
            assert_eq!(GetFocus(), window.get_window_handle());
        }
    }

    #[test]
    fn win32_two_windows_release_focus_keys_and_close_independently() {
        let _serial = crate::window_native_windows_tests::WINDOWS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut a = window("Spotiamp parent focus test").unwrap();
        let input = observe(&mut a);
        let mut b = window("Spotiamp library focus test").unwrap();
        pump(&mut a, &mut b);
        focus(&a);
        for scan in [0x1d, 0x3d, 0x148] {
            key(&a, scan, true);
        }
        a.update();
        assert!(a.is_key_down(Key::LeftCtrl));
        assert!(a.is_key_down(Key::F3));
        assert!(a.is_key_down(Key::Up));
        // Capture/menu cancellation alone must not release keyboard state.
        unsafe {
            SendMessageW(a.get_window_handle(), 0x001f, 0, 0);
        }
        assert!(a.is_key_down(Key::LeftCtrl));
        focus(&b);
        for scan in [0x1d, 0x3d, 0x148] {
            key(&b, scan, false);
        }
        check_focus_loss(&a).unwrap();
        focus(&a);
        input.chars.borrow_mut().clear();
        input.keys.borrow_mut().clear();
        unsafe {
            SendMessageW(a.get_window_handle(), 0x102, 'a' as usize, 0);
        }
        key(&a, 0x3d, true);
        key(&a, 0x148, true);
        check_fresh_input(&input).unwrap();
        let parent = a.get_window_handle();
        // Warm up native classes/caches before counting resources.
        unsafe {
            SendMessageW(b.get_window_handle(), 0x10, 0, 0);
        }
        assert!(!b.is_open());
        drop(b);
        let gdi_before = unsafe { GetGuiResources(GetCurrentProcess(), 0) };
        for _ in 0..20 {
            let mut child = window("Spotiamp library lifecycle test").unwrap();
            child
                .update_with_buffer(&vec![0x29283d; 200 * 120], 200, 120)
                .unwrap();
            unsafe {
                SendMessageW(child.get_window_handle(), 0x10, 0, 0);
            }
            assert!(!child.is_open());
            drop(child);
            a.update();
            assert!(a.is_open());
            assert_ne!(unsafe { IsWindow(parent) }, 0);
        }
        let gdi_after = unsafe { GetGuiResources(GetCurrentProcess(), 0) };
        assert!(
            gdi_after <= gdi_before + 2,
            "GDI resources grow per open: {gdi_before} -> {gdi_after}"
        );
    }
}

#[cfg(all(test, target_os = "linux"))]
mod x11 {
    use super::*;
    use std::ffi::{c_char, c_int, c_long, c_ulong, c_void};

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ClientMessage {
        kind: c_int,
        serial: c_ulong,
        sent: c_int,
        display: *mut c_void,
        window: c_ulong,
        message_type: c_ulong,
        format: c_int,
        data: [c_long; 5],
    }
    #[repr(C)]
    union Event {
        client: ClientMessage,
        pad: [c_long; 24],
    }
    #[link(name = "X11")]
    unsafe extern "C" {
        fn XOpenDisplay(name: *const c_char) -> *mut c_void;
        fn XInternAtom(display: *mut c_void, name: *const c_char, only: c_int) -> c_ulong;
        fn XSendEvent(
            display: *mut c_void,
            window: c_ulong,
            propagate: c_int,
            mask: c_long,
            event: *mut Event,
        ) -> c_int;
        fn XCloseDisplay(display: *mut c_void) -> c_int;
        fn XMoveWindow(display: *mut c_void, window: c_ulong, x: c_int, y: c_int) -> c_int;
        fn XClearArea(
            display: *mut c_void,
            window: c_ulong,
            x: c_int,
            y: c_int,
            width: u32,
            height: u32,
            exposures: c_int,
        ) -> c_int;
        fn XSync(display: *mut c_void, discard: c_int) -> c_int;
        fn XGetImage(
            display: *mut c_void,
            drawable: c_ulong,
            x: c_int,
            y: c_int,
            width: u32,
            height: u32,
            mask: c_ulong,
            format: c_int,
        ) -> *mut c_void;
        fn XGetPixel(image: *mut c_void, x: c_int, y: c_int) -> c_ulong;
        fn XDestroyImage(image: *mut c_void) -> c_int;
    }
    fn close(window: &Window) {
        unsafe {
            let display = XOpenDisplay(std::ptr::null());
            assert!(!display.is_null());
            let mut event = Event { pad: [0; 24] };
            event.client = ClientMessage {
                kind: 33,
                serial: 0,
                sent: 1,
                display,
                window: window.get_window_handle() as c_ulong,
                message_type: XInternAtom(display, c"WM_PROTOCOLS".as_ptr(), 0),
                format: 32,
                data: [
                    XInternAtom(display, c"WM_DELETE_WINDOW".as_ptr(), 0) as c_long,
                    0,
                    0,
                    0,
                    0,
                ],
            };
            let sent = XSendEvent(
                display,
                window.get_window_handle() as c_ulong,
                0,
                0,
                &mut event,
            );
            XCloseDisplay(display); // flushes the protocol message
            assert_ne!(sent, 0);
        }
    }
    fn command(a: &mut Window, b: &mut Window, args: &[&str]) {
        let mut child = std::process::Command::new("xdotool")
            .args(args)
            .spawn()
            .expect("xdotool is required");
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            a.update();
            b.update();
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "xdotool {args:?}");
                break;
            }
            if Instant::now() >= until {
                let _ = child.kill();
                let _ = child.wait();
                panic!("xdotool timed out: {args:?}");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        pump(a, b);
    }

    fn pixel(display: *mut c_void, window: &Window) -> u32 {
        unsafe {
            let image = XGetImage(
                display,
                window.get_window_handle() as c_ulong,
                10,
                10,
                1,
                1,
                c_ulong::MAX,
                2,
            ); // ZPixmap
            assert!(!image.is_null());
            let pixel = XGetPixel(image, 0, 0) as u32 & 0x00ff_ffff;
            XDestroyImage(image);
            pixel
        }
    }

    #[test]
    #[ignore = "requires an isolated Xvfb display"]
    fn x11_expose_after_move_restores_cached_pixels_without_a_new_frame() {
        for scale in [minifb::Scale::X1, minifb::Scale::X2] {
            let mut window = Window::new(
                "Spotiamp cached frame regression",
                200,
                120,
                WindowOptions {
                    borderless: true,
                    title: false,
                    resize: false,
                    scale,
                    ..WindowOptions::default()
                },
            )
            .unwrap();
            window.set_target_fps(0);
            let color = 0x2b9d53;
            // This is the only application frame submitted in this test.
            window
                .update_with_buffer(&vec![color; 200 * 120], 200, 120)
                .unwrap();
            let display = unsafe { XOpenDisplay(std::ptr::null()) };
            assert!(!display.is_null());
            let until = Instant::now() + Duration::from_secs(2);
            while pixel(display, &window) != color {
                assert!(Instant::now() < until, "initial frame did not reach X11");
                window.update();
                std::thread::sleep(Duration::from_millis(5));
            }
            unsafe {
                XMoveWindow(display, window.get_window_handle() as c_ulong, 80, 90);
                // Explicitly erase pixels and request an Expose, just as a
                // window manager may require after restoring an idle window.
                XClearArea(
                    display,
                    window.get_window_handle() as c_ulong,
                    0,
                    0,
                    0,
                    0,
                    1,
                );
                XSync(display, 0);
            }
            assert_eq!(
                pixel(display, &window),
                0,
                "the test must actually erase the native drawable"
            );
            let until = Instant::now() + Duration::from_secs(2);
            loop {
                window.update();
                if pixel(display, &window) == color {
                    break;
                }
                assert!(
                    Instant::now() < until,
                    "Expose lost the cached frame at {scale:?}"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            unsafe {
                XCloseDisplay(display);
            }
        }
    }

    #[test]
    #[ignore = "requires isolated Xvfb and xdotool"]
    fn x11_two_windows_release_focus_keys_and_close_independently() {
        let mut a = window("Spotiamp parent focus test").unwrap();
        let input = observe(&mut a);
        let mut b = window("Spotiamp library focus test").unwrap();
        let aid = (a.get_window_handle() as usize).to_string();
        let bid = (b.get_window_handle() as usize).to_string();
        pump(&mut a, &mut b);
        command(&mut a, &mut b, &["windowfocus", "--sync", &aid]);
        command(
            &mut a,
            &mut b,
            &["keydown", "--window", &aid, "Control_L", "F3", "Up"],
        );
        assert!(a.is_key_down(Key::LeftCtrl));
        assert!(a.is_key_down(Key::F3));
        assert!(a.is_key_down(Key::Up));
        command(&mut a, &mut b, &["windowfocus", "--sync", &bid]);
        command(
            &mut a,
            &mut b,
            &["keyup", "--window", &bid, "Control_L", "F3", "Up"],
        );
        check_focus_loss(&a).unwrap();
        command(&mut a, &mut b, &["windowfocus", "--sync", &aid]);
        input.chars.borrow_mut().clear();
        input.keys.borrow_mut().clear();
        command(&mut a, &mut b, &["key", "--window", &aid, "a", "F3", "Up"]);
        check_fresh_input(&input).unwrap();
        drop(b);
        let descriptors_before = std::fs::read_dir("/proc/self/fd").unwrap().count();
        for _ in 0..20 {
            let mut child = window("Spotiamp library lifecycle test").unwrap();
            child
                .update_with_buffer(&vec![0x29283d; 200 * 120], 200, 120)
                .unwrap();
            close(&child);
            pump(&mut a, &mut child);
            assert!(!child.is_open());
            drop(child);
            a.update();
            assert!(a.is_open());
        }
        let descriptors_after = std::fs::read_dir("/proc/self/fd").unwrap().count();
        assert_eq!(
            descriptors_after, descriptors_before,
            "X11 connections must close with their windows"
        );
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::ffi::{CStr, c_char, c_void};
    type Id = *mut c_void;
    type Sel = *mut c_void;
    #[repr(C)]
    struct Point {
        x: f64,
        y: f64,
    }
    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
        fn object_getClass(object: Id) -> Id;
        fn class_getInstanceVariable(class: Id, name: *const c_char) -> Id;
        fn object_getIvar(object: Id, ivar: Id) -> Id;
        fn objc_initWeak(slot: *mut Id, value: Id) -> Id;
        fn objc_loadWeakRetained(slot: *mut Id) -> Id;
        fn objc_destroyWeak(slot: *mut Id);
    }
    unsafe fn void0(object: Id, name: &CStr) {
        let send: unsafe extern "C" fn(Id, Sel) =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { send(object, sel_registerName(name.as_ptr())) };
    }
    unsafe fn void1(object: Id, name: &CStr, arg: Id) {
        let send: unsafe extern "C" fn(Id, Sel, Id) =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { send(object, sel_registerName(name.as_ptr()), arg) };
    }
    unsafe fn id0(object: Id, name: &CStr) -> Id {
        let send: unsafe extern "C" fn(Id, Sel) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe { send(object, sel_registerName(name.as_ptr())) }
    }
    struct Weak(Box<Id>);
    impl Weak {
        unsafe fn new(object: Id) -> Self {
            let mut value = Self(Box::new(std::ptr::null_mut()));
            unsafe {
                objc_initWeak(&mut *value.0, object);
            }
            value
        }
        fn released(&mut self) -> bool {
            unsafe {
                let object = objc_loadWeakRetained(&mut *self.0);
                if object.is_null() {
                    true
                } else {
                    void0(object, c"release");
                    false
                }
            }
        }
    }
    impl Drop for Weak {
        fn drop(&mut self) {
            unsafe {
                objc_destroyWeak(&mut *self.0);
            }
        }
    }
    unsafe fn key(window: &Window, key: u16, down: bool, control: bool, chars: &CStr) {
        let string: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        let event: unsafe extern "C" fn(
            Id,
            Sel,
            usize,
            Point,
            usize,
            f64,
            isize,
            Id,
            Id,
            Id,
            bool,
            u16,
        ) -> Id = unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        let number: unsafe extern "C" fn(Id, Sel) -> isize =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe {
            let text = string(
                objc_getClass(c"NSString".as_ptr()),
                sel_registerName(c"stringWithUTF8String:".as_ptr()),
                chars.as_ptr(),
            );
            let kind = if key == 0x3b {
                12
            } else if down {
                10
            } else {
                11
            };
            let e = event(objc_getClass(c"NSEvent".as_ptr()), sel_registerName(c"keyEventWithType:location:modifierFlags:timestamp:windowNumber:context:characters:charactersIgnoringModifiers:isARepeat:keyCode:".as_ptr()), kind, Point{x:0.0,y:0.0}, if control { 1 << 18 } else { 0 }, 0.0, number(window.get_window_handle(), sel_registerName(c"windowNumber".as_ptr())), std::ptr::null_mut(), text, text, false, key);
            void1(
                window.get_window_handle(),
                if key == 0x3b {
                    c"flagsChanged:"
                } else if down {
                    c"keyDown:"
                } else {
                    c"keyUp:"
                },
                e,
            );
        }
    }

    pub fn run() -> Result<(), String> {
        let mut a = window("Spotiamp parent focus test")?;
        let input = observe(&mut a);
        let mut b = window("Spotiamp library focus test")?;
        pump(&mut a, &mut b);
        unsafe {
            void1(
                a.get_window_handle(),
                c"makeKeyAndOrderFront:",
                std::ptr::null_mut(),
            );
            key(&a, 0x3b, true, true, c"");
            key(&a, 0x63, true, true, c"");
            key(&a, 0x7e, true, true, c"");
        }
        a.update();
        if ![Key::LeftCtrl, Key::F3, Key::Up]
            .iter()
            .all(|key| a.is_key_down(*key))
        {
            return Err("native Mac key injection failed".into());
        }
        unsafe {
            void1(
                b.get_window_handle(),
                c"makeKeyAndOrderFront:",
                std::ptr::null_mut(),
            );
            key(&b, 0x3b, false, false, c"");
            key(&b, 0x63, false, false, c"");
            key(&b, 0x7e, false, false, c"");
        }
        check_focus_loss(&a)?;
        unsafe {
            void1(
                a.get_window_handle(),
                c"makeKeyAndOrderFront:",
                std::ptr::null_mut(),
            );
        }
        input.chars.borrow_mut().clear();
        input.keys.borrow_mut().clear();
        unsafe {
            key(&a, 0, true, false, c"a");
            key(&a, 0x63, true, false, c"");
            key(&a, 0x7e, true, false, c"");
        }
        check_fresh_input(&input)?;
        drop(b);
        for _ in 0..20 {
            // Pool scope makes the native lifetime assertion deterministic.
            let pool = unsafe {
                id0(
                    id0(objc_getClass(c"NSAutoreleasePool".as_ptr()), c"alloc"),
                    c"init",
                )
            };
            let mut child = window("Spotiamp library lifecycle test")?;
            let mut weak_window = unsafe { Weak::new(child.get_window_handle()) };
            let mut weak_controller = unsafe {
                let class = object_getClass(child.get_window_handle());
                let ivar = class_getInstanceVariable(class, c"view_controller".as_ptr());
                if ivar.is_null() {
                    return Err("missing native view-controller ownership".into());
                }
                Weak::new(object_getIvar(child.get_window_handle(), ivar))
            };
            for _ in 0..4 {
                child
                    .update_with_buffer(&vec![0x29283d; 200 * 120], 200, 120)
                    .map_err(|e| e.to_string())?;
            }
            unsafe {
                void0(child.get_window_handle(), c"close");
            }
            if child.is_open() {
                return Err("native child close was not observed".into());
            }
            drop(child);
            unsafe {
                void0(pool, c"drain");
            }
            if !weak_window.released() || !weak_controller.released() {
                return Err("closed Mac window or Metal controller remains retained".into());
            }
            a.update();
            if !a.is_open() {
                return Err("closing the library closed the player".into());
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub fn smoke() -> Result<(), String> {
    macos::run()
}
