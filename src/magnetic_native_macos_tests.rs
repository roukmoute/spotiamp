//! Main-thread Cocoa regression for magnetic docking, without a Spotify session.
#![cfg(target_os = "macos")]
use minifb::{Scale, Window, WindowOptions};
use std::{
    ffi::{CStr, c_char, c_void},
    time::{Duration, Instant},
};

type Id = *mut c_void;
#[link(name = "objc")]
unsafe extern "C" {
    fn sel_registerName(name: *const c_char) -> Id;
    fn objc_msgSend();
}

fn action(window: &Window, selector: &CStr) {
    let send: unsafe extern "C" fn(Id, Id) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe {
        send(
            window.get_window_handle(),
            sel_registerName(selector.as_ptr()),
        )
    };
}

fn restore(window: &Window) {
    let send: unsafe extern "C" fn(Id, Id, Id) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe {
        send(
            window.get_window_handle(),
            sel_registerName(c"deminiaturize:".as_ptr()),
            std::ptr::null_mut(),
        )
    };
}

fn flag(window: &Window, selector: &CStr) -> bool {
    let send: unsafe extern "C" fn(Id, Id) -> bool =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe {
        send(
            window.get_window_handle(),
            sel_registerName(selector.as_ptr()),
        )
    }
}

fn window(scale: Scale) -> Result<Window, String> {
    let mut window = Window::new(
        "Spotiamp magnetic Cocoa test",
        120,
        80,
        WindowOptions {
            borderless: true,
            title: false,
            resize: false,
            scale,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    window.set_target_fps(0);
    window.set_titlebar_drag_region(Some((4, 2, 90, 16)));
    window
        .update_with_buffer(&[0x47425b; 120 * 80], 120, 80)
        .map_err(|e| e.to_string())?;
    Ok(window)
}

fn expect(condition: bool, label: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(format!("Cocoa magnetic: {label}"))
    }
}

fn offset(a: &Window, b: &Window) -> (isize, isize) {
    let a = a.get_position();
    let b = b.get_position();
    (b.0 - a.0, b.1 - a.1)
}

type SavedPlacement = ((isize, isize), bool);
fn saved(window: &Window) -> Result<SavedPlacement, String> {
    window
        .get_restorable_placement()
        .map(|state| (state.position, state.docked))
        .ok_or_else(|| "Cocoa magnetic: missing restorable placement".into())
}

fn start_drag(window: &mut Window, position: (isize, isize)) {
    // The production title-strip handler calls this exact native method before
    // handing motion to AppKit. Real NSWindowDidMove notifications then run.
    action(window, c"beginDockDrag");
    window.set_position(position.0, position.1);
}

fn free_child(a: &mut Window, b: &mut Window) -> Result<(), String> {
    let parent = a.get_position();
    start_drag(b, (parent.0 + 700, parent.1 + 500));
    expect(
        a.get_position() == parent,
        "companion drag moved the parent before release",
    )?;
    // Deliberately no mouseUp: performWindowDragWithEvent may omit it. The
    // ordinary update loop must finalize with the physical button released.
    b.update();
    let child = b.get_position();
    a.set_position(parent.0 + 20, parent.1 + 10);
    expect(
        b.get_position() == child,
        "far release failed to leave the companion free",
    )
}

fn wait(
    a: &mut Window,
    b: &mut Window,
    ready: impl Fn(&Window, &Window) -> bool,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready(a, b) {
        if Instant::now() >= deadline {
            return Err("Cocoa magnetic: minimize/restore timed out".into());
        }
        a.update();
        b.update();
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

pub fn smoke() -> Result<(), String> {
    for (scale, factor) in [(Scale::X1, 1isize), (Scale::X2, 2)] {
        let mut a = window(scale)?;
        let mut b = window(scale)?;
        let width = a.get_size().0 as isize;
        let height = a.get_size().1 as isize;
        a.set_position(-60, 100);
        b.set_docked_parent(Some(&mut a), width, 0)
            .map_err(|e| e.to_string())?;
        expect(
            b.set_dock_attached(false).is_err(),
            "attachment restore accepted a nonmagnetic pair",
        )?;
        let area = a.get_work_area().ok_or("Cocoa missing work area")?;
        expect(
            a.get_work_area_at(
                area.0 + (area.2 / 2) as isize,
                area.1 + (area.3 / 2) as isize,
            ) == Some(area),
            "screen-point lookup differs from the containing screen",
        )?;
        let nearest = a
            .get_work_area_at(-100_000, -100_000)
            .ok_or("Cocoa missing nearest screen")?;
        expect(
            nearest.2 > 0 && nearest.3 > 0,
            "off-screen point has no usable nearest screen",
        )?;

        // Opt-in preserves the existing rigid API and a simple title click.
        start_drag(&mut b, (300, 180));
        b.update();
        expect(
            offset(&a, &b) == (width, 0),
            "rigid mode changed before opt-in",
        )?;
        b.set_dock_magnetic(true).map_err(|e| e.to_string())?;
        action(&b, c"beginDockDrag");
        b.update();
        a.set_position(-60, 100);
        expect(
            offset(&a, &b) == (width, 0),
            "title click detached without movement",
        )?;
        start_drag(&mut a, (-30, 120));
        expect(
            offset(&a, &b) == (width, 0),
            "parent drag stopped moving its attached companion",
        )?;
        a.update();

        // Each native side and its top/left alignment use the same Rust snap
        // geometry as Windows/X11. Keep the above-side case below the menu bar
        // at both scales: y=100 previously requested y=14/X1 and y=-72/X2,
        // mixing docking with menu-bar/off-screen placement instead of
        // isolating snapping.
        // Negative x remains covered here; negative y has pure geometry tests.
        for side in 0..4 {
            free_child(&mut a, &mut b)?;
            a.set_position(-60, 400);
            let p = a.get_position();
            let near = match side {
                0 => (p.0 + width + 6 * factor, p.1 + 4 * factor),
                1 => (p.0 - width - 6 * factor, p.1 + 4 * factor),
                2 => (p.0 + 4 * factor, p.1 + height + 6 * factor),
                _ => (p.0 + 4 * factor, p.1 - height - 6 * factor),
            };
            let expected = match side {
                0 => (width, 0),
                1 => (-width, 0),
                2 => (0, height),
                _ => (0, -height),
            };
            start_drag(&mut b, near);
            let actual_parent = a.get_position();
            let actual_child = b.get_position();
            let snapped = (p.0 + expected.0, p.1 + expected.1);
            if actual_parent != p || actual_child != near {
                return Err(format!(
                    "Cocoa magnetic: unexpected native position before release: \
                     scale={scale:?} factor={factor} side={side} size=({width},{height}) \
                     parent_expected={p:?} child_requested={near:?} \
                     parent_actual={actual_parent:?} child_actual={actual_child:?} \
                     snapped_position={snapped:?} work_area={:?}",
                    a.get_work_area(),
                ));
            }
            b.update();
            let actual_offset = offset(&a, &b);
            if actual_offset != expected {
                return Err(format!(
                    "Cocoa magnetic: release did not snap: scale={scale:?} side={side} \
                     expected_offset={expected:?} actual_offset={actual_offset:?} \
                     parent={:?} child={:?}",
                    a.get_position(),
                    b.get_position(),
                ));
            }
            a.set_position(p.0 + 30, p.1 + 20);
            let actual_offset = offset(&a, &b);
            if actual_offset != expected {
                return Err(format!(
                    "Cocoa magnetic: snapped pair failed to follow parent: \
                     scale={scale:?} side={side} expected_offset={expected:?} \
                     actual_offset={actual_offset:?} parent={:?} child={:?}",
                    a.get_position(),
                    b.get_position(),
                ));
            }
        }

        // A free player can also be the moving side of a new attachment.
        free_child(&mut a, &mut b)?;
        b.set_position(300, 200);
        let fixed = b.get_position();
        start_drag(&mut a, (fixed.0 - width - 6 * factor, fixed.1 + 4 * factor));
        expect(
            b.get_position() == fixed,
            "free player drag moved its companion",
        )?;
        a.update();
        expect(
            offset(&a, &b) == (width, 0),
            "free player failed to snap to companion",
        )?;
        a.set_position(50, 130);
        expect(
            offset(&a, &b) == (width, 0),
            "player-origin snap failed to reattach",
        )?;

        // Two gestures can precede either window's next update. Starting the
        // second one must finish the previous peer immediately, not later
        // against the companion's new location.
        free_child(&mut a, &mut b)?;
        b.set_position(300, 200);
        let fixed = b.get_position();
        start_drag(&mut a, (fixed.0 - width - 6 * factor, fixed.1 + 4 * factor));
        action(&b, c"beginDockDrag");
        let completed_parent = (fixed.0 - width, fixed.1);
        expect(
            a.get_position() == completed_parent,
            "new companion gesture did not finalize the previous player gesture",
        )?;
        b.set_position(800, 500);
        a.update();
        b.update();
        expect(
            a.get_position() == completed_parent,
            "old player gesture snapped late after the companion moved",
        )?;

        free_child(&mut a, &mut b)?;
        let before = (a.get_position(), b.get_position());
        let saved_before = (saved(&a)?, saved(&b)?);
        expect(
            !saved_before.0.1 && !saved_before.1.1,
            "free pair persisted as attached",
        )?;
        b.minimize();
        wait(&mut a, &mut b, |a, b| {
            flag(a, c"isMiniaturized") && !flag(b, c"isVisible")
        })?;
        expect(
            (saved(&a)?, saved(&b)?) == saved_before,
            "minimization replaced normal placement",
        )?;
        restore(&a);
        wait(&mut a, &mut b, |a, b| {
            !flag(a, c"isMiniaturized") && flag(b, c"isVisible")
        })?;
        expect(
            (a.get_position(), b.get_position()) == before,
            "free restore changed either position",
        )?;

        // Restore the saved attachment flag without moving either endpoint;
        // attaching adopts the current offset, including a freely moved pair.
        b.set_dock_attached(true).map_err(|e| e.to_string())?;
        expect(
            (a.get_position(), b.get_position()) == before && saved(&a)?.1 && saved(&b)?.1,
            "restoring attachment moved a window or lost its state",
        )?;
        a.set_position(before.0.0 + 10, before.0.1 + 10);
        expect(
            b.get_position() == (before.1.0 + 10, before.1.1 + 10),
            "restored attachment failed to adopt the current offset",
        )?;
        a.set_position(before.0.0, before.0.1);
        b.set_dock_attached(false).map_err(|e| e.to_string())?;
        expect(
            (saved(&a)?, saved(&b)?) == saved_before,
            "free placement round-trip differs",
        )?;

        // Explicitly disabling magnetism locks the current offset without jump.
        b.set_dock_magnetic(false).map_err(|e| e.to_string())?;
        expect(
            (a.get_position(), b.get_position()) == before,
            "disabling magnetism jumped",
        )?;
        let fixed_offset = offset(&a, &b);
        a.set_position(80, 160);
        expect(
            offset(&a, &b) == fixed_offset,
            "disabling magnetism failed to restore rigid follow",
        )?;
        b.set_dock_magnetic(true).map_err(|e| e.to_string())?;
        free_child(&mut a, &mut b)?;
        b.set_docked_parent(None, 0, 0).map_err(|e| e.to_string())?;
        expect(
            b.set_dock_magnetic(true).is_err() && a.set_dock_magnetic(true).is_err(),
            "explicit detach retained a magnetic pair",
        )?;
        drop(b);
        for cycle in 0..3 {
            let mut child = window(scale)?;
            child
                .set_docked_parent(Some(&mut a), width, 0)
                .map_err(|e| e.to_string())?;
            child.set_dock_magnetic(true).map_err(|e| e.to_string())?;
            free_child(&mut a, &mut child)?;
            if cycle == 0 {
                child.set_dock_attached(true).map_err(|e| e.to_string())?;
            }
            let before_close = (saved(&a)?, saved(&child)?);
            action(&child, c"close");
            expect(
                (saved(&a)?, saved(&child)?) == before_close,
                "native companion close lost an endpoint's restorable placement",
            )?;
            drop(child);
            a.update();
            expect(a.is_open(), "closing a free companion closed the player")?;
            a.set_docked_parent(None, 0, 0).map_err(|e| e.to_string())?;
            expect(
                !saved(&a)?.1,
                "explicit detach kept the old docked snapshot",
            )?;
        }
        let mut survivor = window(scale)?;
        survivor
            .set_docked_parent(Some(&mut a), width, 0)
            .map_err(|e| e.to_string())?;
        survivor
            .set_dock_magnetic(true)
            .map_err(|e| e.to_string())?;
        let before_close = (saved(&a)?, saved(&survivor)?);
        a.minimize();
        wait(&mut a, &mut survivor, |a, b| {
            flag(a, c"isMiniaturized") && !flag(b, c"isVisible")
        })?;
        expect(
            (saved(&a)?, saved(&survivor)?) == before_close,
            "attached minimization changed restorable placement",
        )?;
        action(&a, c"close");
        expect(
            !a.is_open() && survivor.is_open() && (saved(&a)?, saved(&survivor)?) == before_close,
            "native parent close lost placement or closed the companion",
        )?;
    }
    Ok(())
}
