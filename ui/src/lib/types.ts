/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
export type Field<T> =
  | { state: "known"; value: T }
  | { state: "unsupported" }
  | { state: "timeout" }
  | { state: "error"; value: string };

export type Link = "disconnected" | "wireless" | "wired" | "unknown";

export type DeviceSnapshot = {
  connected: boolean;
  link: Link;
  battery: Field<number>;
  charging: Field<boolean>;
  firmware: Field<string>;
  serial: Field<string>;
  autoPowerOffMin: Field<number>;
  dongleLed: Field<number>;
  speakerPresetEq: Field<number>;
  edition: Field<number>;
  audioOutput: {
    name: string | null;
    isHeadset: boolean;
    error: string | null;
  };
  issues: string[];
};

export type ThxSnapshot = {
  installed: boolean;
  enabled: boolean | null;
  level: number | null;
  spatialEnabled: boolean | null;
  endpoint: string | null;
  dll: string | null;
  dllVersion: string | null;
  abi: string | null;
  error: string | null;
};

export type EqPreset = {
  id: string;
  label: string;
  gains: number[];
};

export function known<T>(field: Field<T> | undefined, fallback: T): T {
  return field?.state === "known" ? field.value : fallback;
}

export function fieldText<T>(field: Field<T> | undefined, suffix = ""): string {
  if (!field) return "—";
  switch (field.state) {
    case "known":
      return `${field.value}${suffix}`;
    case "unsupported":
      return "不支持";
    case "timeout":
      return "超时";
    case "error":
      return `错误：${field.value}`;
  }
}

export function chargingText(field: Field<boolean> | undefined): string {
  if (!field) return "—";
  switch (field.state) {
    case "known":
      return field.value ? "充电中" : "未充电";
    case "unsupported":
      return "不支持";
    case "timeout":
      return "超时";
    case "error":
      return `错误：${field.value}`;
  }
}
