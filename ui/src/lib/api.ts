/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
import { invoke } from "@tauri-apps/api/core";
import type { DeviceSnapshot, EqPreset, ThxSnapshot } from "./types";

export const api = {
  getStatus: () => invoke<DeviceSnapshot>("get_status"),
  getEqPresets: () => invoke<EqPreset[]>("get_eq_presets"),
  setSidetone: (enabled: boolean) => invoke<void>("set_sidetone", { enabled }),
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
