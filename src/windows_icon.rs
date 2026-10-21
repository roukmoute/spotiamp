//! Use the executable's icon resource for each native window, without loading
//! files or decoding images in the player. Two private HICONs live with a window.
use minifb::Window;
use std::{ffi::c_void, ptr};

type Handle = *mut c_void;
const RESOURCE_ID: usize = 1; // winresource::set_icon in build.rs
const IMAGE_ICON: u32 = 1;
const WM_SETICON: u32 = 0x0080;
const WM_GETICON: u32 = 0x007f;
const ICON_SMALL: usize = 0;
const ICON_BIG: usize = 1;
const SM_CXICON: i32 = 11;
const SM_CYICON: i32 = 12;
const SM_CXSMICON: i32 = 49;
const SM_CYSMICON: i32 = 50;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn LoadImageW(
        instance: Handle,
        name: *const u16,
        kind: u32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> Handle;
    fn DestroyIcon(icon: Handle) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn SendMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
    fn IsWindow(window: Handle) -> i32;
}

struct OwnedIcon(Handle);
impl OwnedIcon {
    fn load(module: Handle, width: i32, height: i32) -> Result<Self, &'static str> {
        if module.is_null() || width <= 0 || height <= 0 {
            return Err("Icone Windows indisponible");
        }
        // MAKEINTRESOURCE(1), no LR_LOADFROMFILE and no LR_SHARED. The latter
        // caches only by resource name and can return the wrong icon size.
        let icon = unsafe {
            LoadImageW(
                module,
                RESOURCE_ID as *const u16,
                IMAGE_ICON,
                width,
                height,
                0,
            )
        };
        if icon.is_null() {
            Err("Icone Windows indisponible")
        } else {
            Ok(Self(icon))
        }
    }
}
impl Drop for OwnedIcon {
    fn drop(&mut self) {
        unsafe { DestroyIcon(self.0) };
    }
}

pub(crate) struct WindowIcons {
    window: Handle,
    big: OwnedIcon,
    small: OwnedIcon,
}

impl WindowIcons {
    /// The guard must be retained while the window uses these icons. Loading
    /// either size may fail without preventing the application's window opening.
    pub(crate) fn attach(window: &Window) -> Result<Self, &'static str> {
        let module = unsafe { GetModuleHandleW(ptr::null()) };
        // Follow the application's existing DPI context. Do not change process
        // awareness (and skin geometry) or require newer Windows 10 APIs here.
        let big = OwnedIcon::load(module, unsafe { GetSystemMetrics(SM_CXICON) }, unsafe {
            GetSystemMetrics(SM_CYICON)
        })?;
        let small = OwnedIcon::load(module, unsafe { GetSystemMetrics(SM_CXSMICON) }, unsafe {
            GetSystemMetrics(SM_CYSMICON)
        })?;
        let icons = Self {
            window: window.get_window_handle(),
            big,
            small,
        };
        unsafe {
            SendMessageW(icons.window, WM_SETICON, ICON_BIG, icons.big.0 as isize);
            SendMessageW(icons.window, WM_SETICON, ICON_SMALL, icons.small.0 as isize);
        }
        Ok(icons)
    }
}

impl Drop for WindowIcons {
    fn drop(&mut self) {
        // Locals may be dropped before their Window, whereas LibraryWindow
        // drops its Window field first. Both orders are safe. Do not detach an
        // icon installed by a later owner, or send to an already destroyed HWND.
        if unsafe { IsWindow(self.window) } != 0 {
            for (kind, icon) in [(ICON_BIG, self.big.0), (ICON_SMALL, self.small.0)] {
                if unsafe { SendMessageW(self.window, WM_GETICON, kind, 0) } == icon as isize {
                    unsafe { SendMessageW(self.window, WM_SETICON, kind, 0) };
                }
            }
        }
        // OwnedIcon fields are destroyed only after the detachment above.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minifb::WindowOptions;

    #[repr(C)]
    #[derive(Default)]
    struct IconInfo {
        is_icon: i32,
        hotspot_x: u32,
        hotspot_y: u32,
        mask: Handle,
        color: Handle,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Bitmap {
        kind: i32,
        width: i32,
        height: i32,
        width_bytes: i32,
        planes: u16,
        bits_pixel: u16,
        bits: Handle,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetIconInfo(icon: Handle, info: *mut IconInfo) -> i32;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn GetObjectW(object: Handle, size: i32, data: *mut c_void) -> i32;
        fn DeleteObject(object: Handle) -> i32;
    }

    fn dimensions(icon: Handle) -> Option<(i32, i32)> {
        let mut info = IconInfo::default();
        if unsafe { GetIconInfo(icon, &mut info) } == 0 {
            return None;
        }
        let mut bitmap = Bitmap::default();
        let size = std::mem::size_of::<Bitmap>() as i32;
        let read = unsafe { GetObjectW(info.color, size, (&mut bitmap as *mut Bitmap).cast()) };
        // GetIconInfo allocates both bitmap copies, even for a shared icon.
        unsafe {
            DeleteObject(info.color);
            DeleteObject(info.mask);
        }
        (read == size).then_some((bitmap.width, bitmap.height))
    }

    #[test]
    fn native_resource_icons_have_system_sizes_and_are_detached_before_freeing() {
        let _serial = crate::window_native_windows_tests::WINDOWS
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let window = Window::new(
            "Spotiamp icon ownership test",
            32,
            32,
            WindowOptions::default(),
        )
        .expect("Windows CI must allow native windows");
        for _ in 0..20 {
            let icons = WindowIcons::attach(&window).expect("build.rs must embed icon resource 1");
            let big = icons.big.0;
            let small = icons.small.0;
            assert_ne!(big, small);
            assert_eq!(
                unsafe { SendMessageW(window.get_window_handle(), WM_GETICON, ICON_BIG, 0) },
                big as isize
            );
            assert_eq!(
                unsafe { SendMessageW(window.get_window_handle(), WM_GETICON, ICON_SMALL, 0) },
                small as isize
            );
            assert_eq!(
                dimensions(big),
                Some(unsafe { (GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON)) })
            );
            assert_eq!(
                dimensions(small),
                Some(unsafe { (GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)) })
            );
            drop(icons);
            assert_eq!(
                unsafe { SendMessageW(window.get_window_handle(), WM_GETICON, ICON_BIG, 0) },
                0
            );
            assert_eq!(
                unsafe { SendMessageW(window.get_window_handle(), WM_GETICON, ICON_SMALL, 0) },
                0
            );
            assert_eq!(dimensions(big), None, "the owned large HICON must be freed");
            assert_eq!(
                dimensions(small),
                None,
                "the owned small HICON must be freed"
            );
        }
    }

    #[test]
    fn native_icons_are_freed_when_window_was_already_destroyed() {
        let _serial = crate::window_native_windows_tests::WINDOWS
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let window = Window::new(
            "Spotiamp closed icon test",
            32,
            32,
            WindowOptions::default(),
        )
        .expect("Windows CI must allow native windows");
        let icons = WindowIcons::attach(&window).unwrap();
        let big = icons.big.0;
        let small = icons.small.0;
        drop(window);
        drop(icons);
        assert_eq!(dimensions(big), None);
        assert_eq!(dimensions(small), None);
    }
}
