/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 *
 * One foreground lifecycle for HID refresh and reconnect. UI modules render;
 * this module owns cadence and foreground checks so a hidden window never
 * starts an HID poll.
 */
import { shouldAutoReconnect, shouldPollStatus } from "./ui-state.ts";

export interface LifecycleScheduler {
  setInterval(callback: () => void, milliseconds: number): unknown;
  clearInterval(handle: unknown): void;
}

export interface ForegroundState {
  visible(): boolean;
  focused(): boolean;
}

export interface ForegroundEvents {
  subscribe(callback: () => void): () => void;
}

export interface ConnectionPorts {
  hasSnapshot(): boolean;
  isOffline(): boolean;
  busy(): boolean;
  refresh(silent: boolean, includeThx: boolean): Promise<void>;
}

export interface ConnectionLifecycleOptions {
  scheduler: LifecycleScheduler;
  foreground: ForegroundState;
  events: ForegroundEvents;
  ports: ConnectionPorts;
  offlineMs?: number;
  onlineEveryTicks?: number;
}

export const browserScheduler: LifecycleScheduler = {
  setInterval(callback, milliseconds) {
    return globalThis.setInterval(callback, milliseconds);
  },
  clearInterval(handle) {
    globalThis.clearInterval(handle as ReturnType<typeof globalThis.setInterval>);
  }
};

export const browserForeground: ForegroundState = {
  visible: () => !document.hidden,
  focused: () => document.hasFocus()
};

export const browserForegroundEvents: ForegroundEvents = {
  subscribe(callback) {
    const onForeground = () => callback();
    window.addEventListener("focus", onForeground);
    document.addEventListener("visibilitychange", onForeground);
    return () => {
      window.removeEventListener("focus", onForeground);
      document.removeEventListener("visibilitychange", onForeground);
    };
  }
};

export class ConnectionLifecycle {
  private readonly scheduler: LifecycleScheduler;
  private readonly foreground: ForegroundState;
  private readonly events: ForegroundEvents;
  private readonly ports: ConnectionPorts;
  private readonly offlineMs: number;
  private readonly onlineEveryTicks: number;
  private timer: unknown = null;
  private unsubscribe: (() => void) | null = null;
  private inFlight = false;
  private onlineTicks = 0;

  constructor(options: ConnectionLifecycleOptions) {
    this.scheduler = options.scheduler;
    this.foreground = options.foreground;
    this.events = options.events;
    this.ports = options.ports;
    this.offlineMs = options.offlineMs ?? 5_000;
    this.onlineEveryTicks = options.onlineEveryTicks ?? 2;
  }

  start() {
    if (this.timer !== null) return;
    this.timer = this.scheduler.setInterval(() => {
      void this.tick();
    }, this.offlineMs);
    this.unsubscribe = this.events.subscribe(() => {
      void this.onForeground();
    });

    // The old status strip did an initial full refresh. Keep that single
    // startup read here, without waiting for the first tick.
    if (!this.ports.hasSnapshot()) {
      void this.refreshInitial();
    }
  }

  stop() {
    if (this.timer !== null) {
      this.scheduler.clearInterval(this.timer);
      this.timer = null;
    }
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
  }

  /** Exposed for deterministic tests; the timer calls the same method. */
  async tick() {
    if (this.timer === null) return;
    if (!this.isForeground()) return;

    if (this.ports.isOffline()) {
      await this.reconnect();
      return;
    }

    this.onlineTicks += 1;
    if (this.onlineTicks % this.onlineEveryTicks !== 0) return;
    if (
      !shouldPollStatus({
        visible: this.foreground.visible(),
        focused: this.foreground.focused(),
        offline: false
      })
    ) {
      return;
    }
    await this.refreshSilently(false);
  }

  /** Exposed for deterministic tests; also the window's focus handler. */
  async onForeground() {
    if (this.timer === null) return;
    if (!this.isForeground()) return;

    if (this.ports.isOffline()) {
      await this.reconnect();
      return;
    }

    // A foreground return is a good moment to reconcile a stale status strip.
    await this.refreshSilently(false);
  }

  private isForeground(): boolean {
    return this.foreground.visible() && this.foreground.focused();
  }

  private async refreshInitial() {
    await this.refresh(false, true);
  }

  private async refreshSilently(includeThx: boolean) {
    await this.refresh(true, includeThx);
  }

  private async reconnect() {
    const allowed = shouldAutoReconnect({
      blocked: this.ports.isOffline(),
      visible: this.foreground.visible(),
      focused: this.foreground.focused(),
      busy: this.inFlight || this.ports.busy()
    });
    if (!allowed) return;

    await this.refresh(true, false);
    if (!this.ports.isOffline()) {
      // HID is back; fetch THX once without delaying the unlock.
      await this.refresh(true, true);
    }
  }

  private async refresh(silent: boolean, includeThx: boolean) {
    if (this.inFlight || this.ports.busy()) return;
    this.inFlight = true;
    try {
      await this.ports.refresh(silent, includeThx);
    } catch {
      // The ports own error presentation. A rejected poll must not create an
      // unhandled rejection or kill the interval.
    } finally {
      this.inFlight = false;
    }
  }
}
