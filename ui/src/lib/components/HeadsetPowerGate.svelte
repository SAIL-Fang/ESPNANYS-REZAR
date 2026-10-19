<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";

  let busy = $state(false);
  let error = $state("");

  let blocked = $derived(!!device.status && device.status.connected === false);
  let standby = $derived(device.status?.link === "standby");
  let title = $derived(standby ? "耳机未开机" : "耳机未连接");
  let detail = $derived(
    standby
      ? "接收器仍连接，但耳机本体没有响应。请打开耳机电源；应用会自动恢复。"
      : "没有检测到耳机或接收器。请检查连接后再试。"
  );

  async function refresh() {
    if (busy) return;
    busy = true;
    error = "";
    try {
      await device.refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy = false;
    }
  }

  async function quit() {
    await device.quit();
  }
</script>

{#if blocked}
  <div
    class="route-gate power-gate"
    role="dialog"
    aria-modal="true"
    aria-labelledby="power-gate-title"
    aria-describedby="power-gate-desc"
  >
    <div class="route-gate-card power-gate-card">
      <div class="route-gate-icon power-gate-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">
          <path d="M12 3v8" />
          <path d="M7.5 6.6a7.5 7.5 0 1 0 9 0" />
        </svg>
      </div>

      <div class="route-gate-heading">
        <span class="eyebrow">设备状态</span>
        <h2 id="power-gate-title">{title}</h2>
      </div>
      <p id="power-gate-desc">{detail}</p>

      <div class="power-gate-state">
        <span>接收器</span>
        <strong>{standby ? "已连接" : "未连接"}</strong>
        <span>设置写入</span>
        <strong>已锁定</strong>
      </div>

      {#if error}<p class="route-gate-error">{error}</p>{/if}

      <div class="route-gate-actions">
        <button class="btn primary" type="button" onclick={refresh} disabled={busy || device.busy}>
          {busy ? "检测中…" : "重新检测"}
        </button>
        <div class="route-gate-secondary">
          <button class="btn ghost" type="button" onclick={quit}>退出</button>
        </div>
      </div>

      <p class="route-gate-note">耳机开机后会自动解锁；不会写入默认 EQ 或其他设置。</p>
    </div>
  </div>
{/if}
