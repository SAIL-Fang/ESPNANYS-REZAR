/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  DeviceSnapshot,
  DeviceSummary,
  EqPreset,
  EqPresetConfig,
  ModelDescriptor,
  ThxSnapshot
} from "./types";

export const api = {
  getStatus: () => invoke<DeviceSnapshot>("get_status"),
  getDevices: () => invoke<ModelDescriptor[]>("get_devices"),
  getDeviceSummaries: () => invoke<DeviceSummary[]>("get_device_summaries"),
  getEqConfig: (modelId: string) =>
    invoke<EqPresetConfig>("get_eq_config", { modelId }),
  selectEqPreset: (modelId: string, presetId: string) =>
    invoke<EqPresetConfig>("select_eq_preset", { modelId, presetId }),
  saveEqPreset: (modelId: string, presetId: string, gains: number[]) =>
    invoke<EqPresetConfig>("save_eq_preset", { modelId, presetId, gains }),
  clearEqPreset: (modelId: string, presetId: string, selected: boolean) =>
    invoke<EqPresetConfig>("clear_eq_preset", { modelId, presetId, selected }),
  setActiveDevice: (modelId: string) => invoke<void>("set_active_device", { modelId }),
  getEqPresets: () => invoke<EqPreset[]>("get_eq_presets"),
  setSidetone: (enabled: boolean) => invoke<void>("set_sidetone", { enabled }),
  setSidetoneVolume: (value: number) => invoke<void>("set_sidetone_volume", { value }),
  micCommand: (args: string[]) => invoke<void>("mic_command", { args }),
  startMicPreview: () => invoke<void>("start_mic_preview"),
  setEqPreset: (preset: string) => invoke<void>("set_eq_preset", { preset }),
  setEqGains: (gains: number[]) => invoke<void>("set_eq_gains", { gains }),
  getThxStatus: () => invoke<ThxSnapshot>("get_thx_status"),
  setNormalization: (level: number) => invoke<void>("set_normalization", { level }),
  setNormalizationEnabled: (enabled: boolean) =>
    invoke<void>("set_normalization_enabled", { enabled }),
  setSpatial: (enabled: boolean) => invoke<void>("set_spatial", { enabled }),
  verifyThx: () => invoke<string>("verify_thx"),
  setDefaultAudioOutput: (deviceId?: string) =>
    invoke<void>("set_default_audio_output", { deviceId: deviceId ?? null }),
  openSoundSettings: () => invoke<void>("open_sound_settings"),
  quitApp: () => invoke<void>("quit_app")
};
