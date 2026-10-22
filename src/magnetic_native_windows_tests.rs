//! Send native drag lifecycle messages to actual HWNDs, without frame polling.
use super::*;

fn begin(window: &Window) {
    unsafe { SendMessageW(window.get_window_handle(), 0x0231, 0, 0) }; // ENTERSIZEMOVE
}

fn propose(window: &Window, x: isize, y: isize) {
    let size = window.get_size();
    let mut rectangle = Rect {
        left: x as i32,
        top: y as i32,
        right: (x + size.0 as isize) as i32,
        bottom: (y + size.1 as isize) as i32,
    };
    unsafe {
        SendMessageW(
            window.get_window_handle(),
            0x0216, // WM_MOVING
            0,
            (&mut rectangle as *mut Rect) as isize,
        );
    }
    assert_eq!((rectangle.left as isize, rectangle.top as isize), (x, y));
}

fn end(window: &Window) {
    unsafe { SendMessageW(window.get_window_handle(), 0x0232, 0, 0) }; // EXITSIZEMOVE
}

fn drag(window: &mut Window, x: isize, y: isize) {
    begin(window);
    propose(window, x, y);
    window.set_position(x, y);
    end(window);
}

fn restore(player: &mut Window, library: &mut Window) {
    unsafe { ShowWindow(player.get_window_handle(), 9) }; // SW_RESTORE
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline
        && unsafe {
            IsIconic(player.get_window_handle()) != 0
                || IsWindowVisible(library.get_window_handle()) == 0
        }
    {
        player.update();
        library.update();
    }
    assert_eq!(unsafe { IsIconic(player.get_window_handle()) }, 0);
    assert_ne!(unsafe { IsWindowVisible(library.get_window_handle()) }, 0);
}

#[test]
fn magnetic_companion_detaches_without_moving_player_and_snaps_only_on_release() {
    let _serial = WINDOWS.lock().unwrap_or_else(|e| e.into_inner());
    for (scale, multiplier) in [(Scale::X1, 1isize), (Scale::X2, 2)] {
        let mut player = dock_window(scale);
        let mut library = dock_window(scale);
        player.set_target_fps(0);
        library.set_target_fps(0);
        player.set_position(-60, 90);
        let width = player.get_size().0 as isize;
        library
            .set_docked_parent(Some(&mut player), width, 0)
            .unwrap();
        library.set_dock_magnetic(true).unwrap();

        // Entering the native move loop (a click) is not a real movement.
        let old_child = library.get_position();
        begin(&library);
        propose(&library, old_child.0, old_child.1);
        end(&library);
        player.set_position(-80, 110);
        assert_eq!(library.get_position(), (-80 + width, 110));

        let fixed_player = player.get_position();
        begin(&library);
        propose(&library, 450, 400);
        assert_eq!(
            player.get_position(),
            fixed_player,
            "child drag never pulls player"
        );
        library.set_position(450, 400);
        end(&library);
        assert_eq!(library.get_position(), (450, 400));
        player.set_position(-100, 130);
        assert_eq!(
            library.get_position(),
            (450, 400),
            "free windows move independently"
        );

        // Native ownership survives geometric detachment, including minimize.
        // Keep the owner fully inside the CI desktop for restore-placement tests;
        // signed/off-screen movement is exercised separately above and below.
        player.set_position(80, 130);
        assert_eq!(
            unsafe { GetWindow(library.get_window_handle(), 4) },
            player.get_window_handle()
        );
        library.minimize();
        assert_ne!(unsafe { IsIconic(player.get_window_handle()) }, 0);
        assert_eq!(unsafe { IsWindowVisible(library.get_window_handle()) }, 0);
        restore(&mut player, &mut library);
        assert_eq!(player.get_position(), (80, 130));
        assert_eq!(
            library.get_position(),
            (450, 400),
            "restore must not redock a free companion"
        );

        let (px, py) = player.get_position();
        let near = (px + width + 12 * multiplier - 1, py + 3 * multiplier);
        begin(&library);
        propose(&library, near.0, near.1);
        library.set_position(near.0, near.1);
        assert_eq!(
            library.get_position(),
            near,
            "no magnetic preview during drag"
        );
        assert_eq!(player.get_position(), (px, py));
        end(&library);
        assert_eq!(
            library.get_position(),
            (px + width, py),
            "scaled threshold snaps edge and corner"
        );

        // The player remains the group leader after redocking.
        begin(&player);
        propose(&player, -150, -20);
        assert_eq!(library.get_position(), (-150 + width, -20));
        player.set_position(-150, -20);
        end(&player);
        assert_eq!(library.get_position(), (-150 + width, -20));

        // The free player can also approach a stationary companion and redock.
        drag(&mut library, 480, 350);
        let stationary = library.get_position();
        drag(
            &mut player,
            stationary.0 - width - 5 * multiplier,
            stationary.1 + multiplier,
        );
        assert_eq!(player.get_position(), (stationary.0 - width, stationary.1));
        assert_eq!(library.get_position(), stationary);
        player.set_position(-40, 70);
        assert_eq!(library.get_position(), (-40 + width, 70));
    }
}

#[test]
fn magnetic_native_snap_supports_all_edges_and_rejects_far_or_nonoverlapping_windows() {
    let _serial = WINDOWS.lock().unwrap_or_else(|e| e.into_inner());
    for (scale, multiplier) in [(Scale::X1, 1isize), (Scale::X2, 2)] {
        let mut player = dock_window(scale);
        let mut library = dock_window(scale);
        player.set_position(-200, -100);
        let (pw, ph) = player.get_size();
        let (cw, ch) = library.get_size();
        library
            .set_docked_parent(Some(&mut player), pw as isize, 0)
            .unwrap();
        library.set_dock_magnetic(true).unwrap();
        let gap = 12 * multiplier - 1;
        for (near, expected) in [
            ((-200 + pw as isize + gap, -100), (-200 + pw as isize, -100)),
            ((-200 - cw as isize - gap, -100), (-200 - cw as isize, -100)),
            ((-200, -100 + ph as isize + gap), (-200, -100 + ph as isize)),
            ((-200, -100 - ch as isize - gap), (-200, -100 - ch as isize)),
        ] {
            drag(&mut library, 500, 400);
            drag(&mut library, near.0, near.1);
            assert_eq!(library.get_position(), expected);
            assert_eq!(player.get_position(), (-200, -100));
            player.set_position(-190, -90);
            assert_eq!(library.get_position(), (expected.0 + 10, expected.1 + 10));
            player.set_position(-200, -100);
        }
        for free in [
            (-200 + pw as isize + 12 * multiplier + 1, -100),
            (-200 + pw as isize + 1, -100 + ph as isize + 1),
        ] {
            drag(&mut library, free.0, free.1);
            assert_eq!(library.get_position(), free);
            player.set_position(-210, -110);
            assert_eq!(
                library.get_position(),
                free,
                "outside threshold/overlap must stay free"
            );
            player.set_position(-200, -100);
        }
    }
}

#[test]
fn magnetic_pair_disable_close_and_drop_keep_lifecycle_and_rigid_contract() {
    let _serial = WINDOWS.lock().unwrap_or_else(|e| e.into_inner());
    let mut player = dock_window(Scale::X1);
    let mut library = dock_window(Scale::X1);
    assert!(library.set_dock_magnetic(true).is_err());
    player.set_position(20, 30);
    library
        .set_docked_parent(Some(&mut player), 120, 0)
        .unwrap();
    library.set_dock_magnetic(true).unwrap();
    drag(&mut library, 500, 400);
    library.set_dock_magnetic(false).unwrap();
    assert_eq!(player.get_position(), (20, 30));
    assert_eq!(library.get_position(), (500, 400));
    player.set_position(30, 40);
    assert_eq!(
        library.get_position(),
        (510, 410),
        "disable restores rigid offset without a jump"
    );
    drag(&mut library, 520, 420);
    assert_eq!(
        player.get_position(),
        (40, 50),
        "old bidirectional rigid behavior is preserved"
    );

    library.set_dock_magnetic(true).unwrap();
    drag(&mut library, 700, 500);
    unsafe { SendMessageW(library.get_window_handle(), 0x0010, 0, 0) }; // WM_CLOSE
    assert!(!library.is_open());
    assert_ne!(unsafe { IsWindow(player.get_window_handle()) }, 0);
    player.set_position(50, 60);
    let mut reopened = dock_window(Scale::X1);
    reopened
        .set_docked_parent(Some(&mut player), -120, 0)
        .unwrap();
    reopened.set_dock_magnetic(true).unwrap();
    drag(&mut reopened, 650, 450);
    let child = reopened.get_window_handle();
    drop(player);
    assert_ne!(unsafe { IsWindow(child) }, 0);
    assert!(unsafe { GetWindow(child, 4) }.is_null());
    assert!(reopened.set_dock_magnetic(false).is_err());
    reopened.set_position(-120, -70);
    assert_eq!(reopened.get_position(), (-120, -70));
}
