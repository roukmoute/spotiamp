//! Regression tests for wheel input and playlist navigation. These exercise
//! visible row selection, fractional devices, event batches and queue limits.
use super::*;
use crate::model::TrackRow;

fn playlist(len: usize) -> Snapshot {
    Snapshot {
        queue: (0..len)
            .map(|index| TrackRow {
                title: format!("Titre {}", index + 1),
                ..TrackRow::default()
            })
            .collect(),
        current: Some(4).filter(|index| *index < len),
        playing: len > 4,
        ..Snapshot::default()
    }
}

#[test]
fn one_notch_exposes_track_nine_then_ten_without_changing_playback() {
    let state = Snapshot {
        current: Some(8),
        ..playlist(30)
    };
    let mut view = View {
        selected: Some(2),
        ..View::default()
    };

    view.scroll(-1.0, state.queue.len());
    view.render(&state);
    assert_eq!(view.offset, 1, "one notch must show rows 2 through 9");
    assert_eq!(view.selected, Some(2), "scrolling must retain selection");
    let last_visible_row_y = 221 + (ROWS - 1) * 24 + 12;
    assert!(view.click(100, last_visible_row_y, &state).is_none());
    assert_eq!(view.selected, Some(8), "track 9 must be mouse-accessible");

    view.scroll(-1.0, state.queue.len());
    view.render(&state);
    assert_eq!(view.offset, 2, "next notch must show rows 3 through 10");
    assert_eq!(view.selected, Some(8));
    assert!(view.click(100, last_visible_row_y, &state).is_none());
    assert_eq!(view.selected, Some(9), "track 10 must be mouse-accessible");
    assert_eq!(state.current, Some(8));
    assert!(state.playing);
}

#[test]
fn fractional_wheel_motion_accumulates_before_moving_a_row() {
    let mut view = View::default();
    for _ in 0..3 {
        view.scroll(-0.25, 30);
        assert_eq!(view.offset, 0, "a fraction must not round up to a row");
    }
    view.scroll(-0.25, 30);
    assert_eq!(view.offset, 1);
    for _ in 0..3 {
        view.scroll(0.25, 30);
        assert_eq!(view.offset, 1);
    }
    view.scroll(0.25, 30);
    assert_eq!(view.offset, 0);
}

#[test]
fn wheel_batches_preserve_motion_and_fractional_remainder() {
    let mut batched = View::default();
    batched.scroll(-3.5, 50);
    assert_eq!(batched.offset, 3);
    batched.scroll(-0.5, 50);
    assert_eq!(batched.offset, 4);

    let mut separate = View::default();
    for _ in 0..4 {
        separate.scroll(-1.0, 50);
    }
    assert_eq!(batched.offset, separate.offset);
    batched.scroll(-12.0, 50);
    assert_eq!(
        batched.offset, 16,
        "a batch must not lose motion at an arbitrary ten-row cap"
    );
}

#[test]
fn non_binary_fractions_still_complete_a_whole_notch() {
    let mut view = View::default();
    view.scroll(-70.0 / 120.0, 30);
    assert_eq!(view.offset, 0);
    view.scroll(-50.0 / 120.0, 30);
    assert_eq!(view.offset, 1);
    view.scroll(70.0 / 120.0, 30);
    assert_eq!(view.offset, 1);
    view.scroll(50.0 / 120.0, 30);
    assert_eq!(view.offset, 0);
}

#[test]
fn opposite_fractional_motion_cancels_without_a_phantom_row() {
    let mut view = View::default();
    view.scroll(-5.0, 50);
    view.scroll(-0.5, 50);
    view.scroll(0.5, 50);
    assert_eq!(view.offset, 5);
    view.scroll(-0.5, 50);
    assert_eq!(view.offset, 5, "cancelled fractions must not survive");
    view.scroll(-0.5, 50);
    assert_eq!(view.offset, 6);
}

#[test]
fn short_queues_never_scroll_and_shrinking_queue_clamps_existing_offset() {
    for len in [0, 1, ROWS - 1, ROWS] {
        let mut view = View::default();
        view.scroll(-1_000.0, len);
        view.scroll(1_000.0, len);
        assert_eq!(view.offset, 0, "queue length {len} fits in the viewport");
    }

    let mut view = View::default();
    view.scroll(-30.0, 50);
    view.scroll(-1.0, ROWS + 1);
    assert_eq!(view.offset, 1, "offset must follow a shortened queue");
    view.scroll(1.0, ROWS + 1);
    assert_eq!(view.offset, 0);
}

#[test]
fn reaching_an_edge_discards_outward_motion_so_reversing_works_immediately() {
    let mut view = View::default();
    view.scroll(-f32::MAX, 30);
    assert_eq!(view.offset, 30 - ROWS);
    view.scroll(-0.75, 30);
    view.scroll(1.0, 30);
    assert_eq!(
        view.offset,
        30 - ROWS - 1,
        "bottom edge must not retain scroll debt"
    );

    view.scroll(f32::MAX, 30);
    assert_eq!(view.offset, 0);
    view.scroll(0.75, 30);
    view.scroll(-1.0, 30);
    assert_eq!(view.offset, 1, "top edge must not retain scroll debt");
}

#[test]
fn invalid_scroll_values_do_not_move_or_poison_later_input() {
    let mut view = View::default();
    view.scroll(-5.0, 30);
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        view.scroll(invalid, 30);
        assert_eq!(view.offset, 5);
    }
    view.scroll(-1.0, 30);
    assert_eq!(view.offset, 6);
}

#[cfg(target_os = "linux")]
mod x11_native {
    use super::*;
    use minifb::{Window, WindowOptions};
    use std::time::{Duration, Instant};

    fn click_wheel(window: &mut Window, button: &str) -> f32 {
        let window_id = (window.get_window_handle() as usize).to_string();
        let result = std::process::Command::new("xdotool")
            .args(["click", "--window", &window_id, button])
            .output()
            .expect("native X11 wheel regression requires xdotool on PATH");
        assert!(
            result.status.success(),
            "xdotool click failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );

        // Read several frames so a release delivered after its press still
        // participates in the assertion. The native accumulator resets on update.
        let deadline = Instant::now() + Duration::from_millis(300);
        let mut total = 0.0;
        while Instant::now() < deadline {
            window.update();
            if let Some((_, y)) = window.get_scroll_wheel() {
                total += y;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        total
    }

    #[test]
    #[ignore = "requires X11 display and xdotool; run under Xvfb"]
    fn x11_wheel_press_and_release_count_as_one_notch() {
        let mut window = Window::new(
            "Spotiamp X11 wheel regression",
            32,
            32,
            WindowOptions::default(),
        )
        .expect("native wheel regression requires an X11 display; run under Xvfb");
        window.update();
        assert!(!window.get_window_handle().is_null());

        let mut view = View::default();
        let down = click_wheel(&mut window, "5");
        assert_eq!(down, -1.0, "Button5 press/release must count only once");
        view.scroll(down, 50);
        assert_eq!(view.offset, 1);

        let up = click_wheel(&mut window, "4");
        assert_eq!(up, 1.0, "Button4 press/release must count only once");
        view.scroll(up, 50);
        assert_eq!(view.offset, 0);
    }
}

#[cfg(target_os = "windows")]
mod windows_native {
    use super::*;
    use minifb::{Window, WindowOptions};
    use std::ffi::c_void;

    const WM_MOUSEWHEEL: u32 = 0x020A;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn PostMessageW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> i32;
    }

    fn post_wheel(window: &Window, delta: i16) {
        // Win32 packs the signed wheel delta into the high WORD of WPARAM.
        let wparam = usize::from(delta as u16) << 16;
        let result = unsafe { PostMessageW(window.get_window_handle(), WM_MOUSEWHEEL, wparam, 0) };
        assert_ne!(
            result,
            0,
            "PostMessageW(WM_MOUSEWHEEL) failed: {}",
            std::io::Error::last_os_error()
        );
    }

    fn collect_scroll(window: &mut Window) -> f32 {
        window.update();
        window
            .get_scroll_wheel()
            .expect("posted wheel event was not delivered to the native WndProc")
            .1
    }

    #[test]
    fn win32_wheel_messages_reach_the_playlist_with_notch_and_fraction_units() {
        let _serial = crate::window_native_windows_tests::WINDOWS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut window = Window::new(
            "Spotiamp native wheel regression",
            32,
            32,
            WindowOptions::default(),
        ).unwrap_or_else(|error| panic!(
            "Native wheel regression requires a Windows desktop capable of creating a window; creation failed: {error}. Run this test on a Windows desktop runner; it must not be silently skipped."
        ));
        // Registers the stable WindowState pointer used by the patched WndProc.
        window.update();
        assert!(!window.get_window_handle().is_null());

        let mut view = View::default();
        post_wheel(&window, -120);
        let delta = collect_scroll(&mut window);
        assert_eq!(delta, -1.0, "one Win32 WHEEL_DELTA must become one notch");
        view.scroll(delta, 50);
        assert_eq!(view.offset, 1);

        // Both half-notch events arrive in one message pump iteration.
        post_wheel(&window, -60);
        post_wheel(&window, -60);
        let delta = collect_scroll(&mut window);
        assert_eq!(
            delta, -1.0,
            "WndProc must accumulate events, not overwrite them"
        );
        view.scroll(delta, 50);
        assert_eq!(view.offset, 2);

        // High-resolution wheel fractions may also span several UI frames.
        for frame in 0..4 {
            post_wheel(&window, -30);
            let delta = collect_scroll(&mut window);
            assert_eq!(delta, -0.25);
            view.scroll(delta, 50);
            assert_eq!(view.offset, if frame == 3 { 3 } else { 2 });
        }

        // These fractions are not exactly representable as f32, but their
        // raw Win32 deltas still add up to one complete notch.
        post_wheel(&window, -70);
        view.scroll(collect_scroll(&mut window), 50);
        assert_eq!(view.offset, 3);
        post_wheel(&window, -50);
        view.scroll(collect_scroll(&mut window), 50);
        assert_eq!(view.offset, 4);

        // An empty frame must not replay the previous wheel message.
        window.update();
        assert_eq!(window.get_scroll_wheel(), None);
    }
}
