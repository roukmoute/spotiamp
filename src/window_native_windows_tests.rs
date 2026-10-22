//! Native regression checks for the custom skin's window controls.
use minifb::{InputCallback, Key, Scale, Window, WindowOptions};
use std::{
    cell::RefCell,
    ffi::c_void,
    rc::Rc,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

pub(crate) static WINDOWS: Mutex<()> = Mutex::new(());

#[path = "magnetic_native_windows_tests.rs"]
mod magnetic_tests;

type Hwnd = *mut c_void;
#[repr(C)]
#[derive(Default)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
#[derive(Default)]
struct MinMaxInfo {
    reserved: Point,
    max_size: Point,
    max_position: Point,
    min_track_size: Point,
    max_track_size: Point,
}

#[repr(C)]
struct GuiThreadInfo {
    size: u32,
    flags: u32,
    active: Hwnd,
    focus: Hwnd,
    capture: Hwnd,
    menu_owner: Hwnd,
    move_size: Hwnd,
    caret: Hwnd,
    caret_rect: Rect,
}
#[link(name = "user32")]
unsafe extern "system" {
    fn GetWindowLongW(window: Hwnd, index: i32) -> i32;
    fn GetWindowRect(window: Hwnd, rect: *mut Rect) -> i32;
    fn GetClientRect(window: Hwnd, rect: *mut Rect) -> i32;
    fn ClientToScreen(window: Hwnd, point: *mut Point) -> i32;
    fn SendMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> isize;
    fn PostMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> i32;
    fn GetSystemMenu(window: Hwnd, revert: i32) -> Hwnd;
    fn IsWindow(window: Hwnd) -> i32;
    fn IsWindowVisible(window: Hwnd) -> i32;
    fn GetWindow(window: Hwnd, command: u32) -> Hwnd;
    fn SetWindowPos(
        window: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn IsIconic(window: Hwnd) -> i32;
    fn IsZoomed(window: Hwnd) -> i32;
    fn ShowWindow(window: Hwnd, command: i32) -> i32;
    fn GetWindowThreadProcessId(window: Hwnd, process: *mut u32) -> u32;
    fn GetGUIThreadInfo(thread_id: u32, info: *mut GuiThreadInfo) -> i32;
}

fn dock_window(scale: Scale) -> Window {
    Window::new(
        "Spotiamp native docking test",
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
    .expect("Windows CI must allow creating docked native windows")
}

#[test]
fn native_docked_windows_follow_both_directions_without_frame_polling() {
    let _serial = WINDOWS.lock().unwrap_or_else(|e| e.into_inner());
    for scale in [Scale::X1, Scale::X2] {
        let mut player = dock_window(scale);
        let mut library = dock_window(scale);
        player.set_position(-80, 45);
        let offset = player.get_size().0 as isize;
        let size = library.get_size();
        library
            .set_docked_parent(Some(&mut player), offset, 0)
            .unwrap();
        let main = player.get_window_handle();
        let child = library.get_window_handle();
        assert_eq!(
            unsafe { GetWindow(child, 4) },
            main,
            "GW_OWNER groups native visibility"
        );
        assert_eq!(library.get_position(), (-80 + offset, 45));
        let (_, _, width, height) = player.get_work_area().expect("monitor work area");
        assert!(width > 0 && height > 0);

        // No update() calls: ordinary native position events must move the peer
        // synchronously, independent of the application's polling cadence.
        player.set_position(-100, -40);
        assert_eq!(
            player.get_position(),
            (-100, -40),
            "companion restacking must not undo the owner's move"
        );
        assert_eq!(library.get_position(), (-100 + offset, -40));
        library.set_position(70, 90);
        assert_eq!(player.get_position(), (70 - offset, 90));
        assert_eq!(library.get_size(), size);

        // WM_MOVING is the native modal move loop's proposed screen rectangle.
        // The companion must move before the dragged rectangle is committed.
        let before_proposal = player.get_position();
        let mut proposed = Rect {
            left: -250,
            top: 100,
            right: -250 + offset as i32,
            bottom: 100 + player.get_size().1 as i32,
        };
        unsafe {
            SendMessageW(main, 0x0216, 0, (&mut proposed as *mut Rect) as isize);
        }
        assert_eq!(library.get_position(), (-250 + offset, 100));
        // A native Z-order change is not a second move. In particular, while
        // WM_MOVING's proposal awaits its commit, it must not pull either
        // endpoint back to the other endpoint's previous coordinates.
        assert_ne!(
            unsafe {
                SetWindowPos(
                    child,
                    1usize as Hwnd, // HWND_BOTTOM, forcing a real restack.
                    0,
                    0,
                    0,
                    0,
                    0x0001 | 0x0002 | 0x0010, // NOSIZE | NOMOVE | NOACTIVATE.
                )
            },
            0
        );
        assert_eq!(player.get_position(), before_proposal);
        assert_eq!(library.get_position(), (-250 + offset, 100));
        player.set_position(-250, 100);
        proposed = Rect {
            left: 100,
            top: -20,
            right: 100 + size.0 as i32,
            bottom: -20 + size.1 as i32,
        };
        unsafe {
            SendMessageW(child, 0x0216, 0, (&mut proposed as *mut Rect) as isize);
        }
        assert_eq!(player.get_position(), (100 - offset, -20));
        library.set_position(100, -20);

        library.minimize();
        assert_ne!(
            unsafe { IsIconic(main) },
            0,
            "minimizing either window minimizes the group"
        );
        assert_eq!(
            unsafe { IsWindowVisible(child) },
            0,
            "owned companion hides with owner"
        );
        unsafe {
            ShowWindow(main, 9);
        } // SW_RESTORE
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline
            && unsafe { IsWindowVisible(child) == 0 || IsIconic(main) != 0 }
        {
            player.update();
            library.update();
        }
        assert_eq!(unsafe { IsIconic(main) }, 0);
        assert_ne!(
            unsafe { IsWindowVisible(child) },
            0,
            "restoring player restores companion"
        );
        let p = player.get_position();
        assert_eq!(library.get_position(), (p.0 + offset, p.1));

        library.set_docked_parent(None, 0, 0).unwrap();
        assert!(unsafe { GetWindow(child, 4) }.is_null());
        let detached = library.get_position();
        player.set_position(30, 40);
        assert_eq!(
            library.get_position(),
            detached,
            "explicit detach stops following"
        );
    }
}

#[test]
fn native_docking_cleans_up_closed_reopened_and_dropped_windows() {
    let _serial = WINDOWS.lock().unwrap_or_else(|e| e.into_inner());
    let mut player = dock_window(Scale::X1);
    let main = player.get_window_handle();
    for _ in 0..3 {
        let mut library = dock_window(Scale::X1);
        library
            .set_docked_parent(Some(&mut player), 120, 0)
            .unwrap();
        unsafe {
            SendMessageW(library.get_window_handle(), 0x0010, 0, 0);
        } // WM_CLOSE
        assert!(!library.is_open());
        assert_ne!(
            unsafe { IsWindow(main) },
            0,
            "closing companion preserves player"
        );
        player.set_position(20, 30); // must not retain the closed/reusable HWND.
    }
    let mut library = dock_window(Scale::X1);
    library
        .set_docked_parent(Some(&mut player), -120, 15)
        .unwrap();
    assert_eq!(
        library.get_position(),
        (-100, 45),
        "signed offsets remain signed"
    );
    let child = library.get_window_handle();
    drop(player);
    assert_ne!(
        unsafe { IsWindow(child) },
        0,
        "Drop defensively detaches before native owner destruction"
    );
    assert!(unsafe { GetWindow(child, 4) }.is_null());
    library.set_position(-50, -30);
    assert_eq!(library.get_position(), (-50, -30));
}

fn window(scale: Scale) -> Window {
    let mut window = Window::new(
        "Spotiamp native titlebar test",
        560,
        440,
        WindowOptions {
            borderless: true,
            title: false,
            resize: false,
            scale,
            ..WindowOptions::default()
        },
    )
    .expect("Windows CI must allow creating a native HWND");
    window.set_titlebar_drag_region(Some((6, 2, 486, 16)));
    window.update();
    window
}

fn packed(point: Point) -> isize {
    ((point.x as u16 as u32) | ((point.y as u16 as u32) << 16)) as isize
}
fn hit(window: Hwnd, x: i32, y: i32) -> isize {
    let mut point = Point { x, y };
    assert_ne!(unsafe { ClientToScreen(window, &mut point) }, 0);
    unsafe { SendMessageW(window, 0x0084, 0, packed(point)) } // WM_NCHITTEST
}

#[test]
fn native_borderless_styles_hit_regions_scaling_and_minimize_restore() {
    let _serial = WINDOWS.lock().unwrap_or_else(|e| e.into_inner());
    for (scale, factor) in [(Scale::X1, 1), (Scale::X2, 2)] {
        let mut window = window(scale);
        window.set_position(-20, 30); // includes signed screen coordinates.
        let handle = window.get_window_handle();
        let style = unsafe { GetWindowLongW(handle, -16) } as u32;
        let extended = unsafe { GetWindowLongW(handle, -20) } as u32;
        assert_eq!(
            style & (0x0004_0000 | 0x0001_0000),
            0,
            "fixed size, no maximize"
        );
        assert_ne!(style & 0x0008_0000, 0, "native system menu");
        assert_ne!(style & 0x0002_0000, 0, "native minimization");
        assert_eq!(
            extended & 0x0000_0080,
            0,
            "not a taskbar-hidden tool window"
        );
        assert!(!unsafe { GetSystemMenu(handle, 0) }.is_null());
        let mut limits = MinMaxInfo::default();
        unsafe {
            SendMessageW(handle, 0x0024, 0, (&mut limits as *mut MinMaxInfo) as isize);
        }
        for size in [
            limits.min_track_size,
            limits.max_track_size,
            limits.max_size,
        ] {
            assert_eq!(
                (size.x, size.y),
                (560 * factor, 440 * factor),
                "fixed buffer dimensions must override desktop-sized native limits"
            );
        }
        let mut outer = Rect::default();
        let mut client = Rect::default();
        assert_ne!(unsafe { GetWindowRect(handle, &mut outer) }, 0);
        assert_ne!(unsafe { GetClientRect(handle, &mut client) }, 0);
        assert_eq!(
            (outer.right - outer.left, outer.bottom - outer.top),
            (560 * factor, 440 * factor)
        );
        assert_eq!(
            (client.right, client.bottom),
            (560 * factor, 440 * factor),
            "no native border/titlebar consumes pixels"
        );
        assert_eq!(hit(handle, 6 * factor, 2 * factor), 2); // HTCAPTION
        assert_eq!(hit(handle, 491 * factor, 17 * factor), 2);
        for (x, y) in [
            (5, 2),
            (6, 1),
            (492, 2),
            (6, 18),
            (504, 10),
            (522, 10),
            (544, 10),
            (100, 100),
        ] {
            assert_eq!(
                hit(handle, x * factor, y * factor),
                1,
                "control/client at {x},{y}"
            );
        }
        window.set_titlebar_drag_region(None);
        assert_eq!(hit(handle, 20 * factor, 10 * factor), 1);
        window.set_titlebar_drag_region(Some((6, 2, 486, 16)));
        unsafe {
            SendMessageW(handle, 0x00a3, 2, 0);
        } // WM_NCLBUTTONDBLCLK, HTCAPTION
        assert_eq!(unsafe { IsZoomed(handle) }, 0);
        window.minimize();
        assert_ne!(unsafe { IsIconic(handle) }, 0);
        unsafe {
            ShowWindow(handle, 9);
        } // SW_RESTORE
        window.update();
        assert_eq!(unsafe { IsIconic(handle) }, 0);
        assert_eq!(
            window.get_size(),
            (560 * factor as usize, 440 * factor as usize)
        );
        assert!(window.is_open());
    }
}

#[derive(Clone)]
struct Input(Rc<RefCell<Vec<Key>>>, Rc<RefCell<Vec<u32>>>);
impl InputCallback for Input {
    fn add_char(&mut self, codepoint: u32) {
        self.1.borrow_mut().push(codepoint);
    }
    fn set_key_state(&mut self, key: Key, down: bool) {
        if down {
            self.0.borrow_mut().push(key);
        }
    }
}

fn send_system_menu_message(handle: Hwnd, message: u32) -> bool {
    let thread_id = unsafe { GetWindowThreadProcessId(handle, std::ptr::null_mut()) };
    let address = handle as usize;
    // The system menu runs a native modal loop. Observe it from a helper thread
    // and post cancellation in all cases, so the test cannot leave the menu open.
    let observer = thread::spawn(move || {
        let started = Instant::now();
        let mut observed = false;
        while started.elapsed() < Duration::from_secs(2) {
            let mut info: GuiThreadInfo = unsafe { std::mem::zeroed() };
            info.size = std::mem::size_of::<GuiThreadInfo>() as u32;
            if unsafe { GetGUIThreadInfo(thread_id, &mut info) } != 0 && info.flags & 0x4 != 0 {
                observed = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        unsafe {
            PostMessageW(address as Hwnd, 0x001f, 0, 0); // WM_CANCELMODE
            PostMessageW(address as Hwnd, 0x0100, 0x1b, 0); // Escape fallback
        }
        observed
    });
    unsafe {
        SendMessageW(handle, message, 0x20, (1 << 29) | (0x39 << 16) | 1);
    }
    observer.join().unwrap()
}

#[test]
fn native_alt_space_opens_system_menu_without_player_input_and_alt_f4_closes() {
    let _serial = WINDOWS.lock().unwrap_or_else(|e| e.into_inner());
    let mut window = window(Scale::X1);
    let input = Input(
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(Vec::new())),
    );
    window.set_input_callback(Box::new(input.clone()));
    let handle = window.get_window_handle();
    // Guard both native messages independently; either can enter a modal loop.
    let _ = send_system_menu_message(handle, 0x0104); // WM_SYSKEYDOWN, Alt+Space
    window.update(); // drain cancellation/Escape before opening the next menu
    assert!(
        send_system_menu_message(handle, 0x0106),
        "Alt+Space must enter the native system menu"
    );
    assert!(
        !input.1.borrow().contains(&u32::from(b' ')),
        "Alt+Space must not type into the app"
    );
    assert!(
        !input.0.borrow().contains(&Key::Space),
        "Alt+Space must not toggle playback"
    );
    unsafe {
        SendMessageW(handle, 0x0100, 0x41, (0x1e << 16) | 1); // ordinary A down
        SendMessageW(handle, 0x0102, usize::from(b'a'), 1); // WM_CHAR
        SendMessageW(handle, 0x0101, 0x41, (0x1e << 16) | (1 << 31) | 1);
    }
    assert!(input.0.borrow().contains(&Key::A));
    assert!(input.1.borrow().contains(&u32::from(b'a')));
    unsafe {
        SendMessageW(handle, 0x0104, 0x73, (1 << 29) | (0x3e << 16) | 1);
    } // WM_SYSKEYDOWN, Alt+F4
    // DefWindowProc may post later system/close messages. Exercise the same
    // message pump as the real application instead of requiring synchronous close.
    let close_started = Instant::now();
    while window.is_open() && close_started.elapsed() < Duration::from_secs(2) {
        window.update();
        if window.is_open() {
            thread::sleep(Duration::from_millis(10));
        }
    }
    assert!(
        !window.is_open(),
        "Alt+F4 must reach the native close action"
    );
    assert_eq!(
        unsafe { IsWindow(handle) },
        0,
        "Alt+F4 must destroy the native HWND"
    );
    assert!(!input.0.borrow().contains(&Key::F4));
}
