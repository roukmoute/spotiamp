use super::*;
use model::{AudioQuality, Snapshot};

fn callback() -> (TextInput, Rc<RefCell<Vec<MouseInput>>>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    (
        TextInput {
            chars: Rc::new(RefCell::new(Vec::new())),
            keys: Rc::new(RefCell::new(Vec::new())),
            mouse: events.clone(),
            held: [false; 512],
            modifiers: 0,
        },
        events,
    )
}

#[test]
fn press_changes_pixels_and_only_release_inside_activates_once() {
    let mut view = ui::View::default();
    let state = Snapshot::default();
    view.render(&state);
    let idle = view.pixels.clone();
    let idle_fingerprint = view.fingerprint(&state);
    assert!(view.pointer_down(380, 425, &state).is_none());
    assert_ne!(view.fingerprint(&state), idle_fingerprint);
    view.render(&state);
    assert_ne!(view.pixels, idle, "pressed relief must be visible");
    assert!(matches!(
        view.pointer_up(Some((380, 425)), &state),
        Some(ui::Action::Send(Command::SetQuality(AudioQuality::Kbps96)))
    ));
    assert!(view.pointer_up(Some((380, 425)), &state).is_none());
    view.render(&state);
    assert_eq!(view.pixels, idle, "release restores the raised button");
}

#[test]
fn leaving_cancels_activation_but_returning_before_release_is_allowed() {
    let mut view = ui::View::default();
    let state = Snapshot::default();
    view.render(&state);
    let idle = view.pixels.clone();
    view.pointer_down(380, 425, &state);
    view.pointer_move(None);
    view.render(&state);
    assert_eq!(view.pixels, idle);
    assert!(view.pointer_up(None, &state).is_none());
    view.pointer_down(380, 425, &state);
    assert!(
        view.pointer_up(Some((510, 425)), &state).is_none(),
        "a different button cannot steal the release"
    );
    view.pointer_down(380, 425, &state);
    view.pointer_move(None);
    view.pointer_move(Some((380, 425)));
    assert!(matches!(
        view.pointer_up(Some((380, 425)), &state),
        Some(ui::Action::Send(Command::SetQuality(AudioQuality::Kbps96)))
    ));
}

#[test]
fn disabled_buttons_focus_loss_and_help_do_not_activate() {
    let mut view = ui::View::default();
    let state = Snapshot::default();
    let idle = view.fingerprint(&state);
    assert!(view.pointer_down(70, 125, &state).is_none());
    assert_eq!(view.fingerprint(&state), idle);
    assert!(view.pointer_up(Some((70, 125)), &state).is_none());
    view.pointer_down(380, 425, &state);
    view.set_focus(false);
    view.set_focus(true);
    assert!(view.pointer_up(Some((380, 425)), &state).is_none());
    view.pointer_down(380, 425, &state);
    view.cancel_pointer();
    assert!(view.pointer_up(Some((380, 425)), &state).is_none());
    view.help = true;
    assert!(view.pointer_down(440, 40, &state).is_none());
    assert!(view.pointer_up(Some((440, 40)), &state).is_none());
    assert!(!view.help);
}

#[test]
fn slider_drag_is_immediate_and_releasing_outside_ends_it() {
    let mut view = ui::View::default();
    let state = Snapshot {
        duration_ms: 120_000,
        ..Snapshot::default()
    };
    assert!(matches!(
        view.pointer_down(280, 100, &state),
        Some(ui::Action::Send(Command::Seek(60_000)))
    ));
    assert!(matches!(
        view.drag(1000, 100, &state),
        Some(ui::Action::Send(Command::Seek(120_000)))
    ));
    assert!(view.pointer_up(None, &state).is_none());
    assert!(view.drag(280, 100, &state).is_none());
}

#[test]
fn short_click_edges_are_retained_and_overflow_cancels_instead_of_sticking() {
    let (mut input, events) = callback();
    input.set_mouse_button_state(MouseButton::Left, true, 380.0, 425.0);
    input.set_mouse_button_state(MouseButton::Left, false, 380.0, 425.0);
    let mut view = ui::View::default();
    let state = Snapshot::default();
    let mut activated = 0;
    for event in events.borrow_mut().drain(..) {
        if let MouseInput::Button { down, x, y, .. } = event {
            if down {
                assert!(view.pointer_down(x as usize, y as usize, &state).is_none());
            } else if view.pointer_up(inside_window(x, y), &state).is_some() {
                activated += 1;
            }
        }
    }
    assert_eq!(activated, 1);
    for _ in 0..129 {
        input.set_mouse_button_state(MouseButton::Left, true, 1.0, 1.0);
    }
    assert!(matches!(&events.borrow()[..], [MouseInput::Cancel]));
}

#[cfg(target_os = "windows")]
#[test]
fn win32_short_mouse_click_preserves_edges_and_scaled_coordinates() {
    use std::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn PostMessageW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> i32;
    }
    let mut window = Window::new(
        "Spotiamp pointer regression",
        32,
        32,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .expect("native Win32 pointer test requires a desktop runner");
    let (input, events) = callback();
    window.set_input_callback(Box::new(input));
    window.update();
    events.borrow_mut().clear();
    let position = (30isize << 16) | 26;
    for (message, buttons) in [(0x0201, 1), (0x0202, 0)] {
        assert_ne!(
            unsafe { PostMessageW(window.get_window_handle(), message, buttons, position) },
            0
        );
    }
    window.update();
    let clicks: Vec<_> = events
        .borrow()
        .iter()
        .filter_map(|event| match event {
            MouseInput::Button {
                button: MouseButton::Left,
                down,
                x,
                y,
            } => Some((*down, *x, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(clicks, [(true, 13.0, 15.0), (false, 13.0, 15.0)]);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires X11 display and xdotool; run under Xvfb"]
fn x11_short_mouse_click_preserves_edges_and_scaled_coordinates() {
    let mut window = Window::new(
        "Spotiamp X11 pointer regression",
        32,
        32,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .expect("run native X11 pointer test under Xvfb");
    let (input, events) = callback();
    window.set_input_callback(Box::new(input));
    window.update();
    events.borrow_mut().clear();
    let id = (window.get_window_handle() as usize).to_string();
    let output = std::process::Command::new("xdotool")
        .args([
            "mousemove",
            "--window",
            &id,
            "26",
            "30",
            "click",
            "--window",
            &id,
            "1",
        ])
        .output()
        .expect("xdotool is required");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let deadline = Instant::now() + Duration::from_millis(300);
    while Instant::now() < deadline {
        window.update();
        std::thread::sleep(Duration::from_millis(5));
    }
    let clicks: Vec<_> = events
        .borrow()
        .iter()
        .filter_map(|event| match event {
            MouseInput::Button {
                button: MouseButton::Left,
                down,
                x,
                y,
            } => Some((*down, *x, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(clicks, [(true, 13.0, 15.0), (false, 13.0, 15.0)]);
}
