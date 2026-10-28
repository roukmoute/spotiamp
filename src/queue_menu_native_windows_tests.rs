//! Win32 mouse messages through minifb capture and the real player input loop.
//! SendMessage exercises synchronous WM_CAPTURECHANGED from ReleaseCapture;
//! feeding application MouseInput fixtures alone cannot reproduce that event.
use super::*;
use std::ffi::c_void;

type Hwnd = *mut c_void;

#[link(name = "user32")]
unsafe extern "system" {
    fn SendMessageW(window: Hwnd, message: u32, wparam: usize, lparam: isize) -> isize;
    fn GetCapture() -> Hwnd;
    fn SetCapture(window: Hwnd) -> Hwnd;
    fn ReleaseCapture() -> i32;
    fn SetFocus(window: Hwnd) -> Hwnd;
    fn GetFocus() -> Hwnd;
}

fn window(scale: Scale) -> Window {
    let mut window = Window::new(
        "Spotiamp queue menu native regression",
        ui::WIDTH,
        ui::HEIGHT,
        WindowOptions {
            borderless: true,
            title: false,
            resize: false,
            scale,
            ..Default::default()
        },
    )
    .expect("Windows CI must allow creating the player window");
    window.set_target_fps(0);
    window.update();
    window
}

fn observe(window: &mut Window) -> Rc<RefCell<Vec<MouseInput>>> {
    let mouse = Rc::new(RefCell::new(Vec::new()));
    window.set_input_callback(Box::new(TextInput {
        chars: Rc::default(),
        keys: Rc::default(),
        mouse: mouse.clone(),
        held: [false; 512],
        modifiers: 0,
    }));
    mouse
}

fn state() -> model::Snapshot {
    model::Snapshot {
        connected: true,
        playing: true,
        current: Some(2),
        queue: (0..20)
            .map(|index| model::TrackRow {
                uri: format!("spotify:track:{index}"),
                title: format!("Track {index}"),
                ..Default::default()
            })
            .collect(),
        position_ms: 42_000,
        queue_reorder_request: 7,
        ..Default::default()
    }
}

fn button(window: &Window, factor: i32, right: bool, down: bool, x: i32, y: i32) {
    let message = match (right, down) {
        (false, true) => 0x0201, // WM_LBUTTONDOWN
        (false, false) => 0x0202,
        (true, true) => 0x0204, // WM_RBUTTONDOWN
        (true, false) => 0x0205,
    };
    let left_down = if right {
        window.get_mouse_down(MouseButton::Left)
    } else {
        down
    };
    let right_down = if right {
        down
    } else {
        window.get_mouse_down(MouseButton::Right)
    };
    let flags = usize::from(left_down) | (usize::from(right_down) << 1);
    let point = ((x * factor) as u16 as u32) | (((y * factor) as u16 as u32) << 16);
    assert_eq!(
        unsafe { SendMessageW(window.get_window_handle(), message, flags, point as isize) },
        0
    );
}

fn frame(
    view: &mut ui::View,
    snapshot: &model::Snapshot,
    events: &Rc<RefCell<Vec<MouseInput>>>,
    pointer: (f32, f32),
) -> Vec<ui::Action> {
    let mouse = std::mem::take(&mut *events.borrow_mut());
    let mut actions = Vec::new();
    process_frame_input(
        view,
        snapshot,
        FrameInput {
            active: true,
            mouse: &mouse,
            pointer: Some(pointer),
            ..Default::default()
        },
        &mut actions,
    );
    actions
}

#[test]
fn win32_queue_context_normal_releases_keep_menu_and_activate_every_action() {
    let _serial = window_native_windows_tests::WINDOWS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let snapshot = state();
    for (scale, factor) in [(Scale::X1, 1), (Scale::X2, 2)] {
        let mut native = window(scale);
        let events = observe(&mut native);
        for skin in [appearance::Skin::Classic, appearance::Skin::Modern] {
            for split_frames in [false, true] {
                for item in 0..5 {
                    let mut view = ui::View::default();
                    view.set_skin(skin);
                    button(&native, factor, true, true, 80, 281);
                    assert_eq!(unsafe { GetCapture() }, native.get_window_handle());
                    if split_frames {
                        assert!(frame(&mut view, &snapshot, &events, (80., 281.)).is_empty());
                        assert!(view.has_queue_menu());
                    }
                    button(&native, factor, true, false, 80, 281);
                    assert!(unsafe { GetCapture() }.is_null());
                    assert!(
                        !events
                            .borrow()
                            .iter()
                            .any(|event| matches!(event, MouseInput::Cancel)),
                        "ordinary right release must not cancel the menu"
                    );
                    assert!(frame(&mut view, &snapshot, &events, (80., 281.)).is_empty());
                    assert!(
                        view.has_queue_menu(),
                        "right release must leave the menu open"
                    );
                    assert!(frame(&mut view, &snapshot, &events, (80., 281.)).is_empty());
                    assert!(
                        view.has_queue_menu(),
                        "the menu also survives an idle frame"
                    );

                    let y = 298 + item * 28;
                    button(&native, factor, false, true, 100, y);
                    if split_frames {
                        assert!(frame(&mut view, &snapshot, &events, (100., y as f32)).is_empty());
                        assert!(view.has_queue_menu());
                    }
                    button(&native, factor, false, false, 100, y);
                    assert!(unsafe { GetCapture() }.is_null());
                    assert!(
                        !events
                            .borrow()
                            .iter()
                            .any(|event| matches!(event, MouseInput::Cancel)),
                        "ordinary left release must not cancel the selected action"
                    );
                    let actions = frame(&mut view, &snapshot, &events, (100., y as f32));
                    assert_eq!(actions.len(), 1, "menu item {item}");
                    match (&actions[0], item) {
                        (ui::Action::Send(Command::PlayQueueIndex { index: 2, .. }), 0) => (),
                        (ui::Action::QueueToPlaylist { tracks, create }, 1 | 2) => {
                            assert_eq!(tracks.len(), 1);
                            assert_eq!(tracks[0].uri, snapshot.queue[2].uri);
                            assert_eq!(*create, item == 2);
                        }
                        (
                            ui::Action::Send(Command::EditQueue {
                                indices,
                                keep_selected,
                                ..
                            }),
                            3 | 4,
                        ) => {
                            assert_eq!(indices, &[2]);
                            assert_eq!(*keep_selected, item == 4);
                        }
                        _ => panic!("unexpected action for menu item {item}"),
                    }
                    assert!(!view.has_queue_menu());
                    assert!(frame(&mut view, &snapshot, &events, (100., y as f32)).is_empty());
                }
            }
        }
    }
}

#[test]
fn win32_queue_menu_capture_theft_and_native_cancellation_still_discard_actions() {
    let _serial = window_native_windows_tests::WINDOWS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut native = window(Scale::X1);
    let peer = window(Scale::X1);
    let events = observe(&mut native);
    let snapshot = state();
    for cancellation in 0..3 {
        unsafe {
            SetFocus(native.get_window_handle());
            assert_eq!(GetFocus(), native.get_window_handle());
        }
        events.borrow_mut().clear();
        let mut view = ui::View::default();
        button(&native, 1, true, true, 80, 281);
        button(&native, 1, true, false, 80, 281);
        assert!(frame(&mut view, &snapshot, &events, (80., 281.)).is_empty());
        assert!(view.has_queue_menu());
        button(&native, 1, false, true, 100, 382);
        assert!(frame(&mut view, &snapshot, &events, (100., 382.)).is_empty());
        unsafe {
            match cancellation {
                0 => {
                    SetCapture(peer.get_window_handle());
                }
                1 => {
                    SendMessageW(native.get_window_handle(), 0x001f, 0, 0);
                } // WM_CANCELMODE
                _ => {
                    SetFocus(peer.get_window_handle());
                }
            }
        }
        assert!(
            events
                .borrow()
                .iter()
                .any(|event| matches!(event, MouseInput::Cancel)),
            "real cancellation {cancellation} must remain observable"
        );
        // A delayed release cannot activate the action after native cancellation.
        button(&native, 1, false, false, 100, 382);
        assert!(frame(&mut view, &snapshot, &events, (100., 382.)).is_empty());
        assert!(!view.has_queue_menu());
        unsafe {
            ReleaseCapture();
        }
    }

    // Focus loss must also close a menu after right release, with no button held.
    unsafe {
        SetFocus(native.get_window_handle());
    }
    events.borrow_mut().clear();
    let mut view = ui::View::default();
    button(&native, 1, true, true, 80, 281);
    button(&native, 1, true, false, 80, 281);
    assert!(frame(&mut view, &snapshot, &events, (80., 281.)).is_empty());
    assert!(view.has_queue_menu());
    unsafe {
        SetFocus(peer.get_window_handle());
    }
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| matches!(event, MouseInput::Cancel))
    );
    assert!(frame(&mut view, &snapshot, &events, (80., 281.)).is_empty());
    assert!(!view.has_queue_menu());
}

#[test]
fn win32_capture_theft_cancels_queue_drag_but_normal_release_commits_it() {
    let _serial = window_native_windows_tests::WINDOWS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut native = window(Scale::X1);
    let peer = window(Scale::X1);
    let events = observe(&mut native);
    let snapshot = state();
    for steal in [false, true] {
        events.borrow_mut().clear();
        let mut view = ui::View::default();
        button(&native, 1, false, true, 80, 233);
        assert!(frame(&mut view, &snapshot, &events, (80., 233.)).is_empty());
        let mut motion_actions = Vec::new();
        process_frame_input(
            &mut view,
            &snapshot,
            FrameInput {
                active: true,
                mouse_down: native.get_mouse_down(MouseButton::Left),
                pointer: Some((80., 305.)),
                ..Default::default()
            },
            &mut motion_actions,
        );
        assert!(motion_actions.is_empty());
        if steal {
            unsafe {
                SetCapture(peer.get_window_handle());
            }
        }
        button(&native, 1, false, false, 80, 305);
        let actions = frame(&mut view, &snapshot, &events, (80., 305.));
        if steal {
            assert!(actions.is_empty());
        } else {
            assert!(
                matches!(actions.as_slice(), [ui::Action::Send(Command::ReorderQueue { indices, .. })] if indices == &[0])
            );
        }
        // A second, delayed release never starts or commits a second gesture.
        button(&native, 1, false, false, 80, 305);
        assert!(frame(&mut view, &snapshot, &events, (80., 305.)).is_empty());
        unsafe {
            ReleaseCapture();
        }
    }
}

#[test]
fn win32_releasing_one_of_two_buttons_retains_capture_without_cancelling() {
    let _serial = window_native_windows_tests::WINDOWS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut native = window(Scale::X1);
    let events = observe(&mut native);
    button(&native, 1, true, true, 80, 281);
    assert!(native.get_mouse_down(MouseButton::Right));
    button(&native, 1, false, true, 100, 298);
    assert_eq!(unsafe { GetCapture() }, native.get_window_handle());
    assert!(native.get_mouse_down(MouseButton::Right));
    assert!(native.get_mouse_down(MouseButton::Left));
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| matches!(event, MouseInput::Cancel)),
        "a second button press must not cancel the capture already held by this window"
    );
    // Native APIs can report an otherwise redundant capture to the same HWND.
    // The target still owns both buttons; this is not a stolen capture.
    unsafe {
        SetCapture(native.get_window_handle());
    }
    assert_eq!(unsafe { GetCapture() }, native.get_window_handle());
    assert!(native.get_mouse_down(MouseButton::Right));
    assert!(native.get_mouse_down(MouseButton::Left));
    button(&native, 1, false, false, 100, 298);
    assert_eq!(unsafe { GetCapture() }, native.get_window_handle());
    assert!(native.get_mouse_down(MouseButton::Right));
    assert!(!native.get_mouse_down(MouseButton::Left));
    button(&native, 1, true, false, 100, 298);
    assert!(unsafe { GetCapture() }.is_null());
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| matches!(event, MouseInput::Cancel))
    );
    assert_eq!(
        events.borrow().len(),
        4,
        "both buttons retain their down/up callbacks"
    );
}
