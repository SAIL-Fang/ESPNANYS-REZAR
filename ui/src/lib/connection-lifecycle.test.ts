import test from "node:test";
import assert from "node:assert/strict";
import {
  ConnectionLifecycle,
  type ConnectionLifecycleOptions,
  type ConnectionPorts,
  type ForegroundEvents,
  type ForegroundState,
  type LifecycleScheduler
} from "./connection-lifecycle.ts";

class FakeScheduler implements LifecycleScheduler {
  callback: (() => void) | null = null;
  cleared: unknown = null;
  setInterval(callback: () => void): unknown {
    this.callback = callback;
    return 7;
  }
  clearInterval(handle: unknown) {
    this.cleared = handle;
  }
  fire() {
    this.callback?.();
  }
}

class FakeEvents implements ForegroundEvents {
  callback: (() => void) | null = null;
  unsubscribed = false;
  subscribe(callback: () => void) {
    this.callback = callback;
    return () => {
      this.unsubscribed = true;
    };
  }
  fire() {
    this.callback?.();
  }
}

class FakeForeground implements ForegroundState {
  isVisible = true;
  isFocused = true;
  visible() {
    return this.isVisible;
  }
  focused() {
    return this.isFocused;
  }
}

type Call = [silent: boolean, includeThx: boolean];

function makePorts() {
  const state = {
    snapshot: false,
    offline: true,
    busy: false,
    calls: [] as Call[]
  };
  const ports: ConnectionPorts = {
    hasSnapshot: () => state.snapshot,
    isOffline: () => state.offline,
    busy: () => state.busy,
    async refresh(silent, includeThx) {
      state.calls.push([silent, includeThx]);
      state.snapshot = true;
      state.offline = false;
    }
  };
  return { state, ports };
}

function build(ports: ConnectionPorts) {
  const scheduler = new FakeScheduler();
  const foreground = new FakeForeground();
  const events = new FakeEvents();
  const lifecycle = new ConnectionLifecycle({
    scheduler,
    foreground,
    events,
    ports,
    offlineMs: 5_000,
    onlineEveryTicks: 2
  } satisfies ConnectionLifecycleOptions);
  return { scheduler, foreground, events, lifecycle };
}

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

test("start does one full initial refresh when there is no snapshot", async () => {
  const { state, ports } = makePorts();
  const { scheduler, lifecycle } = build(ports);
  lifecycle.start();
  await flush();
  assert.deepEqual(state.calls, [[false, true]]);
  assert.ok(scheduler.callback);
  lifecycle.stop();
});

test("online polling runs every second tick, not every tick", async () => {
  const { state, ports } = makePorts();
  state.snapshot = true;
  state.offline = false;
  const { scheduler, lifecycle } = build(ports);
  lifecycle.start();
  scheduler.fire();
  await flush();
  assert.deepEqual(state.calls, []);
  scheduler.fire();
  await flush();
  assert.deepEqual(state.calls, [[true, false]]);
  lifecycle.stop();
});

test("offline tick reconnects, then fetches THX after HID returns", async () => {
  const { state, ports } = makePorts();
  state.snapshot = true;
  const { scheduler, lifecycle } = build(ports);
  lifecycle.start();
  scheduler.fire();
  await flush();
  assert.deepEqual(state.calls, [
    [true, false],
    [true, true]
  ]);
  lifecycle.stop();
});

test("unfocused or hidden ticks do not touch the device", async () => {
  const { state, ports } = makePorts();
  state.snapshot = true;
  const { scheduler, foreground, lifecycle } = build(ports);
  lifecycle.start();
  foreground.isFocused = false;
  scheduler.fire();
  await flush();
  foreground.isFocused = true;
  foreground.isVisible = false;  scheduler.fire();
  await flush();
  assert.deepEqual(state.calls, []);
  lifecycle.stop();
});

test("foreground event checks immediately and ignores a busy write", async () => {
  const { state, ports } = makePorts();
  state.snapshot = true;
  const { events, lifecycle } = build(ports);
  lifecycle.start();

  state.busy = true;
  events.fire();
  await flush();
  assert.deepEqual(state.calls, []);

  state.busy = false;
  events.fire();
  await flush();
  assert.deepEqual(state.calls, [
    [true, false],
    [true, true]
  ]);
  lifecycle.stop();
});

test("stop clears the timer and unsubscribes", async () => {
  const { state, ports } = makePorts();
  state.snapshot = true;
  const { scheduler, events, lifecycle } = build(ports);
  lifecycle.start();
  lifecycle.stop();
  scheduler.fire();
  events.fire();
  await flush();
  assert.deepEqual(state.calls, []);
  assert.equal(scheduler.cleared, 7);
  assert.equal(events.unsubscribed, true);
});
