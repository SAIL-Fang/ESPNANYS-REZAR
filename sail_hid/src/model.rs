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
    /// Render EQ is applied by THX rather than the headset HID channel.
    pub eq_via_thx: bool,
    pub sidetone: bool,
    pub thx: bool,
    pub mic_capture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceKind {
    /// Controllable audio device with a workspace.
    Headset,
    /// Telemetry-only pointer device: the hub shows a battery card.
    Mouse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub kind: DeviceKind,
    pub vendor_id: u16,
    pub wireless_pid: Option<u16>,
    pub wired_pid: Option<u16>,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
    pub eq_bands: &'static [u16],
    pub sidetone_max: Option<u8>,
    pub capabilities: ModelCapabilities,
}

pub static RAZER_BLACKSHARK_V2_HYPERSPEED: ModelDescriptor = ModelDescriptor {
    id: "razer.blackshark_v2_hyperspeed",
    display_name: "BlackShark V2 HyperSpeed",
    kind: DeviceKind::Headset,
    vendor_id: RAZER_VID,
    wireless_pid: Some(PID_WIRELESS),
    wired_pid: Some(PID_WIRED),
    usage_page: VENDOR_USAGE_PAGE,
    usage: VENDOR_USAGE,
    interface_number: 3,
    eq_bands: &[31, 63, 125, 250, 500, 1_000, 2_000, 4_000, 8_000, 16_000],
    sidetone_max: Some(15),
    capabilities: ModelCapabilities {
        battery: true,
        charging: true,
        firmware: true,
        serial: true,
        auto_power_off: true,
        dongle_led: true,
        speaker_eq: true,
        eq_via_thx: false,
        sidetone: true,
        thx: true,
        mic_capture: true,
    },
};

/// The analog BlackShark V2 reaches Windows through the Razer USB Sound Card.
/// There is no headset battery or `FF14` control channel on this path.
pub const PID_USB_SOUND_CARD: u16 = 0x0529;

pub static RAZER_BLACKSHARK_V2_USB_SOUND_CARD: ModelDescriptor = ModelDescriptor {
    id: "razer.blackshark_v2_usb_sound_card",
    display_name: "BlackShark V2 (USB Sound Card)",
    kind: DeviceKind::Headset,
    vendor_id: RAZER_VID,
    wireless_pid: None,
    wired_pid: Some(PID_USB_SOUND_CARD),
    usage_page: 0xFFA0,
    usage: 1,
    interface_number: 3,
    eq_bands: &[31, 63, 125, 250, 500, 1_000, 2_000, 4_000, 8_000, 16_000],
    sidetone_max: None,
    capabilities: ModelCapabilities {
        battery: false,
        charging: false,
        firmware: false,
        serial: false,
        auto_power_off: false,
        dongle_led: false,
        speaker_eq: true,
        eq_via_thx: true,
        sidetone: false,
        thx: true,
        mic_capture: true,
    },
};

/// The Viper V3 Pro exposes battery and charging only, over Razer protocol 2.5
/// (a 90-byte feature report on its primary mouse interface).
pub const VIPER_V3_PRO_WIRED_PID: u16 = 0x00C0;
pub const VIPER_V3_PRO_WIRELESS_PID: u16 = 0x00C1;

pub static RAZER_VIPER_V3_PRO: ModelDescriptor = ModelDescriptor {
    id: "razer.viper_v3_pro",
    display_name: "Viper V3 Pro",
    kind: DeviceKind::Mouse,
    vendor_id: RAZER_VID,
    wireless_pid: Some(VIPER_V3_PRO_WIRELESS_PID),
    wired_pid: Some(VIPER_V3_PRO_WIRED_PID),
    usage_page: 0x0001,
    usage: 0x0002,
    interface_number: 0,
    eq_bands: &[],
    sidetone_max: None,
    capabilities: ModelCapabilities {
        battery: true,
        charging: true,
        firmware: false,
        serial: false,
        auto_power_off: false,
        dongle_led: false,
        speaker_eq: false,
        eq_via_thx: false,
        sidetone: false,
        thx: false,
        mic_capture: false,
    },
};

pub static UNKNOWN_MODEL: ModelDescriptor = ModelDescriptor {
    id: "unknown",
    display_name: "未知耳机",
    kind: DeviceKind::Headset,
    vendor_id: 0,
    wireless_pid: None,
    wired_pid: None,
    usage_page: 0,
    usage: 0,
    interface_number: -1,
    eq_bands: &[],
    sidetone_max: None,
    capabilities: ModelCapabilities {
        battery: false,
        charging: false,
        firmware: false,
        serial: false,
        auto_power_off: false,
        dongle_led: false,
        speaker_eq: false,
        eq_via_thx: false,
        sidetone: false,
        thx: false,
        mic_capture: false,
    },
};

/// Resolve a product ID to its model descriptor. Unknown PIDs are never
/// guessed into a supported model.
pub fn model_for_product_id(product_id: u16) -> Option<&'static ModelDescriptor> {
    if RAZER_BLACKSHARK_V2_USB_SOUND_CARD.wired_pid == Some(product_id) {
        Some(&RAZER_BLACKSHARK_V2_USB_SOUND_CARD)
    } else if RAZER_VIPER_V3_PRO.wireless_pid == Some(product_id)
        || RAZER_VIPER_V3_PRO.wired_pid == Some(product_id)
    {
        Some(&RAZER_VIPER_V3_PRO)
    } else if RAZER_BLACKSHARK_V2_HYPERSPEED.wireless_pid == Some(product_id)
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
        assert_eq!(wireless.sidetone_max, Some(15));
    }

    #[test]
    fn usb_sound_card_pid_maps_to_the_analog_profile() {
        let model = model_for_product_id(PID_USB_SOUND_CARD).expect("usb sound card model");
        assert_eq!(model.id, RAZER_BLACKSHARK_V2_USB_SOUND_CARD.id);
        assert_eq!(model.usage_page, 0xFFA0);
        assert!(!model.capabilities.battery);
        assert!(!model.capabilities.sidetone);
        assert!(model.capabilities.eq_via_thx);
    }

    #[test]
    fn unknown_pid_is_not_guessed() {
        assert!(model_for_product_id(0xFFFF).is_none());
        assert_eq!(unknown_model().id, "unknown");
    }

    #[test]
    fn viper_v3_pro_pids_map_to_the_mouse_profile() {
        for pid in [VIPER_V3_PRO_WIRED_PID, VIPER_V3_PRO_WIRELESS_PID] {
            let model = model_for_product_id(pid).expect("viper model");
            assert_eq!(model.id, RAZER_VIPER_V3_PRO.id);
            assert_eq!(model.kind, DeviceKind::Mouse);
            assert_eq!(model.wireless_pid, Some(VIPER_V3_PRO_WIRELESS_PID));
            assert_eq!(model.wired_pid, Some(VIPER_V3_PRO_WIRED_PID));
            assert_eq!(model.usage_page, 0x0001);
            assert_eq!(model.usage, 0x0002);
            assert_eq!(model.interface_number, 0);
            assert!(model.eq_bands.is_empty());
            assert!(model.capabilities.battery);
            assert!(model.capabilities.charging);
            assert!(!model.capabilities.thx);
            assert!(!model.capabilities.speaker_eq);
        }
    }

    #[test]
    fn headsets_declare_the_headset_kind() {
        for pid in [PID_WIRELESS, PID_WIRED] {
            assert_eq!(
                model_for_product_id(pid).expect("headset model").kind,
                DeviceKind::Headset
            );
        }
        assert_eq!(
            model_for_product_id(PID_USB_SOUND_CARD)
                .expect("usb sound card model")
                .kind,
            DeviceKind::Headset
        );
        assert_eq!(unknown_model().kind, DeviceKind::Headset);
    }
}
