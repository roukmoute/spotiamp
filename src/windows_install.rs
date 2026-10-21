//! Per-user Inno Setup coordination. No installer dependency is loaded by the player.
use std::{
    ffi::{OsStr, c_void},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

const APP_MUTEX: &str = "Local\\Spotiamp.Application.4A36A2DD-2701-4EF4-A3B3-5C1EF1A8A1D0";
const SETUP_MUTEX: &str = "Local\\Spotiamp.Setup.4A36A2DD-2701-4EF4-A3B3-5C1EF1A8A1D0";
const UNINSTALL_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{4A36A2DD-2701-4EF4-A3B3-5C1EF1A8A1D0}_is1";
const SETUP_WAIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(25);
const ERROR_FILE_NOT_FOUND: i32 = 2;
const REG_SZ: u32 = 1;
const KEY_QUERY_VALUE: u32 = 1;
const KEY_SET_VALUE: u32 = 2;
const HKCU: *mut c_void = 0x80000001u32 as i32 as isize as *mut c_void;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(attributes: *mut c_void, owner: i32, name: *const u16) -> *mut c_void;
    fn OpenMutexW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
    fn CloseHandle(handle: *mut c_void) -> i32;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        root: *mut c_void,
        path: *const u16,
        options: u32,
        access: u32,
        key: *mut *mut c_void,
    ) -> i32;
    fn RegCloseKey(key: *mut c_void) -> i32;
    fn RegQueryValueExW(
        key: *mut c_void,
        name: *const u16,
        reserved: *mut u32,
        kind: *mut u32,
        data: *mut u8,
        bytes: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        key: *mut c_void,
        name: *const u16,
        reserved: u32,
        kind: u32,
        data: *const u8,
        bytes: u32,
    ) -> i32;
}

fn wide(text: impl AsRef<OsStr>) -> Vec<u16> {
    text.as_ref().encode_wide().chain(Some(0)).collect()
}

/// Keep this guard alive until the process exits, including updater helpers.
pub struct RuntimeGuard(*mut c_void);
impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn create_mutex(name: &str) -> Result<RuntimeGuard, String> {
    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 0, wide(name).as_ptr()) };
    if handle.is_null() {
        Err(format!(
            "Protection de l'installation indisponible : {}",
            std::io::Error::last_os_error()
        ))
    } else {
        // Multiple player/helper processes may share this presence marker.
        // No thread owns the mutex; Inno tests its existence, not ownership.
        Ok(RuntimeGuard(handle))
    }
}

fn mutex_exists(name: &str) -> Result<bool, String> {
    let handle = unsafe { OpenMutexW(0x00100000, 0, wide(name).as_ptr()) }; // SYNCHRONIZE
    if !handle.is_null() {
        unsafe {
            CloseHandle(handle);
        }
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) {
        Ok(false)
    } else {
        // An inaccessible same-name object is not permission to modify files.
        Err(format!("Etat de l'installation indisponible : {error}"))
    }
}

fn acquire_named(app: &str, setup: &str, wait: Duration) -> Result<RuntimeGuard, String> {
    let started = Instant::now();
    loop {
        if !mutex_exists(setup)? {
            let guard = create_mutex(app)?;
            // Close the gap between observing Setup and advertising ourselves.
            // Inno also rechecks AppMutex in PrepareToInstall after acquiring
            // SetupMutex: its built-in initial AppMutex check happens earlier.
            if !mutex_exists(setup)? {
                return Ok(guard);
            }
            drop(guard);
        }
        if started.elapsed() >= wait {
            return Err("Installation ou desinstallation de Spotiamp en cours. Reessayez apres sa fermeture.".into());
        }
        thread::sleep(POLL);
    }
}

/// Only Inno's post-install launch waits for its own Setup process to finish.
/// Ordinary launches and --apply-update helpers fail promptly while Setup is active.
///
/// `finishing_update` is restricted to argv[1] == --cleanup-update or
/// --update-failed, with the updater feature enabled. The caller MUST run
/// dispatch_internal immediately afterwards and exit on Dispatch::Exit: only
/// its validated handoff may continue to UI. Coordinated helper versions already
/// hold our application mutex, so the uninstaller can only be waiting for it.
/// Earlier released helpers lack this marker; validation remains mandatory.
pub fn acquire_runtime(
    wait_for_setup: bool,
    finishing_update: bool,
) -> Result<RuntimeGuard, String> {
    if finishing_update {
        return create_mutex(APP_MUTEX);
    }
    acquire_named(
        APP_MUTEX,
        SETUP_MUTEX,
        if wait_for_setup {
            SETUP_WAIT
        } else {
            Duration::ZERO
        },
    )
}

struct RegistryKey(*mut c_void);
impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

fn open_existing_key(path: &str) -> Result<Option<RegistryKey>, String> {
    let mut key = std::ptr::null_mut();
    let code = unsafe {
        RegOpenKeyExW(
            HKCU,
            wide(path).as_ptr(),
            0,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            &mut key,
        )
    };
    match code {
        0 => Ok(Some(RegistryKey(key))),
        ERROR_FILE_NOT_FOUND => Ok(None),
        _ => Err(format!("Lecture du registre impossible : {code}")),
    }
}

fn query_string(key: &RegistryKey, name: &str) -> Option<String> {
    let name = wide(name);
    let mut kind = 0;
    let mut bytes = 0;
    let code = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if code != 0 || kind != REG_SZ || bytes == 0 || bytes > 32 * 1024 || !bytes.is_multiple_of(2) {
        return None;
    }
    let mut text = vec![0u16; bytes as usize / 2];
    let code = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            text.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if code != 0 || kind != REG_SZ || !bytes.is_multiple_of(2) || bytes as usize > text.len() * 2 {
        return None;
    }
    text.truncate(bytes as usize / 2);
    if text.last() == Some(&0) {
        text.pop();
    }
    if text.contains(&0) {
        return None;
    }
    String::from_utf16(&text).ok()
}

fn set_string(key: &RegistryKey, name: &str, value: &str) -> Result<(), String> {
    let value = wide(value);
    let code = unsafe {
        RegSetValueExW(
            key.0,
            wide(name).as_ptr(),
            0,
            REG_SZ,
            value.as_ptr().cast(),
            (value.len() * 2) as u32,
        )
    };
    if code == 0 {
        Ok(())
    } else {
        Err(format!("Mise a jour du registre impossible : {code}"))
    }
}

fn uninstall_path(command: &str) -> Option<PathBuf> {
    // Inno writes a single quoted executable path, without command arguments.
    let path = if let Some(quoted) = command.strip_prefix('"') {
        quoted.strip_suffix('"')?
    } else {
        command
    };
    if path.contains('"') {
        return None;
    }
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return None;
    }
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let suffix = name.strip_prefix("unins")?.strip_suffix(".exe")?;
    (suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit())).then_some(path)
}

fn sync_for(exe: &Path, version: &str, key_path: &str) -> Result<bool, String> {
    let Some(key) = open_existing_key(key_path)? else {
        return Ok(false);
    };
    let Ok(exe) = exe.canonicalize() else {
        return Ok(false);
    };
    let Some(parent) = exe.parent() else {
        return Ok(false);
    };
    let Some(location) = query_string(&key, "InstallLocation") else {
        return Ok(false);
    };
    let Ok(location) = Path::new(&location).canonicalize() else {
        return Ok(false);
    };
    if location != parent {
        return Ok(false);
    }
    let Some(uninstaller) =
        query_string(&key, "UninstallString").and_then(|value| uninstall_path(&value))
    else {
        return Ok(false);
    };
    let Ok(uninstaller) = uninstaller.canonicalize() else {
        return Ok(false);
    };
    if uninstaller.parent() != Some(parent) || !uninstaller.is_file() {
        return Ok(false);
    }
    if query_string(&key, "DisplayVersion").as_deref() == Some(version) {
        return Ok(true);
    }
    set_string(&key, "DisplayVersion", version)?;
    Ok(true)
}

/// Run after successful internal-update dispatch, never from the copied helper.
/// A portable copy or another installation cannot alter this registry entry.
pub fn sync_display_version() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = sync_for(&exe, env!("CARGO_PKG_VERSION"), UNINSTALL_KEY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn unique() -> String {
        format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
    }
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegCreateKeyExW(
            root: *mut c_void,
            path: *const u16,
            reserved: u32,
            class: *mut u16,
            options: u32,
            access: u32,
            security: *mut c_void,
            result: *mut *mut c_void,
            disposition: *mut u32,
        ) -> i32;
        fn RegDeleteTreeW(root: *mut c_void, path: *const u16) -> i32;
    }
    struct Fixture {
        path: PathBuf,
        key_path: String,
        key: Option<RegistryKey>,
    }
    impl Fixture {
        fn new() -> Self {
            let id = unique();
            let path = std::env::temp_dir().join(format!("Spotiamp installation test é {id}"));
            fs::create_dir(&path).unwrap();
            fs::write(path.join("spotiamp.exe"), b"fixture").unwrap();
            fs::write(path.join("unins000.exe"), b"fixture").unwrap();
            let key_path = format!("Software\\SpotiampTest\\{id}");
            let mut raw = std::ptr::null_mut();
            assert_eq!(
                unsafe {
                    RegCreateKeyExW(
                        HKCU,
                        wide(&key_path).as_ptr(),
                        0,
                        std::ptr::null_mut(),
                        0,
                        KEY_QUERY_VALUE | KEY_SET_VALUE,
                        std::ptr::null_mut(),
                        &mut raw,
                        std::ptr::null_mut(),
                    )
                },
                0
            );
            let key = RegistryKey(raw);
            set_string(&key, "InstallLocation", path.to_str().unwrap()).unwrap();
            set_string(
                &key,
                "UninstallString",
                &format!("\"{}\"", path.join("unins000.exe").display()),
            )
            .unwrap();
            set_string(&key, "DisplayVersion", "0.1.0-alpha.8").unwrap();
            Self {
                path,
                key_path,
                key: Some(key),
            }
        }
        fn exe(&self) -> PathBuf {
            self.path.join("spotiamp.exe")
        }
        fn version(&self) -> Option<String> {
            query_string(self.key.as_ref().unwrap(), "DisplayVersion")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.key.take();
            unsafe {
                RegDeleteTreeW(HKCU, wide(&self.key_path).as_ptr());
            }
            let _ = fs::remove_dir_all(&self.path);
        }
    }
    #[test]
    fn native_registry_updates_only_its_exact_installed_location() {
        let fixture = Fixture::new();
        assert!(sync_for(&fixture.exe(), "0.1.0", &fixture.key_path).unwrap());
        assert_eq!(fixture.version().as_deref(), Some("0.1.0"));
        let other = fixture.path.join("Portable");
        fs::create_dir(&other).unwrap();
        fs::write(other.join("spotiamp.exe"), b"fixture").unwrap();
        assert!(!sync_for(&other.join("spotiamp.exe"), "9.9.9", &fixture.key_path).unwrap());
        assert_eq!(fixture.version().as_deref(), Some("0.1.0"));
    }
    #[test]
    fn native_registry_never_creates_missing_keys_and_rejects_other_uninstallers() {
        let fixture = Fixture::new();
        let absent = format!("{}\\absent", fixture.key_path);
        assert!(!sync_for(&fixture.exe(), "0.1.0", &absent).unwrap());
        assert!(open_existing_key(&absent).unwrap().is_none());
        for command in [
            "relative.exe".into(),
            format!(
                "\"{}\" /SILENT",
                fixture.path.join("unins000.exe").display()
            ),
            fixture.exe().to_string_lossy().into_owned(),
        ] {
            set_string(fixture.key.as_ref().unwrap(), "UninstallString", &command).unwrap();
            assert!(!sync_for(&fixture.exe(), "0.1.0", &fixture.key_path).unwrap());
            assert_eq!(fixture.version().as_deref(), Some("0.1.0-alpha.8"));
        }
    }
    #[test]
    fn native_mutex_presence_is_shared_and_setup_blocks_new_processes() {
        let id = unique();
        let app = format!("Local\\Spotiamp.Test.App.{id}");
        let setup = format!("Local\\Spotiamp.Test.Setup.{id}");
        let first = acquire_named(&app, &setup, Duration::ZERO).unwrap();
        let second = acquire_named(&app, &setup, Duration::ZERO).unwrap();
        assert!(mutex_exists(&app).unwrap());
        drop(first);
        assert!(mutex_exists(&app).unwrap());
        drop(second);
        assert!(!mutex_exists(&app).unwrap());
        let installing = create_mutex(&setup).unwrap();
        assert!(acquire_named(&app, &setup, Duration::ZERO).is_err());
        assert!(!mutex_exists(&app).unwrap());
        drop(installing);
        assert!(acquire_named(&app, &setup, Duration::ZERO).is_ok());
    }
    #[test]
    fn native_postinstall_wait_is_bounded_and_can_observe_setup_exit() {
        let id = unique();
        let app = format!("Local\\Spotiamp.Test.App.{id}");
        let setup = format!("Local\\Spotiamp.Test.Setup.{id}");
        let installing = create_mutex(&setup).unwrap();
        assert!(acquire_named(&app, &setup, Duration::from_millis(50)).is_err());
        drop(installing);
        let (ready, started) = std::sync::mpsc::channel();
        let setup_name = setup.clone();
        let installer = thread::spawn(move || {
            let _guard = create_mutex(&setup_name).unwrap();
            ready.send(()).unwrap();
            thread::sleep(Duration::from_millis(75));
        });
        started.recv().unwrap();
        let guard = acquire_named(&app, &setup, Duration::from_secs(2)).unwrap();
        installer.join().unwrap();
        assert!(mutex_exists(&app).unwrap());
        drop(guard);
    }
}
