// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
use sail_app::{DeviceSnapshot, EqPreset, SailApp, ThxSnapshot};
use std::process::Command as ProcessCommand;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

fn app() -> SailApp {
    SailApp::new()
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
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

#[tauri::command]
async fn set_default_audio_output(device_id: Option<String>) -> Result<(), String> {
    blocking(move || {
        app()
            .set_default_audio_output(device_id.as_deref())
            .map_err(|error| error.to_string())
    })
    .await
}

#[tauri::command]
async fn open_sound_settings() -> Result<(), String> {
    ProcessCommand::new("cmd")
        .args(["/C", "start", "", "ms-settings:sound"])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("open sound settings: {error}"))
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("application icon is missing")?;
            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("ESPNANYS-REZAR")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // This is a tray-resident control panel: closing hides it;
                // the tray menu has an explicit Quit action.
                api.prevent_close();
                let _ = window.hide();
            }
        })
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
            verify_thx,
            set_default_audio_output,
            open_sound_settings,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running SAIL");
}
