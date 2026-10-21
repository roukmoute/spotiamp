use super::*;
use std::process::{Child, Stdio};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn property(id: &str, name: &str) -> String {
    let output = std::process::Command::new("xprop")
        .args(["-id", id, name])
        .output()
        .expect("xprop is required");
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

fn pump(window: &mut Window, duration: Duration) {
    let until = Instant::now() + duration;
    while Instant::now() < until {
        window.update();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn xdotool(window: &mut Window, args: &[&str]) {
    let mut command = Process(
        std::process::Command::new("xdotool")
            .args(args)
            .spawn()
            .expect("xdotool is required"),
    );
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        window.update();
        if let Some(status) = command.0.try_wait().unwrap() {
            assert!(status.success(), "xdotool {args:?}");
            break;
        }
        assert!(Instant::now() < until, "xdotool timed out: {args:?}");
        std::thread::sleep(Duration::from_millis(5));
    }
    pump(window, Duration::from_millis(100));
}

#[test]
#[ignore = "requires isolated Xvfb, Openbox, xprop and xdotool"]
fn x11_borderless_window_drags_minimizes_restores_and_closes_at_both_scales() {
    let mut manager = Process(
        std::process::Command::new("openbox")
            .arg("--sm-disable")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Openbox is required for native window-manager behavior"),
    );
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(
            manager.0.try_wait().unwrap().is_none(),
            "Openbox failed to start"
        );
        let output = std::process::Command::new("xprop")
            .args(["-root", "_NET_SUPPORTING_WM_CHECK"])
            .output()
            .unwrap();
        if String::from_utf8_lossy(&output.stdout).contains("window id #") {
            break;
        }
        assert!(Instant::now() < until, "Openbox did not initialize");
        std::thread::sleep(Duration::from_millis(25));
    }
    for (scale, factor) in [(Scale::X1, 1), (Scale::X2, 2)] {
        let mut window = Window::new(
            "Spotiamp native frame regression",
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
        window.set_titlebar_drag_region(Some((4, 2, 120, 16)));
        window.set_target_fps(0);
        window
            .update_with_buffer(&vec![0x29283d; 200 * 120], 200, 120)
            .unwrap();
        let id = (window.get_window_handle() as usize).to_string();
        xdotool(
            &mut window,
            &[
                "windowmove",
                &id,
                "120",
                "160",
                "windowactivate",
                "--sync",
                &id,
            ],
        );
        assert!(
            property(&id, "_NET_FRAME_EXTENTS").contains("= 0, 0, 0, 0"),
            "native decorations must be absent"
        );
        assert_eq!(window.get_size(), (200 * factor, 120 * factor));
        let before = window.get_position();
        let x = (20 * factor).to_string();
        let y = (9 * factor).to_string();
        xdotool(
            &mut window,
            &[
                "mousemove",
                "--window",
                &id,
                &x,
                &y,
                "mousedown",
                "1",
                "sleep",
                "0.15",
                "mousemove_relative",
                "--",
                "90",
                "50",
                "sleep",
                "0.15",
                "mouseup",
                "1",
            ],
        );
        let after = window.get_position();
        assert!(
            (after.0 - before.0 - 90).abs() <= 2 && (after.1 - before.1 - 50).abs() <= 2,
            "titlebar drag at scale {factor}: {before:?} -> {after:?}"
        );
        // A click outside the titlebar must not initiate a window move.
        let x = (150 * factor).to_string();
        xdotool(
            &mut window,
            &[
                "mousemove",
                "--window",
                &id,
                &x,
                &y,
                "mousedown",
                "1",
                "sleep",
                "0.1",
                "mousemove_relative",
                "--",
                "20",
                "10",
                "mouseup",
                "1",
            ],
        );
        assert_eq!(window.get_position(), after);
        window.minimize();
        pump(&mut window, Duration::from_millis(250));
        assert!(property(&id, "WM_STATE").contains("Iconic"));
        assert!(window.is_open(), "minimizing must keep the player alive");
        xdotool(
            &mut window,
            &["windowmap", &id, "windowactivate", "--sync", &id],
        );
        assert!(property(&id, "WM_STATE").contains("Normal"));
        assert!(window.is_active());
        xdotool(&mut window, &["key", "alt+F4"]);
        assert!(
            !window.is_open(),
            "native close shortcut must remain available"
        );
    }
}
