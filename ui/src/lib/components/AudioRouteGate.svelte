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

  let endpoints = $derived(device.status?.audioOutput?.endpoints ?? []);
  let current = $derived(device.status?.audioOutput?.name ?? "未知设备");
  let target = $derived(endpoints.find((endpoint) => endpoint.isHeadset) ?? null);
  let blocked = $derived(
    !!device.status?.audioOutput && device.status.audioOutput.isHeadset === false
  );

  async function apply() {
    if (!target || busy) return;
    busy = true;
    error = "";
    const ok = await device.setDefaultAudioOutput(target.id);
    if (!ok) error = device.error || "切换默认输出失败";
    busy = false;
  }

  async function openSettings() {
    await device.openSoundSettings();
  }

  async function quit() {
    await device.quit();
  }
</script>

{#if blocked}
  <div class="route-gate" role="dialog" aria-modal="true" aria-labelledby="route-gate-title" aria-describedby="route-gate-desc">
    <div class="route-gate-card">
      <div class="route-gate-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <path d="M4 14v-2a8 8 0 0 1 16 0v2" />
          <path d="M4 14h2a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a1 1 0 0 1-1-1z" />
          <path d="M20 14h-2a2 2 0 0 0-2 2v3a2 2 0 0 0 2 2h1a1 1 0 0 0 1-1z" />
        </svg>
      </div>

      <div class="route-gate-heading">
        <span class="eyebrow">音频输出</span>
        <h2 id="route-gate-title">先切换到 Razer 耳机</h2>
      </div>
      <p id="route-gate-desc">
        THX 空间音效和声音标准化只对耳机输出生效。当前 Windows 正在使用其他设备，
        切换后才能继续。
      </p>

      <div class="route-gate-devices">
        <div class="route-gate-device">
          <span class="route-gate-device-label">当前输出</span>
          <span class="route-gate-device-name">{current}</span>
        </div>
        <div class="route-gate-device">
          <span class="route-gate-device-label">目标输出</span>
          <span class="route-gate-device-name">
            {#if target}
              {target.name}
            {:else}
              未找到 Razer 耳机输出，请检查连接
            {/if}
          </span>
        </div>
      </div>

      {#if error}<p class="route-gate-error">{error}</p>{/if}

      <div class="route-gate-actions">
        <button class="btn primary" type="button" onclick={apply} disabled={busy || !target}>
          {busy ? "正在切换…" : "设为默认输出"}
        </button>
        <div class="route-gate-secondary">
          <button class="btn" type="button" onclick={openSettings}>打开声音设置</button>
          <button class="btn ghost" type="button" onclick={quit}>退出</button>
        </div>
      </div>

      <p class="route-gate-note">用户态 Core Audio 切换，不会安装驱动或修改耳机固件。</p>
    </div>
  </div>
{/if}
