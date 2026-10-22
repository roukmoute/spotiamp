//! Persistence reads use real HWND geometry and survive native close/minimize.
use super::*;

fn placement(window: &Window) -> ((isize, isize), bool) {
    let value = window.get_restorable_placement().expect("native placement");
    assert_ne!(value.position, (-32000, -32000));
    (value.position, value.docked)
}

fn close(window: &Window) {
    unsafe { SendMessageW(window.get_window_handle(), 0x0010, 0, 0) }; // WM_CLOSE
    assert!(!window.is_open());
    assert_eq!(unsafe { IsWindow(window.get_window_handle()) }, 0);
}

#[test]
fn native_restorable_free_and_attached_positions_round_trip_without_jump() {
    let _serial = WINDOWS.lock().unwrap_or_else(|error| error.into_inner());
    for scale in [Scale::X1, Scale::X2] {
        let mut player = dock_window(scale);
        let mut library = dock_window(scale);
        assert!(library.set_dock_attached(false).is_err());
        player.set_position(-70, 80);
        let size = player.get_size();
        let width = size.0 as isize;
        library
            .set_docked_parent(Some(&mut player), width, 0)
            .unwrap();
        assert!(
            library.set_dock_attached(false).is_err(),
            "magnetic opt-in required"
        );
        library.set_dock_magnetic(true).unwrap();
        assert_eq!(placement(&player), ((-70, 80), true));
        assert_eq!(placement(&library), ((-70 + width, 80), true));

        library.set_dock_attached(false).unwrap();
        assert_eq!(library.get_position(), (-70 + width, 80));
        library.set_position(430, 190);
        player.set_position(-90, 60);
        let saved_player = placement(&player);
        let saved_library = placement(&library);
        assert_eq!(saved_player, ((-90, 60), false));
        assert_eq!(saved_library, ((430, 190), false));
        close(&library);
        assert_eq!(placement(&library), saved_library);
        assert_eq!(placement(&player), saved_player);
        close(&player);
        assert_eq!(placement(&player), saved_player);
        drop(library);
        drop(player);

        let mut restored_player = dock_window(scale);
        let mut restored_library = dock_window(scale);
        restored_library
            .set_docked_parent(Some(&mut restored_player), width, 0)
            .unwrap();
        restored_library.set_dock_magnetic(true).unwrap();
        restored_library.set_dock_attached(false).unwrap();
        restored_player.set_position(saved_player.0.0, saved_player.0.1);
        restored_library.set_position(saved_library.0.0, saved_library.0.1);
        assert_eq!(placement(&restored_player), saved_player);
        assert_eq!(placement(&restored_library), saved_library);
        assert_eq!(restored_player.get_size(), size);

        // Reattaching adopts the restored offset instead of moving either HWND.
        restored_library.set_dock_attached(true).unwrap();
        assert_eq!(placement(&restored_player), (saved_player.0, true));
        assert_eq!(placement(&restored_library), (saved_library.0, true));
        restored_player.set_position(-60, 100);
        assert_eq!(placement(&restored_library), ((460, 230), true));
        assert_eq!(
            unsafe { GetWindow(restored_library.get_window_handle(), 4) },
            restored_player.get_window_handle(),
        );
        restored_library.set_docked_parent(None, 0, 0).unwrap();
        assert_eq!(placement(&restored_player), ((-60, 100), false));
        assert_eq!(placement(&restored_library), ((460, 230), false));
    }
}

#[test]
fn native_restorable_minimized_close_preserves_both_endpoints() {
    let _serial = WINDOWS.lock().unwrap_or_else(|error| error.into_inner());
    for scale in [Scale::X1, Scale::X2] {
        for attached in [false, true] {
            for close_parent_first in [false, true] {
                let mut player = dock_window(scale);
                let mut library = dock_window(scale);
                // Use visible positions here: Windows may clamp restore bounds
                // outside the current monitor. Signed positions are tested above.
                player.set_position(50, 70);
                let width = player.get_size().0 as isize;
                library
                    .set_docked_parent(Some(&mut player), width, 0)
                    .unwrap();
                library.set_dock_magnetic(true).unwrap();
                library.set_dock_attached(attached).unwrap();
                if !attached {
                    library.set_position(450, 200);
                }
                let saved_player = placement(&player);
                let saved_library = placement(&library);
                library.minimize();
                assert_ne!(unsafe { IsIconic(player.get_window_handle()) }, 0);
                assert_eq!(unsafe { IsWindowVisible(library.get_window_handle()) }, 0);
                assert_eq!(placement(&player), saved_player);
                assert_eq!(placement(&library), saved_library);
                assert!(library.set_dock_attached(true).is_err());
                if close_parent_first {
                    close(&player);
                    assert_eq!(placement(&player), saved_player);
                    assert_eq!(placement(&library), saved_library);
                    assert_ne!(unsafe { IsWindow(library.get_window_handle()) }, 0);
                    close(&library);
                } else {
                    close(&library);
                    assert_eq!(placement(&player), saved_player);
                    assert_eq!(placement(&library), saved_library);
                    close(&player);
                }
                assert_eq!(placement(&player), saved_player);
                assert_eq!(placement(&library), saved_library);
            }
        }
    }
}

#[repr(C)]
#[derive(Default)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MonitorFromPoint(point: Point, flags: u32) -> Hwnd;
    fn GetMonitorInfoW(monitor: Hwnd, info: *mut MonitorInfo) -> i32;
}

#[test]
fn native_restorable_work_area_uses_signed_point_and_nearest_monitor() {
    let _serial = WINDOWS.lock().unwrap_or_else(|error| error.into_inner());
    let player = dock_window(Scale::X2);
    // Negative virtual-screen points select a real left/top monitor when one is
    // present, otherwise the nearest existing monitor on a single-screen runner.
    for (x, y) in [
        (-1600, -900),
        (-1, 100),
        (0, 0),
        (1600, 900),
        (32000, -32000),
    ] {
        let monitor = unsafe { MonitorFromPoint(Point { x, y }, 2) };
        assert!(!monitor.is_null());
        let mut info = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            ..MonitorInfo::default()
        };
        assert_ne!(unsafe { GetMonitorInfoW(monitor, &mut info) }, 0);
        let expected = (
            info.work.left as isize,
            info.work.top as isize,
            (info.work.right - info.work.left) as usize,
            (info.work.bottom - info.work.top) as usize,
        );
        assert_eq!(
            player.get_work_area_at(x as isize, y as isize),
            Some(expected)
        );
    }
    #[cfg(target_pointer_width = "64")]
    assert!(player.get_work_area_at(isize::MAX, 0).is_none());
}
