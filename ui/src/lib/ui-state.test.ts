/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
import test from "node:test";
import assert from "node:assert/strict";
import { deriveUiState, shouldAutoReconnect, shouldPollStatus } from "./ui-state.ts";
import type { AudioEndpoint, DeviceSnapshot, Field, Link, ThxSnapshot } from "./types";

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
    link: "wireless" as Link,
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

test("booting before the first snapshot", () => {
  const state = deriveUiState({ status: null, thx: null, loading: true });
  assert.equal(state.kind, "booting");
  assert.equal(state.powerBlocked, false);
  assert.equal(state.canWrite, false);
});

test("a failed initial load becomes no_receiver", () => {
  const state = deriveUiState({ status: null, thx: null, loading: false });
  assert.equal(state.kind, "no_receiver");
  assert.equal(state.powerBlocked, true);
});

test("disconnected receiver is no_receiver", () => {
  const state = deriveUiState({
    status: status({ connected: false, link: "disconnected", battery: timeout(), charging: timeout() }),
    thx: thx(),
    loading: false
  });
  assert.equal(state.kind, "no_receiver");
  assert.equal(state.powerBlocked, true);
  assert.equal(state.outputBlocked, false);
});

test("receiver present with powered-off headset is headset_off", () => {
  const state = deriveUiState({
    status: status({ connected: false, link: "standby", battery: timeout(), charging: timeout() }),
    thx: thx({ enabled: null, level: null }),
    loading: false
  });
  assert.equal(state.kind, "headset_off");
  assert.equal(state.powerBlocked, true);
  assert.equal(state.powerTitle, "耳机未开机");
  assert.equal(state.canWrite, false);
  assert.equal(state.thxReady, false);
});

test("power gate wins over a wrong audio route", () => {
  const state = deriveUiState({
    status: status({
      connected: false,
      link: "standby",
      battery: timeout(),
      charging: timeout(),
      audioOutput: {
        name: "音响 (USBAudio2.0)",
        isHeadset: false,
        error: null,
        endpoints: [endpoint(false)]
      }
    }),
    thx: thx(),
    loading: false
  });
  assert.equal(state.kind, "headset_off");
  assert.equal(state.outputBlocked, false);
});

test("online headset with wrong output is output_wrong", () => {
  const state = deriveUiState({
    status: status({
      audioOutput: {
        name: "音响 (USBAudio2.0)",
        isHeadset: false,
        error: null,
        endpoints: [endpoint(false), endpoint(true)]
      }
    }),
    thx: thx(),
    loading: false
  });
  assert.equal(state.kind, "output_wrong");
  assert.equal(state.powerBlocked, false);
  assert.equal(state.outputBlocked, true);
  assert.equal(state.canWrite, false);
  assert.equal(state.thxReady, false);
});

test("wrong output wins over missing THX", () => {
  const state = deriveUiState({
    status: status({
      audioOutput: {
        name: "音响 (USBAudio2.0)",
        isHeadset: false,
        error: null,
        endpoints: [endpoint(false)]
      }
    }),
    thx: null,
    loading: false
  });
  assert.equal(state.kind, "output_wrong");
  assert.equal(state.outputBlocked, true);
});

test("online headset, correct output, THX ready is ready", () => {
  const state = deriveUiState({ status: status(), thx: thx(), loading: false });
  assert.equal(state.kind, "ready");
  assert.equal(state.powerBlocked, false);
  assert.equal(state.outputBlocked, false);
  assert.equal(state.canWrite, true);
  assert.equal(state.thxReady, true);
});

test("missing THX leaves HID settings available", () => {
  const state = deriveUiState({ status: status(), thx: null, loading: false });
  assert.equal(state.kind, "thx_unavailable");
  assert.equal(state.canWrite, true);
  assert.equal(state.thxReady, false);
});

test("THX with an error is unavailable", () => {
  const state = deriveUiState({
    status: status(),
    thx: thx({ installed: false, error: "THX endpoint missing" }),
    loading: false
  });
  assert.equal(state.kind, "thx_unavailable");
});

test("THX without a bound endpoint is unavailable", () => {
  const state = deriveUiState({
    status: status(),
    thx: thx({ endpoint: null }),
    loading: false
  });
  assert.equal(state.kind, "thx_unavailable");
  assert.equal(state.thxReady, false);
});

test("wired telemetry timeout is flagged, but the headset can still be ready", () => {
  const state = deriveUiState({
    status: status({ link: "wired", battery: timeout(), charging: timeout() }),
    thx: thx(),
    loading: false
  });
  assert.equal(state.kind, "ready");
  assert.equal(state.wiredNotReady, true);
});

test("reconnect only runs while blocked, visible, focused, and idle", () => {
  assert.equal(
    shouldAutoReconnect({ blocked: true, visible: true, focused: true, busy: false }),
    true
  );
  assert.equal(
    shouldAutoReconnect({ blocked: false, visible: true, focused: true, busy: false }),
    false
  );
  assert.equal(
    shouldAutoReconnect({ blocked: true, visible: false, focused: true, busy: false }),
    false
  );
  assert.equal(
    shouldAutoReconnect({ blocked: true, visible: true, focused: false, busy: false }),
    false
  );
  assert.equal(
    shouldAutoReconnect({ blocked: true, visible: true, focused: true, busy: true }),
    false
  );
});

test("status polling is foreground-only and ignores the offline gate", () => {
  assert.equal(shouldPollStatus({ visible: true, focused: true, offline: false }), true);
  assert.equal(shouldPollStatus({ visible: false, focused: true, offline: false }), false);
  assert.equal(shouldPollStatus({ visible: true, focused: false, offline: false }), false);
  assert.equal(shouldPollStatus({ visible: true, focused: true, offline: true }), false);
});
