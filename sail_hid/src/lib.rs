// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
//! Windows HID transport for supported Razer devices.
//!
//! Uses the mature `hidapi` crate instead of hand-rolled SetupAPI/HID calls.
//! Headsets expose their vendor control interface on usage page 0xFF14 and
//! speak a 64-byte interrupt-report protocol. Mice such as the Viper V3 Pro
//! carry a 90-byte feature report (Razer protocol 2.5) on their primary
//! interface instead; `MouseHidTransport` implements that exchange.

use hidapi::{HidApi, HidDevice as RawHidDevice};
use sail_core::mouse::{FeatureTransport, MouseDevice, REPORT_LEN};
use sail_core::{Error, SailDevice, Transport, FRAME_LEN};
use std::ffi::CString;
use std::time::Duration;

mod model;
pub use model::{
    model_for_product_id, unknown_model, DeviceKind, ModelCapabilities, ModelDescriptor,
    PID_USB_SOUND_CARD, RAZER_BLACKSHARK_V2_HYPERSPEED, RAZER_BLACKSHARK_V2_USB_SOUND_CARD,
    RAZER_VIPER_V3_PRO, UNKNOWN_MODEL, VIPER_V3_PRO_WIRED_PID, VIPER_V3_PRO_WIRELESS_PID,
};

pub const RAZER_VID: u16 = 0x1532;
pub const PID_WIRELESS: u16 = 0x0565;
pub const PID_WIRED: u16 = 0x056E;
pub const VENDOR_USAGE_PAGE: u16 = 0xFF14;
pub const VENDOR_USAGE: u16 = 0x0001;

#[derive(Debug, Clone)]
pub struct HidInfo {
    pub path: String,
    pub product: Option<String>,
    pub vendor_id: u16,
    pub product_id: u16,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
}

pub struct HidTransport {
    // Keep the HidApi alive for the lifetime of the opened device.
    _api: HidApi,
    device: RawHidDevice,
    info: HidInfo,
}

/// Whether `info` is the model's vendor control collection.
pub fn matches_vendor_usage(model: &ModelDescriptor, info: &HidInfo) -> bool {
    info.vendor_id == model.vendor_id
        && info.usage_page == model.usage_page
        && info.usage == model.usage
}

/// Stable hub order: the primary headset first, the analog fallback next,
/// telemetry-only mice last.
pub fn profile_rank(model: &ModelDescriptor) -> u8 {
    match model.kind {
        DeviceKind::Headset if model.id == RAZER_BLACKSHARK_V2_HYPERSPEED.id => 0,
        DeviceKind::Headset => 1,
        DeviceKind::Mouse => 2,
    }
}

fn select_model(
    devices: Vec<HidInfo>,
    kind: DeviceKind,
    wanted: Option<u16>,
    allow_any_usage: bool,
) -> Option<HidInfo> {
    let mut candidates: Vec<HidInfo> = devices
        .into_iter()
        .filter(|device| {
            let Some(profile) = model_for_product_id(device.product_id) else {
                return false;
            };
            profile.kind == kind
                && device.vendor_id == profile.vendor_id
                && wanted.map_or(true, |pid| device.product_id == pid)
                && (allow_any_usage || matches_vendor_usage(profile, device))
        })
        .collect();
    // Prefer the profile's vendor control interface, then the wireless receiver.
    candidates.sort_by_key(|device| {
        let profile = model_for_product_id(device.product_id);
        let interface_rank =
            if profile.map(|p| device.interface_number == p.interface_number) == Some(true) {
                0
            } else {
                1
            };
        let pid_rank = if profile.map(|p| p.wireless_pid == Some(device.product_id)) == Some(true) {
            0
        } else {
            1
        };
        (interface_rank, pid_rank)
    });
    candidates.into_iter().next()
}

/// Pick the best headset interface. This transport speaks the headset's
/// 64-byte report format, so a mouse collection is never a candidate.
fn select_device(
    devices: Vec<HidInfo>,
    wanted: Option<u16>,
    allow_any_usage: bool,
) -> Option<HidInfo> {
    select_model(devices, DeviceKind::Headset, wanted, allow_any_usage)
}

pub fn list_models() -> Result<Vec<ModelDescriptor>, Error> {
    let mut models = Vec::new();
    for device in enumerate()? {
        if let Some(model) = model_for_product_id(device.product_id) {
            if !models
                .iter()
                .any(|known: &ModelDescriptor| known.id == model.id)
            {
                models.push(model.clone());
            }
        }
    }
    // The device hub keeps a stable order: controllable headsets first, then
    // telemetry-only mice.
    models.sort_by_key(profile_rank);
    Ok(models)
}

impl HidTransport {
    /// Enumerate Razer vendor HID interfaces and open the best matching device.
    /// When no PID is supplied, the 2.4G receiver is preferred over wired.
    pub fn open(pid: Option<u16>) -> Result<Self, Error> {
        let wanted = pid;
        let info = select_device(enumerate()?, wanted, false)
            .or_else(|| {
                enumerate()
                    .ok()
                    .and_then(|all| select_device(all, wanted, true))
            })
            .ok_or_else(|| {
                Error::Unsupported(format!(
                    "no Razer headset HID interface found (VID 0x{RAZER_VID:04X}, PID {:?})",
                    wanted
                ))
            })?;
        Self::open_info(info)
    }

    pub fn open_info(info: HidInfo) -> Result<Self, Error> {
        let api = HidApi::new().map_err(|e| Error::Io(format!("hidapi init: {e}")))?;
        let cpath = CString::new(info.path.as_bytes())
            .map_err(|e| Error::Io(format!("invalid HID path: {e}")))?;
        let device = api
            .open_path(&cpath)
            .map_err(|e| Error::Io(format!("open {}: {e}", info.path)))?;
        Ok(Self {
            _api: api,
            device,
            info,
        })
    }

    pub fn info(&self) -> &HidInfo {
        &self.info
    }

    /// Read a HID feature report. The wired headset caches the last async
    /// event (battery or charging) in feature report 2; this is a read-only
    /// observation and does not disturb the event stream.
    pub fn read_feature_report(&self, report_id: u8, len: usize) -> Result<Vec<u8>, Error> {
        if len == 0 {
            return Err(Error::Io("feature report buffer is empty".into()));
        }
        let mut buffer = vec![0u8; len];
        buffer[0] = report_id;
        let read = self
            .device
            .get_feature_report(&mut buffer)
            .map_err(|e| Error::Io(format!("HID feature report: {e}")))?;
        buffer.truncate(read);
        Ok(buffer)
    }
}

impl Transport for HidTransport {
    fn send(&mut self, frame: &[u8; FRAME_LEN]) -> Result<(), Error> {
        let written = self
            .device
            .write(frame)
            .map_err(|e| Error::Io(format!("HID write: {e}")))?;
        if written != FRAME_LEN {
            return Err(Error::Io(format!(
                "short HID write: {written} of {FRAME_LEN} bytes"
            )));
        }
        Ok(())
    }

    fn recv(&mut self, timeout: Duration) -> Result<Option<[u8; FRAME_LEN]>, Error> {
        let mut buf = [0u8; FRAME_LEN];
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        let read = self
            .device
            .read_timeout(&mut buf, ms)
            .map_err(|e| Error::Io(format!("HID read: {e}")))?;
        if read == 0 {
            Ok(None)
        } else if read != FRAME_LEN {
            Err(Error::Io(format!(
                "short HID read: {read} of {FRAME_LEN} bytes"
            )))
        } else {
            Ok(Some(buf))
        }
    }
}

pub type HidDevice = SailDevice<HidTransport>;

/// HID transport for Razer protocol 2.5 mice. The control interface exposes a
/// 90-byte feature report (report ID 0): write the query, read the reply.
pub struct MouseHidTransport {
    // Keep the HidApi alive for the lifetime of the opened device.
    _api: HidApi,
    device: RawHidDevice,
    info: HidInfo,
}

/// HID feature reads include the leading report ID byte; protocol 2.5 reports
/// start at the report status byte.
fn mouse_report_from_feature_reply(reply: &[u8]) -> Result<[u8; REPORT_LEN], Error> {
    if reply.len() != REPORT_LEN + 1 {
        return Err(Error::Io(format!(
            "mouse feature report: expected {} bytes, got {}",
            REPORT_LEN + 1,
            reply.len()
        )));
    }
    let mut report = [0u8; REPORT_LEN];
    report.copy_from_slice(&reply[1..]);

    Ok(report)
}

/// Delay between SET_REPORT and GET_REPORT, taken from OpenRazer's receiver
/// waits: a wired Viper answers after ~31 ms, a Viper wireless receiver after
/// ~60 ms. Reading earlier hands back a BUSY (0x01) status with empty arguments.
const MOUSE_RECEIVER_WAIT: Duration = Duration::from_millis(31);
const WIRELESS_RECEIVER_WAIT: Duration = Duration::from_millis(60);

fn receiver_wait(product_id: u16) -> Duration {
    let wireless = model_for_product_id(product_id)
        .map(|model| model.wireless_pid == Some(product_id))
        .unwrap_or(false);
    if wireless {
        WIRELESS_RECEIVER_WAIT
    } else {
        MOUSE_RECEIVER_WAIT
    }
}

fn select_mouse(devices: Vec<HidInfo>, wanted: Option<u16>) -> Option<HidInfo> {
    select_model(devices.clone(), DeviceKind::Mouse, wanted, false)
        .or_else(|| select_model(devices, DeviceKind::Mouse, wanted, true))
}

fn select_mouse_device(pid: Option<u16>) -> Result<HidInfo, Error> {
    select_mouse(enumerate()?, pid).ok_or_else(|| {
        Error::Unsupported(format!(
            "no Razer protocol 2.5 mouse HID interface found (VID 0x{RAZER_VID:04X}, PID {pid:?})"
        ))
    })
}

impl MouseHidTransport {
    /// Enumerate Razer HID interfaces and open the best protocol 2.5 mouse.
    pub fn open(pid: Option<u16>) -> Result<Self, Error> {
        let info = select_mouse_device(pid)?;
        Self::open_info(info)
    }

    pub fn open_info(info: HidInfo) -> Result<Self, Error> {
        let api = HidApi::new().map_err(|e| Error::Io(format!("hidapi init: {e}")))?;
        let cpath = CString::new(info.path.as_bytes())
            .map_err(|e| Error::Io(format!("invalid HID path: {e}")))?;
        let device = api
            .open_path(&cpath)
            .map_err(|e| Error::Io(format!("open {}: {e}", info.path)))?;
        Ok(Self {
            _api: api,
            device,
            info,
        })
    }

    pub fn info(&self) -> &HidInfo {
        &self.info
    }
}

impl FeatureTransport for MouseHidTransport {
    fn exchange(&mut self, request: [u8; REPORT_LEN]) -> Result<[u8; REPORT_LEN], Error> {
        let mut frame = [0u8; REPORT_LEN + 1];
        frame[1..].copy_from_slice(&request);
        self.device
            .send_feature_report(&frame)
            .map_err(|e| Error::Io(format!("mouse HID feature write: {e}")))?;
        std::thread::sleep(receiver_wait(self.info.product_id));
        let mut reply = [0u8; REPORT_LEN + 1];
        let read = self
            .device
            .get_feature_report(&mut reply)
            .map_err(|e| Error::Io(format!("mouse HID feature read: {e}")))?;
        mouse_report_from_feature_reply(&reply[..read])
    }
}

/// Open a protocol 2.5 mouse and return its read-only telemetry handle.
pub fn open_mouse_device(pid: Option<u16>) -> Result<MouseDevice<MouseHidTransport>, Error> {
    MouseHidTransport::open(pid).map(MouseDevice::new)
}

pub fn open_device(pid: Option<u16>) -> Result<HidDevice, Error> {
    let transport = HidTransport::open(pid)?;
    let model = model_for_product_id(transport.info().product_id)
        .ok_or_else(|| Error::Unsupported("unknown headset model".into()))?;
    let wired = model.wired_pid == Some(transport.info().product_id);
    let device = SailDevice::new(transport);
    Ok(if wired {
        device.with_get_flags(0x00)
    } else {
        device
    })
}

pub fn enumerate() -> Result<Vec<HidInfo>, Error> {
    let api = HidApi::new().map_err(|e| Error::Io(format!("hidapi init: {e}")))?;
    Ok(api
        .device_list()
        .filter(|d| d.vendor_id() == RAZER_VID)
        .map(|d| HidInfo {
            path: d.path().to_string_lossy().into_owned(),
            product: d.product_string().map(ToOwned::to_owned),
            vendor_id: d.vendor_id(),
            product_id: d.product_id(),
            usage_page: d.usage_page(),
            usage: d.usage(),
            interface_number: d.interface_number(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sail_core::Command;

    fn info(product_id: u16, interface_number: i32, usage_page: u16) -> HidInfo {
        device_info(product_id, interface_number, usage_page, VENDOR_USAGE)
    }

    fn device_info(product_id: u16, interface_number: i32, usage_page: u16, usage: u16) -> HidInfo {
        HidInfo {
            path: format!("test-{product_id}-{interface_number}-{usage_page}"),
            product: Some("test".into()),
            vendor_id: RAZER_VID,
            product_id,
            usage_page,
            usage,
            interface_number,
        }
    }

    #[test]
    fn select_device_rejects_vendor_decoy() {
        let devices = vec![
            info(0x0F00, 3, VENDOR_USAGE_PAGE),
            info(PID_WIRELESS, 3, VENDOR_USAGE_PAGE),
        ];
        let selected = select_device(devices, None, false).expect("no selection");
        assert_eq!(selected.product_id, PID_WIRELESS);
    }

    #[test]
    fn select_device_prefers_interface_three_then_wireless() {
        let devices = vec![
            info(PID_WIRELESS, 4, VENDOR_USAGE_PAGE),
            info(PID_WIRED, 3, VENDOR_USAGE_PAGE),
            info(PID_WIRELESS, 3, VENDOR_USAGE_PAGE),
        ];
        let selected = select_device(devices, None, false).expect("no selection");
        assert_eq!(selected.product_id, PID_WIRELESS);
        assert_eq!(selected.interface_number, 3);
    }

    #[test]
    fn select_device_respects_requested_pid() {
        let devices = vec![
            info(PID_WIRELESS, 3, VENDOR_USAGE_PAGE),
            info(PID_WIRED, 3, VENDOR_USAGE_PAGE),
        ];
        let selected = select_device(devices, Some(PID_WIRED), false).expect("no selection");
        assert_eq!(selected.product_id, PID_WIRED);
    }

    #[test]
    fn select_device_skips_mice_when_no_pid_is_given() {
        let devices = vec![
            device_info(VIPER_V3_PRO_WIRED_PID, 0, 0x0001, 0x0002),
            info(PID_WIRELESS, 3, VENDOR_USAGE_PAGE),
        ];
        let selected = select_device(devices, None, false).expect("no selection");
        assert_eq!(selected.product_id, PID_WIRELESS);
    }

    #[test]
    fn select_mouse_prefers_the_profiles_wireless_pid() {
        let devices = vec![
            device_info(VIPER_V3_PRO_WIRED_PID, 0, 0x0001, 0x0002),
            device_info(VIPER_V3_PRO_WIRELESS_PID, 0, 0x0001, 0x0002),
        ];
        let selected = select_mouse(devices, None).expect("no selection");
        assert_eq!(selected.product_id, VIPER_V3_PRO_WIRELESS_PID);
    }

    #[test]
    fn select_mouse_finds_the_mouse_control_collection() {
        let devices = vec![
            device_info(VIPER_V3_PRO_WIRED_PID, 1, 0x0001, 0x0006),
            device_info(VIPER_V3_PRO_WIRED_PID, 0, 0x0001, 0x0002),
        ];
        let selected = select_mouse(devices, None).expect("no selection");
        assert_eq!(selected.interface_number, 0);
        assert_eq!(selected.usage, 0x0002);
    }

    #[test]
    fn mouse_feature_reply_drops_the_report_id_byte() {
        let mut reply = vec![0u8; sail_core::mouse::REPORT_LEN + 1];
        reply[1] = 0x02;
        reply[2] = 0x1F;
        let report = mouse_report_from_feature_reply(&reply).expect("valid reply");
        assert_eq!(report[0], 0x02);
        assert_eq!(report[1], 0x1F);
    }

    #[test]
    fn the_wireless_receiver_gets_a_longer_reply_wait() {
        assert_eq!(
            receiver_wait(VIPER_V3_PRO_WIRELESS_PID),
            WIRELESS_RECEIVER_WAIT
        );
        assert_eq!(receiver_wait(VIPER_V3_PRO_WIRED_PID), MOUSE_RECEIVER_WAIT);
    }

    #[test]
    fn mouse_feature_reply_rejects_short_buffers() {
        let short = vec![0u8; sail_core::mouse::REPORT_LEN];
        assert!(mouse_report_from_feature_reply(&short).is_err());
    }

    #[test]
    fn select_mouse_skips_headsets() {
        let devices = vec![
            info(PID_WIRELESS, 3, VENDOR_USAGE_PAGE),
            device_info(VIPER_V3_PRO_WIRED_PID, 0, 0x0001, 0x0002),
        ];
        let selected = select_mouse(devices, None).expect("no mouse selected");
        assert_eq!(selected.product_id, VIPER_V3_PRO_WIRED_PID);
    }

    #[test]
    #[ignore = "requires the physical headset"]
    fn opens_and_reads_battery() {
        let mut device = open_device(None).expect("headset not found");
        let battery = device.get(Command::Battery).expect("battery query failed");
        assert_eq!(battery.len(), 1);
        assert!(battery[0] <= 100);
    }

    #[test]
    #[ignore = "requires the physical headset"]
    fn enumerates_vendor_interface() {
        let devices = enumerate().expect("enumeration failed");
        assert!(
            devices
                .iter()
                .any(|d| d.usage_page == VENDOR_USAGE_PAGE && d.usage == VENDOR_USAGE),
            "no 0xFF14 interface: {devices:#?}"
        );
    }

    #[test]
    #[ignore = "requires the physical mouse"]
    fn opens_and_reads_mouse_battery() {
        let mut device = open_mouse_device(None).expect("mouse not found");
        let battery = device.battery_percent().expect("battery query failed");
        assert!(battery <= 100, "battery out of range: {battery}");
        let _charging = device.charging().expect("charging query failed");
    }
}
