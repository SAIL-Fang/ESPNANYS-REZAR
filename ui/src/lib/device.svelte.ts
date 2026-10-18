/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
import { api } from "./api";
import type { DeviceSnapshot, EqPreset, ThxSnapshot } from "./types";

export class DeviceStore {
  status = $state<DeviceSnapshot | null>(null);
  thx = $state<ThxSnapshot | null>(null);
  presets = $state<EqPreset[]>([]);
  loading = $state(true);
  busy = $state(false);
  error = $state("");
  notice = $state("");
  verifyOutput = $state("");

  get thxError(): string {
    return this.thx?.error ?? "";
  }

  async refresh(silent = false) {
    if (!silent) {
      this.loading = true;
      this.error = "";
      this.notice = "";
    }
    try {
      this.status = await api.getStatus();
    } catch (error) {
      this.status = null;
      this.error = `无法读取耳机：${errorText(error)}`;
    }
    try {
      this.thx = await api.getThxStatus();
    } catch (error) {
      this.thx = null;
      this.error = this.error || `无法读取 THX：${errorText(error)}`;
    }
    try {
      this.presets = await api.getEqPresets();
    } catch (error) {
      this.error = this.error || `无法读取均衡器预设：${errorText(error)}`;
    }
    this.loading = false;
  }

  async setSidetone(enabled: boolean) {
    await this.run(() => api.setSidetone(enabled), enabled ? "侧音已开启" : "侧音已关闭");
  }

  async setEqPreset(preset: string, label: string) {
    await this.run(() => api.setEqPreset(preset), `${label} 预设已写入`);
  }

  async setEqGains(gains: number[]) {
    await this.run(() => api.setEqGains([...gains]), "EQ 已写入");
  }

  async saveNormalization(level: number) {
    await this.run(() => api.setNormalization(level), `声音标准化 ${level}`);
    await this.refreshThx();
  }

  async setNormalizationEnabled(enabled: boolean) {
    await this.run(
      () => api.setNormalizationEnabled(enabled),
      enabled ? "声音标准化已开启" : "声音标准化已关闭"
    );
    await this.refreshThx();
  }

  async verifyThx() {
    this.busy = true;
    this.verifyOutput = "";
    try {
      this.verifyOutput = await api.verifyThx();
      await this.refreshThx();
    } catch (error) {
      this.verifyOutput = errorText(error);
    } finally {
      this.busy = false;
    }
  }

  private async refreshThx() {
    try {
      this.thx = await api.getThxStatus();
    } catch (error) {
      this.thx = null;
      this.error = `无法读取 THX：${errorText(error)}`;
    }
  }

  private async run(action: () => Promise<void>, success: string) {
    this.busy = true;
    this.notice = "";
    try {
      await action();
      this.notice = success;
    } catch (error) {
      this.notice = `写入失败：${errorText(error)}`;
    } finally {
      this.busy = false;
    }
  }
}

export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const device = new DeviceStore();
