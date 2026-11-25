/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
import test from "node:test";
import assert from "node:assert/strict";
import { deriveHubCard, derivePresentation, outputMatchesModel } from "./presentation.ts";
import { deriveUiState } from "./ui-state.ts";
import type {
  AudioEndpoint,
  DeviceSnapshot,
  DeviceSummary,
  Field,
  ModelDescriptor,
  ThxSnapshot
} from "./types";

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
      kind: "headset",
      vendorId: 0x1532,
      wirelessPid: 0x0565,
      wiredPid: 0x056e,
      usagePage: 0xff14,
      usage: 1,
      interfaceNumber: 3,
      eqBands: [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000],
      sidetoneMax: 15,
      capabilities: {
        battery: true,
        charging: true,
        firmware: true,
        serial: true,
        autoPowerOff: true,
        dongleLed: true,
        speakerEq: true,
        eqViaThx: false,
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

function usbStatus(outputName: string): DeviceSnapshot {
  const base = status();
  return {
    ...base,
    link: "wired",
    model: {
      ...base.model,
      id: "razer.blackshark_v2_usb_sound_card",
      displayName: "BlackShark V2 (USB Sound Card)",
      wirelessPid: null,
      wiredPid: 0x0529,
      usagePage: 0xffa0,
      sidetoneMax: null,
      capabilities: {
        ...base.model.capabilities,
        battery: false,
        charging: false,
        firmware: false,
        serial: false,
        autoPowerOff: false,
        dongleLed: false,
        eqViaThx: true,
        sidetone: false
      }
    },
    audioOutput: { ...base.audioOutput, name: outputName, isHeadset: true }
  };
}

test("output target follows the active model", () => {
  assert.equal(outputMatchesModel(status()), true);
  assert.equal(
    outputMatchesModel({
      ...status(),
      audioOutput: { ...status().audioOutput, name: "Razer USB Sound Card" }
    }),
    false
  );
  assert.equal(outputMatchesModel(usbStatus("扬声器 (Razer USB Sound Card)")), true);
  assert.equal(outputMatchesModel(usbStatus("Razer BlackShark V2 HS 2.4")), false);
});

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

function mouseModel(): ModelDescriptor {
  return {
    id: "razer.viper_v3_pro",
    displayName: "Viper V3 Pro",
    kind: "mouse",
    vendorId: 0x1532,
    wirelessPid: 0x00c1,
    wiredPid: 0x00c0,
    usagePage: 0x0001,
    usage: 0x0002,
    interfaceNumber: 0,
    eqBands: [],
    sidetoneMax: null,
    capabilities: {
      battery: true,
      charging: true,
      firmware: false,
      serial: false,
      autoPowerOff: false,
      dongleLed: false,
      speakerEq: false,
      eqViaThx: false,
      sidetone: false,
      thx: false,
      micCapture: false
    }
  };
}

function mouseSummary(overrides: Partial<DeviceSummary> = {}): DeviceSummary {
  return {
    model: mouseModel(),
    connected: true,
    link: "wired",
    battery: field(41),
    charging: field(true),
    thxCapable: false,
    currentOutput: false,
    active: false,
    ...overrides
  };
}

function headsetSummary(overrides: Partial<DeviceSummary> = {}): DeviceSummary {
  return {
    model: status().model,
    connected: true,
    link: "wireless",
    battery: field(94),
    charging: field(false),
    thxCapable: true,
    currentOutput: true,
    active: true,
    ...overrides
  };
}

test("mouse hub card is telemetry only", () => {
  const card = deriveHubCard(mouseSummary());
  assert.equal(card.title, "Viper V3 Pro");
  assert.equal(card.subtitle, "USB 有线");
  assert.equal(card.stateText, "在线");
  assert.equal(card.batteryText, "41% · 充电中");
  assert.equal(card.thxText, null);
  assert.equal(card.outputText, null);
  assert.equal(card.typeText, "有线鼠标");
  assert.equal(card.actionText, "仅显示电量");
  assert.equal(card.interactive, false);
  assert.equal(card.connected, true);
  assert.equal(card.standby, false);
});

test("wireless mouse hub card shows the 2.4 GHz link", () => {
  const card = deriveHubCard(mouseSummary({ link: "wireless" }));
  assert.equal(card.subtitle, "2.4 GHz 无线");
  assert.equal(card.typeText, "无线鼠标");
});

test("sleeping mouse hub card stays visible as a standby card", () => {
  const card = deriveHubCard(
    mouseSummary({
      connected: false,
      link: "standby",
      battery: timeout(),
      charging: timeout()
    })
  );
  assert.equal(card.stateText, "鼠标未开机");
  assert.equal(card.batteryText, "—");
  assert.equal(card.standby, true);
  assert.equal(card.connected, false);
});

test("wired mouse without a reply reads as unavailable, not online", () => {
  const card = deriveHubCard(
    mouseSummary({
      connected: false,
      link: "wired",
      battery: timeout(),
      charging: timeout()
    })
  );
  assert.equal(card.stateText, "鼠标未响应");
  assert.equal(card.batteryText, "—");
  assert.equal(card.standby, false);
  assert.equal(card.connected, false);
});

test("headset hub card keeps the enter action and output row", () => {
  const card = deriveHubCard(headsetSummary());
  assert.equal(card.title, "BlackShark V2 HyperSpeed");
  assert.equal(card.subtitle, "2.4 GHz 无线");
  assert.equal(card.stateText, "当前输出");
  assert.equal(card.batteryText, "94% · 未充电");
  assert.equal(card.thxText, "可用");
  assert.equal(card.outputText, "当前输出");
  assert.equal(card.typeText, "无线耳机");
  assert.equal(card.actionText, "进入设备 →");
  assert.equal(card.interactive, true);
  assert.equal(card.active, true);
});

test("headset hub card reports a powered-off headset", () => {
  const card = deriveHubCard(
    headsetSummary({
      connected: false,
      link: "standby",
      battery: timeout(),
      charging: timeout()
    })
  );
  assert.equal(card.stateText, "耳机未开机");
  assert.equal(card.batteryText, "—");
  assert.equal(card.standby, true);
});
