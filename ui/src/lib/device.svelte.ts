/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
import { api } from "./api";
import { DEFAULT_EQ_GAINS, DEFAULT_NORMALIZATION_LEVEL } from "./presets";
import type {
  AudioEndpoint,
  DeviceSnapshot,
  DeviceSummary,
  EqPreset,
  EqPresetConfig,
  ModelDescriptor,
  ThxSnapshot
} from "./types";

const NORMALIZATION_KEY = "sail.normalizationLevel";
const ACTIVE_MODEL_KEY = "sail.activeModel";

// Debug-only probe: lets the focus/foreground test prove that no HID refresh
// is attempted while the app is unfocused. Vite folds this out of release.
function countRefreshAttempt() {
  if (import.meta.env.VITE_SAIL_DEBUG !== "1") return;
  const debugWindow = window as Window & { __sailRefreshCount?: number };
  debugWindow.__sailRefreshCount = (debugWindow.__sailRefreshCount ?? 0) + 1;
}

function storedNormalizationLevel(): number {
  try {
    const raw = localStorage.getItem(NORMALIZATION_KEY);
    const value = raw ? Number(raw) : NaN;
    if (Number.isFinite(value) && value >= 1 && value <= 100) return Math.round(value);
  } catch {
    // localStorage can be unavailable in a restricted webview; use default.
  }
  return DEFAULT_NORMALIZATION_LEVEL;
}

export class DeviceStore {
  status = $state<DeviceSnapshot | null>(null);
  thx = $state<ThxSnapshot | null>(null);
  presets = $state<EqPreset[]>([]);
  eqConfig = $state<EqPresetConfig | null>(null);
  eqConfigNotice = $state("");
  summaries = $state<DeviceSummary[]>([]);
  devices = $state<ModelDescriptor[]>([]);
  activeModelId = $state("");
  loading = $state(true);
  busy = $state(false);
  error = $state("");
  statusError = $state("");
  thxReadError = $state("");
  notice = $state("");
  verifyOutput = $state("");
  private refreshEpoch = 0;
  private pending = 0;
  private eqApplied = false;
  private normalizationApplied = false;

  get thxError(): string {
    return this.thx?.error ?? this.thxReadError;
  }

  private desiredNormalizationLevel(): number {
    return storedNormalizationLevel();
  }

  private async applyDesiredNormalization(level: number) {
    this.pending += 1;
    this.busy = true;
    try {
      await api.setNormalization(level);
      if (this.thx) this.thx = { ...this.thx, level };
      this.notice = `声音标准化默认 ${level}`;
    } catch (error) {
      // Retry on a later successful THX read rather than silently showing a
      // level that was never written.
      this.normalizationApplied = false;
      this.error = `声音标准化默认值写入失败：${errorText(error)}`;
    } finally {
      this.pending -= 1;
      this.busy = this.pending > 0;
    }
  }

  factoryGains(presetId: string): number[] {
    const preset = this.presets.find((item) => item.id === presetId);
    return preset ? [...preset.gains] : [...DEFAULT_EQ_GAINS];
  }

  effectiveGains(presetId: string): number[] {
    const override = this.eqConfig?.presets[presetId];
    if (Array.isArray(override) && override.length === 10) {
      return [...override];
    }
    return this.factoryGains(presetId);
  }

  outputMatchesActive(): boolean {
    const model = this.status?.model;
    const name = this.status?.audioOutput.name?.toLowerCase() ?? "";
    if (!model || !name) return false;
    return model.capabilities.eqViaThx
      ? name.includes("usb sound card")
      : name.includes("blackshark");
  }

  async loadEqConfig() {
    if (!this.activeModelId) {
      this.eqConfig = null;
      return;
    }
    try {
      this.eqConfig = await api.getEqConfig(this.activeModelId);
      this.eqConfigNotice = this.eqConfig.notice ?? "";
      this.eqApplied = false;
      if (this.eqConfig.notice) this.notice = this.eqConfig.notice;
    } catch (error) {
      this.eqConfig = null;
      this.eqConfigNotice = "";
      this.error = `无法读取 EQ 配置：${errorText(error)}`;
    }
  }

  private async applyDefaultEqOnce() {
    if (this.eqApplied || !this.status?.connected) return;
    this.eqApplied = true;
    this.pending += 1;
    this.busy = true;
    try {
      await api.setEqGains([...DEFAULT_EQ_GAINS]);
      this.notice = "默认 EQ 已写入";
    } catch (error) {
      // Allow the next reconnect/poll to retry after a transient failure.
      this.eqApplied = false;
      this.error = `默认 EQ 写入失败：${errorText(error)}`;
    } finally {
      this.pending -= 1;
      this.busy = this.pending > 0;
    }
  }

  private async applyStoredEqOnce() {
    if (this.eqApplied || !this.status?.connected) return;
    if (!this.eqConfig) await this.loadEqConfig();
    const selected = this.eqConfig?.selected;
    if (!selected) {
      await this.applyDefaultEqOnce();
      return;
    }
    // THX applies to the current endpoint. Do not write a stored curve into
    // the wrong device; the output gate will trigger a refresh after switching.
    if (!this.outputMatchesActive()) return;
    const preset = this.presets.find((item) => item.id === selected);
    if (!preset) return;
    const gains = this.effectiveGains(selected);
    this.eqApplied = true;
    this.pending += 1;
    this.busy = true;
    try {
      await api.setEqPreset(selected);
      await api.setEqGains(gains);
      this.notice = `已恢复 ${preset.label} EQ`;
    } catch (error) {
      this.eqApplied = false;
      this.error = `恢复 ${preset.label} EQ 失败：${errorText(error)}`;
    } finally {
      this.pending -= 1;
      this.busy = this.pending > 0;
    }
  }

  private settle(epoch: number, apply: () => void) {
    if (epoch === this.refreshEpoch) apply();
  }

  private applySummaries(summaries: DeviceSummary[]) {
    this.summaries = summaries;
    this.devices = summaries.map((summary) => summary.model);
  }

  async loadHub() {
    this.loading = true;
    try {
      // The descriptor list is cheap; render the card shells while the
      // per-device battery/route summary is still being collected.
      const devices = await api.getDevices();
      this.devices = devices;
      if (!devices.length) {
        this.activeModelId = "";
        return;
      }
      let preferred = "";
      try {
        preferred = localStorage.getItem(ACTIVE_MODEL_KEY) ?? "";
      } catch {
        // localStorage can be unavailable; fall back to the first model.
      }
      const active = devices.find((model) => model.id === preferred) ?? devices[0];
      this.activeModelId = active.id;
      await api.setActiveDevice(active.id);
      const summaries = await api.getDeviceSummaries();
      if (!summaries.length) {
        return;
      }
      this.applySummaries(
        summaries.map((summary) => ({
          ...summary,
          active: summary.model.id === active.id
        }))
      );
    } catch (error) {
      this.error = `无法读取设备列表：${errorText(error)}`;
    } finally {
      this.loading = false;
    }
  }

  async refreshSummaries() {
    try {
      const summaries = await api.getDeviceSummaries();
      if (!summaries.length) {
        this.applySummaries([]);
        this.activeModelId = "";
        return;
      }
      let activeId = this.activeModelId;
      if (!summaries.some((summary) => summary.model.id === activeId)) {
        activeId = summaries[0].model.id;
        this.activeModelId = activeId;
        await api.setActiveDevice(activeId);
      }
      this.applySummaries(
        summaries.map((summary) => ({
          ...summary,
          active: summary.model.id === activeId
        }))
      );
    } catch (error) {
      this.error = `无法刷新设备库：${errorText(error)}`;
    }
  }

  async activateDevice(modelId: string) {
    if (!modelId) return;
    await api.setActiveDevice(modelId);
    this.activeModelId = modelId;
    try {
      localStorage.setItem(ACTIVE_MODEL_KEY, modelId);
    } catch {
      // Remembering the selection is best-effort.
    }
    this.status = null;
    this.thx = null;
    this.presets = [];
    this.eqConfig = null;
    this.eqApplied = false;
    this.normalizationApplied = false;
    await this.loadEqConfig();
  }

  outputEndpoint(): AudioEndpoint | null {
    const model = this.devices.find((item) => item.id === this.activeModelId);
    if (!model) return null;
    const needle = model.capabilities.eqViaThx ? "usb sound card" : "blackshark";
    return (
      this.status?.audioOutput.endpoints.find(
        (endpoint) => endpoint.isHeadset && endpoint.name.toLowerCase().includes(needle)
      ) ?? null
    );
  }

  async refresh(silent = false, includeThx = true) {
    countRefreshAttempt();
    // A write owns the device; do not overlap it with a 10-second poll.
    if (this.busy) return;
    const epoch = ++this.refreshEpoch;
    if (!silent) {
      this.loading = true;
      this.error = "";
      this.statusError = "";
      this.thxReadError = "";
      this.notice = "";
    }
    try {
      try {
        const status = await api.getStatus();
        this.settle(epoch, () => {
          this.status = status;
          this.statusError = "";
          if (!status.connected) {
            this.eqApplied = false;
          }
        });
      } catch (error) {
        this.settle(epoch, () => {
          if (!silent) {
            this.status = null;
            this.statusError = `无法读取耳机：${errorText(error)}`;
          }
        });
      }
      if (includeThx) {
        try {
          const thx = await api.getThxStatus();
          this.settle(epoch, () => {
            let next = thx;
            if (!this.normalizationApplied && thx.enabled === true) {
              const desired = this.desiredNormalizationLevel();
              if (thx.level !== desired) {
                // Show and write the preferred level before Synapse/THX can
                // leave the preset's default (usually 10) in place.
                next = { ...thx, level: desired };
                this.normalizationApplied = true;
                void this.applyDesiredNormalization(desired);
              } else {
                this.normalizationApplied = true;
              }
            }
            this.thx = next;
            this.thxReadError = "";
          });
        } catch (error) {
          this.settle(epoch, () => {
            if (!silent) {
              this.thx = null;
              this.thxReadError = `无法读取 THX：${errorText(error)}`;
            }
          });
        }
      }
      try {
        const presets = await api.getEqPresets();
        this.settle(epoch, () => (this.presets = presets));
      } catch (error) {
        this.settle(epoch, () => {
          if (!silent) {
            this.error = this.error || `无法读取均衡器预设：${errorText(error)}`;
          }
        });
      }
      await this.applyStoredEqOnce();
    } finally {
      this.settle(epoch, () => (this.loading = false));
    }
  }

  async setSidetone(enabled: boolean) {
    return this.run(() => api.setSidetone(enabled), enabled ? "侧音已开启" : "侧音已关闭");
  }

  async setSidetoneVolume(value: number) {
    return this.run(
      () => api.setSidetoneVolume(Math.max(0, Math.min(15, Math.round(value)))),
      `侧音音量 ${Math.round(value)}`
    );
  }

  async micCommand(args: string[], notice: string) {
    return this.run(() => api.micCommand(args), notice);
  }

  async startMicPreview(notice: string) {
    return this.run(() => api.startMicPreview(), notice);
  }

  async setEqPreset(preset: string, label: string) {
    return this.run(() => api.setEqPreset(preset), `${label} 预设已写入`);
  }

  async setEqGains(gains: number[]) {
    return this.run(() => api.setEqGains([...gains]), "EQ 已写入");
  }

  private async configCommand(work: () => Promise<EqPresetConfig>): Promise<boolean> {
    if (!this.activeModelId) return false;
    try {
      this.eqConfig = await work();
      if (this.eqConfig.notice) this.notice = this.eqConfig.notice;
      return true;
    } catch (error) {
      this.error = `保存 EQ 配置失败：${errorText(error)}`;
      return false;
    }
  }

  async selectEqPreset(presetId: string): Promise<boolean> {
    return this.configCommand(() => api.selectEqPreset(this.activeModelId, presetId));
  }

  async saveEqPreset(presetId: string, gains: number[]): Promise<boolean> {
    return this.configCommand(() =>
      api.saveEqPreset(this.activeModelId, presetId, [...gains])
    );
  }

  async clearEqPreset(presetId: string): Promise<boolean> {
    return this.configCommand(() =>
      api.clearEqPreset(this.activeModelId, presetId, true)
    );
  }

  async saveNormalization(level: number) {
    const ok = await this.run(() => api.setNormalization(level), `声音标准化 ${level}`);
    if (ok) {
      this.normalizationApplied = true;
      try {
        localStorage.setItem(NORMALIZATION_KEY, String(level));
      } catch {
        // Persistence is best-effort; the write to THX already succeeded.
      }
    }
    await this.refreshThx();
  }

  async setNormalizationEnabled(enabled: boolean) {
    const ok = await this.run(
      () => api.setNormalizationEnabled(enabled),
      enabled ? "声音标准化已开启" : "声音标准化已关闭"
    );
    await this.refreshThx();
    if (ok && enabled) {
      const desired = this.desiredNormalizationLevel();
      if (this.thx?.level !== desired) {
        this.normalizationApplied = false;
        void this.applyDesiredNormalization(desired);
      }
    }
    return ok;
  }

  async setSpatial(enabled: boolean) {
    const ok = await this.run(
      () => api.setSpatial(enabled),
      enabled ? "THX 空间音效已开启" : "已切换到立体声"
    );
    if (ok) {
      // Reflect the confirmed write immediately; a poll that started before
      // the write must not put the old value back.
      if (this.thx) this.thx = { ...this.thx, spatialEnabled: enabled };
      await this.refreshThx();
    }
  }

  async setDefaultAudioOutput(deviceId?: string): Promise<boolean> {
    this.error = "";
    try {
      await api.setDefaultAudioOutput(deviceId);
      // The switch itself is confirmed by the backend. Reflect it immediately
      // so the blocking modal can close; a full HID refresh can take seconds.
      const target = this.status?.audioOutput?.endpoints.find(
        (endpoint) => endpoint.id === deviceId
      );
      if (this.status && target) {
        this.status = {
          ...this.status,
          audioOutput: {
            ...this.status.audioOutput,
            name: target.name,
            isHeadset: target.isHeadset
          }
        };
      }
      void this.refresh();
      return true;
    } catch (error) {
      this.error = `无法切换默认输出：${errorText(error)}`;
      return false;
    }
  }

  async openSoundSettings() {
    try {
      await api.openSoundSettings();
    } catch (error) {
      this.error = `无法打开声音设置：${errorText(error)}`;
    }
  }

  async quit() {
    await api.quitApp();
  }

  async verifyThx() {
    this.pending += 1;
    this.busy = true;
    this.verifyOutput = "";
    try {
      this.verifyOutput = await api.verifyThx();
      await this.refreshThx();
    } catch (error) {
      this.verifyOutput = errorText(error);
    } finally {
      this.pending -= 1;
      this.busy = this.pending > 0;
    }
  }

  private async refreshThx() {
    const epoch = ++this.refreshEpoch;
    try {
      const thx = await api.getThxStatus();
      this.settle(epoch, () => {
        this.thx = thx;
        this.thxReadError = "";
      });
    } catch (error) {
      this.settle(epoch, () => {
        this.thx = null;
        this.thxReadError = `无法读取 THX：${errorText(error)}`;
      });
    }
  }

  private async run(action: () => Promise<void>, success: string): Promise<boolean> {
    // A write invalidates any in-flight poll. Otherwise the poll can land
    // after the confirmed write and restore the old snapshot.
    this.refreshEpoch += 1;
    this.loading = false;
    this.pending += 1;
    this.busy = true;
    this.notice = "";
    try {
      await action();
      this.notice = success;
      return true;
    } catch (error) {
      this.notice = `写入失败：${errorText(error)}`;
      return false;
    } finally {
      this.pending -= 1;
      this.busy = this.pending > 0;
    }
  }
}

export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const device = new DeviceStore();
