// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
//! Application-level orchestration for SAIL.
//!
//! This module owns the "what to do" decisions: which HID interface is
//! connected, how to degrade when one field times out, and how to talk to the
//! optional THX helper. `sail_core` stays protocol-only and `sail_hid` stays a
//! transport adapter; the CLI and Tauri are both thin callers here.

mod audio_route;

pub use audio_route::{AudioEndpoint, AudioOutput};
use sail_core::{
    parse_event, Command, Error as CoreError, Event, SailDevice, SpeakerEqPreset, Transport,
    FRAME_LEN,
};
use sail_hid::{HidTransport, ModelDescriptor, PID_WIRED, PID_WIRELESS};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Default, Clone, Copy)]
struct EventState {
    battery: Option<u8>,
    charging: Option<bool>,
}

static EVENT_STATE: OnceLock<Mutex<EventState>> = OnceLock::new();
static HID_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn event_state() -> &'static Mutex<EventState> {
    EVENT_STATE.get_or_init(|| Mutex::new(EventState::default()))
}

/// Serialize HID use across snapshot reads and setters. The UI polls every
/// 10 seconds and commands are async, so two handles can otherwise interleave
/// sequence numbers and steal input reports from each other.
fn hid_lock() -> std::sync::MutexGuard<'static, ()> {
    HID_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Remember an async event for later snapshots. Reading the feature-report
/// cache is cheap and does not require a background HID thread.
fn remember_event(event: &Event) {
    if let Ok(mut current) = event_state().lock() {
        match event.record_id {
            33 => current.battery = event.data.first().copied(),
            42 => current.charging = event.data.first().map(|value| *value != 0),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", content = "value", rename_all = "camelCase")]
pub enum Field<T> {
    Known(T),
    Unsupported,
    Timeout,
    Error(String),
}

impl<T> Field<T> {
    pub fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Link {
    Disconnected,
    /// The wireless receiver is present, but the headset itself did not answer
    /// the battery/charging handshake. This is the "receiver plugged in, headset
    /// powered off" state Razer shows as a red power icon.
    Standby,
    Wireless,
    Wired,
    Unknown,
}

impl Link {
    pub fn label(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Standby => "standby",
            Self::Wireless => "wireless",
            Self::Wired => "wired",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSnapshot {
    pub connected: bool,
    pub link: Link,
    pub model: ModelDescriptor,
    pub battery: Field<u8>,
    pub charging: Field<bool>,
    pub firmware: Field<String>,
    pub serial: Field<String>,
    pub auto_power_off_min: Field<u8>,
    pub dongle_led: Field<u8>,
    pub speaker_preset_eq: Field<u8>,
    pub edition: Field<u8>,
    pub audio_output: AudioOutput,
    pub issues: Vec<String>,
}

impl DeviceSnapshot {
    fn disconnected(reason: String) -> Self {
        Self {
            connected: false,
            link: Link::Disconnected,
            model: sail_hid::unknown_model().clone(),
            battery: Field::Error(reason.clone()),
            charging: Field::Error(reason.clone()),
            firmware: Field::Error(reason.clone()),
            serial: Field::Error(reason.clone()),
            auto_power_off_min: Field::Error(reason.clone()),
            dongle_led: Field::Error(reason.clone()),
            speaker_preset_eq: Field::Error(reason.clone()),
            edition: Field::Error(reason.clone()),
            audio_output: audio_route::current_output(),
            issues: vec![reason],
        }
    }

    /// The dongle is alive but the headset did not answer its own battery /
    /// charging handshake. Keep the receiver visible while making it explicit
    /// that the headset is off, instead of reporting a fake wireless link.
    fn standby(
        link: Link,
        model: ModelDescriptor,
        battery: Field<u8>,
        charging: Field<bool>,
        issues: Vec<String>,
    ) -> Self {
        Self {
            connected: false,
            link,
            model,
            battery,
            charging,
            firmware: Field::Timeout,
            serial: Field::Timeout,
            auto_power_off_min: Field::Timeout,
            dongle_led: Field::Timeout,
            speaker_preset_eq: Field::Timeout,
            edition: Field::Timeout,
            audio_output: audio_route::current_output(),
            issues,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EqPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub gains: [i8; 10],
}

/// The UI reads the default gains from here instead of keeping a second copy
/// in TypeScript. Labels live here for now; a future i18n layer can move them.
pub fn eq_presets() -> Vec<EqPreset> {
    let gains = sail_core::SpeakerEqGains::default();
    vec![
        EqPreset {
            id: "game",
            label: "游戏",
            gains: gains.game,
        },
        EqPreset {
            id: "music",
            label: "音乐",
            gains: gains.music,
        },
        EqPreset {
            id: "cinema",
            label: "电影",
            gains: gains.cinema,
        },
        EqPreset {
            id: "custom",
            label: "自定义",
            gains: gains.custom,
        },
    ]
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshot {
    pub device: Option<String>,
    pub voice_gate: Option<bool>,
    pub voice_gate_db: Option<f32>,
    pub noise_reduction: Option<bool>,
    pub noise_reduction_level: Option<i32>,
    pub normalization: Option<bool>,
    pub normalization_level: Option<i32>,
    pub vocal_clarity: Option<bool>,
    pub vocal_clarity_level: Option<i32>,
    pub volume: Option<i32>,
    pub sidetone: Option<bool>,
    pub sidetone_level: Option<i32>,
    pub preview: Option<bool>,
    pub eq_enabled: Option<bool>,
    pub eq_gains: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThxSnapshot {
    pub installed: bool,
    pub enabled: Option<bool>,
    pub level: Option<i32>,
    pub spatial_enabled: Option<bool>,
    pub endpoint: Option<String>,
    pub dll: Option<String>,
    pub dll_version: Option<String>,
    pub abi: Option<String>,
    #[serde(default)]
    pub capture: Option<CaptureSnapshot>,
    pub error: Option<String>,
}

impl ThxSnapshot {
    fn unavailable(error: String) -> Self {
        Self {
            installed: false,
            enabled: None,
            level: None,
            spatial_enabled: None,
            endpoint: None,
            dll: None,
            dll_version: None,
            abi: None,
            capture: None,
            error: Some(error),
        }
    }
}

#[derive(Debug)]
pub enum SailError {
    Device(CoreError),
    Invalid(String),
    Thx(String),
}

impl fmt::Display for SailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Device(error) => write!(f, "{error}"),
            Self::Invalid(reason) => write!(f, "{reason}"),
            Self::Thx(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for SailError {}

impl From<CoreError> for SailError {
    fn from(value: CoreError) -> Self {
        Self::Device(value)
    }
}

/// The seam to the optional THX control helper. The process implementation is
/// the shipping adapter; tests can inject a fake without loading the THX DLL.
pub trait ThxHost {
    fn status(&self) -> Result<ThxSnapshot, String>;
    fn set_level(&self, level: i32) -> Result<(), String>;
    fn set_enabled(&self, enabled: bool) -> Result<(), String>;
    fn set_spatial(&self, enabled: bool) -> Result<(), String>;
    fn mic(&self, args: &[String]) -> Result<(), String>;
    fn start_preview(&self) -> Result<(), String>;
    fn verify(&self) -> Result<String, String>;
}

pub struct ProcessThxHost {
    helper: Option<PathBuf>,
}

impl Default for ProcessThxHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessThxHost {
    pub fn new() -> Self {
        Self {
            helper: find_thx_helper(),
        }
    }

    fn run(&self, args: &[&str]) -> Result<String, String> {
        const TIMEOUT: Duration = Duration::from_secs(8);
        let helper = self
            .helper
            .as_ref()
            .ok_or_else(|| "THX helper not found; run scripts\\build.cmd -Stage".to_string())?;
        let mut child = command_for(helper)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("run {}: {error}", helper.display()))?;
        let deadline = Instant::now() + TIMEOUT;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut stdout = String::new();
                    let mut stderr = String::new();
                    if let Some(mut pipe) = child.stdout.take() {
                        pipe.read_to_string(&mut stdout)
                            .map_err(|error| format!("read THX stdout: {error}"))?;
                    }
                    if let Some(mut pipe) = child.stderr.take() {
                        pipe.read_to_string(&mut stderr)
                            .map_err(|error| format!("read THX stderr: {error}"))?;
                    }
                    if !status.success() {
                        let detail = stderr.trim();
                        return Err(if detail.is_empty() {
                            format!("THX helper failed with exit code {:?}", status.code())
                        } else {
                            format!(
                                "THX helper failed with exit code {:?}: {detail}",
                                status.code()
                            )
                        });
                    }
                    return Ok(stdout);
                }
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("THX helper timed out after {}s", TIMEOUT.as_secs()));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("wait THX helper: {error}"));
                }
            }
        }
    }

    fn run_mic(&self, args: &[String]) -> Result<(), String> {
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&borrowed).map(|_| ())
    }
}

impl ThxHost for ProcessThxHost {
    fn status(&self) -> Result<ThxSnapshot, String> {
        let raw = self.run(&["status", "--json"])?;
        let mut snapshot: ThxSnapshot =
            serde_json::from_str(&raw).map_err(|error| format!("parse THX JSON: {error}"))?;
        snapshot.installed = true;
        Ok(snapshot)
    }

    fn set_level(&self, level: i32) -> Result<(), String> {
        if !(1..=100).contains(&level) {
            return Err("normalization level must be 1..100".into());
        }
        self.run(&["level", &level.to_string()]).map(|_| ())
    }

    fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        self.run(&["enable", if enabled { "on" } else { "off" }])
            .map(|_| ())
    }

    fn set_spatial(&self, enabled: bool) -> Result<(), String> {
        self.run(&["spatial", if enabled { "on" } else { "off" }])
            .map(|_| ())
    }

    fn mic(&self, args: &[String]) -> Result<(), String> {
        self.run_mic(args)
    }

    fn start_preview(&self) -> Result<(), String> {
        let helper = self
            .helper
            .as_ref()
            .ok_or_else(|| "THX helper not found; run scripts\\build.cmd -Stage".to_string())?;
        // Keep this process alive while THX's preview worker runs. An
        // ephemeral helper crashes in the DLL at exit (0xC0000005) right after
        // SetCapturePreview(true), even though the preview itself started.
        command_for(helper)
            .args(["mic-preview-hold"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("start THX preview: {error}"))
    }

    fn verify(&self) -> Result<String, String> {
        self.run(&["verify"])
    }
}

fn command_for(path: &PathBuf) -> ProcessCommand {
    let mut command = ProcessCommand::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn find_thx_helper() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SAIL_THX_HOST") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    // Green build: sail_ui.exe, sail_thx.exe and sail_cli.exe live in the
    // same folder. Prefer the sibling helper so the folder can run without
    // install-sail.cmd or a machine-wide copy under %LOCALAPPDATA%.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in [
                dir.join("sail_thx.exe"),
                dir.join("payload").join("sail_thx.exe"),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
    let path = local.join("SAIL").join("bin").join("sail_thx.exe");
    path.is_file().then_some(path)
}

pub struct SailApp {
    thx: Box<dyn ThxHost>,
}

impl Default for SailApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SailApp {
    pub fn new() -> Self {
        Self {
            thx: Box::new(ProcessThxHost::new()),
        }
    }

    pub fn with_thx(host: Box<dyn ThxHost>) -> Self {
        Self { thx: host }
    }

    /// Read every field independently. A single timeout or unsupported command
    /// does not discard the fields that did respond.
    pub fn snapshot(&self) -> DeviceSnapshot {
        let _guard = hid_lock();
        let transport = match HidTransport::open(None) {
            Ok(transport) => transport,
            Err(error) => return DeviceSnapshot::disconnected(error.to_string()),
        };
        let link = match transport.info().product_id {
            PID_WIRELESS => Link::Wireless,
            PID_WIRED => Link::Wired,
            _ => Link::Unknown,
        };
        let model = sail_hid::model_for_product_id(transport.info().product_id)
            .cloned()
            .unwrap_or_else(|| sail_hid::unknown_model().clone());
        // The wired headset answers normal commands with silence, but caches
        // the last asynchronous event in feature report 2. Read it before any
        // command can overwrite that cache. Wireless uses the active GET path;
        // skipping the feature read there also avoids its unbounded hidapi call.
        let cached_event = if link == Link::Wired {
            cached_event(&transport)
        } else {
            None
        };
        let mut device = SailDevice::new(transport).with_timeout(Duration::from_millis(500));
        if link == Link::Wired {
            device = device.with_get_flags(0x00);
        }
        let mut issues = Vec::new();
        let mut battery = read_u8(&mut device, Command::Battery, &mut issues);
        // Wireless handshake: the receiver opens even when the headset is off.
        // Battery/charging are headset-side commands, so if both time out the
        // headset is not powered on. Stop after two short probes instead of
        // waiting for every unrelated command to time out.
        let mut charging_probe = None;
        if link == Link::Wireless && !battery.is_known() {
            let charging = read_bool(&mut device, Command::Charging, &mut issues);
            if !charging.is_known() {
                issues.push("headset handshake: battery/charging timed out".into());
                return DeviceSnapshot::standby(
                    Link::Standby,
                    model.clone(),
                    battery,
                    charging,
                    issues,
                );
            }
            charging_probe = Some(charging);
        }
        let mut charging = match charging_probe {
            Some(charging) => charging,
            None => read_bool(&mut device, Command::Charging, &mut issues),
        };
        if let Some(event) = cached_event {
            apply_event(&event, &mut battery, &mut charging);
            remember_event(&event);
        }
        if let Ok(current) = event_state().lock() {
            if !battery.is_known() {
                if let Some(level) = current.battery {
                    battery = Field::Known(level);
                }
            }
            if !charging.is_known() {
                if let Some(flag) = current.charging {
                    charging = Field::Known(flag);
                }
            }
        }
        let firmware = read_string(&mut device, Command::Firmware, &mut issues);
        let serial = read_serial(&mut device, &mut issues);
        let auto_power_off_min = read_u8(&mut device, Command::AutoPowerOff, &mut issues);
        let dongle_led = read_u8(&mut device, Command::DongleLed, &mut issues);
        let speaker_preset_eq = read_u8(&mut device, Command::SpeakerPresetEq, &mut issues);
        let edition = read_u8(&mut device, Command::Edition, &mut issues);

        DeviceSnapshot {
            connected: true,
            link,
            model,
            battery,
            charging,
            firmware,
            serial,
            auto_power_off_min,
            dongle_led,
            speaker_preset_eq,
            edition,
            audio_output: audio_route::current_output(),
            issues,
        }
    }

    pub fn thx_snapshot(&self) -> ThxSnapshot {
        match self.thx.status() {
            Ok(snapshot) => snapshot,
            Err(error) => ThxSnapshot::unavailable(error),
        }
    }

    /// Cheap write gate. The receiver can be open while the headset is off;
    /// never let a queued UI write touch EQ/sidetone/THX in that state.
    pub fn headset_online(&self) -> bool {
        let _guard = hid_lock();
        let Ok(transport) = HidTransport::open(None) else {
            return false;
        };
        let wired = transport.info().product_id == PID_WIRED;
        let mut device = SailDevice::new(transport).with_timeout(Duration::from_millis(600));
        if wired {
            return true;
        }
        device.get(Command::Battery).is_ok() || device.get(Command::Charging).is_ok()
    }

    pub fn require_online(&self) -> Result<(), SailError> {
        if self.headset_online() {
            Ok(())
        } else {
            Err(SailError::Invalid(
                "耳机未开机，设置已锁定；请先打开耳机电源".into(),
            ))
        }
    }

    pub fn set_default_audio_output(&self, device_id: Option<&str>) -> Result<(), String> {
        // An empty selector means "pick the first Razer headset endpoint".
        audio_route::set_default(device_id.unwrap_or(""))
    }

    pub fn eq_presets(&self) -> Vec<EqPreset> {
        eq_presets()
    }

    pub fn set_sidetone(&self, enabled: bool) -> Result<(), SailError> {
        let _guard = hid_lock();
        let mut device = sail_hid::open_device(None)?;
        device.set_sidetone(enabled)?;
        Ok(())
    }

    pub fn set_sidetone_volume(&self, value: u8) -> Result<(), SailError> {
        if value > 15 {
            return Err(SailError::Invalid("sidetone-volume is 0..15".into()));
        }
        let _guard = hid_lock();
        let mut device = sail_hid::open_device(None)?;
        device.set_sidetone_volume(value)?;
        Ok(())
    }

    pub fn set_eq_preset(&self, preset: &str) -> Result<(), SailError> {
        let preset = SpeakerEqPreset::from_name(preset)
            .ok_or_else(|| SailError::Invalid(format!("unknown EQ preset: {preset}")))?;
        let _guard = hid_lock();
        let mut device = sail_hid::open_device(None)?;
        device.set_speaker_eq_preset(preset)?;
        Ok(())
    }

    pub fn set_eq_gains(&self, gains: [i8; 10]) -> Result<(), SailError> {
        if gains.iter().any(|gain| !(-12..=12).contains(gain)) {
            return Err(SailError::Invalid("EQ gains must be -12..12".into()));
        }
        let _guard = hid_lock();
        let mut device = sail_hid::open_device(None)?;
        device.set_speaker_eq_gains(&gains)?;
        Ok(())
    }

    pub fn set_thx_level(&self, level: i32) -> Result<(), SailError> {
        self.thx.set_level(level).map_err(SailError::Thx)
    }

    pub fn set_thx_enabled(&self, enabled: bool) -> Result<(), SailError> {
        self.thx.set_enabled(enabled).map_err(SailError::Thx)
    }

    pub fn set_thx_spatial(&self, enabled: bool) -> Result<(), SailError> {
        self.thx.set_spatial(enabled).map_err(SailError::Thx)
    }

    pub fn verify_thx(&self) -> Result<String, SailError> {
        self.thx.verify().map_err(SailError::Thx)
    }

    /// The microphone feature set is exposed through the same checked helper
    /// command surface as the existing THX calls. Callers must pass one of the
    /// `mic-*` commands understood by `sail_thx`; the helper validates every
    /// range again.
    pub fn mic_command(&self, args: &[String]) -> Result<(), SailError> {
        self.thx.mic(args).map_err(SailError::Thx)
    }

    pub fn start_mic_preview(&self) -> Result<(), SailError> {
        self.thx.start_preview().map_err(SailError::Thx)
    }
}

fn cached_event(transport: &HidTransport) -> Option<Event> {
    let data = transport.read_feature_report(0x02, FRAME_LEN).ok()?;
    if data.len() != FRAME_LEN {
        return None;
    }
    let mut frame = [0u8; FRAME_LEN];
    frame.copy_from_slice(&data);
    parse_event(&frame).ok().flatten()
}

fn apply_event(event: &Event, battery: &mut Field<u8>, charging: &mut Field<bool>) {
    match event.record_id {
        33 if !battery.is_known() => {
            if let Some(level) = event.data.first() {
                *battery = Field::Known(*level);
            }
        }
        42 if !charging.is_known() => {
            if let Some(state) = event.data.first() {
                *charging = Field::Known(*state != 0);
            }
        }
        _ => {}
    }
}

fn link_error<T>(error: CoreError) -> Field<T> {
    match error {
        CoreError::Timeout => Field::Timeout,
        CoreError::Unsupported(_) => Field::Unsupported,
        CoreError::DeviceStatus { status: 5, .. } => Field::Unsupported,
        other => Field::Error(other.to_string()),
    }
}

fn read_u8<T: Transport>(
    device: &mut SailDevice<T>,
    command: Command,
    issues: &mut Vec<String>,
) -> Field<u8> {
    match device.get(command) {
        Ok(data) => match data.first().copied() {
            Some(value) => Field::Known(value),
            None => Field::Unsupported,
        },
        Err(error) => {
            issues.push(format!("{}: {error}", command.code()));
            link_error(error)
        }
    }
}

fn read_bool<T: Transport>(
    device: &mut SailDevice<T>,
    command: Command,
    issues: &mut Vec<String>,
) -> Field<bool> {
    match device.get(command) {
        Ok(data) => match data.first().copied() {
            Some(value) => Field::Known(value != 0),
            None => Field::Unsupported,
        },
        Err(error) => {
            issues.push(format!("{}: {error}", command.code()));
            link_error(error)
        }
    }
}

fn read_string<T: Transport>(
    device: &mut SailDevice<T>,
    command: Command,
    issues: &mut Vec<String>,
) -> Field<String> {
    match device.get(command) {
        Ok(data) => Field::Known(
            data.iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join("."),
        ),
        Err(error) => {
            issues.push(format!("{}: {error}", command.code()));
            link_error(error)
        }
    }
}

fn read_serial<T: Transport>(
    device: &mut SailDevice<T>,
    issues: &mut Vec<String>,
) -> Field<String> {
    match device.get(Command::Serial) {
        Ok(data) => {
            let end = data
                .iter()
                .position(|value| *value == 0)
                .unwrap_or(data.len());
            Field::Known(String::from_utf8_lossy(&data[..end]).to_string())
        }
        Err(error) => {
            issues.push(format!("00: {error}"));
            link_error(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeThx {
        status: ThxSnapshot,
    }

    impl ThxHost for FakeThx {
        fn status(&self) -> Result<ThxSnapshot, String> {
            Ok(self.status.clone())
        }
        fn set_level(&self, _level: i32) -> Result<(), String> {
            Ok(())
        }
        fn set_enabled(&self, _enabled: bool) -> Result<(), String> {
            Ok(())
        }
        fn set_spatial(&self, _enabled: bool) -> Result<(), String> {
            Ok(())
        }
        fn mic(&self, _args: &[String]) -> Result<(), String> {
            Ok(())
        }
        fn start_preview(&self) -> Result<(), String> {
            Ok(())
        }
        fn verify(&self) -> Result<String, String> {
            Ok("OK".into())
        }
    }

    #[test]
    fn field_serializes_as_tagged_state() {
        let value = Field::Known(14u8);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#"{"state":"known","value":14}"#);
        let timeout = serde_json::to_string(&Field::<u8>::Timeout).unwrap();
        assert_eq!(timeout, r#"{"state":"timeout"}"#);
    }

    #[test]
    fn thx_snapshot_uses_injected_host() {
        let app = SailApp::with_thx(Box::new(FakeThx {
            status: ThxSnapshot {
                installed: true,
                enabled: Some(true),
                level: Some(10),
                spatial_enabled: Some(true),
                endpoint: Some("endpoint".into()),
                dll: None,
                dll_version: None,
                abi: None,
                capture: None,
                error: None,
            },
        }));
        let snapshot = app.thx_snapshot();
        assert!(snapshot.installed);
        assert_eq!(snapshot.level, Some(10));
    }

    #[test]
    fn disconnected_snapshot_has_no_fake_values() {
        let snapshot = DeviceSnapshot::disconnected("no device".into());
        assert!(!snapshot.connected);
        assert_eq!(snapshot.link, Link::Disconnected);
        assert!(matches!(snapshot.battery, Field::Error(_)));
    }

    #[test]
    fn standby_snapshot_keeps_receiver_but_is_offline() {
        let snapshot = DeviceSnapshot::standby(
            Link::Standby,
            sail_hid::RAZER_BLACKSHARK_V2_HYPERSPEED.clone(),
            Field::Timeout,
            Field::Timeout,
            vec!["headset handshake: timeout".into()],
        );
        assert!(!snapshot.connected);
        assert_eq!(snapshot.link, Link::Standby);
        assert!(matches!(snapshot.battery, Field::Timeout));
        assert!(matches!(snapshot.firmware, Field::Timeout));
        assert_eq!(
            serde_json::to_string(&Link::Standby).unwrap(),
            "\"standby\""
        );
    }
}
