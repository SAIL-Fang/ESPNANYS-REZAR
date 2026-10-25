// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
use sail_thx_bridge::{print_status, ThxBridge, ThxError, ThxStatus};
use std::env;
use std::process::{Command, Stdio};

fn main() {
    if let Err(error) = run() {
        eprintln!("sail_thx: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut dll = None;
    if let Some(index) = args.iter().position(|a| a == "--dll") {
        if index + 1 >= args.len() {
            return Err("--dll needs a path".into());
        }
        let path = args.remove(index + 1);
        args.remove(index);
        dll = Some(path);
    }
    let command = args.first().map(String::as_str).unwrap_or("status");
    let json = args.iter().any(|value| value == "--json");
    // Reading must not change user state: THX only accepts render level writes
    // after render normalization has been enabled. Capture setters must never
    // turn the speaker chain on, so every mic command stays passive too.
    let passive = matches!(
        command,
        "status"
            | "probe"
            | "spatial"
            | "mic-status"
            | "mic-gate"
            | "mic-nr"
            | "mic-norm"
            | "mic-clarity"
            | "mic-eq"
            | "mic-eq-toggle"
            | "mic-volume"
            | "mic-preview"
            | "mic-preview-hold"
            | "mic-sidetone"
    );
    if command == "verify" {
        // Sample the real user state with a passive bridge first. The active
        // constructor enables normalization, so sampling through it can never
        // observe "off".
        let observed = match &dll {
            Some(path) => ThxBridge::with_dll_passive(path)?,
            None => ThxBridge::open_passive()?,
        }
        .status()?;
        // Do not construct the activating bridge unless we can restore both
        // pieces of user state afterwards. The active constructor enables
        // normalization; an unknown prior state would leave it on.
        if observed.enabled.is_none()
            || !matches!(observed.level, Some(level) if (1..=100).contains(&level))
        {
            return Err("cannot verify: normalization state is unavailable".into());
        }
        let bridge = match &dll {
            Some(path) => ThxBridge::with_dll(path)?,
            None => ThxBridge::open()?,
        };
        verify(&bridge, &observed)?;
        return Ok(());
    }
    let bridge = match (dll, passive) {
        (Some(path), true) => ThxBridge::with_dll_passive(path)?,
        (Some(path), false) => ThxBridge::with_dll(path)?,
        (None, true) => ThxBridge::open_passive()?,
        (None, false) => ThxBridge::open()?,
    };

    match command {
        "probe" | "status" => {
            let status = bridge.status()?;
            if json {
                print_json(&status)?;
            } else {
                print_status(&status);
            }
        }
        "enable" => {
            let value = args.get(1).ok_or("enable needs on|off")?;
            let enabled = match value.as_str() {
                "on" => true,
                "off" => false,
                _ => return Err("enable needs on|off".into()),
            };
            bridge.set_enabled(enabled)?;
            println!("normalization {}", if enabled { "on" } else { "off" });
        }
        "level" => match args.get(1) {
            Some(value) => {
                let level: i32 = value.parse()?;
                bridge.set_level(level)?;
                println!("set {level}, readback {}", bridge.get_level()?);
            }
            None => {
                println!("{}", bridge.get_level()?);
            }
        },
        "render-eq" => {
            let raw = args.get(1).ok_or("render-eq needs a JSON array")?;
            let values: Vec<i8> = serde_json::from_str(raw)?;
            if values.len() != 10 {
                return Err("render-eq needs 10 dB values".into());
            }
            let gains: [i8; 10] = values
                .try_into()
                .map_err(|_| "render-eq needs 10 dB values")?;
            bridge.set_render_eq_gains(&gains)?;
            println!("render-eq {}", serde_json::to_string(&gains)?);
        }
        "spatial" => match args.get(1) {
            Some(value) => {
                let enabled = match value.as_str() {
                    "on" => true,
                    "off" => false,
                    _ => return Err("spatial needs on|off".into()),
                };
                bridge.set_spatial(enabled)?;
                println!(
                    "spatial {}, readback {}",
                    if enabled { "on" } else { "off" },
                    if bridge.get_spatial()? { "on" } else { "off" }
                );
            }
            None => println!("{}", if bridge.get_spatial()? { "on" } else { "off" }),
        },
        "mic-status" => {
            let status = bridge.capture_status();
            if json {
                println!("{}", serde_json::to_string(&status)?);
            } else {
                println!("{status:#?}");
            }
        }
        "mic-gate" => {
            let enabled = parse_on_off(args.get(1).ok_or("mic-gate needs on|off")?)?;
            let db = args.get(2).map(|value| value.parse::<f32>()).transpose()?;
            bridge.set_capture_voice_gate(enabled, db)?;
            println!("mic-gate {}", if enabled { "on" } else { "off" });
        }
        "mic-nr" => {
            let enabled = parse_on_off(args.get(1).ok_or("mic-nr needs on|off")?)?;
            let level = args.get(2).map(|value| value.parse::<i32>()).transpose()?;
            bridge.set_capture_noise_reduction(enabled, level)?;
            println!("mic-nr {}", if enabled { "on" } else { "off" });
        }
        "mic-norm" => {
            let enabled = parse_on_off(args.get(1).ok_or("mic-norm needs on|off")?)?;
            let level = args.get(2).map(|value| value.parse::<i32>()).transpose()?;
            bridge.set_capture_normalization(enabled, level)?;
            println!("mic-norm {}", if enabled { "on" } else { "off" });
        }
        "mic-clarity" => {
            let enabled = parse_on_off(args.get(1).ok_or("mic-clarity needs on|off")?)?;
            let level = args.get(2).map(|value| value.parse::<i32>()).transpose()?;
            bridge.set_capture_vocal_clarity(enabled, level)?;
            println!("mic-clarity {}", if enabled { "on" } else { "off" });
        }
        "mic-eq" => {
            let values = &args[1..];
            if values.len() != 10 {
                return Err("mic-eq needs 10 dB values".into());
            }
            let mut gains = [0.0f64; 10];
            for (index, value) in values.iter().enumerate() {
                gains[index] = value.parse()?;
            }
            bridge.set_capture_eq_gains(&gains)?;
            println!("mic-eq {}", serde_json::to_string(&gains)?);
        }
        "mic-eq-toggle" => {
            let enabled = parse_on_off(args.get(1).ok_or("mic-eq-toggle needs on|off")?)?;
            bridge.set_capture_eq(enabled)?;
            println!("mic-eq {}", if enabled { "on" } else { "off" });
        }
        "mic-volume" => {
            let level: i32 = args.get(1).ok_or("mic-volume needs 0..100")?.parse()?;
            bridge.set_capture_volume(level)?;
            println!("mic-volume {level}");
        }
        "mic-preview" => {
            let enabled = parse_on_off(args.get(1).ok_or("mic-preview needs on|off")?)?;
            if enabled {
                // Starting preview in this short-lived helper would unload the
                // DLL while THX's preview worker is still running (0xC0000005).
                // Spawn the persistent holder and let it own the preview.
                let exe = env::current_exe()?;
                let mut command = Command::new(exe);
                command
                    .arg("mic-preview-hold")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    command.creation_flags(0x0800_0000);
                }
                command.spawn()?;
                println!("mic-preview on");
            } else {
                bridge.set_capture_preview(false)?;
                println!("mic-preview off");
            }
        }
        "mic-preview-hold" => {
            bridge.set_capture_preview(true)?;
            println!("mic-preview started");
            // The DLL's preview starts a worker thread. An ephemeral helper
            // must stay alive while that thread runs; unloading the DLL right
            // after SetCapturePreview(true) raises 0xC0000005 at exit. Keep
            // the process alive until a separate `mic-preview off` flips the
            // state (or a bounded timeout expires).
            for _ in 0..1200 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if !bridge.get_capture_preview().unwrap_or(false) {
                    break;
                }
            }
            let _ = bridge.set_capture_preview(false);
            println!("mic-preview stopped");
        }
        "mic-sidetone" => {
            let enabled = parse_on_off(args.get(1).ok_or("mic-sidetone needs on|off")?)?;
            bridge.set_capture_sidetone(enabled)?;
            println!("mic-sidetone {}", if enabled { "on" } else { "off" });
        }
        other => return Err(format!("unknown command: {other}").into()),
    }
    Ok(())
}

/// Stable machine-readable form for `sail_app` and the UI. The text output
/// remains for humans and for older scripts.
fn print_json(status: &ThxStatus) -> Result<(), Box<dyn std::error::Error>> {
    let dll_version = status
        .dll
        .file_stem()
        .and_then(|value| value.to_str())
        .and_then(|value| value.split_once("_v").map(|(_, version)| version))
        .map(ToOwned::to_owned);
    let value = serde_json::json!({
        "installed": true,
        "enabled": status.enabled,
        "level": status.level,
        "spatialEnabled": status.spatial_enabled,
        "endpoint": status.endpoint,
        "dll": status.dll.display().to_string(),
        "dllVersion": dll_version,
        "abi": "thx-v3",
        "capture": status.capture,
        "error": null
    });
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}

fn parse_on_off(value: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err("expected on|off".into()),
    }
}

/// Prove the 1..100 range is accepted by the DLL, not just by the Razer UI.
///
/// Writes a probe level, reads it back, then restores the level that was
/// present before. The probe differs from the current level because THX
/// reports failure when asked to apply the value it already holds, and the
/// restore always runs, including after a failed probe.
fn verify(bridge: &ThxBridge, observed: &ThxStatus) -> Result<(), ThxError> {
    let before = match observed.level {
        Some(level) if (1..=100).contains(&level) => level,
        _ => {
            return Err(ThxError::Status(
                "cannot verify: current normalization level is unavailable".into(),
            ))
        }
    };
    let before_enabled = observed.enabled;
    println!("before: {before} (enabled: {before_enabled:?})");

    let probe = if before == 1 { 2 } else { 1 };
    let mut failure: Option<ThxError> = None;

    match bridge.set_level(probe) {
        Ok(()) => match bridge.get_level() {
            Ok(read) => {
                println!("set {probe} -> readback {read}");
                if read != probe {
                    failure = Some(ThxError::Status(format!(
                        "expected readback {probe}, got {read}"
                    )));
                }
            }
            Err(error) => failure = Some(error),
        },
        Err(error) => failure = Some(error),
    }

    // Restore the previous value even if the probe failed.
    bridge.set_level(before)?;
    let after = bridge.get_level()?;
    println!("restored {before} -> readback {after}");
    if after != before {
        return Err(ThxError::Status(format!(
            "expected restore {before}, got {after}"
        )));
    }
    if before_enabled == Some(false) {
        // The active constructor had to enable normalization for the probe.
        // Put the user's off state back after restoring the level.
        bridge.set_enabled(false)?;
    }

    if let Some(error) = failure {
        return Err(error);
    }
    println!("OK: THX accepts normalization level {probe} and restores {before}");
    Ok(())
}
