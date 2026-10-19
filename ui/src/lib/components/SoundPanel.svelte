<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";
  import { chargingText, fieldText, known } from "$lib/types";
  import { DEFAULT_EQ_GAINS, DEFAULT_NORMALIZATION_LEVEL, formatEqBands } from "$lib/presets";
  import EqEditor from "$lib/components/EqEditor.svelte";
  import NormalizationPanel from "$lib/components/NormalizationPanel.svelte";
  import SpatialModeToggle from "$lib/components/SpatialModeToggle.svelte";
  import { onDestroy } from "svelte";

  const DEFAULT_GAINS = DEFAULT_EQ_GAINS;

  let eqPreset = $state("custom");
  let gains = $state<number[]>([...DEFAULT_GAINS]);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let savingEq = $state(false);
  let level = $state(DEFAULT_NORMALIZATION_LEVEL);
  let normalizationOn = $state(false);
  let synced = $state(false);

  let available = $derived(
    !!device.thx?.installed &&
      device.status?.audioOutput?.isHeadset !== false &&
      device.status?.model.capabilities.thx !== false
  );
  let thxInstalled = $derived(!!device.thx?.installed);
  let modelFrequencies = $derived(formatEqBands(device.status?.model.eqBands));
  let spatial = $derived(device.thx?.spatialEnabled ?? null);
  let spatialBusy = $derived(device.busy || !available || spatial === null);
  let battery = $derived(known(device.status?.battery, 0));
  let outputLabel = $derived(
    available ? `THX ${device.thx?.level ?? "—"}` : thxInstalled ? "输出不是耳机" : "THX 未安装"
  );

  $effect(() => {
    if (!synced && device.thx) {
      level = device.thx.level ?? level;
      normalizationOn = device.thx.enabled ?? false;
      synced = true;
    }
  });

  function cancelAutoSave() {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
  }

  function scheduleGainSave(next: number[]) {
    cancelAutoSave();
    saveTimer = setTimeout(() => {
      saveTimer = null;
      void persistGains(next);
    }, 350);
  }

  async function persistGains(next: number[]) {
    savingEq = true;
    await device.setEqGains([...next]);
    savingEq = false;
  }

  async function selectPreset(id: string) {
    const preset = device.presets.find((item) => item.id === id);
    if (!preset) return;
    // A pending drag must not overwrite the preset after it is applied.
    cancelAutoSave();
    // Do not show a curve until the device accepted the preset. Otherwise a
    // failed write would leave a convincing but false UI state.
    const ok = await device.setEqPreset(id, preset.label);
    if (!ok) return;
    eqPreset = id;
    gains = [...preset.gains];
    // Write the full 10-band curve as well as the preset index, so the
    // hardware actually matches the curve shown in the editor.
    await device.setEqGains([...preset.gains]);
  }

  function updateGain(index: number, value: number) {
    if (!Number.isFinite(value)) return;
    eqPreset = "custom";
    const next = gains.map((gain, slot) => (slot === index ? value : gain));
    gains = next;
    scheduleGainSave(next);
  }

  function resetGains() {
    cancelAutoSave();
    eqPreset = "custom";
    const next = [...DEFAULT_GAINS];
    gains = next;
    scheduleGainSave(next);
  }

  onDestroy(() => {
    if (saveTimer) {
      cancelAutoSave();
      void persistGains(gains);
    }
  });

  async function saveNormalization() {
    await device.saveNormalization(level);
  }

  async function chooseSpatial(enabled: boolean) {
    if (spatialBusy) return;
    await device.setSpatial(enabled);
  }

  async function toggleNormalization() {
    const next = !normalizationOn;
    const ok = await device.setNormalizationEnabled(next);
    if (ok) normalizationOn = next;
  }
</script>

<div class="page-head">
  <div>
    <h1>声音</h1>
    <p>拖动曲线上的圆点调整，或直接输入数值。当前频段会同步显示。</p>
  </div>
  <span class="badge" class:ok={available} class:warn={!available}>
    {outputLabel}
  </span>
</div>

<section class="panel output-panel" aria-labelledby="output-title">
  <div class="output-copy">
    <div class="eyebrow">输出模式</div>
    <h2 id="output-title">立体声 / THX 空间音效</h2>
    <p>空间音效由 THX APO 处理，和声音标准化相互独立。</p>
  </div>
      <SpatialModeToggle
        value={spatial ?? false}
        disabled={spatialBusy}
        onchange={chooseSpatial}
      />
</section>

<section class="panel eq-panel" aria-labelledby="eq-title">
  <div class="eq-head">
    <div>
      <h2 id="eq-title">均衡器</h2>
      <p>拖动曲线，或直接输入 <b>dB</b> 值。</p>
    </div>
    <div class="preset-seg" role="group" aria-label="均衡器预设">
      {#each device.presets as preset}
        <button
          type="button"
          class:on={eqPreset === preset.id}
          onclick={() => selectPreset(preset.id)}
          disabled={device.busy}
        >
          {preset.label}
        </button>
      {/each}
    </div>
  </div>

  {#if gains.length === 10}
    <EqEditor
      {gains}
      frequencies={modelFrequencies}
      busy={device.busy || savingEq}
      oninput={updateGain}
      onreset={resetGains}
    />
    <p class="eq-default-note">
      拖动/输入会自动写入；「重置」回到默认曲线 {DEFAULT_GAINS.join(", ")} dB，也会立即写入。
    </p>
  {:else}
    <p class="muted">均衡器需要 10 段数据。</p>
  {/if}

  <p class="inline-status">{savingEq ? "均衡器写入中…" : device.notice}</p>
</section>

<div class="bottom-grid">
  <NormalizationPanel
    {level}
    enabled={normalizationOn}
    {available}
    busy={device.busy}
    thxError={device.thxError}
    onlevel={(value: number) => (level = value)}
    ontoggle={toggleNormalization}
    onsave={saveNormalization}
  />

  <section class="panel status-summary" aria-labelledby="status-title">
    <div class="status-line">
      <div>
        <div class="eyebrow">连接</div>
        <h3 id="status-title">耳机状态</h3>
      </div>
      <span class="badge" class:ok={device.status?.connected}>
        {device.status?.link === "standby" ? "耳机未开机" : chargingText(device.status?.charging)}
      </span>
    </div>
    <dl class="kv">
      <dt>链路</dt><dd>{device.status?.link === "standby" ? "耳机未开机" : device.status?.link === "wired" ? "有线 USB" : device.status?.link === "wireless" ? "无线 2.4G" : "—"}</dd>
      <dt>电量</dt><dd>{fieldText(device.status?.battery, "%")}</dd>
      <dt>固件</dt><dd>{fieldText(device.status?.firmware)}</dd>
    </dl>
    <div class="battery-line">
      <span class="battery-bar"><span style={`width:${Math.min(100, Math.max(0, battery))}%`}></span></span>
      <span class="battery-value">{battery}%</span>
    </div>
  </section>
</div>
