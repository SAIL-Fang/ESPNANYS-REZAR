// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
//!
//! Per-model EQ preset overrides and the last selected preset.
//!
//! The file is deliberately sparse: an absent preset means "factory default".
//! This lets a factory-curve fix reach users who never modified that preset.
//! Writes are atomic at the file level: a temporary file is renamed over the
//! destination, and a corrupt file is backed up before falling back to empty.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const EQ_CONFIG_VERSION: u32 = 1;
pub const PRESET_IDS: [&str; 4] = ["game", "music", "cinema", "custom"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqConfig {
    pub version: u32,
    #[serde(default)]
    pub models: BTreeMap<String, ModelEqConfig>,
}

impl Default for EqConfig {
    fn default() -> Self {
        Self {
            version: EQ_CONFIG_VERSION,
            models: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEqConfig {
    /// Last explicitly selected preset for this model.
    #[serde(default)]
    pub selected: Option<String>,
    /// Sparse overrides; missing IDs use the factory curve.
    #[serde(default)]
    pub presets: BTreeMap<String, [i8; 10]>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EqConfigView {
    pub selected: Option<String>,
    pub presets: BTreeMap<String, [i8; 10]>,
    pub notice: Option<String>,
}

impl EqConfig {
    pub fn view(&self, model_id: &str, notice: Option<String>) -> EqConfigView {
        let model = self.models.get(model_id).cloned().unwrap_or_default();
        EqConfigView {
            selected: model.selected,
            presets: model.presets,
            notice,
        }
    }

    pub fn set_selected(&mut self, model_id: &str, preset_id: &str) {
        let model = self.models.entry(model_id.to_owned()).or_default();
        model.selected = Some(preset_id.to_owned());
    }

    pub fn set_override(&mut self, model_id: &str, preset_id: &str, gains: [i8; 10]) {
        let model = self.models.entry(model_id.to_owned()).or_default();
        model.presets.insert(preset_id.to_owned(), gains);
        model.selected = Some(preset_id.to_owned());
    }

    pub fn clear_override(&mut self, model_id: &str, preset_id: &str, selected: bool) {
        let model = self.models.entry(model_id.to_owned()).or_default();
        model.presets.remove(preset_id);
        if selected {
            model.selected = Some(preset_id.to_owned());
        }
    }

    fn normalize(&mut self) {
        self.version = EQ_CONFIG_VERSION;
        self.models.retain(|_, model| {
            model
                .presets
                .retain(|preset, gains| is_preset_id(preset) && valid_gains(gains));
            if let Some(selected) = &model.selected {
                if !is_preset_id(selected) {
                    model.selected = None;
                }
            }
            !model.presets.is_empty() || model.selected.is_some()
        });
    }
}

pub fn is_preset_id(id: &str) -> bool {
    PRESET_IDS.contains(&id)
}

pub fn valid_gains(gains: &[i8; 10]) -> bool {
    gains.iter().all(|gain| (-12..=12).contains(gain))
}

/// Load the configuration. A missing file is an empty configuration, not an
/// error. A malformed file is copied to a timestamped backup and replaced in
/// memory with the empty configuration; the notice is shown by the UI.
pub fn load(path: &Path) -> (EqConfig, Option<String>) {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return (EqConfig::default(), None);
        }
        Err(error) => {
            return (
                EqConfig::default(),
                Some(format!("无法读取 EQ 配置：{error}")),
            );
        }
    };
    match serde_json::from_slice::<EqConfig>(&bytes) {
        Ok(mut config) => {
            config.normalize();
            (config, None)
        }
        Err(error) => {
            let backup = backup_path(path);
            let notice = match fs::copy(path, &backup) {
                Ok(_) => format!(
                    "EQ 配置损坏（{error}），已备份到 {}，本次按空配置启动",
                    backup.display()
                ),
                Err(copy_error) => {
                    format!("EQ 配置损坏（{error}），备份失败（{copy_error}），本次按空配置启动")
                }
            };
            (EqConfig::default(), Some(notice))
        }
    }
}

pub fn save_atomic(path: &Path, config: &EqConfig) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut normalized = config.clone();
    normalized.normalize();
    let data = serde_json::to_vec_pretty(&normalized)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, data)?;
    // std::fs::rename does not replace an existing destination on Windows.
    // Remove the old destination only after the complete temporary file has
    // been written; a failure here leaves the temporary file for diagnosis.
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temporary, path)
}

fn backup_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    path.with_file_name(format!("eq-presets.corrupt-{stamp}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("eq-config-tests");
        fs::create_dir_all(&dir).unwrap();
        dir.join(format!("sail-eq-{name}-{stamp}.json"))
    }

    #[test]
    fn missing_file_is_empty_not_an_error() {
        let path = path("missing");
        let (config, notice) = load(&path);
        assert!(config.models.is_empty());
        assert!(notice.is_none());
    }

    #[test]
    fn sparse_override_round_trips_and_clear_removes_it() {
        let path = path("roundtrip");
        let mut config = EqConfig::default();
        config.set_selected("model", "game");
        config.set_override("model", "game", [-3, -3, -4, 0, 5, 5, 4, 1, 0, -1]);
        save_atomic(&path, &config).unwrap();
        let (loaded, notice) = load(&path);
        assert!(notice.is_none());
        assert_eq!(loaded.view("model", None).selected.as_deref(), Some("game"));
        assert_eq!(
            loaded.view("model", None).presets.get("game").copied(),
            Some([-3, -3, -4, 0, 5, 5, 4, 1, 0, -1])
        );
        let mut cleared = loaded;
        cleared.clear_override("model", "game", true);
        save_atomic(&path, &cleared).unwrap();
        let (loaded, _) = load(&path);
        assert!(loaded.view("model", None).presets.is_empty());
        assert_eq!(loaded.view("model", None).selected.as_deref(), Some("game"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn invalid_entries_are_ignored() {
        let mut config = EqConfig::default();
        config.set_override("model", "game", [0; 10]);
        config
            .models
            .get_mut("model")
            .unwrap()
            .presets
            .insert("bogus".into(), [0; 10]);
        config
            .models
            .get_mut("model")
            .unwrap()
            .presets
            .insert("music".into(), [13; 10]);
        config.normalize();
        let view = config.view("model", None);
        assert_eq!(view.presets.len(), 1);
        assert!(view.presets.contains_key("game"));
    }
}
