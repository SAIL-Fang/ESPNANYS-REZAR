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
  private refreshEpoch = 0;
  private pending = 0;

  get thxError(): string {
    return this.thx?.error ?? "";
  }

  async refresh(silent = false) {
    // A write owns the device; do not overlap it with a 10-second poll.
    if (this.busy) return;
    const epoch = ++this.refreshEpoch;
    if (!silent) {
      this.loading = true;
      this.error = "";
      this.notice = "";
    }
    try {
      try {
        const status = await api.getStatus();
        if (epoch === this.refreshEpoch) this.status = status;
      } catch (error) {
        if (epoch === this.refreshEpoch && !silent) {
          this.status = null;
          this.error = `无法读取耳机：${errorText(error)}`;
        }
      }
      try {
        const thx = await api.getThxStatus();
        if (epoch === this.refreshEpoch) this.thx = thx;
      } catch (error) {
        if (epoch === this.refreshEpoch && !silent) {
          this.thx = null;
          this.error = this.error || `无法读取 THX：${errorText(error)}`;
        }
      }
      try {
        const presets = await api.getEqPresets();
        if (epoch === this.refreshEpoch) this.presets = presets;
      } catch (error) {
        if (epoch === this.refreshEpoch && !silent) {
          this.error = this.error || `无法读取均衡器预设：${errorText(error)}`;
        }
      }
    } finally {
      if (epoch === this.refreshEpoch) this.loading = false;
    }
  }

  async setSidetone(enabled: boolean) {
    await this.run(() => api.setSidetone(enabled), enabled ? "侧音已开启" : "侧音已关闭");
  }

  async setEqPreset(preset: string, label: string) {
    return this.run(() => api.setEqPreset(preset), `${label} 预设已写入`);
  }

  async setEqGains(gains: number[]) {
    return this.run(() => api.setEqGains([...gains]), "EQ 已写入");
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
      if (epoch === this.refreshEpoch) this.thx = thx;
    } catch (error) {
      if (epoch === this.refreshEpoch) {
        this.thx = null;
        this.error = `无法读取 THX：${errorText(error)}`;
      }
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
