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

#[test]
#[ignore = "requires isolated Xvfb, Openbox, xprop and xdotool"]
fn x11_magnetic_pair_detaches_snaps_on_release_and_keeps_group_minimize() {
    let ready_atom = format!("_SPOTIAMP_MAGNETIC_READY_{}", std::process::id());
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
