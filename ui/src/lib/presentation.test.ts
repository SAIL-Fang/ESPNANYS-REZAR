/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
import test from "node:test";
import assert from "node:assert/strict";
import { derivePresentation } from "./presentation.ts";
import { deriveUiState } from "./ui-state.ts";
import type { AudioEndpoint, DeviceSnapshot, Field, ThxSnapshot } from "./types";

function field<T>(value: T): Field<T> {
  return { state: "known", value };
}

function timeout<T>(): Field<T> {
  return { state: "timeout" };
}

function endpoint(isHeadset = true): AudioEndpoint {
  return {
    id: isHeadset ? "headset" : "speakers",
    name: isHeadset ? "Razer BlackShark V2 HS 2.4" : "音响 (USBAudio2.0)",
    isHeadset,
    isDefault: true
  };
}

function status(overrides: Partial<DeviceSnapshot> = {}): DeviceSnapshot {
  return {
    connected: true,
    link: "wireless",
    model: {
      id: "razer.blackshark_v2_hyperspeed",
      displayName: "BlackShark V2 HyperSpeed",
      vendorId: 0x1532,
      wirelessPid: 0x0565,
      wiredPid: 0x056e,
      usagePage: 0xff14,
      usage: 1,
      interfaceNumber: 3,
      eqBands: [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000],
      capabilities: {
        battery: true,
        charging: true,
        firmware: true,
        serial: true,
        autoPowerOff: true,
        dongleLed: true,
        speakerEq: true,
        sidetone: true,
        thx: true,
        micCapture: true
      }
    },
    battery: field(94),
    charging: field(false),
    firmware: field("1.6.11.0"),
    serial: field("serial"),
    autoPowerOffMin: field(15),
    dongleLed: field(2),
    speakerPresetEq: field(0),
    edition: field(1),
    audioOutput: {
      name: "Razer BlackShark V2 HS 2.4",
      isHeadset: true,
      error: null,
      endpoints: [endpoint(true), endpoint(false)]
    },
    issues: [],
    ...overrides
  };
}

function thx(overrides: Partial<ThxSnapshot> = {}): ThxSnapshot {
  return {
    installed: true,
    enabled: true,
    level: 5,
    spatialEnabled: true,
    endpoint: "Razer BlackShark V2 HS 2.4",
    dll: "ThxV3Native_v1.1.42.1.dll",
    dllVersion: "1.1.42.1",
    abi: "x64",
    capture: null,
    error: null,
    ...overrides
  };
}

function present(
  snapshot: DeviceSnapshot | null = status(),
  thxSnapshot: ThxSnapshot | null = thx(),
  loading = false
) {
  const uiState = deriveUiState({ status: snapshot, thx: thxSnapshot, loading });
  return {
    uiState,
    view: derivePresentation({ status: snapshot, thx: thxSnapshot, uiState, loading })
  };
}

test("wireless presentation keeps state and transport labels separate", () => {
  const { view } = present();
  assert.equal(view.connectionLabel, "无线已连接");
  assert.equal(view.transportLabel, "无线 2.4G");
  assert.equal(view.batteryText, "94%");
  assert.equal(view.chargeText, "未充电");
  assert.equal(view.thxLabel, "THX 就绪");
});

test("powered-off presentation hides telemetry and owns the gate copy", () => {
  const { view } = present(
    status({ connected: false, link: "standby", battery: timeout(), charging: timeout() })
  );
  assert.equal(view.connectionLabel, "耳机未开机");
  assert.equal(view.transportLabel, "未连接");
  assert.equal(view.batteryText, "—");
  assert.equal(view.chargeText, "—");
  assert.equal(view.powerTitle, "耳机未开机");
  assert.equal(view.powerWaitText, "正在等待耳机开机…");
});

test("missing receiver presentation is explicit and does not claim a transport", () => {
  const { view } = present(null, null, false);
  assert.equal(view.connectionLabel, "未连接");
  assert.equal(view.transportLabel, "未连接");
  assert.equal(view.powerTitle, "耳机未连接");
  assert.equal(view.powerWaitText, "正在等待接收器连接…");
});

test("wrong output pauses THX and names the model-specific target", () => {
  const { view } = present(
    status({
      audioOutput: {
        name: "音响 (USBAudio2.0)",
        isHeadset: false,
        error: null,
        endpoints: [endpoint(true), endpoint(false)]
      }
    })
  );
  assert.equal(view.connectionLabel, "无线已连接");
  assert.equal(view.thxLabel, "THX 暂停");
  assert.equal(view.routeTitle, "先切换到 BlackShark V2 HyperSpeed");
  assert.equal(view.routeCurrentLabel, "音响 (USBAudio2.0)");
  assert.equal(view.routeTargetLabel, "Razer BlackShark V2 HS 2.4");
});

test("model without THX capability never presents THX as ready", () => {
  const { view } = present(
    status({
      model: {
        ...status().model,
        capabilities: { ...status().model.capabilities, thx: false }
      }
    })
  );
  assert.equal(view.thxLabel, "THX 不支持");
});
