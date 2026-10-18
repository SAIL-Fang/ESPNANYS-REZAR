//! SAIL protocol core for the Razer BlackShark V2 HyperSpeed.
//!
//! This crate is OS-independent: it only knows the 64-byte report format,
//! sequence/checksum rules, commands, responses and events. The Windows HID
//! transport lives in `sail_hid`.

use std::fmt;
use std::time::{Duration, Instant};

pub const REPORT_ID: u8 = 0x02;
pub const FRAME_LEN: usize = 64;
pub const DEFAULT_SEQUENCE: u8 = 0x60;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestClass {
    Get,
    Set1,
    Set10,
}

impl RequestClass {
    pub const fn class_bytes(self) -> (u8, u8) {
        match self {
            Self::Get => (0x04, 0x00),
            Self::Set1 => (0x05, 0x00),
            Self::Set10 => (0x0E, 0x00),
        }
    }

    pub const fn payload_len(self) -> usize {
        match self {
            Self::Get => 0,
            Self::Set1 => 1,
            Self::Set10 => 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Serial,
    Edition,
    Firmware,
    SpeakerPresetEq,
    Battery,
    Charging,
    AutoPowerOff,
    DongleLed,
    PresetEqIndex,
    EqGains,
    MicPresetEq,
    Sidetone,
    SidetoneVolume,
    SpeakerEqStatus,
}

impl Command {
    pub const fn code(self) -> u8 {
        match self {
            Self::Serial => 0x00,
            Self::Edition => 0x01,
            Self::Firmware => 0x02,
            Self::SpeakerPresetEq => 0x1E,
            Self::Battery => 0x21,
            Self::Charging => 0x2A,
            Self::AutoPowerOff => 0x2C,
            Self::DongleLed => 0x66,
            Self::PresetEqIndex => 0x93,
            Self::EqGains => 0x95,
            Self::MicPresetEq => 0x96,
            Self::Sidetone => 0x98,
            Self::SidetoneVolume => 0x99,
            Self::SpeakerEqStatus => 0x9E,
        }
    }

    pub const fn class(self) -> RequestClass {
        match self {
            Self::Serial
            | Self::Edition
            | Self::Firmware
            | Self::SpeakerPresetEq
            | Self::Battery
            | Self::Charging
            | Self::AutoPowerOff
            | Self::DongleLed => RequestClass::Get,
            Self::EqGains => RequestClass::Set10,
            Self::PresetEqIndex
            | Self::MicPresetEq
            | Self::Sidetone
            | Self::SidetoneVolume
            | Self::SpeakerEqStatus => RequestClass::Set1,
        }
    }

    pub const fn flags9(self) -> u8 {
        match self {
            Self::DongleLed => 0x00,
            // Wireless 0x0565 accepts 0x80 for get requests; wired 0x056E
            // requires 0x00. `SailDevice::with_get_flags` applies the wired
            // override after the transport knows the PID.
            _ => 0x80,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakerEqPreset {
    Game,
    Music,
    Cinema,
    Custom,
}

impl SpeakerEqPreset {
    pub const fn code(self) -> u8 {
        match self {
            Self::Game => 7,
            Self::Music => 8,
            Self::Cinema => 9,
            Self::Custom => 255,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "game" | "gaming" => Some(Self::Game),
            "music" => Some(Self::Music),
            "cinema" | "movie" => Some(Self::Cinema),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeakerEqGains {
    pub game: [i8; 10],
    pub music: [i8; 10],
    pub cinema: [i8; 10],
    pub custom: [i8; 10],
}

impl Default for SpeakerEqGains {
    fn default() -> Self {
        Self {
            game: [1, -5, -3, 0, 1, 1, 1, 1, 1, -3],
            music: [3, 3, 2, 0, 1, 1, 1, 1, 1, -3],
            cinema: [4, 4, 3, -4, -6, -3, 1, 3, 1, -1],
            custom: [3, 3, 2, 0, 1, 1, 1, 1, 1, -1],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadFrame(&'static str),
    BadPayload(&'static str),
    DeviceStatus { command: u8, status: u8 },
    Timeout,
    Disconnected,
    Io(String),
    Unsupported(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadFrame(reason) => write!(f, "bad frame: {reason}"),
            Self::BadPayload(reason) => write!(f, "bad payload: {reason}"),
            Self::DeviceStatus { command, status } => {
                write!(
                    f,
                    "device returned status {status} for command 0x{command:02X}"
                )
            }
            Self::Timeout => write!(f, "device timed out"),
            Self::Disconnected => write!(f, "device disconnected"),
            Self::Io(reason) => write!(f, "I/O error: {reason}"),
            Self::Unsupported(reason) => write!(f, "unsupported: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

pub fn checksum(frame: &[u8; FRAME_LEN]) -> u8 {
    let mut x = 0u8;
    for b in &frame[2..62] {
        x ^= *b;
    }
    x ^ 0x02
}

pub fn build_request(
    command: Command,
    sequence: u8,
    payload: &[u8],
) -> Result<[u8; FRAME_LEN], Error> {
    build_request_with_flags(command, sequence, payload, command.flags9())
}

pub fn build_request_with_flags(
    command: Command,
    sequence: u8,
    payload: &[u8],
    flags9: u8,
) -> Result<[u8; FRAME_LEN], Error> {
    let expected = command.class().payload_len();
    if payload.len() != expected {
        return Err(Error::BadPayload(
            "payload length does not match command class",
        ));
    }
    let mut frame = [0u8; FRAME_LEN];
    frame[0] = REPORT_ID;
    frame[1] = 0x00;
    frame[2] = sequence;
    let (class0, class1) = command.class().class_bytes();
    frame[6] = class0;
    frame[7] = class1;
    frame[9] = flags9;
    frame[10] = command.code();
    frame[11] = 0x00;
    frame[12] = payload.len() as u8;
    frame[13..13 + payload.len()].copy_from_slice(payload);
    frame[62] = checksum(&frame);
    Ok(frame)
}

#[derive(Debug, Clone)]
pub struct Response {
    pub sequence: u8,
    pub command: u8,
    pub status: u8,
    pub data: Vec<u8>,
    pub raw: [u8; FRAME_LEN],
}

impl Response {
    pub fn is_ok(&self) -> bool {
        self.status == 1
    }

    pub fn first_u8(&self) -> Option<u8> {
        self.data.first().copied()
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub record_id: u8,
    pub data: Vec<u8>,
    pub raw: [u8; FRAME_LEN],
}

impl Event {
    pub fn name(&self) -> &'static str {
        match self.record_id {
            0x20 => "wireless_connection_status",
            0x21 => "battery_status",
            0x27 => "do_not_disturb",
            0x2A => "charging_status",
            0x55 => "mic_status",
            _ => "unknown",
        }
    }
}

pub fn parse_response(frame: &[u8; FRAME_LEN]) -> Result<Option<Response>, Error> {
    if frame[0] != REPORT_ID {
        return Err(Error::BadFrame("missing report id 0x02"));
    }
    if frame[1] != 0x02 {
        return Ok(None);
    }
    let len = frame[12] as usize;
    if 13 + len > 63 {
        return Err(Error::BadFrame("response data length exceeds frame"));
    }
    Ok(Some(Response {
        sequence: frame[2],
        command: frame[10],
        status: frame[11],
        data: frame[13..13 + len].to_vec(),
        raw: *frame,
    }))
}

pub fn parse_event(frame: &[u8; FRAME_LEN]) -> Result<Option<Event>, Error> {
    if frame[0] != REPORT_ID {
        return Err(Error::BadFrame("missing report id 0x02"));
    }
    if frame[1] != 0x0A && frame[1] != 0x09 {
        return Ok(None);
    }
    let len = frame[12] as usize;
    if 13 + len > 63 {
        return Err(Error::BadFrame("event data length exceeds frame"));
    }
    Ok(Some(Event {
        record_id: frame[10],
        data: frame[13..13 + len].to_vec(),
        raw: *frame,
    }))
}

pub trait Transport {
    fn send(&mut self, frame: &[u8; FRAME_LEN]) -> Result<(), Error>;
    fn recv(&mut self, timeout: Duration) -> Result<Option<[u8; FRAME_LEN]>, Error>;
}

pub struct SailDevice<T: Transport> {
    transport: T,
    sequence: u8,
    timeout: Duration,
    get_flags9: u8,
}

impl<T: Transport> SailDevice<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            sequence: DEFAULT_SEQUENCE,
            timeout: DEFAULT_TIMEOUT,
            get_flags9: 0x80,
        }
    }

    /// Wired 0x056E wants `0x00` for get requests; wireless 0x0565 wants
    /// `0x80`. The transport layer knows the PID and applies the override.
    pub fn with_get_flags(mut self, flags9: u8) -> Self {
        self.get_flags9 = flags9;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn into_inner(self) -> T {
        self.transport
    }

    fn next_sequence(&mut self) -> u8 {
        self.sequence = self.sequence.wrapping_add(1);
        if self.sequence == 0 {
            self.sequence = 1;
        }
        self.sequence
    }

    /// Send a command and wait for the matching response. Device status is returned
    /// as a `Response` so callers can inspect "not supported" (status 5) explicitly.
    pub fn transact(&mut self, command: Command, payload: &[u8]) -> Result<Response, Error> {
        let sequence = self.next_sequence();
        let frame = build_request_with_flags(command, sequence, payload, self.get_flags9)?;
        self.transport.send(&frame)?;
        let deadline = Instant::now() + self.timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(Error::Timeout);
            }
            let remaining = deadline - now;
            match self.transport.recv(remaining)? {
                Some(raw) => {
                    if let Some(response) = parse_response(&raw)? {
                        if response.sequence == sequence && response.command == command.code() {
                            return Ok(response);
                        }
                    }
                }
                None => return Err(Error::Timeout),
            }
        }
    }

    pub fn get(&mut self, command: Command) -> Result<Vec<u8>, Error> {
        let response = self.transact(command, &[])?;
        if !response.is_ok() {
            return Err(Error::DeviceStatus {
                command: command.code(),
                status: response.status,
            });
        }
        Ok(response.data)
    }

    pub fn set1(&mut self, command: Command, value: u8) -> Result<(), Error> {
        let response = self.transact(command, &[value])?;
        if !response.is_ok() {
            return Err(Error::DeviceStatus {
                command: command.code(),
                status: response.status,
            });
        }
        Ok(())
    }

    pub fn set_sidetone(&mut self, enabled: bool) -> Result<(), Error> {
        self.set1(Command::Sidetone, u8::from(enabled))
    }

    pub fn set_sidetone_volume(&mut self, value: u8) -> Result<(), Error> {
        self.set1(Command::SidetoneVolume, value)
    }

    pub fn set_speaker_eq_preset(&mut self, preset: SpeakerEqPreset) -> Result<(), Error> {
        self.set1(Command::PresetEqIndex, preset.code())
    }

    pub fn set_speaker_eq_gains(&mut self, gains: &[i8; 10]) -> Result<(), Error> {
        let bytes: Vec<u8> = gains.iter().map(|v| *v as u8).collect();
        let response = self.transact(Command::EqGains, &bytes)?;
        if !response.is_ok() {
            return Err(Error::DeviceStatus {
                command: Command::EqGains.code(),
                status: response.status,
            });
        }
        Ok(())
    }

    pub fn read_event(&mut self, timeout: Duration) -> Result<Option<Event>, Error> {
        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let remaining = deadline - now;
            match self.transport.recv(remaining)? {
                Some(raw) => {
                    if let Some(event) = parse_event(&raw)? {
                        return Ok(Some(event));
                    }
                }
                None => return Ok(None),
            }
        }
    }

    pub fn read_status(&mut self) -> Result<DeviceStatus, Error> {
        let battery = self.get(Command::Battery)?.first().copied();
        let charging = self.get(Command::Charging)?.first().map(|v| *v != 0);
        let auto_power_off_min = self.get(Command::AutoPowerOff)?.first().copied();
        let dongle_led = self.get(Command::DongleLed)?.first().copied();
        let speaker_preset_eq = self.get(Command::SpeakerPresetEq)?.first().copied();
        let edition = self.get(Command::Edition)?.first().copied();
        let firmware = self
            .get(Command::Firmware)?
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(".");
        let serial = {
            let data = self.get(Command::Serial)?;
            let end = data.iter().position(|b| *b == 0).unwrap_or(data.len());
            String::from_utf8_lossy(&data[..end]).to_string()
        };

        Ok(DeviceStatus {
            battery,
            charging,
            auto_power_off_min,
            dongle_led,
            speaker_preset_eq,
            edition,
            firmware,
            serial,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceStatus {
    pub battery: Option<u8>,
    pub charging: Option<bool>,
    pub auto_power_off_min: Option<u8>,
    pub dongle_led: Option<u8>,
    pub speaker_preset_eq: Option<u8>,
    pub edition: Option<u8>,
    pub firmware: String,
    pub serial: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn response_frame(sequence: u8, command: u8, status: u8, data: &[u8]) -> [u8; 64] {
        let mut frame = [0u8; 64];
        frame[0] = REPORT_ID;
        frame[1] = 0x02;
        frame[2] = sequence;
        frame[6] = 0x05;
        frame[8] = 0x80;
        frame[9] = 0x80;
        frame[10] = command;
        frame[11] = status;
        frame[12] = data.len() as u8;
        frame[13..13 + data.len()].copy_from_slice(data);
        frame[62] = checksum(&frame);
        frame
    }

    #[test]
    fn checksum_matches_known_battery_response() {
        let frame = response_frame(0x61, 0x21, 1, &[39]);
        assert_eq!(frame[62], 0x60);
    }

    #[test]
    fn builds_get_battery_frame() {
        let frame = build_request(Command::Battery, 0x61, &[]).unwrap();
        assert_eq!(frame[0], 0x02);
        assert_eq!(frame[2], 0x61);
        assert_eq!(frame[6], 0x04);
        assert_eq!(frame[9], 0x80);
        assert_eq!(frame[10], 0x21);
        assert_eq!(frame[12], 0);
        assert_eq!(frame[62], checksum(&frame));
    }

    #[test]
    fn wired_battery_request_uses_zero_flag() {
        let frame = build_request_with_flags(Command::Battery, 0x61, &[], 0x00).unwrap();
        assert_eq!(frame[9], 0x00);
        assert_eq!(frame[10], 0x21);
    }

    #[test]
    fn parses_battery_response() {
        let frame = response_frame(0x61, 0x21, 1, &[39]);
        let response = parse_response(&frame).unwrap().unwrap();
        assert_eq!(response.sequence, 0x61);
        assert_eq!(response.command, 0x21);
        assert_eq!(response.first_u8(), Some(39));
        assert!(response.is_ok());
    }

    #[test]
    fn parses_battery_event() {
        let mut frame = [0u8; 64];
        frame[0] = 0x02;
        frame[1] = 0x0A;
        frame[10] = 0x21;
        frame[11] = 0x02;
        frame[12] = 1;
        frame[13] = 0x26;
        let event = parse_event(&frame).unwrap().unwrap();
        assert_eq!(event.record_id, 0x21);
        assert_eq!(event.data, vec![0x26]);
        assert_eq!(event.name(), "battery_status");
    }

    #[test]
    fn parses_charging_event() {
        let mut frame = [0u8; 64];
        frame[0] = 0x02;
        frame[1] = 0x0A;
        frame[10] = 0x2A;
        frame[11] = 0x02;
        frame[12] = 1;
        frame[13] = 1;
        let event = parse_event(&frame).unwrap().unwrap();
        assert_eq!(event.record_id, 0x2A);
        assert_eq!(event.data, vec![1]);
        assert_eq!(event.name(), "charging_status");
    }

    struct MockTransport {
        responses: VecDeque<[u8; 64]>,
        sent: Vec<[u8; 64]>,
    }

    impl MockTransport {
        fn new(responses: Vec<[u8; 64]>) -> Self {
            Self {
                responses: responses.into(),
                sent: Vec::new(),
            }
        }
    }

    impl Transport for MockTransport {
        fn send(&mut self, frame: &[u8; 64]) -> Result<(), Error> {
            self.sent.push(*frame);
            Ok(())
        }

        fn recv(&mut self, _timeout: Duration) -> Result<Option<[u8; 64]>, Error> {
            Ok(self.responses.pop_front())
        }
    }

    #[test]
    fn device_reads_status_from_mock_transport() {
        let responses = vec![
            response_frame(0x61, 0x21, 1, &[39]),
            response_frame(0x62, 0x2A, 1, &[0]),
            response_frame(0x63, 0x2C, 1, &[15]),
            response_frame(0x64, 0x66, 1, &[2]),
            response_frame(0x65, 0x1E, 1, &[0]),
            response_frame(0x66, 0x01, 1, &[0]),
            response_frame(0x67, 0x02, 1, &[1, 6, 11, 0]),
            response_frame(0x68, 0x00, 1, b"HN0000000000000"),
        ];
        let mut device = SailDevice::new(MockTransport::new(responses));
        let status = device.read_status().unwrap();
        assert_eq!(status.battery, Some(39));
        assert_eq!(status.charging, Some(false));
        assert_eq!(status.auto_power_off_min, Some(15));
        assert_eq!(status.dongle_led, Some(2));
        assert_eq!(status.speaker_preset_eq, Some(0));
        assert_eq!(status.edition, Some(0));
        assert_eq!(status.firmware, "1.6.11.0");
        assert_eq!(status.serial, "HN0000000000000");
    }

    #[test]
    fn unsupported_command_returns_status_error() {
        let responses = vec![response_frame(0x61, 0x96, 5, &[1])];
        let mut device = SailDevice::new(MockTransport::new(responses));
        let error = device.set1(Command::MicPresetEq, 0).unwrap_err();
        assert_eq!(
            error,
            Error::DeviceStatus {
                command: 0x96,
                status: 5
            }
        );
    }
}
