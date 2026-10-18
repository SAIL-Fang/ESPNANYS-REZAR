// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
//! Minimal SAIL bridge to the installed THX APO control DLL.
//!
//! This does not bundle THX. It loads the user's installed
//! `ThxV3Native_v1.1.42.1.dll` with kernel32 and calls the container-scoped
//! functions the Razer page uses:
//!
//! ```text
//! DeviceInitialize(containerId) -> bool
//! GetRenderDevice(containerId) -> char* (JSON)
//! SetDefaultRenderDevice(endpointId) -> bool
//! GetRenderNormalization(containerId) -> bool
//! SetRenderNormalization(containerId, bool) -> bool
//! GetRenderNormalizationLevel(containerId) -> int
//! SetRenderNormalizationLevel(containerId, int) -> bool
//! ```
//!
//! The level is an `int`, not a `double`. A `double` call can return true by
//! accident but does not set the level; the verified setter/getter are int.

#![cfg(windows)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub const CONTAINER: &str = "{FEC3BF0F-2180-5CED-AEEB-DFB45C54FB17}";
pub const DEFAULT_DLL: &str =
    r"Razer\RazerAppEngine\User Data\Apps\Common\thxv3common\ThxV3Native_v1.1.42.1.dll";

type FnBool1 = unsafe extern "system" fn(*const c_char) -> bool;
type FnBool2 = unsafe extern "system" fn(*const c_char, bool) -> bool;
type FnSetLevel = unsafe extern "system" fn(*const c_char, c_int) -> bool;
type FnGetLevel = unsafe extern "system" fn(*const c_char) -> c_int;
type FnPtr1 = unsafe extern "system" fn(*const c_char) -> *mut c_char;
type FnList = unsafe extern "system" fn() -> *mut c_char;

/// Level writes are applied asynchronously by the THX APO; a write issued too
/// early is rejected instead of queued. Five attempts with short sleeps cover
/// the settling window without stalling the UI.
const RETRIES: u32 = 5;
const RETRY_DELAY_MS: u64 = 80;

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
    fn FreeLibrary(h_lib_module: *mut c_void) -> i32;
}

#[derive(Debug)]
pub enum ThxError {
    DllNotFound(PathBuf),
    Load(String),
    MissingExport(String),
    InitFailed,
    NoRenderDevice,
    InvalidLevel(i32),
    Status(String),
}

impl fmt::Display for ThxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DllNotFound(path) => write!(f, "THX DLL not found: {}", path.display()),
            Self::Load(reason) => write!(f, "cannot load THX DLL: {reason}"),
            Self::MissingExport(name) => write!(f, "THX DLL missing export: {name}"),
            Self::InitFailed => write!(f, "THX DeviceInitialize failed"),
            Self::NoRenderDevice => write!(f, "THX has no render device for this container"),
            Self::InvalidLevel(level) => {
                write!(f, "normalization level must be 1..100, got {level}")
            }
            Self::Status(reason) => write!(f, "THX call failed: {reason}"),
        }
    }
}

impl std::error::Error for ThxError {}

#[derive(Debug, Clone)]
pub struct ThxStatus {
    pub dll: PathBuf,
    pub endpoint: Option<String>,
    pub enabled: Option<bool>,
    pub level: Option<i32>,
    pub spatial_enabled: Option<bool>,
}

pub struct ThxBridge {
    module: *mut c_void,
    dll: PathBuf,
    container: CString,
    endpoint: Option<CString>,
}

impl Drop for ThxBridge {
    fn drop(&mut self) {
        if !self.module.is_null() {
            unsafe {
                let _ = FreeLibrary(self.module);
            }
        }
    }
}

fn discover_container(module: *mut c_void) -> Option<String> {
    unsafe {
        let init_ptr = GetProcAddress(module, b"DeviceInitialize\0".as_ptr());
        if init_ptr.is_null() {
            return None;
        }
        let init: FnBool1 = std::mem::transmute(init_ptr);
        // A bootstrap call initializes the DLL's device enumerator; the actual
        // container is selected from the enumerated render devices below.
        let bootstrap = CString::new("{}").ok()?;
        let _ = init(bootstrap.as_ptr());

        let list_ptr = GetProcAddress(module, b"GetAudioDeviceListBySystem\0".as_ptr());
        if list_ptr.is_null() {
            return None;
        }
        let list: FnList = std::mem::transmute(list_ptr);
        let ptr = list();
        if ptr.is_null() {
            return None;
        }
        let json = CStr::from_ptr(ptr).to_string_lossy();
        let value: serde_json::Value = serde_json::from_str(&json).ok()?;
        let devices = value.get("devices")?.as_array()?;
        let is_headset = |device: &&serde_json::Value| {
            device
                .get("name")
                .and_then(|name| name.as_str())
                .map(|name| name.contains("BlackShark"))
                .unwrap_or(false)
        };
        devices
            .iter()
            .find(|device| {
                is_headset(device) && device.get("type").and_then(|kind| kind.as_u64()) == Some(0)
            })
            .or_else(|| devices.iter().find(is_headset))
            .and_then(|device| device.get("deviceContainerId"))
            .and_then(|id| id.as_str())
            .map(ToOwned::to_owned)
    }
}

impl ThxBridge {
    /// Open for reading and writing: also drives THX into the state where
    /// level writes are accepted.
    pub fn open() -> Result<Self, ThxError> {
        let dll = std::env::var_os("SAIL_THX_DLL")
            .map(PathBuf::from)
            .unwrap_or_else(default_dll_path);
        Self::with_dll(dll)
    }

    /// Open for observation only: binds the render device but never enables
    /// normalization or writes a level, so a reader cannot change user state.
    pub fn open_passive() -> Result<Self, ThxError> {
        let dll = std::env::var_os("SAIL_THX_DLL")
            .map(PathBuf::from)
            .unwrap_or_else(default_dll_path);
        Self::with_dll_passive(dll)
    }

    pub fn with_dll(path: impl Into<PathBuf>) -> Result<Self, ThxError> {
        Self::load(path.into(), true)
    }

    pub fn with_dll_passive(path: impl Into<PathBuf>) -> Result<Self, ThxError> {
        Self::load(path.into(), false)
    }

    fn load(dll: PathBuf, activate: bool) -> Result<Self, ThxError> {
        if !dll.is_file() {
            return Err(ThxError::DllNotFound(dll));
        }
        let wide: Vec<u16> = dll
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let module = unsafe { LoadLibraryW(wide.as_ptr()) };
        if module.is_null() {
            return Err(ThxError::Load(format!(
                "LoadLibraryA({}): {}",
                dll.display(),
                std::io::Error::last_os_error()
            )));
        }
        let container_value = std::env::var("SAIL_THX_CONTAINER")
            .ok()
            .or_else(|| discover_container(module))
            .unwrap_or_else(|| CONTAINER.to_string());
        let container = CString::new(container_value)
            .map_err(|e| ThxError::Status(format!("invalid THX container: {e}")))?;
        let mut bridge = Self {
            module,
            dll,
            container,
            endpoint: None,
        };
        bridge.initialize(activate)?;
        Ok(bridge)
    }

    pub fn dll_path(&self) -> &PathBuf {
        &self.dll
    }

    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_ref().and_then(|c| c.to_str().ok())
    }

    fn initialize(&mut self, activate: bool) -> Result<(), ThxError> {
        let init: FnBool1 = unsafe { self.sym(b"DeviceInitialize\0")? };
        let _ = unsafe { init(self.container.as_ptr()) };
        let endpoint = self
            .render_device_json()?
            .and_then(|json| json_string(&json, "id"))
            .ok_or(ThxError::NoRenderDevice)?;
        let endpoint_c = CString::new(endpoint).map_err(|e| ThxError::Status(e.to_string()))?;
        // The DLL needs a selected render device before per-endpoint getters
        // work, including in passive mode. This is a device binding, not a
        // normalization/level write.
        let set_default: FnBool1 = unsafe { self.sym(b"SetDefaultRenderDevice\0")? };
        let _ = unsafe { set_default(endpoint_c.as_ptr()) };

        if activate {
            // Call this even when already enabled: the DLL uses the call to
            // (re)enter a state where SetRenderNormalizationLevel is accepted.
            let set_enabled: FnBool2 = unsafe { self.sym(b"SetRenderNormalization\0")? };
            let _ = unsafe { set_enabled(self.container.as_ptr(), true) };
        }

        self.endpoint = Some(endpoint_c);
        Ok(())
    }

    pub fn get_enabled(&self) -> Result<bool, ThxError> {
        let f: FnBool1 = unsafe { self.sym(b"GetRenderNormalization\0")? };
        Ok(unsafe { f(self.container.as_ptr()) })
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<(), ThxError> {
        let f: FnBool2 = unsafe { self.sym(b"SetRenderNormalization\0")? };
        for attempt in 0..RETRIES {
            if unsafe { f(self.container.as_ptr(), enabled) } {
                return Ok(());
            }
            // The DLL reports failure when asked to apply the state it already
            // has, so a matching readback means the request is satisfied.
            if self.get_enabled().map(|v| v == enabled).unwrap_or(false) {
                return Ok(());
            }
            if attempt + 1 < RETRIES {
                std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
            }
        }
        Err(ThxError::Status(
            "SetRenderNormalization returned false".into(),
        ))
    }

    pub fn get_level(&self) -> Result<i32, ThxError> {
        let f: FnGetLevel = unsafe { self.sym(b"GetRenderNormalizationLevel\0")? };
        Ok(unsafe { f(self.container.as_ptr()) })
    }

    pub fn set_level(&self, level: i32) -> Result<(), ThxError> {
        validate_level(level)?;
        let f: FnSetLevel = unsafe { self.sym(b"SetRenderNormalizationLevel\0")? };

        // THX applies a level asynchronously. A second write issued while the
        // previous one is still in flight returns false and is dropped, so a
        // quick "set A, set B" sequence would silently keep A. Retry while
        // treating a matching readback as success.
        for attempt in 0..RETRIES {
            if unsafe { f(self.container.as_ptr(), level) } {
                return Ok(());
            }
            if self.get_level().map(|v| v == level).unwrap_or(false) {
                return Ok(());
            }
            if attempt + 1 < RETRIES {
                std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
            }
        }
        Err(ThxError::Status(format!(
            "SetRenderNormalizationLevel({level}) returned false"
        )))
    }

    pub fn status(&self) -> Result<ThxStatus, ThxError> {
        Ok(ThxStatus {
            dll: self.dll.clone(),
            endpoint: self.endpoint().map(ToOwned::to_owned),
            enabled: self.get_enabled().ok(),
            level: self.get_level().ok(),
            spatial_enabled: self.get_spatial().ok(),
        })
    }

    pub fn get_spatial(&self) -> Result<bool, ThxError> {
        let f: FnBool1 = unsafe { self.sym(b"GetRenderSpatialProcessing\0")? };
        Ok(unsafe { f(self.container.as_ptr()) })
    }

    pub fn set_spatial(&self, enabled: bool) -> Result<(), ThxError> {
        let f: FnBool2 = unsafe { self.sym(b"SetRenderSpatialProcessing\0")? };
        for attempt in 0..RETRIES {
            if unsafe { f(self.container.as_ptr(), enabled) } {
                return Ok(());
            }
            if self.get_spatial().map(|v| v == enabled).unwrap_or(false) {
                return Ok(());
            }
            if attempt + 1 < RETRIES {
                std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
            }
        }
        Err(ThxError::Status(
            "SetRenderSpatialProcessing returned false".into(),
        ))
    }

    fn render_device_json(&self) -> Result<Option<String>, ThxError> {
        let f: FnPtr1 = unsafe { self.sym(b"GetRenderDevice\0")? };
        let ptr = unsafe { f(self.container.as_ptr()) };
        if ptr.is_null() {
            return Ok(None);
        }
        let json = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        // `GetRenderDevice` returns a pointer owned by the DLL; do not free it
        // here. Python's read-only probe also leaves it alone and works.
        Ok(Some(json))
    }

    unsafe fn sym<T: Copy>(&self, name: &[u8]) -> Result<T, ThxError> {
        self.proc::<T>(name).ok_or_else(|| {
            ThxError::MissingExport(String::from_utf8_lossy(&name[..name.len() - 1]).into())
        })
    }

    fn proc<T: Copy>(&self, name: &[u8]) -> Option<T> {
        let ptr = unsafe { GetProcAddress(self.module, name.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute_copy(&ptr) })
        }
    }
}

fn validate_level(level: i32) -> Result<(), ThxError> {
    if (1..=100).contains(&level) {
        Ok(())
    } else {
        Err(ThxError::InvalidLevel(level))
    }
}

fn default_dll_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let fallback = base.join(DEFAULT_DLL);

    // Do not hard-code one THX build: vendors ship versioned DLLs
    // (ThxV3Native_v1.1.42.1.dll, ...). Pick the highest numeric version
    // present and keep the old path as a fallback. SAIL_THX_DLL still wins.
    let Some(dir) = fallback.parent() else {
        return fallback;
    };
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| {
                            name.starts_with("ThxV3Native_v")
                                && name.to_ascii_lowercase().ends_with(".dll")
                        })
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    candidates.sort_by_key(|path| version_key(path));
    candidates.pop().unwrap_or(fallback)
}

fn version_key(path: &Path) -> Vec<u64> {
    let mut groups = Vec::new();
    let mut current = String::new();
    for ch in path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .chars()
    {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if !current.is_empty() {
            if let Ok(value) = current.parse() {
                groups.push(value);
            }
            current.clear();
        }
    }
    if let Ok(value) = current.parse() {
        groups.push(value);
    }
    groups
}

/// Extract a simple top-level JSON string field. The render-device payload is a
/// small known object; no general JSON parser is needed.
fn json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

pub fn print_status(status: &ThxStatus) {
    println!("dll: {}", status.dll.display());
    println!(
        "endpoint: {}",
        status.endpoint.as_deref().unwrap_or("(none)")
    );
    println!(
        "enabled: {}",
        status
            .enabled
            .map(|v| if v { "on" } else { "off" })
            .unwrap_or("unknown")
    );
    println!(
        "level: {}",
        status
            .level
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!(
        "spatial: {}",
        status
            .spatial_enabled
            .map(|v| if v { "on" } else { "off" })
            .unwrap_or("unknown")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_render_device_id() {
        let json =
            r#"{"deviceContainerId":"{C}","id":"{0.0.0.00000000}.{abc}","name":"x","result":true}"#;
        assert_eq!(
            json_string(json, "id").as_deref(),
            Some("{0.0.0.00000000}.{abc}")
        );
    }

    #[test]
    fn level_range_is_1_to_100() {
        assert!(validate_level(1).is_ok());
        assert!(validate_level(100).is_ok());
        assert!(matches!(validate_level(0), Err(ThxError::InvalidLevel(0))));
        assert!(matches!(
            validate_level(101),
            Err(ThxError::InvalidLevel(101))
        ));
    }
}
