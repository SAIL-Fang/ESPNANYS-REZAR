<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { device } from "$lib/device.svelte";
  import { fieldText, known } from "$lib/types";
  import { deriveUiState } from "$lib/ui-state";

  interface Props {
    label?: string;
  }

  let { label = "声音" }: Props = $props();

  onMount(() => {
    if (!device.status && !device.thx) device.refresh();
    // Do not spawn the THX helper every 10 seconds: its DLL binding calls
    // SetDefaultRenderDevice and can briefly interrupt playback. THX state is
    // refreshed on startup, explicit refresh, and after THX writes.
    const timer = setInterval(() => {
      if (document.hidden || !document.hasFocus()) return;
      // The power gate owns reconnect polling while the headset is offline.
      if (device.status && !device.status.connected) return;
      void device.refresh(true, false);
    }, 10_000);
    return () => clearInterval(timer);
  });

  let battery = $derived(known(device.status?.battery, 0));
  let batteryOn = $derived(Math.max(0, Math.min(5, Math.round(battery / 20))));
  let batteryText = $derived(
    device.status?.connected ? fieldText(device.status.battery, "%") : "—"
  );
  let audioOk = $derived(device.status?.audioOutput?.isHeadset !== false);
  let uiState = $derived(
    deriveUiState({ status: device.status, thx: device.thx, loading: device.loading })
  );
  let wiredNotReady = $derived(uiState.wiredNotReady);
  let thxInstalled = $derived(!!device.thx?.installed);
  let thxReady = $derived(uiState.thxReady);
  let thxLabel = $derived(
    !thxInstalled
      ? "THX 未连接"
      : !audioOk
        ? "THX 暂停"
        : thxReady
          ? "THX 就绪"
          : "THX 不可用"
  );
  let charging = $derived(
    device.status?.charging.state === "known" && device.status.charging.value
  );

  function linkText(): string {
    if (!device.status) return "未连接";
    if (device.status.link === "standby") return "耳机未开机";
    if (device.status.link === "wireless") return "无线已连接";
    if (device.status.link === "wired") return "有线已连接";
    return device.status.connected ? "已连接" : "未连接";
  }
</script>

<header class="status-strip">
  <div class="topbar-context">ESPNANYS-REZAR <span>›</span> <b>{label}</b></div>
  <div class="status-item">
    <span class="dot" class:on={device.status?.connected} class:standby={device.status?.link === "standby"} class:warn={wiredNotReady}></span>
    <strong>{device.loading ? "读取中…" : linkText()}</strong>
    {#if device.status?.link === "standby"}
      <span class="badge warn">请开机</span>
    {/if}
    {#if wiredNotReady}
      <span class="badge warn">控制未就绪</span>
    {/if}
    {#if charging}
      <span class="badge ok">充电中</span>
    {/if}
  </div>
  <div class="status-item">
    <div class="battery" aria-label="电量">
      {#each [0, 1, 2, 3, 4] as i}
        <span class:on={i < batteryOn}></span>
      {/each}
    </div>
    <strong class="mono">{batteryText}</strong>
  </div>
  <div class="status-item">
    <span class="dot" class:on={thxReady} class:warn={thxInstalled && !thxReady}></span>
    <strong>{thxLabel}</strong>
  </div>
  <div class="spacer"></div>
  <button class="btn" onclick={() => device.refresh()} disabled={device.loading}>
    {device.loading ? "读取中…" : "刷新"}
  </button>
</header>
