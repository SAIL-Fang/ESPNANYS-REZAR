// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
use sail_app::{DeviceSnapshot, Field, Link, SailApp};
use sail_core::{Command, SailDevice, SpeakerEqPreset};
use sail_hid::{enumerate, open_device, HidDevice, HidTransport};
use std::env;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(args) {
        eprintln!("sail: {error}");
        std::process::exit(1);
    }
}

fn run(mut args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut pid = None;
    if let Some(index) = args.iter().position(|a| a == "--pid") {
        if index + 1 >= args.len() {
            return Err("--pid needs a value".into());
        }
        let raw = args.remove(index + 1);
        args.remove(index);
        pid = Some(parse_u16(&raw)?);
    }

    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "list" => {
            for info in enumerate()? {
                println!(
                    "{:04X}:{:04X} usage={:04X}/{:04X} if={} {}{}",
                    info.vendor_id,
                    info.product_id,
                    info.usage_page,
                    info.usage,
                    info.interface_number,
                    info.product.as_deref().unwrap_or("(no product string)"),
                    if info.usage_page == 0xFF14 && info.usage == 1 {
                        "  [vendor control]"
                    } else {
                        ""
                    }
                );
                println!("  {}", info.path);
            }
            return Ok(());
        }
        "help" | "--help" | "-h" => {
            print_help();
            return Ok(());
        }
        "probe" => {
            for info in enumerate()? {
                if info.usage_page != 0xFF14 || info.usage != 1 {
                    continue;
                }
                println!(
                    "probe {:04X}:{:04X} if={} {}",
                    info.vendor_id, info.product_id, info.interface_number, info.path
                );
                match HidTransport::open_info(info) {
                    Ok(transport) => {
                        let wired = transport.info().product_id == 0x056E;
                        let base = SailDevice::new(transport);
                        let mut device = if wired {
                            base.with_get_flags(0x00)
                        } else {
                            base
                        };
                        match device.get(Command::Battery) {
                            Ok(value) => println!("  response: {:02X?}", value),
                            Err(error) => println!("  no response: {error}"),
                        }
                    }
                    Err(error) => println!("  open failed: {error}"),
                }
            }
            return Ok(());
        }
        "status" if pid.is_none() => {
            print_snapshot(&SailApp::new().snapshot());
            return Ok(());
        }
        "audio-default" if pid.is_none() => {
            let id = args.get(1).map(String::as_str);
            SailApp::new()
                .set_default_audio_output(id)
                .map_err(std::io::Error::other)?;
            println!("default audio output switched");
            return Ok(());
        }
        "mic-status" if pid.is_none() => {
            let capture = SailApp::new().thx_snapshot().capture;
            println!(
                "{}",
                serde_json::to_string(&capture).map_err(std::io::Error::other)?
            );
            return Ok(());
        }
        "mic-gate" | "mic-nr" | "mic-norm" | "mic-clarity" | "mic-eq" | "mic-eq-toggle"
        | "mic-volume" | "mic-preview" | "mic-sidetone"
            if pid.is_none() =>
        {
            SailApp::new()
                .mic_command(&args)
                .map_err(std::io::Error::other)?;
            println!("{command} ok");
            return Ok(());
        }
        _ => {}
    }

    let mut device = open_device(pid)?;
    match command {
        "status" => print_status(&mut device)?,
        "battery" => println!("{}", get_u8(&mut device, Command::Battery)?),
        "charging" => println!(
            "{}",
            if get_u8(&mut device, Command::Charging)? != 0 {
                "on"
            } else {
                "off"
            }
        ),
        "firmware" => println!("{}", get_string(&mut device, Command::Firmware)?),
        "serial" => println!("{}", get_string(&mut device, Command::Serial)?),
        "sidetone" => {
            let value = args.get(1).ok_or("sidetone needs on|off")?;
            let enabled = match value.as_str() {
                "on" => true,
                "off" => false,
                _ => return Err("sidetone needs on|off".into()),
            };
            device.set_sidetone(enabled)?;
            println!("sidetone {}", if enabled { "on" } else { "off" });
        }
        "sidetone-volume" => {
            let value = parse_u8(args.get(1).ok_or("sidetone-volume needs 0..15")?)?;
            if value > 15 {
                return Err("sidetone-volume is 0..15".into());
            }
            device.set_sidetone_volume(value)?;
            println!("sidetone-volume {value}");
        }
        "eq-preset" => {
            let value = args
                .get(1)
                .ok_or("eq-preset needs game|music|cinema|custom|N")?;
            let preset = parse_preset(value)?;
            device.set_speaker_eq_preset(preset)?;
            println!("eq-preset {:?} ({})", preset, preset.code());
        }
        "eq-gains" => {
            let values = &args[1..];
            if values.len() != 10 {
                return Err("eq-gains needs 10 signed values".into());
            }
            let mut gains = [0i8; 10];
            for (slot, value) in gains.iter_mut().zip(values) {
                *slot = value.parse::<i8>()?;
                if !(-12..=12).contains(slot) {
                    return Err("each EQ gain must be -12..12".into());
                }
            }
            device.set_speaker_eq_gains(&gains)?;
            println!("eq-gains {gains:?}");
        }
        "watch" => {
            let count = args
                .iter()
                .position(|a| a == "--count")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.parse::<usize>())
                .transpose()?
                .unwrap_or(usize::MAX);
            let mut seen = 0usize;
            while seen < count {
                if let Some(event) = device.read_event(Duration::from_secs(30))? {
                    seen += 1;
                    println!(
                        "event {} record=0x{:02X} len={} data={:02X?}",
                        event.name(),
                        event.record_id,
                        event.data.len(),
                        event.data
                    );
                }
            }
        }
        other => return Err(format!("unknown command: {other}").into()),
    }
    Ok(())
}

fn print_help() {
    println!(
        r#"sail_cli — SAIL headset control

Usage:
  sail_cli [--pid 0x0565|0x056E] <command>

Commands:
  list                       enumerate Razer HID interfaces
  probe                      try a battery read on every vendor interface
  status                     battery/charging/firmware/serial/auto-off
  battery                    battery percent
  charging                   charging state
  firmware                   firmware version
  serial                     serial number
  sidetone on|off            hardware sidetone
  sidetone-volume 0..15      hardware sidetone volume
  eq-preset game|music|cinema|custom|7|8|9|255
  eq-gains v0 ... v9         ten signed dB values
  audio-default [endpoint-id] switch default render device (headset if omitted)
  mic-status                 THX microphone capture state as JSON
  mic-gate on|off [db]       noise gate, db -60..-20
  mic-nr on|off [0..100]     microphone noise reduction
  mic-norm on|off [1..100]   microphone volume normalization
  mic-clarity on|off [0..100] vocal clarity
  mic-eq v0 ... v9           ten capture EQ dB values
  mic-eq-toggle on|off       enable/disable capture EQ
  mic-volume 0..100          system microphone volume via THX
  mic-preview on|off          THX microphone preview
  mic-sidetone on|off         THX capture sidetone chain
  watch [--count N]          print device events

Examples:
  sail_cli status
  sail_cli audio-default
  sail_cli eq-preset game
  sail_cli eq-gains 1 -5 -3 0 1 1 1 1 1 -3
"#
    );
}

fn field_display<T: std::fmt::Display>(field: &Field<T>) -> String {
    match field {
        Field::Known(value) => value.to_string(),
        Field::Unsupported => "unsupported".into(),
        Field::Timeout => "timeout".into(),
        Field::Error(error) => format!("error({error})"),
    }
}

fn link_display(link: Link) -> &'static str {
    match link {
        Link::Disconnected => "disconnected",
        Link::Wireless => "wireless",
        Link::Wired => "wired",
        Link::Unknown => "unknown",
    }
}

fn print_snapshot(snapshot: &DeviceSnapshot) {
    println!("connected: {}", snapshot.connected);
    println!("link: {}", link_display(snapshot.link));
    println!("battery: {}", field_display(&snapshot.battery));
    println!("charging: {}", field_display(&snapshot.charging));
    println!("firmware: {}", field_display(&snapshot.firmware));
    println!("serial: {}", field_display(&snapshot.serial));
    println!("auto-off: {}", field_display(&snapshot.auto_power_off_min));
    println!("dongle-led: {}", field_display(&snapshot.dongle_led));
    println!("speaker-eq: {}", field_display(&snapshot.speaker_preset_eq));
    println!("edition: {}", field_display(&snapshot.edition));
    println!(
        "audio-output: {} ({})",
        snapshot.audio_output.name.as_deref().unwrap_or("unknown"),
        if snapshot.audio_output.is_headset {
            "headset"
        } else {
            "not headset"
        }
    );
    for endpoint in &snapshot.audio_output.endpoints {
        println!(
            "audio-endpoint: [{}] {}{}{}",
            endpoint.id,
            endpoint.name,
            if endpoint.is_default {
                " (default)"
            } else {
                ""
            },
            if endpoint.is_headset {
                " (headset)"
            } else {
                ""
            }
        );
    }
    if !snapshot.issues.is_empty() {
        println!("issues: {}", snapshot.issues.join("; "));
    }
}

fn print_status(device: &mut HidDevice) -> Result<(), Box<dyn std::error::Error>> {
    let status = device.read_status()?;
    println!("battery: {}", fmt_opt(status.battery, "%"));
    println!(
        "charging: {}",
        match status.charging {
            Some(true) => "on",
            Some(false) => "off",
            None => "unknown",
        }
    );
    println!("firmware: {}", status.firmware);
    println!("serial: {}", status.serial);
    println!("edition: {}", fmt_opt(status.edition, ""));
    println!("auto-off: {} min", fmt_opt(status.auto_power_off_min, ""));
    println!("dongle-led: {}", fmt_opt(status.dongle_led, ""));
    println!("speaker-eq: {}", fmt_opt(status.speaker_preset_eq, ""));
    Ok(())
}

fn fmt_opt<T: ToString>(value: Option<T>, suffix: &str) -> String {
    value
        .map(|v| format!("{}{}", v.to_string(), suffix))
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_u8(device: &mut HidDevice, command: Command) -> Result<u8, Box<dyn std::error::Error>> {
    let data = device.get(command)?;
    data.first().copied().ok_or_else(|| "empty response".into())
}

fn get_string(
    device: &mut HidDevice,
    command: Command,
) -> Result<String, Box<dyn std::error::Error>> {
    let data = device.get(command)?;
    let end = data.iter().position(|b| *b == 0).unwrap_or(data.len());
    Ok(String::from_utf8_lossy(&data[..end]).to_string())
}

fn parse_preset(value: &str) -> Result<SpeakerEqPreset, Box<dyn std::error::Error>> {
    if let Some(preset) = SpeakerEqPreset::from_name(value) {
        return Ok(preset);
    }
    let code = parse_u8(value)?;
    match code {
        7 => Ok(SpeakerEqPreset::Game),
        8 => Ok(SpeakerEqPreset::Music),
        9 => Ok(SpeakerEqPreset::Cinema),
        255 => Ok(SpeakerEqPreset::Custom),
        _ => Err(format!("unknown EQ preset: {value}").into()),
    }
}

fn parse_u8(value: &str) -> Result<u8, Box<dyn std::error::Error>> {
    Ok(value.parse::<u8>()?)
}

fn parse_u16(value: &str) -> Result<u16, Box<dyn std::error::Error>> {
    let trimmed = value.trim_start_matches("0x").trim_start_matches("0X");
    Ok(u16::from_str_radix(trimmed, 16)?)
}
