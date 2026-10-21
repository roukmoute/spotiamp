# Local patch to minifb 0.29.0

Spotiamp carries the following fixes in its bundled minifb backend.

## X11 window initialization

`DisplayInfo::setup` previously retained the display-owned `XDefaultGC`.
If window creation failed before replacing it with `XCreateGC` (for example,
`XOpenIM` failed because an input-method server was unavailable), `Drop` freed
that borrowed GC and then `XCloseDisplay` freed it again. A minimal reproduction
crashed before `Window::new` returned; GDB showed `_XFreeExtData → XFreeGC →
XCloseDisplay → DisplayInfo::drop`.

Initialize the GC pointer to null and free it only when non-null. Successful
window creation still assigns its owned `XCreateGC` result. An unavailable input
method now returns the intended window-creation error instead of a segmentation
fault. The application does not overwrite users' input-method settings.

For an isolated Xvfb test server without IBus, use `XMODIFIERS=@im=none` in the
**test environment**. Also test with a deliberately unavailable input method to
verify that creation fails cleanly. Both paths were checked with a standalone
minifb reproduction and Spotiamp's interface-only build on Linux x86_64.

## Wheel input

- Windows (`src/os/windows/mod.rs`): normalize `WM_MOUSEWHEEL` by
  `WHEEL_DELTA` (120) instead of multiplying by 0.1. One mouse detent now
  produces 1.0 rather than 12.0. Accumulate all messages in an update instead
  of retaining only the last one, including high-resolution fractional input.
- X11 (`src/os/posix/x11.rs`): count wheel button presses only. Releases no
  longer count a second time; ordinary mouse button releases still work.
- macOS (`src/native/macosx/OSXWindowFrameView.m`): accumulate native deltas
  within an update instead of overwriting previous events. Native sensitivity
  is preserved.

The application consumes fractional deltas across frames and moves one queue
row per complete unit. Its Windows regression test posts native wheel messages
to a minifb window to exercise this conversion and event accumulation.

Windows wheel semantics: https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mousewheel

## Mouse-button events and interaction cancellation

Two default-empty `InputCallback` methods preserve compatibility with existing
callback implementations:

- `set_mouse_button_state(button, down, x, y)` delivers each native button
  transition on Windows, X11 and macOS. Coordinates are taken from that event,
  divided by the window scale, and remain signed/unclamped outside the window.
  A press and release received in one update are both delivered, even though
  `get_mouse_down` only exposes the final state.
- `mouse_input_cancelled()` reports loss of focus or capture. Applications must
  cancel active interactions without activating the control as a release would.

Windows captures the mouse during a button interaction to receive releases
outside the client area, releases capture when all buttons are up, and cancels
on `WM_CAPTURECHANGED`, `WM_CANCELMODE` or `WM_KILLFOCUS`. References to mutable
window state are no longer used when the capture API is called, because it can
synchronously re-enter the window procedure. Normal `ReleaseCapture` can also
emit a cancellation after the button-up callback.

X11 retains the server's implicit button grab and emits cancellation on
`FocusOut`. Wheel button handling remains separate from ordinary button events.

macOS forwards native mouse-event coordinates through a small C callback bridge
and registers data/callbacks before pumping the first events. Key-window loss
clears mouse state and emits cancellation. Active-window reporting now tests
the key window and active application, and the Rust wrapper no longer inverts
that result. Native middle-button events are mapped to the existing middle
button slot.

The callbacks add no event buffer or dependencies; Spotiamp owns a bounded
callback queue. Its native Windows/X11 tests exercise same-update press/release
delivery and coordinate scaling. macOS execution requires the macOS CI runner.

Native behavior references:
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setcapture
- https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-capturechanged
- https://www.x.org/Development/Documentation/GrabProcessing/
- https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/HandlingMouseEvents/HandlingMouseEvents.html

Upstream source: https://crates.io/crates/minifb/0.29.0
Original licenses are preserved in this directory.
