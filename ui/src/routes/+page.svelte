<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";
  import { chargingText, fieldText } from "$lib/types";
  import StatusStrip from "$lib/components/StatusStrip.svelte";
  import DevicePanel from "$lib/components/DevicePanel.svelte";
  import SoundPanel from "$lib/components/SoundPanel.svelte";
  import DemoPanel from "$lib/components/DemoPanel.svelte";
  import MicPanel from "$lib/components/MicPanel.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import AudioRouteGate from "$lib/components/AudioRouteGate.svelte";
  import HeadsetPowerGate from "$lib/components/HeadsetPowerGate.svelte";
  import { deriveUiState } from "$lib/ui-state";

  type Tab = "device" | "sound" | "demo" | "mic" | "settings";

  let tab = $state<Tab>("sound");

  const labels: Record<Tab, string> = {
    device: "设备",
    sound: "声音",
    demo: "演示",
    mic: "麦克风",
    settings: "设置"
  };

  let sectionLabel = $derived(labels[tab]);

  function linkText(): string {
    switch (device.status?.link) {
      case "wireless":
        return "无线 2.4G";
      case "wired":
        return "有线 USB";
      default:
        return "未连接";
    }
  }

  let batteryText = $derived(
    device.status?.connected ? fieldText(device.status.battery, "%") : "—"
  );
  let chargeText = $derived(
    device.status?.connected ? chargingText(device.status.charging) : "—"
  );
  let uiState = $derived(
    deriveUiState({ status: device.status, thx: device.thx, loading: device.loading })
  );
  let pageLocked = $derived(!uiState.canWrite);
  let modelName = $derived(device.status?.model.displayName ?? "耳机");
</script>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand"><span class="brand-full">ESPNANYS-REZAR</span><span class="brand-short">ER</span></div>

    <nav class="side-nav" aria-label="主导航">
      <button class:active={tab === "device"} onclick={() => (tab = "device")} aria-label="设备">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M4 14v-2a8 8 0 0 1 16 0v2"/><path d="M4 14h3v5H5a1 1 0 0 1-1-1zM20 14h-3v5h2a1 1 0 0 0 1-1z"/></svg>
        <span>设备</span>
      </button>
      <button class:active={tab === "sound"} onclick={() => (tab = "sound")} aria-label="声音">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M4 7h10M18 7h2M4 17h2M10 17h10"/><circle cx="16" cy="7" r="2.2"/><circle cx="8" cy="17" r="2.2"/></svg>
        <span>声音</span>
      </button>
      <button class:active={tab === "demo"} onclick={() => (tab = "demo")} aria-label="演示">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"><path d="M8 5.5v13l10-6.5z"/><circle cx="12" cy="12" r="9" opacity=".35"/></svg>
        <span>演示</span>
      </button>
      <button class:active={tab === "mic"} onclick={() => (tab = "mic")} aria-label="麦克风">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><rect x="8" y="3.5" width="8" height="12" rx="4"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3M9 21h6"/></svg>
        <span>麦克风</span>
      </button>
      <button class:active={tab === "settings"} onclick={() => (tab = "settings")} aria-label="设置">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><circle cx="12" cy="12" r="3.1"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.3 2.3-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-3.2v-.2a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L6.1 17l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.6-1H4.7v-3.2h.2a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L6.1 8l2.3-2.3.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.6v-.2h3.2v.2a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 8l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v3.2H21a1.7 1.7 0 0 0-1.6 1z"/></svg>
        <span>设置</span>
      </button>
    </nav>

    <div class="side-device">
      <div class="side-device-name"><span class="dot" class:on={device.status?.connected}></span> {modelName}</div>
      <div class="side-device-meta">
        {linkText()} · <span class="mono">{batteryText}</span> · {chargeText}
      </div>
    </div>
  </aside>

  <main class="main-surface">
    <StatusStrip label={sectionLabel} />

    <div class="page-content" inert={pageLocked}>
      {#if uiState.outputBlocked}
        <div class="route-warning">
          <strong>默认输出不是 Razer 耳机</strong>
          <span>
            当前：{device.status?.audioOutput.name ?? "未知设备"}。
            请先在 Windows 声音设置里把 {modelName} 设为默认输出；THX 功能暂不可用，HID 设置仍可用。
          </span>
        </div>
      {/if}

      {#if device.statusError || device.error}
        <div class="inline-status error">{device.statusError || device.error}</div>
      {/if}

      {#if tab === "device"}
        <DevicePanel />
      {:else if tab === "sound"}
        <SoundPanel />
      {:else if tab === "demo"}
        <DemoPanel />
      {:else if tab === "mic"}
        <MicPanel />
      {:else}
        <SettingsPanel />
      {/if}
    </div>
  </main>
</div>

<AudioRouteGate />
<HeadsetPowerGate />
