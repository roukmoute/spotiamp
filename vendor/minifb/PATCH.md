# Local patch to minifb 0.29.0

Spotiamp carries the following fixes in its bundled minifb backend.

## Multiple windows: keyboard focus and native resource ownership

`KeyHandler::release_all` sends one key-up callback for each held key and
immediately resets repeat durations. Windows calls it on `WM_KILLFOCUS`, X11
on `FocusOut`, and macOS through a separate `DidResignKey` callback. It is not
called on ordinary mouse-capture/menu cancellation. This prevents Ctrl, F3 or
arrow keys remaining held when their physical release reaches another window.
The unit test checks callback delivery, repeat cancellation, idempotence and
the next key-down.

macOS registers the boxed Rust state immediately after window construction:
another window's global event pump can dispatch its events before its first
update. Closing clears every native callback/data pointer before releasing
the owned `NSWindow`. `setReleasedWhenClosed:NO` remains in force, so a prior
native close cannot release the window underneath Rust. The `close` override
records that closure directly; subclass methods named `windowWillClose` were
not registered as a delegate or notification observer.

Each Mac window now owns its Metal controller. The view hierarchy owns the
Metal view, and temporary texture descriptors and tracking areas are released.
Controller destruction first waits for all three in-flight command buffers,
then releases the textures, vertex buffers and semaphore. Failure paths before
command submission return their semaphore permit too. The command queue is
created once and shared; repeatedly opening a window no longer replaces and
leaks it. Window-owned C drawing/menu data is freed after the controller stops.
Windows also deletes its owned background brush after destroying its DC/window.

Spotiamp's native tests switch focus between two real windows while Ctrl/F3/Up
are held, release them in the other window, and verify fresh text/shortcuts and
no stale repeats. Twenty open/render/native-close/drop cycles leave the parent
open. Windows checks GDI counts with all HWND tests serialized; X11 checks open
file descriptors. The macOS main-thread smoke additionally uses zeroing weak
references to assert that closed windows and Metal controllers are destroyed.
These are lifecycle checks, not playback-memory benchmarks. Native macOS and
Windows execution still requires their respective CI runners.

Ownership references:
- https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/MemoryMgmt/Articles/mmRules.html
- https://developer.apple.com/documentation/appkit/nswindow/isreleasedwhenclosed
- https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-deleteobject

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

## X11 idle-window repaint

An application that renders only when its state changes must still recover its
pixels after the window manager exposes/restores the window. X11 now selects
`ExposureMask` and re-presents its existing scaled `XImage` on the final event
of an Expose batch. This path uses `XPutImage` and flushes the connection; it
does not resample the application buffer, allocate another image or trigger a
continuous render loop. `ConfigureNotify` retains the image when only position
or stacking changes; reallocation occurs only for a real dimension change.

The native regression draws one frame at scales 1 and 2, moves the window,
erases its native drawable with `XClearArea` requesting exposures, then pumps
event-only `update()` calls. `XGetImage` verifies that the pixels were erased
and that the cached color returns without another application frame.

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

## Skin title bar and native window controls

Two small public methods support a framebuffer-drawn title bar:
`set_titlebar_drag_region(Some((x, y, width, height)))` defines the draggable
rectangle in logical buffer pixels, before minifb scaling; `None` disables it.
`minimize()` asks the native window manager to minimize the window. Applications
keep their buttons outside the drag rectangle. No renderer, event thread or
additional dependency is introduced.

Windows retains the caption/system-menu/minimize style bits required for native
window commands, but `WM_NCCALCSIZE` gives the framebuffer the entire window:
there is no visible system title bar or border. The heap state pointer is
installed and `SWP_FRAMECHANGED` applied before the window is shown, preventing
an initial native-frame flash. A fixed borderless window has no resize or
maximize styles, and caption double-click does not maximize it.
`WM_GETMINMAXINFO` supplies the requested fixed framebuffer dimensions instead
of inheriting desktop-sized tracking limits; an explicit scale-2 window is not
silently reduced on a small display. These limits are stored independently of
the current `WM_SIZE`, which can report a minimized or initially clamped size. The window
remains an ordinary unowned top-level window eligible for the taskbar.

`WM_NCHITTEST` decodes signed screen coordinates, converts them to client
coordinates with `ScreenToClient`, then divides by minifb's scale. Only the
specified rectangle returns `HTCAPTION`; skin buttons remain `HTCLIENT`.
Windows therefore owns the drag operation. `ShowWindow(SW_MINIMIZE)` implements
minimization. System key messages reach `DefWindowProc`, including Alt+F4 and
Alt+Space; their Space/F4 key callbacks and the system-space character are
suppressed so opening the system menu cannot toggle playback or type a space.
After cancelling application capture, `WM_CANCELMODE` also reaches the default
procedure so Windows can end its internal menu/move processing.

X11 delegates movement to the window manager through `_NET_WM_MOVERESIZE`,
releasing pointer/keyboard grabs before requesting the move. Title-bar presses
and releases are kept out of application button callbacks; a locally received
short release cancels an unfinished move. `XIconifyWindow` requests minimization.
macOS uses `performWindowDragWithEvent:` and `miniaturize:` with the appropriate
window style. Its pointer polling now converts coordinates through the content
view, matching button callbacks without assuming a native titled frame. A drag
that does not deliver MouseUp is reset on the next Down. Wayland's unused drag
API is explicitly a no-op; minimization uses `xdg_toplevel.set_minimized`.

Spotiamp's Windows tests use real HWNDs to check the complete client rectangle,
styles, signed-coordinate hit testing at scales 1 and 2, excluded controls,
drag disabling, fixed-size double-click, minimize/restore, the actual system
menu and Alt+F4, pumping the message queue until native closure (which may be
posted asynchronously). They also check that system shortcuts do not become application
input while ordinary keys still do. These tests run as part of the application's
normal Windows test suite. Native macOS behavior requires a macOS runner;
compiling its Objective-C bridge alone does not establish interaction behavior.

Native behavior references:
- https://learn.microsoft.com/en-us/windows/win32/dwm/customframe
- https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest
- https://learn.microsoft.com/en-us/windows/win32/winmsg/window-styles
- https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-getminmaxinfo
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-minmaxinfo
- https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-syskeydown
- https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-cancelmode
- https://specifications.freedesktop.org/wm/latest/ar01s04.html
- https://developer.apple.com/documentation/appkit/nswindow/performdrag(with:)
- https://developer.apple.com/documentation/appkit/nswindow/miniaturize(_:)

Upstream source: https://crates.io/crates/minifb/0.29.0
Original licenses are preserved in this directory.
