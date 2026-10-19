/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 *
 * One pure state derivation for every blocking/usable UI surface. Components
 * must not independently combine connected/link/audio/THX conditions.
 */
import type { DeviceSnapshot, ThxSnapshot } from "./types";

export type UiStateKind =
  | "booting"
  | "no_receiver"
  | "headset_off"
  | "output_wrong"
  | "thx_unavailable"
  | "ready";

export interface UiState {
  kind: UiStateKind;
  /** Full-screen power/receiver gate. */
  powerBlocked: boolean;
  /** Audio-route blocking dialog. Never true at the same time as powerBlocked. */
  outputBlocked: boolean;
  /** Settings writes are allowed by the UI. Backend still re-checks HID. */
  canWrite: boolean;
  /** THX is installed, bound, and usable on the current output. */
  thxReady: boolean;
  wiredNotReady: boolean;
}

export interface UiStateInput {
  status: DeviceSnapshot | null;
  thx: ThxSnapshot | null;
  loading: boolean;
}

export interface ReconnectPolicyInput {
  blocked: boolean;
  visible: boolean;
  focused: boolean;
  busy: boolean;
}

/** The reconnect loop is UI-driven: no foreground visibility, no HID polling. */
export function shouldAutoReconnect({
  blocked,
  visible,
  focused,
  busy
}: ReconnectPolicyInput): boolean {
  return blocked && visible && focused && !busy;
}

export interface StatusPollPolicyInput {
  visible: boolean;
  focused: boolean;
  offline: boolean;
}

/** The general status poll never runs in the background or while offline. */
export function shouldPollStatus({
  visible,
  focused,
  offline
}: StatusPollPolicyInput): boolean {
  return visible && focused && !offline;
}

export function deriveUiState({ status, thx, loading }: UiStateInput): UiState {
  if (!status) {
    return {
      kind: loading ? "booting" : "no_receiver",
      powerBlocked: !loading,
      outputBlocked: false,
      canWrite: false,
      thxReady: false,
      wiredNotReady: false
    };
  }

  const wiredNotReady =
    status.link === "wired" &&
    (status.battery.state !== "known" || status.charging.state !== "known");

  if (!status.connected) {
    const standby = status.link === "standby";
    return {
      kind: standby ? "headset_off" : "no_receiver",
      powerBlocked: true,
      outputBlocked: false,
      canWrite: false,
      thxReady: false,
      wiredNotReady
    };
  }

  if (status.audioOutput.isHeadset === false) {
    return {
      kind: "output_wrong",
      powerBlocked: false,
      outputBlocked: true,
      canWrite: false,
      thxReady: false,
      wiredNotReady
    };
  }

  const thxReady = !!thx?.installed && !thx.error && !!thx.endpoint;
  if (!thxReady) {
    return {
      kind: "thx_unavailable",
      powerBlocked: false,
      outputBlocked: false,
      canWrite: true,
      thxReady: false,
      wiredNotReady
    };
  }

  return {
    kind: "ready",
    powerBlocked: false,
    outputBlocked: false,
    canWrite: true,
    thxReady: true,
    wiredNotReady
  };
}
