// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
use sail_app::{DeviceSnapshot, EqPreset, SailApp, ThxSnapshot};

fn app() -> SailApp {
    SailApp::new()
}

/// Run blocking HID/THX work off Tauri's UI thread. A snapshot can spend up to
/// 500 ms per field and THX setters launch a helper process; keeping those calls
/// synchronous would freeze the window.
async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| format!("background task failed: {error}"))?
}

#[tauri::command]
async fn get_status() -> Result<DeviceSnapshot, String> {
    blocking(|| Ok(app().snapshot())).await
}

#[tauri::command]
fn get_eq_presets() -> Vec<EqPreset> {
    app().eq_presets()
}

#[tauri::command]
async fn set_sidetone(enabled: bool) -> Result<(), String> {
    blocking(move || {
        app()
            .set_sidetone(enabled)
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
async fn set_eq_preset(preset: String) -> Result<(), String> {
    blocking(move || {
        app()
            .set_eq_preset(&preset)
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
async fn set_eq_gains(gains: Vec<i8>) -> Result<(), String> {
    if gains.len() != 10 {
        return Err("eq gains need exactly 10 values".into());
    }
    let mut array = [0i8; 10];
    array.copy_from_slice(&gains);
    blocking(move || {
        app()
            .set_eq_gains(array)
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
async fn get_thx_status() -> Result<ThxSnapshot, String> {
    blocking(|| Ok(app().thx_snapshot())).await
}

#[tauri::command]
async fn set_normalization(level: i32) -> Result<(), String> {
    blocking(move || {
        app()
            .set_thx_level(level)
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
async fn set_normalization_enabled(enabled: bool) -> Result<(), String> {
    blocking(move || {
        app()
            .set_thx_enabled(enabled)
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
async fn set_spatial(enabled: bool) -> Result<(), String> {
    blocking(move || {
        app()
            .set_thx_spatial(enabled)
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
async fn verify_thx() -> Result<String, String> {
    blocking(|| app().verify_thx().map_err(|error| error.to_string())).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_eq_presets,
            set_sidetone,
            set_eq_preset,
            set_eq_gains,
            get_thx_status,
            set_normalization,
            set_normalization_enabled,
            set_spatial,
            verify_thx
        ])
        .run(tauri::generate_context!())
        .expect("error while running SAIL");
}
