//! One native window pair. HWND properties hold only handles/integers, never
//! Rust pointers, so synchronous move messages do not borrow another WindowState.
use crate::{Error, Result};
use std::{convert::TryFrom, mem, ptr};
use winapi::{shared::windef, um::winuser};

const PARENT: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 112, 0];
const CHILD: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 99, 0];
const X: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 120, 0];
const Y: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 121, 0];
const OLD_OWNER: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 111, 0];
const MOVING: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 109, 0];

fn get(window: windef::HWND, key: &[u16]) -> isize {
    unsafe { winuser::GetPropW(window, key.as_ptr()) as isize }
}

fn set(window: windef::HWND, key: &[u16], value: isize) -> bool {
    unsafe { winuser::SetPropW(window, key.as_ptr(), value as _) != 0 }
}

fn remove(window: windef::HWND, key: &[u16]) {
    unsafe {
        winuser::RemovePropW(window, key.as_ptr());
    }
}

pub(super) fn parent(window: windef::HWND) -> Option<windef::HWND> {
    let parent = get(window, PARENT) as windef::HWND;
    (!parent.is_null() && get(parent, CHILD) == window as isize).then_some(parent)
}

/// Clear the reciprocal links before native calls that could re-enter WndProc.
pub(super) fn detach(window: windef::HWND) {
    let child = get(window, CHILD) as windef::HWND;
    if !child.is_null() && get(child, PARENT) == window as isize {
        detach(child);
    }
    let owner = get(window, PARENT) as windef::HWND;
    let was_attached = !owner.is_null();
    let previous = get(window, OLD_OWNER);
    if was_attached && get(owner, CHILD) == window as isize {
        remove(owner, CHILD);
        remove(owner, MOVING);
    }
    for key in [PARENT, CHILD, X, Y, OLD_OWNER, MOVING] {
        remove(window, key);
    }
    if was_attached && unsafe { winuser::IsWindow(window) } != 0 {
        unsafe {
            set_owner(window, previous);
        }
    }
}

#[cfg(target_pointer_width = "64")]
unsafe fn set_owner(window: windef::HWND, owner: isize) {
    winuser::SetWindowLongPtrW(window, winuser::GWLP_HWNDPARENT, owner);
}
#[cfg(target_pointer_width = "32")]
unsafe fn set_owner(window: windef::HWND, owner: isize) {
    winuser::SetWindowLongW(window, winuser::GWLP_HWNDPARENT, owner as i32);
}

pub(super) fn attach(
    window: windef::HWND,
    owner: Option<windef::HWND>,
    offset_x: isize,
    offset_y: isize,
) -> Result<()> {
    let Some(owner) = owner else {
        detach(window);
        return Ok(());
    };
    if window == owner || unsafe { winuser::IsWindow(window) == 0 || winuser::IsWindow(owner) == 0 }
    {
        return Err(Error::UpdateFailed("Invalid docked window".into()));
    }
    let offset_x = i32::try_from(offset_x)
        .map_err(|_| Error::UpdateFailed("Dock offset is outside screen coordinates".into()))?;
    let offset_y = i32::try_from(offset_y)
        .map_err(|_| Error::UpdateFailed("Dock offset is outside screen coordinates".into()))?;
    if get(window, CHILD) != 0
        || get(owner, PARENT) != 0
        || (get(owner, CHILD) != 0 && get(owner, CHILD) != window as isize)
    {
        return Err(Error::UpdateFailed(
            "Only one docked window pair is supported".into(),
        ));
    }
    detach(window);
    let previous = unsafe { winuser::GetWindow(window, winuser::GW_OWNER) };
    // Native properties remain bounded to this one pair. Zero-valued offsets
    // and guards are valid property values; PARENT/CHILD identify the relation.
    let installed = set(window, OLD_OWNER, previous as isize)
        && set(window, X, offset_x as isize)
        && set(window, Y, offset_y as isize)
        && set(window, MOVING, 1)
        && set(owner, MOVING, 1)
        && set(window, PARENT, owner as isize)
        && set(owner, CHILD, window as isize);
    if !installed {
        detach(window);
        remove(owner, MOVING);
        return Err(Error::UpdateFailed("Cannot attach native windows".into()));
    }
    // An owned top-level window hides with its owner and returns on restore;
    // it remains an independent native surface, not a clipped child control.
    unsafe {
        set_owner(window, owner as isize);
    }
    if unsafe { winuser::GetWindow(window, winuser::GW_OWNER) } != owner {
        detach(window);
        return Err(Error::UpdateFailed(
            "Cannot set the native window owner".into(),
        ));
    }
    set(window, MOVING, 0);
    set(owner, MOVING, 0);
    follow(owner, None);
    Ok(())
}

/// Follow both ordinary SetWindowPos and WM_MOVING inside the native modal loop.
pub(super) fn follow(window: windef::HWND, proposed: Option<&windef::RECT>) {
    if get(window, MOVING) != 0 || unsafe { winuser::IsIconic(window) } != 0 {
        return;
    }
    let (other, child, direction) = if let Some(owner) = parent(window) {
        (owner, window, -1i64)
    } else {
        let child = get(window, CHILD) as windef::HWND;
        if child.is_null() || parent(child) != Some(window) {
            return;
        }
        (child, child, 1i64)
    };
    if unsafe { winuser::IsIconic(other) } != 0 {
        return;
    }
    let mut source: windef::RECT = unsafe { mem::zeroed() };
    if let Some(rect) = proposed {
        source = *rect;
    } else if unsafe { winuser::GetWindowRect(window, &mut source) } == 0 {
        return;
    }
    let x = (source.left as i64 + direction * get(child, X) as i64)
        .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    let y = (source.top as i64 + direction * get(child, Y) as i64)
        .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    let mut current: windef::RECT = unsafe { mem::zeroed() };
    if unsafe { winuser::GetWindowRect(other, &mut current) } == 0
        || (current.left == x && current.top == y)
    {
        return;
    }
    if !set(window, MOVING, 1) {
        return;
    }
    if !set(other, MOVING, 1) {
        set(window, MOVING, 0);
        return;
    }
    unsafe {
        winuser::SetWindowPos(
            other,
            ptr::null_mut(),
            x,
            y,
            0,
            0,
            winuser::SWP_NOACTIVATE
                | winuser::SWP_NOSIZE
                | winuser::SWP_NOZORDER
                | winuser::SWP_NOOWNERZORDER,
        );
    }
    // Calls above can synchronously close/detach a window. Do not recreate
    // properties on a handle whose pair no longer exists (or has been reused).
    let expected_parent = if child == window { other } else { window };
    if parent(child) == Some(expected_parent) {
        set(window, MOVING, 0);
        set(other, MOVING, 0);
    }
}

pub(super) fn work_area(window: windef::HWND) -> Option<(isize, isize, usize, usize)> {
    unsafe {
        let monitor = winuser::MonitorFromWindow(window, winuser::MONITOR_DEFAULTTONEAREST);
        let mut info: winuser::MONITORINFO = mem::zeroed();
        info.cbSize = mem::size_of::<winuser::MONITORINFO>() as u32;
        if monitor.is_null() || winuser::GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
        let r = info.rcWork;
        Some((
            r.left as isize,
            r.top as isize,
            (r.right - r.left).max(0) as usize,
            (r.bottom - r.top).max(0) as usize,
        ))
    }
}
