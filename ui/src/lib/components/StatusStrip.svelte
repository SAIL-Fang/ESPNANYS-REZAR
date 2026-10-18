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

  interface Props {
    label?: string;
  }

  let { label = "声音" }: Props = $props();

  onMount(() => {
    if (!device.status && !device.thx) device.refresh();
    const timer = setInterval(() => device.refresh(true), 10_000);
    return () => clearInterval(timer);
  });

  let battery = $derived(known(device.status?.battery, 0));
  let batteryOn = $derived(Math.max(0, Math.min(5, Math.round(battery / 20))));
  let wiredNotReady = $derived(
    device.status?.link === "wired" &&
      (device.status.battery.state !== "known" || device.status.charging.state !== "known")
  );
  let charging = $derived(
    device.status?.charging.state === "known" && device.status.charging.value
  );

  function linkText(): string {
    if (!device.status) return "未连接";
    if (device.status.link === "wireless") return "无线已连接";
    if (device.status.link === "wired") return "有线已连接";
    return device.status.connected ? "已连接" : "未连接";
  }
</script>

<header class="status-strip">
  <div class="topbar-context">ESPNANYS-REZAR <span>›</span> <b>{label}</b></div>
  <div class="status-item">
    <span class="dot" class:on={device.status?.connected}></span>
    <strong>{device.loading ? "读取中…" : linkText()}</strong>
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
    <strong class="mono">{fieldText(device.status?.battery, "%")}</strong>
  </div>
  <div class="status-item">
    <span class="dot" class:on={device.thx?.installed} class:warn={!device.thx?.installed}></span>
    <strong>{device.thx?.installed ? "THX 就绪" : "THX 未连接"}</strong>
  </div>
  <div class="spacer"></div>
  <button class="btn" onclick={() => device.refresh()} disabled={device.loading}>
    {device.loading ? "读取中…" : "刷新"}
  </button>
</header>
