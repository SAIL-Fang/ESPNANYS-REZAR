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
import type {
  DeviceSnapshot,
  DeviceSummary,
  Link,
  ModelDescriptor,
  ThxSnapshot
} from "./types.ts";
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

/** Copy that frames the hub as a launcher for control surfaces only. */
export const HUB_COPY = {
  headline: "选择一个已连接的雷蛇设备。只有耳机可以进入控制；选择设备不会自动更改 Windows 默认输出。",
  emptyTitle: "未检测到受支持的雷蛇设备",
  emptyDetail: "请连接受支持的雷蛇设备：耳机可进入控制，鼠标仅显示电量。",
  scanningTitle: "正在扫描设备…",
  scanningDetail: "正在识别已连接的雷蛇设备和输出状态。"
} as const;

/**
 * One hub card, already resolved to display strings. Mice are telemetry-only:
 * they show battery and link state, never THX/output rows, and cannot open a
 * workspace.
 */
export interface HubCard {
  title: string;
  subtitle: string;
  stateText: string;
  batteryText: string;
  thxText: string | null;
  outputText: string | null;
  typeText: string;
  actionText: string;
  interactive: boolean;
  active: boolean;
  connected: boolean;
  standby: boolean;
}

export function modelTitle(model: ModelDescriptor): string {
  if (model.capabilities.eqViaThx) return "BlackShark V2";
  return model.displayName;
}

export function modelSubtitle(model: ModelDescriptor, link?: Link): string {
  if (model.kind === "mouse") {
    if (link === "wireless") return "2.4 GHz 无线";
    if (link === "wired") return "USB 有线";
    return "无线鼠标";
  }
  if (model.capabilities.eqViaThx) {
    return "Razer USB Sound Card · 3.5mm";
  }
  if (link === "wireless") return "2.4 GHz 无线";
  if (link === "wired") return "USB 有线";
  return "无线耳机";
}

function hubStateText(summary: DeviceSummary): string {
  const { model } = summary;
  if (!summary.connected) {
    if (model.kind === "mouse") {
      if (summary.link === "standby") return "鼠标未开机";
      if (summary.link === "wired") return "鼠标未响应";
      return "未连接";
    }
    if (summary.link !== "standby") return "未连接";
    return "耳机未开机";
  }
  // Output routing is a headset concern; a mouse is simply online.
  if (model.kind === "mouse") return "在线";
  return summary.currentOutput ? "当前输出" : "输出未指向";
}

function hubBatteryText(summary: DeviceSummary): string {
  if (!summary.model.capabilities.battery) return "无电池遥测";
  if (!summary.connected) return "—";
  return `${fieldText(summary.battery, "%")} · ${chargingText(summary.charging)}`;
}

function hubTypeText(model: ModelDescriptor, link: Link): string {
  if (model.kind === "mouse") {
    if (link === "wireless") return "无线鼠标";
    if (link === "wired") return "有线鼠标";
    return "鼠标";
  }
  return model.capabilities.eqViaThx ? "USB 声卡" : "无线耳机";
}

export function deriveHubCard(summary: DeviceSummary): HubCard {
  const { model } = summary;
  const telemetryOnly = model.kind === "mouse";
  return {
    title: modelTitle(model),
    subtitle: modelSubtitle(model, summary.link),
    stateText: hubStateText(summary),
    batteryText: hubBatteryText(summary),
    thxText: telemetryOnly ? null : summary.thxCapable ? "可用" : "不支持",
    outputText: telemetryOnly ? null : summary.currentOutput ? "当前输出" : "未指向",
    typeText: hubTypeText(model, summary.link),
    actionText: telemetryOnly ? "仅显示电量" : "进入设备 →",
    interactive: !telemetryOnly,
    active: summary.active,
    connected: summary.connected,
    standby: summary.link === "standby"
  };
}
