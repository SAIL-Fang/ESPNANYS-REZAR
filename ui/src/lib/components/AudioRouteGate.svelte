<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";

  let selected = $state("");
  let busy = $state(false);
  let error = $state("");

  let endpoints = $derived(device.status?.audioOutput?.endpoints ?? []);
  let current = $derived(device.status?.audioOutput?.name ?? "未知设备");
  let blocked = $derived(
    !!device.status?.audioOutput && device.status.audioOutput.isHeadset === false
  );

  $effect(() => {
    if (!endpoints.length) return;
    if (endpoints.some((endpoint) => endpoint.id === selected)) return;
    selected =
      endpoints.find((endpoint) => endpoint.isHeadset)?.id ??
      endpoints.find((endpoint) => endpoint.isDefault)?.id ??
      endpoints[0].id;
  });

  async function apply() {
    if (!selected || busy) return;
    busy = true;
    error = "";
    const ok = await device.setDefaultAudioOutput(selected);
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
  <div class="route-gate" role="dialog" aria-modal="true" aria-labelledby="route-gate-title">
    <div class="route-gate-card">
      <div class="eyebrow">默认输出设备</div>
      <h2 id="route-gate-title">默认输出不是 Razer 耳机</h2>
      <p>
        将 Razer BlackShark V2 HyperSpeed 设为默认音频设备，才能继续使用 THX
        空间音效和声音标准化。HID 设置在切换后立即可用。
      </p>
      <p class="route-gate-current">当前输出：{current}</p>

      {#if endpoints.length}
        <label class="route-gate-select">
          <span>选择输出设备</span>
          <select bind:value={selected}>
            {#each endpoints as endpoint}
              <option value={endpoint.id}>
                {endpoint.name}{endpoint.isHeadset ? " · Razer" : ""}
              </option>
            {/each}
          </select>
        </label>
      {:else}
        <p class="route-gate-current">没有找到可切换的输出设备，请检查耳机连接。</p>
      {/if}

      {#if error}<p class="route-gate-error">{error}</p>{/if}

      <div class="route-gate-actions">
        <button class="btn primary" type="button" onclick={apply} disabled={busy || !selected}>
          {busy ? "切换中…" : "设为默认"}
        </button>
        <button class="btn" type="button" onclick={openSettings}>打开声音设置</button>
        <button class="btn ghost" type="button" onclick={quit}>退出</button>
      </div>
      <p class="route-gate-note">
        这是 Windows Core Audio 的用户态切换，不会安装内核驱动，也不会修改耳机固件。
      </p>
    </div>
  </div>
{/if}
