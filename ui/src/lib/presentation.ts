/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 *
 * The single presentation interface. UI modules render these strings instead
 * of recombining connection/THX/telemetry flags themselves.
 */
import { chargingText, fieldText } from "./types.ts";
import type { DeviceSnapshot, ThxSnapshot } from "./types.ts";
import type { UiState } from "./ui-state.ts";

export interface PresentationInput {
  status: DeviceSnapshot | null;
  thx: ThxSnapshot | null;
  uiState: UiState;
  loading: boolean;
}

export interface Presentation {
  modelName: string;
  connectionLabel: string;
  transportLabel: string;
  stateBadge: string;
  batteryText: string;
  chargeText: string;
  thxLabel: string;
  powerTitle: string;
  powerDetail: string;
  powerWaitText: string;
  routeTitle: string;
  routeDetail: string;
  routeWarningTitle: string;
  routeWarningDetail: string;
  routeCurrentLabel: string;
  routeTargetLabel: string;
}

const POWER_COPY = {
  standby: {
    title: "耳机未开机",
    detail: "接收器仍连接，但耳机本体没有响应。请打开耳机电源；应用会自动恢复。"
  },
  missing: {
    title: "耳机未连接",
    detail: "没有检测到耳机或接收器。请检查连接后再试。"
  }
} as const;

const ROUTE_DETAIL =
  "THX 空间音效和声音标准化只对耳机输出生效。当前 Windows 正在使用其他设备，切换后才能继续。";

function connectionLabel(status: DeviceSnapshot | null): string {
  if (!status) return "未连接";
  switch (status.link) {
    case "standby":
      return "耳机未开机";
    case "wireless":
      return "无线已连接";
    case "wired":
      return "有线已连接";
    case "unknown":
      return status.connected ? "已连接" : "未连接";
    default:
      return "未连接";
  }
}

function transportLabel(status: DeviceSnapshot | null): string {
  switch (status?.link) {
    case "wireless":
      return "无线 2.4G";
    case "wired":
      return "有线 USB";
    default:
      return "未连接";
  }
}

function stateBadge(status: DeviceSnapshot | null): string {
  if (!status) return "离线";
  if (status.link === "standby") return "待开机";
  return status.connected ? "在线" : "离线";
}

function thxLabel(status: DeviceSnapshot | null, thx: ThxSnapshot | null, ready: boolean): string {
  if (status?.model.capabilities.thx === false) return "THX 不支持";
  if (!thx?.installed) return "THX 未连接";
  if (status?.audioOutput.isHeadset === false) return "THX 暂停";
  return ready ? "THX 就绪" : "THX 不可用";
}

function powerCopy(uiState: UiState): { title: string; detail: string } {
  switch (uiState.kind) {
    case "booting":
      return { title: "正在检测设备", detail: "请稍候。" };
    case "headset_off":
      return POWER_COPY.standby;
    case "no_receiver":
      return POWER_COPY.missing;
    default:
      return { title: "", detail: "" };
  }
}

export function derivePresentation({
  status,
  thx,
  uiState,
  loading
}: PresentationInput): Presentation {
  const modelName = status?.model.displayName ?? "耳机";
  const power = powerCopy(uiState);
  const needle = status?.model.capabilities.eqViaThx ? "usb sound card" : "blackshark";
  const target =
    status?.audioOutput.endpoints.find(
      (endpoint) => endpoint.isHeadset && endpoint.name.toLowerCase().includes(needle)
    ) ??
    status?.audioOutput.endpoints.find((endpoint) => endpoint.isHeadset) ??
    null;
  const current = status?.audioOutput.name ?? "未知设备";

  return {
    modelName,
    connectionLabel: loading ? "读取中…" : connectionLabel(status),
    transportLabel: transportLabel(status),
    stateBadge: stateBadge(status),
    batteryText: status?.connected ? fieldText(status.battery, "%") : "—",
    chargeText: status?.connected ? chargingText(status.charging) : "—",
    thxLabel: thxLabel(status, thx, uiState.thxReady),
    powerTitle: power.title,
    powerDetail: power.detail,
    powerWaitText:
      uiState.kind === "headset_off" ? "正在等待耳机开机…" : "正在等待接收器连接…",
    routeTitle: `先切换到 ${modelName}`,
    routeDetail: ROUTE_DETAIL,
    routeWarningTitle: `默认输出不是 ${modelName}`,
    routeWarningDetail: `当前：${current}。请先在 Windows 声音设置里把 ${modelName} 设为默认输出；THX 功能暂不可用，HID 设置仍可用。`,
    routeCurrentLabel: current,
    routeTargetLabel: target?.name ?? `未找到 ${modelName} 输出，请检查连接`
  };
}

/** The active model's output target, not just "some headset". */
export function outputMatchesModel(status: DeviceSnapshot | null): boolean {
  const name = status?.audioOutput.name?.toLowerCase() ?? "";
  if (!name) return false;
  return status?.model.capabilities.eqViaThx
    ? name.includes("usb sound card")
    : name.includes("blackshark");
}
