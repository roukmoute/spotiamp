//! Exercises the native pair, with no Spotify session or application renderer.
#![cfg(any(all(test, target_os = "linux"), target_os = "macos"))]
use minifb::{Scale, Window, WindowOptions};
use std::time::{Duration, Instant};

fn window(name: &str, scale: Scale) -> Result<Window, String> {
    let mut window = Window::new(
        name,
        120,
        80,
        WindowOptions {
            borderless: true,
            title: false,
            resize: false,
            scale,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| e.to_string())?;
    window.set_target_fps(0);
    window.set_titlebar_drag_region(Some((4, 2, 90, 16)));
    window
        .update_with_buffer(&vec![0x47425b; 120 * 80], 120, 80)
        .map_err(|e| e.to_string())?;
    Ok(window)
}

fn pump(a: &mut Window, b: &mut Window, duration: Duration) {
    let until = Instant::now() + duration;
    while Instant::now() < until {
        a.update();
        b.update();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn attached(a: &Window, b: &Window, offset: isize) -> bool {
    let parent = a.get_position();
    b.get_position() == (parent.0 + offset, parent.1)
}

#[cfg(all(test, target_os = "linux"))]
mod x11 {
    use super::*;
    use std::process::{Child, Command};
    struct Process(Child);
    impl Drop for Process {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn property(id: &str, name: &str) -> String {
        let result = Command::new("xprop")
            .args(["-id", id, name])
            .output()
            .unwrap();
        assert!(result.status.success());
        String::from_utf8(result.stdout).unwrap()
    }
    fn until(a: &mut Window, b: &mut Window, condition: impl Fn(&Window, &Window) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            a.update();
            b.update();
            if condition(a, b) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "native docking condition timed out: {:?} / {:?}",
                a.get_position(),
                b.get_position()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn command(a: &mut Window, b: &mut Window, args: &[&str]) {
        let mut process = Process(Command::new("xdotool").args(args).spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            a.update();
            b.update();
            if let Some(status) = process.0.try_wait().unwrap() {
                assert!(status.success(), "{args:?}");
                break;
            }
            assert!(Instant::now() < deadline, "xdotool timed out: {args:?}");
            std::thread::sleep(Duration::from_millis(5));
        }
        pump(a, b, Duration::from_millis(100));
    }
    fn manager() -> Process {
        // This startup marker, unlike _NET_SUPPORTING_WM_CHECK, proves that
        // Openbox has finished initialization on Ubuntu as well as Debian.
        let atom = format!("_SPOTIAMP_DOCK_READY_{}", std::process::id());
        let startup = format!("xprop -root -f {atom} 8s -set {atom} ready");
        let mut process = Process(
            Command::new("openbox")
                .args(["--sm-disable", "--startup", &startup])
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(process.0.try_wait().unwrap().is_none());
            let result = Command::new("xprop")
                .args(["-root", &atom])
                .output()
                .unwrap();
            if String::from_utf8_lossy(&result.stdout).contains("= \"ready\"") {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Openbox initialization timed out"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        process
    }
    fn drag(a: &mut Window, b: &mut Window, id: &str, factor: usize, offset: isize) {
        let x = (20 * factor).to_string();
        let y = (9 * factor).to_string();
        let before = a.get_position();
        let mut process = Process(
            Command::new("xdotool")
                .args([
                    "mousemove",
                    "--window",
                    id,
                    &x,
                    &y,
                    "mousedown",
                    "1",
                    "sleep",
                    "0.15",
                    "mousemove_relative",
                    "--",
                    "75",
                    "40",
                    "sleep",
                    "0.8",
                    "mouseup",
                    "1",
                ])
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut followed_before_release = false;
        loop {
            a.update();
            b.update();
            if let Some(status) = process.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if a.get_position() != before && attached(a, b, offset) {
                followed_before_release = true;
            }
            assert!(Instant::now() < deadline, "native drag timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            followed_before_release,
            "peer must follow while mouse is held, not just after release"
        );
        until(a, b, |a, b| attached(a, b, offset));
        let after = a.get_position();
        assert!(
            (after.0 - before.0 - 75).abs() <= 2 && (after.1 - before.1 - 40).abs() <= 2,
            "native drag moved pair incorrectly: {before:?} -> {after:?}"
        );
    }

    #[test]
    #[ignore = "requires an isolated Xvfb display without a window manager"]
    fn x11_queued_reparent_to_destroyed_frame_is_safe() {
        use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};
        #[link(name = "X11")]
        unsafe extern "C" {
            fn XOpenDisplay(name: *const c_char) -> *mut c_void;
            fn XDefaultRootWindow(display: *mut c_void) -> c_ulong;
            fn XCreateSimpleWindow(
                display: *mut c_void,
                parent: c_ulong,
                x: c_int,
                y: c_int,
                width: c_uint,
                height: c_uint,
                border: c_uint,
                border_pixel: c_ulong,
                background: c_ulong,
            ) -> c_ulong;
            fn XReparentWindow(
                display: *mut c_void,
                window: c_ulong,
                parent: c_ulong,
                x: c_int,
                y: c_int,
            ) -> c_int;
            fn XDestroyWindow(display: *mut c_void, window: c_ulong) -> c_int;
            fn XSync(display: *mut c_void, discard: c_int) -> c_int;
            fn XCloseDisplay(display: *mut c_void) -> c_int;
        }
        let mut a = window("Spotiamp queued frame parent", Scale::X1).unwrap();
        let mut b = window("Spotiamp queued frame companion", Scale::X1).unwrap();
        b.set_docked_parent(Some(&mut a), 120, 0).unwrap();
        pump(&mut a, &mut b, Duration::from_millis(50));
        unsafe {
            let display = XOpenDisplay(std::ptr::null());
            assert!(!display.is_null());
            let root = XDefaultRootWindow(display);
            let frame = XCreateSimpleWindow(display, root, 50, 50, 300, 200, 0, 0, 0);
            let client = a.get_window_handle() as c_ulong;
            XReparentWindow(display, client, frame, 0, 0);
            XReparentWindow(display, client, root, 80, 80);
            XDestroyWindow(display, frame);
            XSync(display, 0);
            // Both reparent events are queued, but the first frame is already
            // gone. Selecting its stale XID would terminate the process.
            pump(&mut a, &mut b, Duration::from_millis(100));
            XCloseDisplay(display);
        }
        assert!(a.is_open() && b.is_open());
        assert_eq!(a.get_size(), (120, 80));
        assert!(attached(&a, &b, 120));
    }

    #[test]
    #[ignore = "requires isolated Xvfb, Openbox, xprop and xdotool"]
    fn x11_docked_windows_follow_live_drag_minimize_detach_and_reopen_at_both_scales() {
        let _manager = manager();
        for (scale, factor) in [(Scale::X1, 1), (Scale::X2, 2)] {
            let mut a = window("Spotiamp dock parent test", scale).unwrap();
            let mut b = window("Spotiamp dock companion test", scale).unwrap();
            let aid = (a.get_window_handle() as usize).to_string();
            let bid = (b.get_window_handle() as usize).to_string();
            until(&mut a, &mut b, |_, _| {
                property(&aid, "WM_STATE").contains("Normal")
                    && property(&bid, "WM_STATE").contains("Normal")
            });
            let offset = a.get_size().0 as isize;
            b.set_docked_parent(Some(&mut a), offset, 0).unwrap();
            a.set_position(100, 140);
            until(&mut a, &mut b, |a, b| {
                a.get_position() == (100, 140) && attached(a, b, offset)
            });
            assert_eq!(a.get_size(), (120 * factor, 80 * factor));
            let (_, _, width, height) = a.get_work_area().expect("native work area");
            assert!(width > 0 && height > 0);
            drag(&mut a, &mut b, &aid, factor, offset);
            drag(&mut a, &mut b, &bid, factor, offset);
            // Root coordinates must not be doubled or converted to unsigned.
            a.set_position(-70, 100);
            until(&mut a, &mut b, |a, b| {
                a.get_position() == (-70, 100) && attached(a, b, offset)
            });
            b.set_position(180, 180);
            until(&mut a, &mut b, |a, b| {
                b.get_position() == (180, 180) && attached(a, b, offset)
            });
            let positions = (a.get_position(), b.get_position());
            pump(&mut a, &mut b, Duration::from_millis(250));
            assert_eq!(
                (a.get_position(), b.get_position()),
                positions,
                "idle must not oscillate"
            );
            // API and native window-manager minimization both group the pair.
            for external in [false, true] {
                if external {
                    command(&mut a, &mut b, &["windowminimize", &bid]);
                } else {
                    b.minimize();
                }
                until(&mut a, &mut b, |_, _| {
                    property(&aid, "WM_STATE").contains("Iconic")
                        && property(&bid, "WM_STATE").contains("Iconic")
                });
                command(
                    &mut a,
                    &mut b,
                    &["windowmap", &aid, "windowactivate", "--sync", &aid],
                );
                until(&mut a, &mut b, |a, b| {
                    property(&aid, "WM_STATE").contains("Normal")
                        && property(&bid, "WM_STATE").contains("Normal")
                        && attached(a, b, offset)
                });
            }
            b.set_docked_parent(None, 0, 0).unwrap();
            let old_child = b.get_position();
            a.set_position(100, 100);
            until(&mut a, &mut b, |a, _| a.get_position() == (100, 100));
            pump(&mut a, &mut b, Duration::from_millis(100));
            assert_eq!(b.get_position(), old_child);
            drop(b);
            for _ in 0..5 {
                let mut child = window("Spotiamp reopened dock test", scale).unwrap();
                child.set_docked_parent(Some(&mut a), offset, 0).unwrap();
                until(&mut a, &mut child, |a, b| attached(a, b, offset));
                drop(child);
                a.update();
                assert!(a.is_open());
            }
            let mut child = window("Spotiamp surviving dock test", scale).unwrap();
            child.set_docked_parent(Some(&mut a), offset, 0).unwrap();
            until(&mut a, &mut child, |a, b| attached(a, b, offset));
            drop(a);
            child.set_position(240, 220);
            for _ in 0..10 {
                child.update();
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                child.is_open(),
                "dropping parent must detach the native survivor safely"
            );
        }
    }
}

#[cfg(target_os = "macos")]
pub fn smoke() -> Result<(), String> {
    use std::ffi::{CStr, c_char, c_void};
    type Id = *mut c_void;
    #[link(name = "objc")]
    unsafe extern "C" {
        fn sel_registerName(name: *const c_char) -> Id;
        fn objc_msgSend();
    }
    unsafe fn bool0(window: &Window, selector: &CStr) -> bool {
        let send: unsafe extern "C" fn(Id, Id) -> bool =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe {
            send(
                window.get_window_handle(),
                sel_registerName(selector.as_ptr()),
            )
        }
    }
    unsafe fn action(window: &Window, selector: &CStr) {
        let send: unsafe extern "C" fn(Id, Id, Id) =
            unsafe { std::mem::transmute(objc_msgSend as *const ()) };
        unsafe {
            send(
                window.get_window_handle(),
                sel_registerName(selector.as_ptr()),
                std::ptr::null_mut(),
            )
        };
    }
    for scale in [Scale::X1, Scale::X2] {
        let mut a = window("Spotiamp native dock parent", scale)?;
        let mut b = window("Spotiamp native dock companion", scale)?;
        pump(&mut a, &mut b, Duration::from_millis(100));
        let offset = a.get_size().0 as isize;
        a.set_position(-60, 100);
        b.set_docked_parent(Some(&mut a), offset, 0)
            .map_err(|e| e.to_string())?;
        if !attached(&a, &b, offset) {
            return Err("Cocoa initial docking differs from native size/position units".into());
        }
        a.set_position(-80, 140);
        b.set_position(180, 200);
        // No update call between moves: Cocoa notifications, including those
        // dispatched by the native drag loop, must perform the follow-up.
        if !attached(&a, &b, offset) || a.get_position() != (180 - offset, 200) {
            return Err("Cocoa bidirectional move did not follow without frame polling".into());
        }
        let area = a.get_work_area().ok_or("Cocoa visibleFrame unavailable")?;
        if area.2 == 0 || area.3 == 0 {
            return Err("Cocoa work area is empty".into());
        }
        b.minimize();
        let deadline = Instant::now() + Duration::from_secs(5);
        while unsafe { !bool0(&a, c"isMiniaturized") || bool0(&b, c"isVisible") } {
            if Instant::now() > deadline {
                return Err("Cocoa companion minimize did not hide the pair".into());
            }
            pump(&mut a, &mut b, Duration::from_millis(20));
        }
        unsafe {
            action(&a, c"deminiaturize:");
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while unsafe { bool0(&a, c"isMiniaturized") || !bool0(&b, c"isVisible") } {
            if Instant::now() > deadline {
                return Err("Cocoa restore did not restore the companion".into());
            }
            pump(&mut a, &mut b, Duration::from_millis(20));
        }
        if !attached(&a, &b, offset) {
            return Err("Cocoa restore broke docking".into());
        }
        b.set_docked_parent(None, 0, 0).map_err(|e| e.to_string())?;
        let detached = b.get_position();
        a.set_position(100, 100);
        if b.get_position() != detached {
            return Err("Cocoa detach kept moving companion".into());
        }
        drop(b);
        for _ in 0..5 {
            let mut child = window("Spotiamp reopened dock companion", scale)?;
            child
                .set_docked_parent(Some(&mut a), offset, 0)
                .map_err(|e| e.to_string())?;
            unsafe {
                action(&child, c"close");
            }
            drop(child);
            a.set_position(100, 120);
            if !a.is_open() {
                return Err("Cocoa companion close closed the player".into());
            }
        }
        let mut child = window("Spotiamp surviving native companion", scale)?;
        child
            .set_docked_parent(Some(&mut a), offset, 0)
            .map_err(|e| e.to_string())?;
        drop(a);
        child.set_position(160, 160);
        child.update();
        if !child.is_open() {
            return Err("Cocoa parent Drop left an invalid companion".into());
        }
    }
    Ok(())
}
