// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
//! Model descriptors isolate product identity from protocol code. A new
//! headset should add a descriptor/profile, not edit the HID selector or UI.

use crate::{PID_WIRED, PID_WIRELESS, RAZER_VID, VENDOR_USAGE, VENDOR_USAGE_PAGE};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    pub battery: bool,
    pub charging: bool,
    pub firmware: bool,
    pub serial: bool,
    pub auto_power_off: bool,
    pub dongle_led: bool,
    pub speaker_eq: bool,
    pub sidetone: bool,
    pub thx: bool,
    pub mic_capture: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub vendor_id: u16,
    pub wireless_pid: Option<u16>,
    pub wired_pid: Option<u16>,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
    pub eq_bands: &'static [u16],
    pub capabilities: ModelCapabilities,
}

pub static RAZER_BLACKSHARK_V2_HYPERSPEED: ModelDescriptor = ModelDescriptor {
    id: "razer.blackshark_v2_hyperspeed",
    display_name: "BlackShark V2 HyperSpeed",
    vendor_id: RAZER_VID,
    wireless_pid: Some(PID_WIRELESS),
    wired_pid: Some(PID_WIRED),
    usage_page: VENDOR_USAGE_PAGE,
    usage: VENDOR_USAGE,
    interface_number: 3,
    eq_bands: &[31, 63, 125, 250, 500, 1_000, 2_000, 4_000, 8_000, 16_000],
    capabilities: ModelCapabilities {
        battery: true,
        charging: true,
        firmware: true,
        serial: true,
        auto_power_off: true,
        dongle_led: true,
        speaker_eq: true,
        sidetone: true,
        thx: true,
        mic_capture: true,
    },
};

pub static UNKNOWN_MODEL: ModelDescriptor = ModelDescriptor {
    id: "unknown",
    display_name: "未知耳机",
    vendor_id: 0,
    wireless_pid: None,
    wired_pid: None,
    usage_page: 0,
    usage: 0,
    interface_number: -1,
    eq_bands: &[],
    capabilities: ModelCapabilities {
        battery: false,
        charging: false,
        firmware: false,
        serial: false,
        auto_power_off: false,
        dongle_led: false,
        speaker_eq: false,
        sidetone: false,
        thx: false,
        mic_capture: false,
    },
};

/// Resolve a product ID to its model descriptor. Unknown PIDs are never
/// guessed into a supported model.
pub fn model_for_product_id(product_id: u16) -> Option<&'static ModelDescriptor> {
    if RAZER_BLACKSHARK_V2_HYPERSPEED.wireless_pid == Some(product_id)
        || RAZER_BLACKSHARK_V2_HYPERSPEED.wired_pid == Some(product_id)
    {
        Some(&RAZER_BLACKSHARK_V2_HYPERSPEED)
    } else {
        None
    }
}

pub fn unknown_model() -> &'static ModelDescriptor {
    &UNKNOWN_MODEL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_pids_map_to_the_razer_profile() {
        let wireless = model_for_product_id(PID_WIRELESS).expect("wireless model");
        let wired = model_for_product_id(PID_WIRED).expect("wired model");
        assert_eq!(wireless.id, RAZER_BLACKSHARK_V2_HYPERSPEED.id);
        assert_eq!(wired.id, RAZER_BLACKSHARK_V2_HYPERSPEED.id);
        assert_eq!(wireless.eq_bands.len(), 10);
    }

    #[test]
    fn unknown_pid_is_not_guessed() {
        assert!(model_for_product_id(0xFFFF).is_none());
        assert_eq!(unknown_model().id, "unknown");
    }
}
