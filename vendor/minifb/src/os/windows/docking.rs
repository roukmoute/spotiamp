//! One native window pair. HWND properties hold only handles/integers, never
//! Rust pointers, so synchronous move messages do not borrow another WindowState.
use crate::{Error, RestorablePlacement, Result};
use std::{convert::TryFrom, mem, ptr};
use winapi::{shared::windef, um::winuser};

const PARENT: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 112, 0];
const CHILD: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 99, 0];
const X: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 120, 0];
const Y: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 121, 0];
const OLD_OWNER: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 111, 0];
const MOVING: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 109, 0];
const MAGNETIC: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 103, 0];
const ATTACHED: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 97, 0];
const DISTANCE: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 100, 0];
const DRAGGED: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 114, 0];
// Kept after a native peer closure: 1 = separate, 2 = attached. Native window
// destruction frees the property; no heap pointer or cross-window Rust borrow.
const LAST_DOCKED: &[u16] = &[109, 102, 98, 46, 100, 111, 99, 107, 46, 108, 0];

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

fn pair(window: windef::HWND) -> Option<(windef::HWND, windef::HWND)> {
    if let Some(owner) = parent(window) {
        Some((owner, window))
    } else {
        let child = get(window, CHILD) as windef::HWND;
        (!child.is_null() && parent(child) == Some(window)).then_some((window, child))
    }
}

/// Clear the reciprocal links before native calls that could re-enter WndProc.
pub(super) fn detach(window: windef::HWND, preserve_placement: bool) {
    if let Some((owner, child)) = pair(window) {
        let docked = preserve_placement && get(child, ATTACHED) != 0;
        set(owner, LAST_DOCKED, 1 + isize::from(docked));
        set(child, LAST_DOCKED, 1 + isize::from(docked));
    } else if !preserve_placement {
        set(window, LAST_DOCKED, 1);
    }
    let child = get(window, CHILD) as windef::HWND;
    if !child.is_null() && get(child, PARENT) == window as isize {
        detach(child, preserve_placement);
    }
    let owner = get(window, PARENT) as windef::HWND;
    let was_attached = !owner.is_null();
    let previous = get(window, OLD_OWNER);
    if was_attached && get(owner, CHILD) == window as isize {
        remove(owner, CHILD);
        remove(owner, MOVING);
        remove(owner, DRAGGED);
    }
    for key in [
        PARENT, CHILD, X, Y, OLD_OWNER, MOVING, MAGNETIC, ATTACHED, DISTANCE, DRAGGED,
    ] {
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
        detach(window, false);
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
    detach(window, false);
    let previous = unsafe { winuser::GetWindow(window, winuser::GW_OWNER) };
    // Native properties remain bounded to this one pair. Zero-valued offsets
    // and guards are valid property values; PARENT/CHILD identify the relation.
    let installed = set(window, OLD_OWNER, previous as isize)
        && set(window, X, offset_x as isize)
        && set(window, Y, offset_y as isize)
        && set(window, ATTACHED, 1)
        && set(window, MOVING, 1)
        && set(owner, MOVING, 1)
        && set(window, PARENT, owner as isize)
        && set(owner, CHILD, window as isize);
    if !installed {
        detach(window, false);
        remove(owner, MOVING);
        return Err(Error::UpdateFailed("Cannot attach native windows".into()));
    }
    // An owned top-level window hides with its owner and returns on restore;
    // it remains an independent native surface, not a clipped child control.
    unsafe {
        set_owner(window, owner as isize);
    }
    if unsafe { winuser::GetWindow(window, winuser::GW_OWNER) } != owner {
        detach(window, false);
        return Err(Error::UpdateFailed(
            "Cannot set the native window owner".into(),
        ));
    }
    set(window, MOVING, 0);
    set(owner, MOVING, 0);
    follow(owner, None);
    Ok(())
}

pub(super) fn magnetic(window: windef::HWND, enabled: bool, distance: i32) -> Result<()> {
    let (owner, child) = pair(window).ok_or_else(|| {
        Error::UpdateFailed("Attach a window pair before enabling magnets".into())
    })?;
    if !enabled && get(child, ATTACHED) == 0 {
        // Returning to rigid mode must not jump back to the old dock offset.
        if unsafe { winuser::IsIconic(owner) != 0 || winuser::IsIconic(child) != 0 } {
            return Err(Error::UpdateFailed(
                "Cannot restore rigid docking while windows are minimized".into(),
            ));
        }
        if !remember_offset(owner, child) || !set(child, ATTACHED, 1) {
            return Err(Error::UpdateFailed("Cannot restore rigid docking".into()));
        }
    }
    if !set(child, DISTANCE, distance.max(0) as isize)
        || !set(child, MAGNETIC, isize::from(enabled))
    {
        return Err(Error::UpdateFailed(
            "Cannot configure window magnets".into(),
        ));
    }
    remove(owner, DRAGGED);
    remove(child, DRAGGED);
    Ok(())
}

pub(super) fn set_attached(window: windef::HWND, attached: bool) -> Result<()> {
    let (owner, child) = pair(window)
        .filter(|(_, child)| get(*child, MAGNETIC) != 0)
        .ok_or_else(|| Error::UpdateFailed("A magnetic window pair is required".into()))?;
    if attached {
        if unsafe { winuser::IsIconic(owner) != 0 || winuser::IsIconic(child) != 0 } {
            return Err(Error::UpdateFailed(
                "Cannot attach windows while minimized".into(),
            ));
        }
        if !remember_offset(owner, child) {
            return Err(Error::UpdateFailed("Cannot read the dock offset".into()));
        }
    }
    if !set(child, ATTACHED, isize::from(attached)) {
        return Err(Error::UpdateFailed(
            "Cannot restore window attachment".into(),
        ));
    }
    remove(owner, DRAGGED);
    remove(child, DRAGGED);
    Ok(())
}

fn restorable_docked(window: windef::HWND) -> bool {
    pair(window)
        .map(|(_, child)| get(child, ATTACHED) != 0)
        .unwrap_or_else(|| get(window, LAST_DOCKED) == 2)
}

pub(super) fn placement(
    window: windef::HWND,
    cached: Option<RestorablePlacement>,
) -> Option<RestorablePlacement> {
    let current = (unsafe { winuser::IsIconic(window) } == 0)
        .then(|| rect(window))
        .flatten()
        .map(|rect| (rect.left as isize, rect.top as isize))
        .filter(|position| *position != (-32000, -32000));
    Some(RestorablePlacement {
        position: current.or_else(|| cached.map(|placement| placement.position))?,
        docked: restorable_docked(window),
    })
}

fn rect(window: windef::HWND) -> Option<windef::RECT> {
    let mut result = unsafe { mem::zeroed() };
    (unsafe { winuser::GetWindowRect(window, &mut result) } != 0).then_some(result)
}

fn remember_offset(owner: windef::HWND, child: windef::HWND) -> bool {
    let (Some(owner_rect), Some(child_rect)) = (rect(owner), rect(child)) else {
        return false;
    };
    let x = isize::try_from(child_rect.left as i64 - owner_rect.left as i64);
    let y = isize::try_from(child_rect.top as i64 - owner_rect.top as i64);
    match (x, y) {
        (Ok(x), Ok(y)) => set(child, X, x) && set(child, Y, y),
        _ => false,
    }
}

pub(super) fn begin_drag(window: windef::HWND) {
    remove(window, DRAGGED);
}

/// Unlike WM_ENTERSIZEMOVE, an actually changed WM_MOVING rectangle proves a
/// drag occurred. A simple click on the titlebar must leave the pair attached.
pub(super) fn moving(window: windef::HWND, proposed: &windef::RECT) {
    if let Some((_, child)) = pair(window) {
        if get(child, MAGNETIC) != 0 && get(window, MOVING) == 0 {
            if let Some(current) = rect(window) {
                if current.left != proposed.left || current.top != proposed.top {
                    set(window, DRAGGED, 1);
                    if window == child {
                        remove(child, ATTACHED);
                    }
                }
            }
        }
    }
    follow(window, Some(proposed));
}

/// Snap only at the end of a native drag. Reciprocal links and the native owner
/// remain present while free, so minimization and destruction still group safely.
pub(super) fn end_drag(window: windef::HWND) {
    let dragged = get(window, DRAGGED) != 0;
    remove(window, DRAGGED);
    let Some((owner, child)) = pair(window) else {
        return;
    };
    if !dragged || get(child, MAGNETIC) == 0 || get(child, ATTACHED) != 0 {
        return;
    }
    let other = if window == child { owner } else { child };
    if unsafe { winuser::IsIconic(window) != 0 || winuser::IsIconic(other) != 0 } {
        return;
    }
    let (Some(moving), Some(stationary)) = (rect(window), rect(other)) else {
        return;
    };
    let to_rect = |r: windef::RECT| crate::magnet::Rect {
        x: r.left as f64,
        y: r.top as f64,
        width: r.right as f64 - r.left as f64,
        height: r.bottom as f64 - r.top as f64,
    };
    let Some((x, y)) = crate::magnet::snap(
        to_rect(moving),
        to_rect(stationary),
        get(child, DISTANCE) as f64,
    ) else {
        return;
    };
    if !set(window, MOVING, 1) {
        return;
    }
    if !set(other, MOVING, 1) {
        set(window, MOVING, 0);
        return;
    }
    let moved = unsafe {
        winuser::SetWindowPos(
            window,
            ptr::null_mut(),
            x.round() as i32,
            y.round() as i32,
            0,
            0,
            winuser::SWP_NOACTIVATE
                | winuser::SWP_NOSIZE
                | winuser::SWP_NOZORDER
                | winuser::SWP_NOOWNERZORDER,
        )
    } != 0;
    if pair(window) != Some((owner, child)) {
        return;
    }
    set(window, MOVING, 0);
    set(other, MOVING, 0);
    if moved && remember_offset(owner, child) {
        set(child, ATTACHED, 1);
    }
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
    if get(child, ATTACHED) == 0 {
        return;
    }
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
    let monitor = unsafe { winuser::MonitorFromWindow(window, winuser::MONITOR_DEFAULTTONEAREST) };
    monitor_work_area(monitor)
}

pub(super) fn work_area_at(x: isize, y: isize) -> Option<(isize, isize, usize, usize)> {
    let point = windef::POINT {
        x: i32::try_from(x).ok()?,
        y: i32::try_from(y).ok()?,
    };
    let monitor = unsafe { winuser::MonitorFromPoint(point, winuser::MONITOR_DEFAULTTONEAREST) };
    monitor_work_area(monitor)
}

fn monitor_work_area(monitor: windef::HMONITOR) -> Option<(isize, isize, usize, usize)> {
    unsafe {
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
