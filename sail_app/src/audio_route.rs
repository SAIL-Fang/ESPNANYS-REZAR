// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
//! Windows default-render-device inspection and switching.
//!
//! This is user-mode Core Audio (MMDevice + the classic IPolicyConfig COM
//! object). It does not install a kernel driver and does not touch the HID
//! protocol.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEndpoint {
    pub id: String,
    pub name: String,
    pub is_headset: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutput {
    pub name: Option<String>,
    pub is_headset: bool,
    pub error: Option<String>,
    pub endpoints: Vec<AudioEndpoint>,
}

#[cfg(not(windows))]
pub fn current_output() -> AudioOutput {
    AudioOutput {
        name: None,
        is_headset: true,
        error: Some("audio output check is Windows-only".into()),
        endpoints: Vec::new(),
    }
}

#[cfg(not(windows))]
pub fn set_default(_device_id: &str) -> Result<(), String> {
    Err("audio output switching is Windows-only".into())
}

#[cfg(windows)]
mod windows_impl {
    use super::{AudioEndpoint, AudioOutput};
    use windows::core::{IUnknown, IUnknown_Vtbl, Interface, GUID, HRESULT, PCWSTR};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDevice, IMMDeviceCollection, IMMDeviceEnumerator, MMDeviceEnumerator,
        DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
    };
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;

    const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
    const IID_POLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

    #[repr(transparent)]
    #[derive(Clone)]
    struct PolicyConfig(IUnknown);

    #[repr(C)]
    struct PolicyConfigVtable {
        base__: IUnknown_Vtbl,
        get_mix_format: *const std::ffi::c_void,
        get_device_format: *const std::ffi::c_void,
        reset_device_format: *const std::ffi::c_void,
        set_device_format: *const std::ffi::c_void,
        get_processing_period: *const std::ffi::c_void,
        set_processing_period: *const std::ffi::c_void,
        get_share_mode: *const std::ffi::c_void,
        set_share_mode: *const std::ffi::c_void,
        get_property_value: *const std::ffi::c_void,
        set_property_value: *const std::ffi::c_void,
        set_default_endpoint:
            unsafe extern "system" fn(*mut std::ffi::c_void, PCWSTR, u32) -> HRESULT,
        set_endpoint_visibility: *const std::ffi::c_void,
    }

    unsafe impl Interface for PolicyConfig {
        type Vtable = PolicyConfigVtable;
        const IID: GUID = IID_POLICY_CONFIG;
    }

    fn com_ready() {
        // A UI thread may already be STA; RPC_E_CHANGED_MODE is harmless.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    }

    fn enumerator() -> Result<IMMDeviceEnumerator, String> {
        com_ready();
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .map_err(|error| format!("create MMDeviceEnumerator: {error}"))
    }

    fn id_of(device: &IMMDevice) -> Result<String, String> {
        let id = unsafe { device.GetId() }.map_err(|error| format!("read endpoint id: {error}"))?;
        unsafe { id.to_string() }.map_err(|error| format!("decode endpoint id: {error}"))
    }

    fn friendly_name(device: &IMMDevice) -> Option<String> {
        unsafe {
            let store: IPropertyStore = device.OpenPropertyStore(STGM_READ).ok()?;
            let value = store.GetValue(&PKEY_Device_FriendlyName).ok()?;
            let name = format!("{value}");
            (!name.is_empty()).then_some(name)
        }
    }

    fn collect() -> Result<(Vec<AudioEndpoint>, Option<String>), String> {
        let enumerator = enumerator()?;
        let default_id = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }
            .ok()
            .and_then(|device| id_of(&device).ok());
        let collection: IMMDeviceCollection = unsafe {
            enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(|error| format!("enumerate render endpoints: {error}"))?
        };
        let count = unsafe { collection.GetCount() }
            .map_err(|error| format!("count render endpoints: {error}"))?;
        let mut endpoints = Vec::new();
        for index in 0..count {
            let device = match unsafe { collection.Item(index) } {
                Ok(device) => device,
                Err(_) => continue,
            };
            let id = match id_of(&device) {
                Ok(id) => id,
                Err(_) => continue,
            };
            let name = friendly_name(&device).unwrap_or_else(|| id.clone());
            let lower = name.to_ascii_lowercase();
            endpoints.push(AudioEndpoint {
                id: id.clone(),
                name,
                is_headset: lower.contains("blackshark"),
                is_default: default_id.as_deref() == Some(id.as_str()),
            });
        }
        Ok((endpoints, default_id))
    }

    pub fn current_output() -> AudioOutput {
        match collect() {
            Ok((endpoints, default_id)) => {
                let default = endpoints
                    .iter()
                    .find(|endpoint| endpoint.is_default)
                    .or_else(|| {
                        default_id
                            .as_deref()
                            .and_then(|id| endpoints.iter().find(|endpoint| endpoint.id == id))
                    });
                AudioOutput {
                    name: default.map(|endpoint| endpoint.name.clone()),
                    is_headset: default.is_some_and(|endpoint| endpoint.is_headset),
                    error: None,
                    endpoints,
                }
            }
            Err(error) => AudioOutput {
                name: None,
                is_headset: false,
                error: Some(error),
                endpoints: Vec::new(),
            },
        }
    }

    fn set_endpoint(device_id: &str) -> Result<(), String> {
        com_ready();
        let wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
        let policy: PolicyConfig =
            unsafe { CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL) }
                .map_err(|error| format!("create IPolicyConfig: {error}"))?;
        let raw = Interface::as_raw(&policy);
        let vtable = Interface::vtable(&policy);
        for role in 0..=2 {
            let hr = unsafe { (vtable.set_default_endpoint)(raw, PCWSTR(wide.as_ptr()), role) };
            if hr.is_err() {
                return Err(format!("set default endpoint (role {role}): {hr:?}"));
            }
        }
        Ok(())
    }

    pub fn set_default(device_id: &str) -> Result<(), String> {
        let (endpoints, _) = collect()?;
        let selected = endpoints
            .iter()
            .find(|endpoint| endpoint.id == device_id)
            .or_else(|| endpoints.iter().find(|endpoint| endpoint.is_headset))
            .ok_or_else(|| "no Razer headset output endpoint found".to_string())?;
        set_endpoint(&selected.id)
    }
}

#[cfg(windows)]
pub use windows_impl::{current_output, set_default};

#[cfg(windows)]
#[allow(dead_code)]
fn _keep_types_used() {
    let _ = AudioOutput {
        name: None,
        is_headset: false,
        error: None,
        endpoints: Vec::new(),
    };
}
