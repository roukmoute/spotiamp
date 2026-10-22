use minifb::{Scale, Window, WindowOptions};
use std::{
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn pump(parent: &mut Window, child: &mut Window, duration: Duration) {
    let until = Instant::now() + duration;
    while Instant::now() < until {
        parent.update();
        child.update();
        thread::sleep(Duration::from_millis(5));
    }
}

fn id(window: &Window) -> String {
    (window.get_window_handle() as usize).to_string()
}

fn property(window: &Window, name: &str) -> String {
    let output = Command::new("xprop")
        .args(["-id", &id(window), name])
        .output()
        .expect("xprop is required");
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

fn wait_state(parent: &mut Window, child: &mut Window, state: &str) {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        pump(parent, child, Duration::from_millis(20));
        let states = (property(parent, "WM_STATE"), property(child, "WM_STATE"));
        if states.0.contains(state) && states.1.contains(state) {
            return;
        }
        assert!(Instant::now() < until, "expected {state}: {states:?}");
    }
}

fn xdotool(parent: &mut Window, child: &mut Window, args: &[&str]) {
    let mut process = Process(Command::new("xdotool").args(args).spawn().unwrap());
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        pump(parent, child, Duration::from_millis(5));
        if let Some(status) = process.0.try_wait().unwrap() {
            assert!(status.success(), "xdotool {args:?}");
            break;
        }
        assert!(Instant::now() < until, "xdotool timed out: {args:?}");
    }
    pump(parent, child, Duration::from_millis(100));
}

fn move_held(
    parent: &mut Window,
    child: &mut Window,
    child_moves: bool,
    target: (isize, isize),
    factor: isize,
) {
    let moving = if child_moves { &*child } else { &*parent };
    let origin = moving.get_position();
    let window = id(moving);
    xdotool(
        parent,
        child,
        &[
            "windowactivate",
            "--sync",
            &window,
            "mousemove",
            "--window",
            &window,
            &(20 * factor).to_string(),
            &(9 * factor).to_string(),
            "mousedown",
            "1",
            "sleep",
            "0.12",
            "mousemove_relative",
            "--",
            &(target.0 - origin.0).to_string(),
            &(target.1 - origin.1).to_string(),
            "sleep",
            "0.12",
        ],
    );
}

fn release(parent: &mut Window, child: &mut Window) {
    xdotool(parent, child, &["mouseup", "1"]);
}

fn assert_position(window: &Window, expected: (isize, isize), context: &str) {
    let actual = window.get_position();
    assert!(
        (actual.0 - expected.0).abs() <= 1 && (actual.1 - expected.1).abs() <= 1,
        "{context}: expected {expected:?}, observed {actual:?}"
    );
}

fn window(name: &str, size: (usize, usize), scale: Scale) -> Window {
    let mut result = Window::new(
        name,
        size.0,
        size.1,
        WindowOptions {
            borderless: true,
            title: false,
            resize: false,
            scale,
            ..WindowOptions::default()
        },
    )
    .unwrap();
    result.set_target_fps(0);
    result.set_titlebar_drag_region(Some((4, 2, size.0 - 35, 16)));
    result
        .update_with_buffer(&vec![0x29283d; size.0 * size.1], size.0, size.1)
        .unwrap();
    result
}

fn openbox(label: &str) -> Process {
    let ready_atom = format!("_SPOTIAMP_{label}_READY_{}", std::process::id());
    let announce = format!("xprop -root -f {ready_atom} 8s -set {ready_atom} ready");
    let mut manager = Process(
        Command::new("openbox")
            .args(["--sm-disable", "--startup", &announce])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(manager.0.try_wait().unwrap().is_none(), "Openbox exited");
        let output = Command::new("xprop")
            .args(["-root", &ready_atom])
            .output()
            .unwrap();
        if String::from_utf8_lossy(&output.stdout).contains("= \"ready\"") {
            break;
        }
        assert!(Instant::now() < until, "Openbox did not initialize");
        thread::sleep(Duration::from_millis(25));
    }
    manager
}

#[test]
#[ignore = "requires isolated Xvfb, Openbox, xprop and xdotool"]
fn x11_magnetic_pair_detaches_snaps_on_release_and_keeps_group_minimize() {
    let _manager = openbox("MAGNETIC");
    for (scale, factor) in [(Scale::X1, 1isize), (Scale::X2, 2)] {
        let mut parent = window("Spotiamp magnetic player", (200, 120), scale);
        let mut child = window("Spotiamp magnetic library", (160, 100), scale);
        wait_state(&mut parent, &mut child, "Normal");
        parent.set_position(360, 300);
        pump(&mut parent, &mut child, Duration::from_millis(150));
        child
            .set_docked_parent(Some(&mut parent), 200 * factor, 0)
            .unwrap();
        child.set_dock_magnetic(true).unwrap();
        pump(&mut parent, &mut child, Duration::from_millis(150));

        // An immobile titlebar click does not detach the companion.
        let child_id = id(&child);
        xdotool(
            &mut parent,
            &mut child,
            &[
                "mousemove",
                "--window",
                &child_id,
                &(20 * factor).to_string(),
                &(9 * factor).to_string(),
                "mousedown",
                "1",
                "sleep",
                "0.05",
                "mouseup",
                "1",
            ],
        );
        parent.set_position(370, 310);
        pump(&mut parent, &mut child, Duration::from_millis(150));
        assert_position(
            &child,
            (370 + 200 * factor, 310),
            "click keeps pair attached",
        );

        let origin = parent.get_position();
        let group_target = (origin.0 + 30, origin.1 + 20);
        move_held(&mut parent, &mut child, false, group_target, factor);
        assert_position(&parent, group_target, "parent moves while held");
        assert_position(
            &child,
            (group_target.0 + 200 * factor, group_target.1),
            "attached companion follows while held",
        );
        release(&mut parent, &mut child);

        let parent_position = parent.get_position();
        move_held(&mut parent, &mut child, true, (900, 700), factor);
        assert_position(
            &parent,
            parent_position,
            "companion drag leaves parent fixed",
        );
        release(&mut parent, &mut child);
        assert_position(&child, (900, 700), "distant companion remains free");
        parent.set_position(parent_position.0 - 25, parent_position.1);
        pump(&mut parent, &mut child, Duration::from_millis(150));
        assert_position(&child, (900, 700), "free parent does not move companion");

        // Eleven logical pixels avoid Openbox's default ten-pixel resistance
        // while remaining inside our twelve-pixel magnet, at both scales.
        for side in 0..4 {
            let (x, y) = parent.get_position();
            let (near, snapped) = match side {
                0 => ((x + 211 * factor, y + 10 * factor), (x + 200 * factor, y)),
                1 => ((x - 171 * factor, y + 10 * factor), (x - 160 * factor, y)),
                2 => ((x + 10 * factor, y + 131 * factor), (x, y + 120 * factor)),
                _ => ((x + 10 * factor, y - 111 * factor), (x, y - 100 * factor)),
            };
            move_held(&mut parent, &mut child, true, near, factor);
            assert_position(&parent, (x, y), "free move must not move parent");
            assert_position(&child, near, "magnet must wait for release");
            release(&mut parent, &mut child);
            assert_position(
                &child,
                snapped,
                "released companion snaps edge and alignment",
            );
            parent.set_position(x + 7, y + 5);
            pump(&mut parent, &mut child, Duration::from_millis(150));
            assert_position(
                &child,
                (snapped.0 + 7, snapped.1 + 5),
                "snapped companion reattaches",
            );
            move_held(&mut parent, &mut child, true, (900, 700), factor);
            release(&mut parent, &mut child);
        }

        // The relationship survives free movement for minimize and restore.
        child.minimize();
        wait_state(&mut parent, &mut child, "Iconic");
        let parent_id = id(&parent);
        xdotool(
            &mut parent,
            &mut child,
            &[
                "windowmap",
                &parent_id,
                "windowactivate",
                "--sync",
                &parent_id,
            ],
        );
        wait_state(&mut parent, &mut child, "Normal");
        assert_position(&child, (900, 700), "restore retains free position");

        // A free player can also approach the companion and reattach.
        let child_position = child.get_position();
        let target = (
            child_position.0 - 200 * factor - 11 * factor,
            child_position.1,
        );
        move_held(&mut parent, &mut child, false, target, factor);
        assert_position(
            &child,
            child_position,
            "free player drag leaves companion fixed",
        );
        release(&mut parent, &mut child);
        assert_position(
            &parent,
            (child_position.0 - 200 * factor, child_position.1),
            "free player snaps on release",
        );

        child.set_docked_parent(None, 0, 0).unwrap();
        parent.set_position(100, 100);
        pump(&mut parent, &mut child, Duration::from_millis(150));
        assert_position(&child, child_position, "None removes magnetic relationship");
        assert!(
            child.set_dock_magnetic(true).is_err(),
            "inactive pair cannot be enabled"
        );
        child
            .set_docked_parent(Some(&mut parent), 200 * factor, 0)
            .unwrap();
        child.set_dock_magnetic(true).unwrap();
        pump(&mut parent, &mut child, Duration::from_millis(100));
        let child_id = id(&child);
        xdotool(
            &mut parent,
            &mut child,
            &["windowactivate", "--sync", &child_id, "key", "alt+F4"],
        );
        assert!(!child.is_open() && parent.is_open());
        drop(child);
        parent.set_position(200, 150);
        parent.update();
        assert!(parent.is_open(), "closed companion leaves no live relation");
    }
}

fn saved(window: &Window) -> minifb::RestorablePlacement {
    window
        .get_restorable_placement()
        .expect("native X11 placement")
}

fn wait_positions(
    parent: &mut Window,
    child: &mut Window,
    expected: ((isize, isize), (isize, isize)),
) {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        pump(parent, child, Duration::from_millis(20));
        let actual = (parent.get_position(), child.get_position());
        if actual == expected {
            return;
        }
        assert!(
            Instant::now() < until,
            "positions {actual:?}, expected {expected:?}"
        );
    }
}

fn restorable_pair(scale: Scale, factor: isize) -> (Window, Window) {
    let mut parent = window("Spotiamp restorable player", (200, 120), scale);
    let mut child = window("Spotiamp restorable library", (160, 100), scale);
    wait_state(&mut parent, &mut child, "Normal");
    // Set the leader first; awaiting the actual origin avoids depending on
    // which X connection the WM processes first when creating the pair.
    let child_origin = child.get_position();
    parent.set_position(80, 100);
    wait_positions(&mut parent, &mut child, ((80, 100), child_origin));
    child
        .set_docked_parent(Some(&mut parent), 200 * factor, 0)
        .unwrap();
    child.set_dock_magnetic(true).unwrap();
    wait_positions(
        &mut parent,
        &mut child,
        ((80, 100), (80 + 200 * factor, 100)),
    );
    (parent, child)
}

#[test]
#[ignore = "requires isolated Xvfb, Openbox, xprop and xdotool"]
fn x11_restorable_attachment_changes_preserve_positions_and_explicit_detach() {
    let _manager = openbox("RESTORABLE_ATTACHMENT");
    for (scale, factor) in [(Scale::X1, 1), (Scale::X2, 2)] {
        let (mut parent, mut child) = restorable_pair(scale, factor);
        let before = (saved(&parent), saved(&child));
        assert!(before.0.docked && before.1.docked);
        child.set_dock_attached(false).unwrap();
        assert_eq!(saved(&parent).position, before.0.position);
        assert_eq!(saved(&child).position, before.1.position);
        assert!(!saved(&parent).docked && !saved(&child).docked);
        child.set_position(700, 350);
        wait_positions(&mut parent, &mut child, (before.0.position, (700, 350)));

        child.set_dock_attached(true).unwrap();
        assert_eq!(saved(&parent).position, before.0.position);
        assert_eq!(saved(&child).position, (700, 350));
        assert!(saved(&parent).docked && saved(&child).docked);
        parent.set_position(95, 112);
        wait_positions(&mut parent, &mut child, ((95, 112), (715, 362)));
        child.set_docked_parent(None, 0, 0).unwrap();
        assert!(!saved(&parent).docked && !saved(&child).docked);
        assert_eq!(saved(&parent).position, (95, 112));
        assert_eq!(saved(&child).position, (715, 362));
        assert!(child.set_dock_attached(true).is_err());
        parent.set_position(110, 120);
        wait_positions(&mut parent, &mut child, ((110, 120), (715, 362)));
    }
}

mod native_close {
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
    }

    pub(super) fn request(window: &Window) {
        // WM_DELETE_WINDOW is the WM's native close request. xdotool windowclose
        // calls XDestroyWindow directly, which bypasses this application path.
        unsafe {
            let display = XOpenDisplay(std::ptr::null());
            assert!(!display.is_null());
            let mut event = Event { pad: [0; 24] };
            let target = window.get_window_handle() as c_ulong;
            event.client = ClientMessage {
                kind: 33,
                serial: 0,
                sent: 1,
                display,
                window: target,
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
            let sent = XSendEvent(display, target, 0, 0, &mut event);
            XCloseDisplay(display); // Flush before testing the native event pump.
            assert_ne!(sent, 0);
        }
    }
}

fn close_pair_endpoint(parent: &mut Window, child: &mut Window, parent_first: bool) {
    native_close::request(if parent_first { &*parent } else { &*child });
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        // Mirror the application: a closed surface is not pumped again, while
        // the surviving companion can still process its native notifications.
        if parent.is_open() {
            parent.update();
        }
        if child.is_open() {
            child.update();
        }
        if !(if parent_first {
            parent.is_open()
        } else {
            child.is_open()
        }) {
            return;
        }
        assert!(Instant::now() < until, "native close was not received");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "requires isolated Xvfb, Openbox, xprop and xdotool"]
fn x11_restorable_normal_and_minimized_native_close_preserves_both_endpoints() {
    let _manager = openbox("RESTORABLE_CLOSE");
    for (scale, factor) in [(Scale::X1, 1), (Scale::X2, 2)] {
        for attached in [false, true] {
            for close_parent_first in [false, true] {
                for minimized in [false, true] {
                    let (mut parent, mut child) = restorable_pair(scale, factor);
                    if !attached {
                        child.set_dock_attached(false).unwrap();
                        child.set_position(700, 350);
                        wait_positions(&mut parent, &mut child, ((80, 100), (700, 350)));
                    }
                    let before = (saved(&parent), saved(&child));
                    assert_eq!((before.0.docked, before.1.docked), (attached, attached));
                    if minimized {
                        child.minimize();
                        wait_state(&mut parent, &mut child, "Iconic");
                        assert_eq!((saved(&parent), saved(&child)), before);
                        assert!(child.set_dock_attached(true).is_err());
                    }

                    close_pair_endpoint(&mut parent, &mut child, close_parent_first);
                    assert_eq!(
                        (saved(&parent), saved(&child)),
                        before,
                        "first native close lost placement: attached={attached}, parent_first={close_parent_first}"
                    );
                    assert!(if close_parent_first {
                        child.is_open()
                    } else {
                        parent.is_open()
                    });
                    close_pair_endpoint(&mut parent, &mut child, !close_parent_first);
                    assert!(!parent.is_open() && !child.is_open());
                    assert_eq!(
                        (saved(&parent), saved(&child)),
                        before,
                        "second native close lost cached placement"
                    );
                }
            }
        }
    }
}
