use sail_app::{DeviceSnapshot, EqPreset, SailApp, ThxSnapshot};

fn app() -> SailApp {
    SailApp::new()
}

#[tauri::command]
fn get_status() -> DeviceSnapshot {
    app().snapshot()
}

#[tauri::command]
fn get_eq_presets() -> Vec<EqPreset> {
    app().eq_presets()
}

#[tauri::command]
fn set_sidetone(enabled: bool) -> Result<(), String> {
    app().set_sidetone(enabled).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_eq_preset(preset: String) -> Result<(), String> {
    app()
        .set_eq_preset(&preset)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_eq_gains(gains: Vec<i8>) -> Result<(), String> {
    if gains.len() != 10 {
        return Err("eq gains need exactly 10 values".into());
    }
    let mut array = [0i8; 10];
    array.copy_from_slice(&gains);
    app()
        .set_eq_gains(array)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_thx_status() -> ThxSnapshot {
    app().thx_snapshot()
}

#[tauri::command]
fn set_normalization(level: i32) -> Result<(), String> {
    app()
        .set_thx_level(level)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_normalization_enabled(enabled: bool) -> Result<(), String> {
    app()
        .set_thx_enabled(enabled)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn verify_thx() -> Result<String, String> {
    app().verify_thx().map_err(|error| error.to_string())
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
            verify_thx
        ])
        .run(tauri::generate_context!())
        .expect("error while running SAIL");
}
