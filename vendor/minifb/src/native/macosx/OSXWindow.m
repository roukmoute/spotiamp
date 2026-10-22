#import "OSXWindow.h"
#import "OSXWindowFrameView.h"
#include <string.h>

// Pure Rust geometry; scalar coordinates avoid an NSRect ABI boundary.
extern bool mfb_snap_dock_rect(double mx, double my, double mw, double mh,
                              double fx, double fy, double fw, double fh,
                              double distance, double* out_x, double* out_y);

@implementation OSXWindow

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (id)initWithContentRect:(NSRect)contentRect
                styleMask:(NSWindowStyleMask)style
                  backing:(NSBackingStoreType)backingStoreType
                    defer:(BOOL)flag {
    self = [super initWithContentRect:contentRect styleMask:style backing:backingStoreType defer:flag];
    if (self) {
        [[NSNotificationCenter defaultCenter] addObserver:self selector:@selector(dockWindowMoved:)
            name:NSWindowDidMoveNotification object:self];
        [[NSNotificationCenter defaultCenter] addObserver:self selector:@selector(dockWindowRestored:)
            name:NSWindowDidDeminiaturizeNotification object:self];
        // Install keyUp monitor to handle keyUp events when Command is pressed
        __block OSXWindow* blockSelf = self;
        keyUpMonitor = [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskKeyUp
                                                              handler:^NSEvent*(NSEvent* event)
        {
            if ([event modifierFlags] & NSEventModifierFlagCommand)
            {
                if ([event window] == blockSelf)
                    [blockSelf sendEvent:event];
            }
            return event;
        }];
    }
    return self;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)dealloc {
    [self detachDock];
    if (keyUpMonitor) {
        [NSEvent removeMonitor:keyUpMonitor];
        keyUpMonitor = nil;
    }
    
    [[NSNotificationCenter defaultCenter]
        removeObserver:self];
    // Stop new draws before waiting for/releasing the controller's GPU work.
    // The content-view hierarchy owns metal_view; it is released by NSWindow.
    [(MTKView*)metal_view setDelegate:nil];
    if (frame_view) {
        ((OSXWindowFrameView*)frame_view)->m_view_controller = nil;
    }
    [view_controller release];
    view_controller = nil;
    free(draw_parameters);
    free(menu_data);
    [super dealloc];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)setContentSize:(NSSize)newSize {
    NSSize sizeDelta = newSize;
    NSSize childBoundsSize = [childContentView bounds].size;
    sizeDelta.width -= childBoundsSize.width;
    sizeDelta.height -= childBoundsSize.height;

    OSXWindowFrameView *frameView = [super contentView];
    NSSize newFrameSize = [frameView bounds].size;
    newFrameSize.width += sizeDelta.width;
    newFrameSize.height += sizeDelta.height;

    [super setContentSize:newFrameSize];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

-(void)flagsChanged:(NSEvent *)event {
    if (!key_callback) {
        [super flagsChanged:event];
        return;
    }
    const uint32_t flags = [event modifierFlags];

    // Ctrl checking - First check device dependent flags, otherwise fallback to none-device dependent

    if ((flags & NX_DEVICELCTLKEYMASK) || (flags & NX_DEVICERCTLKEYMASK)) {
        key_callback(rust_data, 0x3b, (flags & NX_DEVICELCTLKEYMASK) ? 1 : 0); // Left Ctrl
        key_callback(rust_data, 0x3e, (flags & NX_DEVICERCTLKEYMASK) ? 1 : 0); // Right Ctrl
    } else if (flags & NX_CONTROLMASK) {
        key_callback(rust_data, 0x3b, 1); // Left Ctrl
        key_callback(rust_data, 0x3e, 1); // Right Ctrl
    } else {
        key_callback(rust_data, 0x3b, 0); // Left Ctrl
        key_callback(rust_data, 0x3e, 0); // Right Ctrl
    }

    // Shift checking - First check device dependent flags, otherwise fallback to none-device dependent

    if ((flags & NX_DEVICELSHIFTKEYMASK) || (flags & NX_DEVICERSHIFTKEYMASK)) {
        key_callback(rust_data, 0x38, (flags & NX_DEVICELSHIFTKEYMASK) ? 1 : 0); // Left Shift
        key_callback(rust_data, 0x3c, (flags & NX_DEVICERSHIFTKEYMASK) ? 1 : 0); // Right Shift
    } else if (flags & NX_SHIFTMASK) {
        key_callback(rust_data, 0x38, 1); // Left Shift
        key_callback(rust_data, 0x3c, 1); // Right Shift
    } else {
        key_callback(rust_data, 0x38, 0); // Left Shift
        key_callback(rust_data, 0x3c, 0); // Right Shift
    }

    // Alt checking - First check device dependent flags, otherwise fallback to none-device dependent

    if ((flags & NX_DEVICELALTKEYMASK) || (flags & NX_DEVICERALTKEYMASK)) {
        key_callback(rust_data, 0x3a, (flags & NX_DEVICELALTKEYMASK) ? 1 : 0); // Left Alt
        key_callback(rust_data, 0x3d, (flags & NX_DEVICERALTKEYMASK) ? 1 : 0); // Right Alt
    } else if (flags & NX_ALTERNATEMASK) {
        key_callback(rust_data, 0x3a, 1); // Left Alt
        key_callback(rust_data, 0x3d, 1); // Right Alt
    } else {
        key_callback(rust_data, 0x3a, 0); // Left Alt
        key_callback(rust_data, 0x3d, 0); // Right Alt
    }

    // Cmd checking - First check device dependent flags, otherwise fallback to none-device dependent

    if ((flags & NX_DEVICELCMDKEYMASK) || (flags & NX_DEVICERCMDKEYMASK)) {
        key_callback(rust_data, 0x37, (flags & NX_DEVICELCMDKEYMASK) ? 1 : 0); // Left Cmd
        key_callback(rust_data, 0x36, (flags & NX_DEVICERCMDKEYMASK) ? 1 : 0); // Right Cmd
    } else if (flags & NX_COMMANDMASK) {
        key_callback(rust_data, 0x37, 1); // Left Cmd
        key_callback(rust_data, 0x36, 1); // Right Cmd
    } else {
        key_callback(rust_data, 0x37, 0); // Left Cmd
        key_callback(rust_data, 0x36, 0); // Right Cmd
    }

    [super flagsChanged:event];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)keyDown:(NSEvent *)event {
    if (key_callback) {
        key_callback(rust_data, [event keyCode], 1);
    }

    if (char_callback) {
        NSString* characters = [event characters];
        NSUInteger i, length = [characters length];

        for (i = 0; i < length; i++) {
            char_callback(rust_data, [characters characterAtIndex:i]);
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)keyUp:(NSEvent *)event {
    if (key_callback) {
        key_callback(rust_data, [event keyCode], 0);
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)mainWindowChanged:(NSNotification *)note {
    self->is_active = [[note name] isEqualToString:NSWindowDidBecomeKeyNotification];
    if (!self->is_active) {
        if (focus_lost_callback) {
            focus_lost_callback(rust_data);
        }
        if (shared_data) {
            memset(shared_data->mouse_state, 0, sizeof(shared_data->mouse_state));
        }
        if (mouse_cancel_callback) {
            mouse_cancel_callback(rust_data);
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

// NSWindowDidMove follows movements made by the Window Server. AppKit's
// performWindowDragWithEvent: returns immediately and may omit mouseUp, so the
// existing update loop checks the button only while a magnetic gesture is live.
// Cocoa child windows would apply a second automatic movement to the companion.
- (void)syncDockPosition {
    if (docking_move || should_close) return;
    OSXWindow* peer = docked_parent ? docked_parent : docked_child;
    if (!peer || peer->should_close) return;
    OSXWindow* child = docked_parent ? self : docked_child;
    if (child->dock_magnetic && !child->dock_attached) return;
    NSRect frame = [self frame];
    NSRect target = [peer frame];
    if (docked_parent) {
        target.origin.x = frame.origin.x - dock_offset.x;
        target.origin.y = NSMaxY(frame) + dock_offset.y - target.size.height;
    } else {
        target.origin.x = frame.origin.x + peer->dock_offset.x;
        target.origin.y = NSMaxY(frame) - peer->dock_offset.y - target.size.height;
    }
    if (NSEqualPoints([peer frame].origin, target.origin)) return;
    docking_move = true;
    peer->docking_move = true;
    [peer setFrameOrigin:target.origin];
    peer->docking_move = false;
    docking_move = false;
}

- (void)dockWindowMoved:(NSNotification*)notification {
    (void)notification;
    if (docking_move || should_close) return;
    if (dock_drag_active && !NSEqualPoints(dock_drag_origin, [self frame].origin)) {
        dock_drag_moved = true;
        // Only a real companion movement separates the pair. A click in its
        // title strip, or a programmatic reposition, keeps the rigid relation.
        if (docked_parent && dock_magnetic) dock_attached = false;
    }
    [self syncDockPosition];
}

- (void)beginDockDrag {
    [self finishDockDrag];
    // A new mouseDown proves any previous drag has ended, even when AppKit
    // omitted its mouseUp and this window's next update has not run yet.
    OSXWindow* peer = docked_parent ? docked_parent : docked_child;
    if (peer && peer->dock_drag_active) [peer finishDockDrag];
    OSXWindow* child = docked_parent ? self : docked_child;
    if (should_close || !child || child->should_close || !child->dock_magnetic) return;
    dock_drag_origin = [self frame].origin;
    dock_drag_moved = false;
    dock_drag_active = true;
}

- (void)finishDockDrag {
    if (!dock_drag_active) return;
    dock_drag_active = false;
    if (!dock_drag_moved || should_close) return;
    OSXWindow* owner = docked_parent ? docked_parent : self;
    OSXWindow* child = owner->docked_child;
    if (!child || owner->should_close || child->should_close ||
        !child->dock_magnetic || child->dock_attached) return;
    OSXWindow* peer = docked_parent ? owner : child;
    const NSRect moving = [self frame];
    const NSRect fixed = [peer frame];
    double x, y;
    // Negated Cocoa top edges form one signed top-left coordinate space. The
    // common screen-origin translation cancels out of proximity and offsets.
    if (!mfb_snap_dock_rect(moving.origin.x, -NSMaxY(moving),
                           moving.size.width, moving.size.height,
                           fixed.origin.x, -NSMaxY(fixed),
                           fixed.size.width, fixed.size.height,
                           12.0 * (scale > 0 ? scale : 1), &x, &y)) return;
    docking_move = true;
    peer->docking_move = true;
    [self setFrameOrigin:NSMakePoint(x, -y - moving.size.height)];
    peer->docking_move = false;
    docking_move = false;
    const NSRect parentFrame = [owner frame];
    const NSRect childFrame = [child frame];
    child->dock_offset = NSMakePoint(childFrame.origin.x - parentFrame.origin.x,
                                    NSMaxY(parentFrame) - NSMaxY(childFrame));
    child->dock_attached = true;
}

- (void)updateDockDrag {
    if (dock_drag_active && ([NSEvent pressedMouseButtons] & 1) == 0) {
        [self finishDockDrag];
    }
}

- (void)miniaturize:(id)sender {
    if (docked_parent) {
        [docked_parent miniaturize:sender];
        return;
    }
    if (docked_child && [docked_child isVisible]) {
        dock_child_hidden = true;
        [docked_child orderOut:nil];
    }
    [super miniaturize:sender];
}

- (void)dockWindowRestored:(NSNotification*)notification {
    (void)notification;
    if (dock_child_hidden && docked_child && !docked_child->should_close) {
        dock_child_hidden = false;
        [self syncDockPosition];
        [docked_child orderFront:nil];
    }
}

- (void)detachDock {
    dock_drag_active = false;
    dock_drag_moved = false;
    if (docked_parent) {
        docked_parent->dock_drag_active = false;
        const bool wasHidden = docked_parent->dock_child_hidden;
        docked_parent->docked_child = nil;
        docked_parent->dock_child_hidden = false;
        docked_parent = nil;
        if (wasHidden && !should_close) [self orderFront:nil];
    }
    if (docked_child) {
        docked_child->dock_drag_active = false;
        docked_child->dock_magnetic = false;
        docked_child->dock_attached = false;
        docked_child->docked_parent = nil;
        // Closing or detaching the parent must not strand a surviving hidden
        // companion. orderFront does not activate the app or steal key focus.
        if (dock_child_hidden && !docked_child->should_close) [docked_child orderFront:nil];
        docked_child = nil;
    }
    dock_child_hidden = false;
    dock_magnetic = false;
    dock_attached = false;
}

- (void)windowWillClose:(NSNotification *)notification {
    (void)notification;
    should_close = true;
}

// NSWindow does not call delegate methods implemented only by its subclass.
// Track both the native close action and an explicit close without a delegate.
- (void)close {
    should_close = true;
    [self detachDock];
    [super close];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)windowShouldClose:(id)sender {
    (void)sender;
    should_close = true;
    return TRUE;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)setContentView:(NSView *)aView {
    if ([childContentView isEqualTo:aView]) {
        return;
    }

    NSRect bounds = [self frame];
    bounds.origin = NSZeroPoint;

    OSXWindowFrameView* frameView = [super contentView];
    if (!frameView) {
        [[NSNotificationCenter defaultCenter]
            addObserver:self
            selector:@selector(mainWindowChanged:)
            name:NSWindowDidBecomeKeyNotification
            object:self];

        [[NSNotificationCenter defaultCenter]
            addObserver:self
            selector:@selector(mainWindowChanged:)
            name:NSWindowDidResignKeyNotification
            object:self];

        frameView = [[[OSXWindowFrameView alloc] initWithFrame:bounds] autorelease];

        [super setContentView:frameView];
    }

    frame_view = frameView;

    if (childContentView) {
        [childContentView removeFromSuperview];
    }

    //printf("osxwindow: setContentFrameView %p\n", frameView);
    //printf("osxwindow: setting controller %p\n", view_controller);
    //frameView->m_view_controller = view_controller;

    //NSRect t = [self contentRectForFrameRect:bounds];

    childContentView = aView;
    [childContentView setFrame:[self contentRectForFrameRect:bounds]];
    [childContentView setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
    [frameView addSubview:childContentView];
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (NSView *)contentView {
    return childContentView;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)canBecomeKeyWindow {
    return YES;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (BOOL)canBecomeMainWindow {
    return YES;
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (NSRect)contentRectForFrameRect:(NSRect)windowFrame {
    windowFrame.origin = NSZeroPoint;
    return NSInsetRect(windowFrame, 0, 0);
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)updateSize {
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

- (void)onMenuPress:(id)sender {
    int menu_id = (int)((NSButton*)sender).tag;
    self->active_menu_id = menu_id;
}

@end
