// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
//! Razer Protocol 2.5: the 90-byte feature-report format used by Razer mice.
//!
//! This is deliberately separate from the headset's 64-byte interrupt-report
//! protocol in the parent module. The two products share a vendor ID, not a
//! wire format.

/// Feature-report length declared by the Viper V3 Pro control interface.
pub const REPORT_LEN: usize = 90;
/// `struct razer_report.arguments`: the payload area of the report.
pub const ARGUMENTS_LEN: usize = 80;
/// Command class shared by the battery/charging telemetry queries.
pub const COMMAND_CLASS: u8 = 0x07;
/// Battery level query (`razer_chroma_misc_get_battery_level`).
pub const BATTERY_LEVEL: u8 = 0x80;
/// Charging status query (`razer_chroma_misc_get_charging_status`).
pub const CHARGING_STATUS: u8 = 0x84;
/// Transaction id the Viper V3 Pro expects for telemetry queries.
pub const DEFAULT_TRANSACTION_ID: u8 = 0x1F;
/// Report status: the device answered the query.
pub const STATUS_SUCCESSFUL: u8 = 0x02;
/// Report status: the device is busy; the same query may be retried.
pub const STATUS_BUSY: u8 = 0x01;
/// Report status: the device timed out the command; retrying is allowed.
pub const STATUS_TIMEOUT: u8 = 0x04;
/// How often a busy or silent device is queried before giving up.
const ATTEMPTS: usize = 3;
/// Razer's driver retries follow-up queries after 10 ms.
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(10);

use crate::Error;

const CRC_START: usize = 2;
const CRC_END: usize = 88;

/// Razer's checksum: XOR every byte after the transaction id, before the CRC.
pub fn checksum(report: &[u8; REPORT_LEN]) -> u8 {
    report[CRC_START..CRC_END]
        .iter()
        .fold(0u8, |crc, byte| crc ^ byte)
}

/// Build a telemetry query. The query carries no arguments, only the command.
pub fn build_request(command: u8, transaction_id: u8) -> [u8; REPORT_LEN] {
    let mut report = [0u8; REPORT_LEN];
    report[1] = transaction_id;
    report[5] = 0x02; // data size
    report[6] = COMMAND_CLASS;
    report[7] = command;
    report[88] = checksum(&report);
    report
}

/// A successful reply to a protocol 2.5 query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub command: u8,
    pub arguments: [u8; ARGUMENTS_LEN],
}

/// Validate a reply and expose its command and argument bytes.
///
/// Only a successful reply is a `Response`; busy and timed-out replies become
/// `Error::DeviceStatus` so the caller can retry them.
pub fn parse_response(report: &[u8; REPORT_LEN]) -> Result<Response, Error> {
    if checksum(report) != report[88] {
        return Err(Error::BadFrame("mouse reply checksum mismatch"));
    }
    if report[6] != COMMAND_CLASS {
        return Err(Error::BadFrame("mouse reply command class mismatch"));
    }
    if report[0] != STATUS_SUCCESSFUL {
        return Err(Error::DeviceStatus {
            command: report[7],
            status: report[0],
        });
    }
    let mut arguments = [0u8; ARGUMENTS_LEN];
    arguments.copy_from_slice(&report[8..8 + ARGUMENTS_LEN]);
    Ok(Response {
        command: report[7],
        arguments,
    })
}

/// Razer reports battery as 0-255; the UI shows whole percent.
pub fn battery_percent(raw: u8) -> u8 {
    ((u16::from(raw) * 100 + 127) / 255) as u8
}

/// The transport-level seam: swap the request for a reply of the same size.
///
/// The HID implementation lives in `sail_hid`; tests inject canned replies.
pub trait FeatureTransport {
    fn exchange(&mut self, request: [u8; REPORT_LEN]) -> Result<[u8; REPORT_LEN], Error>;
}

/// Read-only mouse telemetry the app can ask for without knowing the transport.
pub trait Telemetry {
    fn battery_percent(&mut self) -> Result<u8, Error>;
    fn charging(&mut self) -> Result<bool, Error>;
}

/// A mouse addressed over Razer protocol 2.5.
pub struct MouseDevice<T: FeatureTransport> {
    transport: T,
    transaction_id: u8,
}

impl<T: FeatureTransport> MouseDevice<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            transaction_id: DEFAULT_TRANSACTION_ID,
        }
    }

    pub fn battery_percent(&mut self) -> Result<u8, Error> {
        self.read_argument(BATTERY_LEVEL).map(battery_percent)
    }

    pub fn charging(&mut self) -> Result<bool, Error> {
        self.read_argument(CHARGING_STATUS).map(|value| value != 0)
    }

    /// Query `command` and return the second argument byte, retrying while the
    /// device is busy or silent.
    fn read_argument(&mut self, command: u8) -> Result<u8, Error> {
        let request = build_request(command, self.transaction_id);
        let mut last_error = Error::Timeout;
        for attempt in 0..ATTEMPTS {
            let reply = match self.transport.exchange(request) {
                Ok(report) => report,
                Err(Error::Timeout) => {
                    self.retry_delay(attempt);
                    continue;
                }
                Err(error) => return Err(error),
            };
            match parse_response(&reply) {
                Ok(response) => {
                    if response.command != command {
                        return Err(Error::BadFrame("mouse reply command mismatch"));
                    }
                    return Ok(response.arguments[1]);
                }
                Err(error @ Error::DeviceStatus { status, .. })
                    if status == STATUS_BUSY || status == STATUS_TIMEOUT =>
                {
                    last_error = error;
                    self.retry_delay(attempt);
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error)
    }

    /// Back off between attempts, but not after the final one.
    fn retry_delay(&self, attempt: usize) {
        if attempt + 1 < ATTEMPTS {
            std::thread::sleep(RETRY_DELAY);
        }
    }
}

impl<T: FeatureTransport> Telemetry for MouseDevice<T> {
    fn battery_percent(&mut self) -> Result<u8, Error> {
        MouseDevice::battery_percent(self)
    }

    fn charging(&mut self) -> Result<bool, Error> {
        MouseDevice::charging(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured on the physical Viper V3 Pro (PID 0x00C0) while the mouse was
    /// awake. Layout: status, transaction id, remaining packets, protocol type,
    /// data size, command class, command id, 80 argument bytes, CRC, reserved.
    fn captured_battery_request() -> [u8; REPORT_LEN] {
        let mut report = [0u8; REPORT_LEN];
        report[1] = 0x1F; // transaction id used by the Viper V3 Pro
        report[5] = 0x02; // data size
        report[6] = 0x07; // command class
        report[7] = 0x80; // battery level
        report[88] = 0x85; // captured checksum
        report
    }

    #[test]
    fn battery_request_matches_the_captured_wire_format() {
        assert_eq!(
            build_request(BATTERY_LEVEL, 0x1F),
            captured_battery_request()
        );
    }

    #[test]
    fn checksum_is_the_captured_xor() {
        assert_eq!(checksum(&captured_battery_request()), 0x85);
        let mut charging = captured_battery_request();
        charging[7] = CHARGING_STATUS;
        assert_eq!(checksum(&charging), 0x81);
    }

    /// Captured on the physical Viper V3 Pro as the reply to the request above.
    fn captured_battery_reply() -> [u8; REPORT_LEN] {
        let mut report = [0u8; REPORT_LEN];
        report[0] = STATUS_SUCCESSFUL;
        report[1] = 0x1F; // transaction id echoed
        report[5] = 0x02; // data size
        report[6] = COMMAND_CLASS;
        report[7] = BATTERY_LEVEL;
        report[8] = 0x00; // arguments[0]
        report[9] = 0x67; // arguments[1]: 103 out of 255
        report[88] = 0xE2; // captured checksum
        report
    }

    #[test]
    fn parse_response_reads_the_captured_battery_level() {
        let response = parse_response(&captured_battery_reply()).unwrap();
        assert_eq!(response.command, BATTERY_LEVEL);
        assert_eq!(response.arguments[1], 0x67);
    }

    #[test]
    fn parse_response_rejects_a_corrupted_checksum() {
        let mut report = captured_battery_reply();
        report[88] ^= 0xFF;
        assert!(matches!(parse_response(&report), Err(Error::BadFrame(_))));
    }

    #[test]
    fn battery_percent_maps_the_raw_scale_to_percent() {
        assert_eq!(battery_percent(0x00), 0);
        assert_eq!(battery_percent(0x67), 40);
        assert_eq!(battery_percent(0x69), 41);
        assert_eq!(battery_percent(0xFF), 100);
    }

    /// A reply with the device-busy status. The checksum follows the same
    /// documented XOR rule as the captured reports.
    fn busy_reply() -> [u8; REPORT_LEN] {
        let mut report = [0u8; REPORT_LEN];
        report[0] = STATUS_BUSY;
        report[1] = 0x1F;
        report[5] = 0x02;
        report[6] = COMMAND_CLASS;
        report[7] = BATTERY_LEVEL;
        report[88] = 0x85;
        report
    }

    #[test]
    fn a_busy_reply_is_retried() {
        let mut device = MouseDevice::new(FakeTransport::new(vec![
            Ok(busy_reply()),
            Ok(captured_battery_reply()),
        ]));
        assert_eq!(device.battery_percent(), Ok(40));
    }

    #[test]
    fn a_failed_reply_is_not_retried() {
        let mut report = busy_reply();
        report[0] = 0x05; // not supported; the status byte is outside the CRC range
        let mut device = MouseDevice::new(FakeTransport::new(vec![Ok(report)]));
        assert!(matches!(
            device.battery_percent(),
            Err(Error::DeviceStatus { status: 0x05, .. })
        ));
        assert_eq!(device.transport.sent.len(), 1, "a terminal status is final");
    }

    #[test]
    fn a_missing_reply_times_out_after_retries() {
        let mut device = MouseDevice::new(FakeTransport::new(vec![
            Err(Error::Timeout),
            Err(Error::Timeout),
            Err(Error::Timeout),
        ]));
        assert_eq!(device.battery_percent(), Err(Error::Timeout));
        assert_eq!(
            device.transport.sent.len(),
            ATTEMPTS,
            "every attempt is sent"
        );
    }

    #[test]
    fn charging_reply_maps_to_a_boolean() {
        let mut report = captured_battery_reply();
        report[7] = CHARGING_STATUS;
        report[9] = 0x01;
        report[88] = 0x80; // documented XOR rule, not recomputed by the test
        let mut device = MouseDevice::new(FakeTransport::new(vec![Ok(report)]));
        assert_eq!(device.charging(), Ok(true));
    }

    struct FakeTransport {
        replies: std::collections::VecDeque<Result<[u8; REPORT_LEN], Error>>,
        sent: Vec<[u8; REPORT_LEN]>,
    }

    impl FakeTransport {
        fn new(replies: Vec<Result<[u8; REPORT_LEN], Error>>) -> Self {
            Self {
                replies: replies.into(),
                sent: Vec::new(),
            }
        }
    }

    impl FeatureTransport for FakeTransport {
        fn exchange(&mut self, request: [u8; REPORT_LEN]) -> Result<[u8; REPORT_LEN], Error> {
            self.sent.push(request);
            self.replies.pop_front().unwrap_or(Err(Error::Timeout))
        }
    }
}
