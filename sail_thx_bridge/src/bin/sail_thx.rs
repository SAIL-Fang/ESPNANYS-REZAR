// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
use sail_thx_bridge::{print_status, ThxBridge, ThxError, ThxStatus};
use std::env;

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
    // Reading must not change user state: THX only accepts level writes after
    // normalization has been enabled, so the write commands use the activating
    // constructor and everything that only reports state stays passive.
    let passive = matches!(command, "status" | "probe");
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
        "verify" => verify(&bridge)?,
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
        "endpoint": status.endpoint,
        "dll": status.dll.display().to_string(),
        "dllVersion": dll_version,
        "abi": "thx-v3",
        "error": null
    });
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}

/// Prove the 1..100 range is accepted by the DLL, not just by the Razer UI.
///
/// Writes a probe level, reads it back, then restores the level that was
/// present before. The probe differs from the current level because THX
/// reports failure when asked to apply the value it already holds, and the
/// restore always runs, including after a failed probe.
fn verify(bridge: &ThxBridge) -> Result<(), ThxError> {
    let before = bridge.get_level()?;
    let before_enabled = bridge.get_enabled().unwrap_or(true);
    println!("before: {before} (enabled: {before_enabled})");

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
    let restore = if (1..=100).contains(&before) {
        before
    } else {
        10
    };
    bridge.set_level(restore)?;
    let after = bridge.get_level()?;
    println!("restored {restore} -> readback {after}");
    if after != restore {
        return Err(ThxError::Status(format!(
            "expected restore {restore}, got {after}"
        )));
    }
    if !before_enabled {
        // The probe needs normalization enabled to accept a level write.
        // Put the user's off state back after restoring the level.
        let _ = bridge.set_enabled(false);
    }

    if let Some(error) = failure {
        return Err(error);
    }
    println!("OK: THX accepts normalization level {probe} and restores {restore}");
    Ok(())
}
