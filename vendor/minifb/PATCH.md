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

Upstream source: https://crates.io/crates/minifb/0.29.0
Original licenses are preserved in this directory.
