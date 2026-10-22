//! Native wheel dispatch to the window under the event, without activation.
use minifb::{MouseMode, Scale, Window, WindowOptions};
use std::{
    ffi::c_void,
    ptr,
    time::{Duration, Instant},
};

type Hwnd = *mut c_void;
#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Default, Debug)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn ClientToScreen(window: Hwnd, point: *mut Point) -> i32;
    fn WindowFromPoint(point: Point) -> Hwnd;
    fn GetClassNameW(window: Hwnd, class: *mut u16, capacity: i32) -> i32;
    fn GetWindowRect(window: Hwnd, rect: *mut Rect) -> i32;
    fn IsWindowVisible(window: Hwnd) -> i32;
    fn ShowWindow(window: Hwnd, command: i32) -> i32;
    fn SetWindowPos(
        window: Hwnd,
        insert_after: Hwnd,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn GetFocus() -> Hwnd;
    fn SetFocus(window: Hwnd) -> Hwnd;
    fn SendMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> isize;
    fn PostMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> i32;
    fn CreateWindowExW(
        extended_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: Hwnd,
        instance: Hwnd,
        parameter: *mut c_void,
    ) -> Hwnd;
    fn DestroyWindow(window: Hwnd) -> i32;
}

// Runner desktop windows can cover freshly created test windows. Only this
// test module changes z-order; production routing must use the real hit test.
pub(crate) fn show_for_wheel_test(window: &Window) {
    unsafe {
        ShowWindow(window.get_window_handle(), 4); // SW_SHOWNOACTIVATE
        assert_ne!(
            SetWindowPos(
                window.get_window_handle(),
                -1isize as Hwnd, // HWND_TOPMOST
                0,
                0,
                0,
                0,
                0x0053, // SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW
            ),
            0,
            "cannot expose wheel test window: {}",
            std::io::Error::last_os_error()
        );
    }
}

fn describe(window: Hwnd) -> String {
    let mut class = [0u16; 128];
    let mut rect = Rect::default();
    let length = unsafe { GetClassNameW(window, class.as_mut_ptr(), class.len() as i32) };
    let valid_rect = unsafe { GetWindowRect(window, &mut rect) } != 0;
    let visible = unsafe { IsWindowVisible(window) } != 0;
    format!(
        "HWND={window:p} class={:?} rect={rect:?} rect_valid={valid_rect} visible={visible}",
        String::from_utf16_lossy(&class[..length.max(0) as usize]),
    )
}

pub(crate) fn wait_for_wheel_target(
    window: &mut Window,
    client_x: i32,
    client_y: i32,
    expected: Hwnd,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let point = screen_point(window, client_x, client_y);
        let actual = unsafe { WindowFromPoint(point) };
        if actual == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "wheel test point ({}, {}) remains obstructed: expected [{}], actual [{}], host [{}]",
            point.x,
            point.y,
            describe(expected),
            describe(actual),
            describe(window.get_window_handle()),
        );
        window.update();
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn window(name: &str, scale: Scale, x: isize) -> Window {
    let mut window = Window::new(
        name,
        160,
        100,
        WindowOptions {
            borderless: true,
            title: false,
            resize: false,
            scale,
            ..WindowOptions::default()
        },
    )
    .expect("native wheel window");
    window.set_target_fps(0);
    window.set_position(x, 40);
    show_for_wheel_test(&window);
    window.update();
    window
}

fn screen_point(window: &Window, x: i32, y: i32) -> Point {
    let mut point = Point { x, y };
    assert_ne!(
        unsafe { ClientToScreen(window.get_window_handle(), &mut point) },
        0
    );
    point
}

fn wheel_parameters(delta: i16, point: Point) -> (usize, isize) {
    let wparam = (delta as u16 as usize) << 16;
    let lparam = ((point.x as u16 as u32) | ((point.y as u16 as u32) << 16)) as isize;
    (wparam, lparam)
}

fn wheel(source: &Window, delta: i16, point: Point) {
    let (wparam, lparam) = wheel_parameters(delta, point);
    assert_eq!(
        unsafe { SendMessageW(source.get_window_handle(), 0x020a, wparam, lparam) },
        0,
        "a processed wheel event must not fall through to DefWindowProc"
    );
}

fn assert_delta(window: &Window, expected: f32) {
    let delta = window.get_scroll_wheel().map_or(0.0, |(_, y)| y);
    assert!(
        (delta - expected).abs() < 0.00001,
        "expected {expected}, received {delta}"
    );
}

#[test]
fn win32_hover_wheel_routes_once_without_focus_and_keeps_event_coordinates() {
    let _guard = crate::window_native_windows_tests::WINDOWS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for (scale, factor) in [(Scale::X1, 1), (Scale::X2, 2)] {
        let mut a = window("Spotiamp wheel focused reader", scale, 40);
        let mut b = window("Spotiamp wheel inactive library", scale, 430);
        b.set_docked_parent(Some(&mut a), 390, 0).unwrap();
        a.update();
        b.update();
        unsafe {
            SetFocus(a.get_window_handle());
        }
        assert_eq!(unsafe { GetFocus() }, a.get_window_handle());
        let handle_b = b.get_window_handle();
        wait_for_wheel_target(&mut b, 80 * factor, 40 * factor, handle_b);
        let point_b = screen_point(&b, 80 * factor, 40 * factor);
        assert_eq!(unsafe { WindowFromPoint(point_b) }, b.get_window_handle());

        // Legacy Windows routing sends to the focused HWND, while the event's
        // point belongs to the inactive library. A's pump dispatches that event
        // globally; B must already expose the correct position before B.update.
        let (wparam, lparam) = wheel_parameters(-120, point_b);
        assert_ne!(
            unsafe { PostMessageW(a.get_window_handle(), 0x020a, wparam, lparam) },
            0
        );
        a.update();
        assert_delta(&a, 0.0);
        assert_delta(&b, -1.0);
        assert_eq!(b.get_mouse_pos(MouseMode::Pass), Some((80.0, 40.0)));
        assert_eq!(unsafe { GetFocus() }, a.get_window_handle());

        b.update();
        assert_delta(&b, 0.0);
        // Modern routing already sends to the hovered HWND. Neither rerouting
        // nor the owned-window relationship may count the event a second time.
        wheel(&b, -60, point_b);
        wheel(&b, -60, point_b);
        assert_delta(&a, 0.0);
        assert_delta(&b, -1.0);
        assert_eq!(unsafe { GetFocus() }, a.get_window_handle());
        b.update();
        wheel(&a, -70, point_b);
        wheel(&a, -50, point_b);
        assert_delta(&b, -1.0);
        a.update();
        b.update();
        assert_delta(&a, 0.0);
        assert_delta(&b, 0.0);

        unsafe {
            SetFocus(b.get_window_handle());
        }
        assert_eq!(unsafe { GetFocus() }, b.get_window_handle());
        let handle_a = a.get_window_handle();
        wait_for_wheel_target(&mut a, 30 * factor, 65 * factor, handle_a);
        let point_a = screen_point(&a, 30 * factor, 65 * factor);
        assert_eq!(unsafe { WindowFromPoint(point_a) }, a.get_window_handle());
        wheel(&b, 120, point_a);
        assert_delta(&a, 1.0);
        assert_delta(&b, 0.0);
        assert_eq!(a.get_mouse_pos(MouseMode::Pass), Some((30.0, 65.0)));
        assert_eq!(unsafe { GetFocus() }, b.get_window_handle());
    }
}

struct NativeChild(Hwnd);
impl Drop for NativeChild {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

#[test]
fn win32_hover_wheel_does_not_read_foreign_window_state_or_scroll_the_focus_window() {
    let _guard = crate::window_native_windows_tests::WINDOWS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut a = window("Spotiamp wheel foreign target", Scale::X1, 40);
    let class: Vec<u16> = "BUTTON\0".encode_utf16().collect();
    let child = NativeChild(unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            ptr::null(),
            0x5000_0000, // WS_CHILD | WS_VISIBLE
            20,
            20,
            80,
            50,
            a.get_window_handle(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    });
    assert!(!child.0.is_null());
    a.update();
    unsafe {
        SetFocus(a.get_window_handle());
    }
    wait_for_wheel_target(&mut a, 40, 40, child.0);
    let point = screen_point(&a, 40, 40);
    assert_eq!(unsafe { WindowFromPoint(point) }, child.0);
    wheel(&a, -120, point);
    assert_delta(&a, 0.0);
    assert_eq!(unsafe { GetFocus() }, a.get_window_handle());
}
