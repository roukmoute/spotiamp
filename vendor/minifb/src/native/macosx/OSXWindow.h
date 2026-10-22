#import <Cocoa/Cocoa.h>
#include "shared_data.h"

#define MAX_MENUS 512

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct Menu {
    const char* name;
    NSMenu* menu;
    NSMenuItem* menu_item;
} Menu;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct MenuData {
    Menu menus[MAX_MENUS];
    int menu_count;
} MenuData;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef struct MenuDesc {
    char name[512];
    struct MenuDesc* sub_menu;
    int menu_id;
    int key;
    int special_key;
    int modifier;
    int modifier_mac;
    int enabled;
} MenuDesc;

void build_submenu(NSMenu* menu, MenuDesc* desc);

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

@interface OSXWindow : NSWindow {
    NSView* childContentView;
    @public void (*key_callback)(void* user_data, int key, int state);
    @public void (*char_callback)(void* user_data, unsigned int key);
    @public void (*mouse_button_callback)(void* user_data, int button, int state, float x, float y);
    @public void (*mouse_cancel_callback)(void* user_data);
    @public void (*focus_lost_callback)(void* user_data);
    @public float width;
    @public float height;
    @public int scale;
    @public DrawParameters* draw_parameters;
    @public void* rust_data;
    @public SharedData* shared_data;
    @public bool should_close;
    @public bool is_active;
    // Top-left logical coordinates, before the explicit minifb scale.
    @public NSRect titlebar_drag_region;
    @public bool titlebar_drag_enabled;
    @public bool titlebar_dragging;
    // Non-owning pair. close/dealloc clears BOTH endpoints before either
    // NSWindow can be released; Rust owns each native window independently.
    @public OSXWindow* docked_parent;
    @public OSXWindow* docked_child;
    @public NSPoint dock_offset;
    @public bool docking_move;
    @public bool dock_child_hidden;
    // Pair ownership survives magnetic separation; only the companion owns
    // magnetic/attached. Drag state belongs to the window being dragged.
    @public bool dock_magnetic;
    @public bool dock_attached;
    @public bool dock_drag_active;
    @public bool dock_drag_moved;
    @public NSPoint dock_drag_origin;
    @public int active_menu_id;
    @public int prev_cursor;
    @public MenuData* menu_data;
    @public void* frame_view;
    @public void* metal_view;
    // Owns the +1 returned by [WindowViewController new]. MTKView's delegate
    // and OSXWindowFrameView's pointer are borrowed references.
    @public id view_controller;
    @public id keyUpMonitor;
}
- (void)detachDock;
- (void)syncDockPosition;
- (void)beginDockDrag;
- (void)finishDockDrag;
- (void)updateDockDrag;
@end
