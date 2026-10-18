// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
//! Windows HID transport for the Razer BlackShark V2 HyperSpeed.
//!
//! Uses the mature `hidapi` crate instead of hand-rolled SetupAPI/HID calls.
//! The headset exposes its vendor control interface on usage page 0xFF14.

use hidapi::{HidApi, HidDevice as RawHidDevice};
use sail_core::{Error, SailDevice, Transport, FRAME_LEN};
use std::ffi::CString;
use std::time::Duration;

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

impl HidTransport {
    /// Enumerate Razer vendor HID interfaces and open the first matching device.
    /// Prefers the wireless PID unless one is supplied.
    pub fn open(pid: Option<u16>) -> Result<Self, Error> {
        let devices = enumerate()?;
        let wanted = pid;
        let info = devices
            .into_iter()
            .find(|d| {
                d.usage_page == VENDOR_USAGE_PAGE
                    && d.usage == VENDOR_USAGE
                    && wanted.map_or(true, |p| d.product_id == p)
            })
            .or_else(|| {
                let all = enumerate().ok()?;
                all.into_iter().find(|d| {
                    d.vendor_id == RAZER_VID
                        && matches!(d.product_id, PID_WIRELESS | PID_WIRED)
                        && wanted.map_or(true, |p| d.product_id == p)
                })
            })
            .ok_or_else(|| {
                Error::Unsupported(format!(
                    "no Razer vendor HID interface found (VID 0x{RAZER_VID:04X}, PID {:?})",
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
        } else {
            Ok(Some(buf))
        }
    }
}

pub type HidDevice = SailDevice<HidTransport>;

pub fn open_device(pid: Option<u16>) -> Result<HidDevice, Error> {
    let transport = HidTransport::open(pid)?;
    let wired = transport.info().product_id == PID_WIRED;
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
}
