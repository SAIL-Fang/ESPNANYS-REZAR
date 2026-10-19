<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";
  import { deriveUiState } from "$lib/ui-state";

  let uiState = $derived(
    deriveUiState({ status: device.status, thx: device.thx, loading: device.loading })
  );
  let blocked = $derived(uiState.powerBlocked);
  let standby = $derived(uiState.kind === "headset_off");
  let title = $derived(uiState.powerTitle);
  let detail = $derived(uiState.powerDetail);
  let waitText = $derived(
    standby ? "正在等待耳机开机…" : "正在等待接收器连接…"
  );

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

      <div class="power-gate-wait" role="status">
        <span class="power-gate-pulse" aria-hidden="true"></span>
        {waitText}
      </div>

      <div class="route-gate-actions">
        <button class="btn ghost" type="button" onclick={quit}>退出</button>
      </div>

      <p class="route-gate-note">打开耳机电源后会自动恢复，不需要手动刷新；不会写入默认 EQ 或其他设置。</p>
    </div>
  </div>
{/if}
