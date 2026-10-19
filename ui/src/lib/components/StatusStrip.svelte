<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";
  import { known } from "$lib/types";
  import { derivePresentation } from "$lib/presentation";
  import { deriveUiState } from "$lib/ui-state";

  interface Props {
    label?: string;
  }

  let { label = "声音" }: Props = $props();

  let battery = $derived(known(device.status?.battery, 0));
  let batteryOn = $derived(Math.max(0, Math.min(5, Math.round(battery / 20))));
  let uiState = $derived(
    deriveUiState({ status: device.status, thx: device.thx, loading: device.loading })
  );
  let presentation = $derived(
    derivePresentation({
      status: device.status,
      thx: device.thx,
      uiState,
      loading: device.loading
    })
  );
  let wiredNotReady = $derived(uiState.wiredNotReady);
  let thxReady = $derived(uiState.thxReady);
  let thxInstalled = $derived(!!device.thx?.installed);
  let charging = $derived(
    device.status?.charging.state === "known" && device.status.charging.value
  );
</script>

<header class="status-strip">
  <div class="topbar-context">ESPNANYS-REZAR <span>›</span> <b>{label}</b></div>
  <div class="status-item">
    <span class="dot" class:on={device.status?.connected} class:standby={device.status?.link === "standby"} class:warn={wiredNotReady}></span>
    <strong>{presentation.connectionLabel}</strong>
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
    <strong class="mono">{presentation.batteryText}</strong>
  </div>
  <div class="status-item">
    <span class="dot" class:on={thxReady} class:warn={thxInstalled && !thxReady}></span>
    <strong>{presentation.thxLabel}</strong>
  </div>
  <div class="spacer"></div>
  <button class="btn" onclick={() => device.refresh()} disabled={device.loading}>
    {device.loading ? "读取中…" : "刷新"}
  </button>
</header>
