<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";
  import { chargingText, fieldText, known } from "$lib/types";
  import { frequencies } from "$lib/presets";
  import EqEditor from "$lib/components/EqEditor.svelte";
  import NormalizationPanel from "$lib/components/NormalizationPanel.svelte";

  let eqPreset = $state<string | null>(null);
  let gains = $state<number[] | null>(null);
  let level = $state(10);
  let normalizationOn = $state(false);
  let synced = $state(false);

  let available = $derived(!!device.thx?.installed);
  let spatial = $derived(device.thx?.spatialEnabled ?? null);
  let spatialBusy = $derived(device.busy || !available || spatial === null);
  let battery = $derived(known(device.status?.battery, 0));

  $effect(() => {
    if (!synced && device.thx) {
      level = device.thx.level ?? level;
      normalizationOn = device.thx.enabled ?? false;
      synced = true;
    }
  });

  async function selectPreset(id: string) {
    const preset = device.presets.find((item) => item.id === id);
    if (!preset) return;
    // Do not show a curve until the device accepted the preset. Otherwise a
    // failed write would leave a convincing but false UI state.
    const ok = await device.setEqPreset(id, preset.label);
    if (!ok) return;
    eqPreset = id;
    gains = [...preset.gains];
  }

  function startFlat() {
    eqPreset = "custom";
    gains = Array.from({ length: 10 }, () => 0);
  }

  function updateGain(index: number, value: number) {
    if (!Number.isFinite(value)) return;
    eqPreset = "custom";
    gains = (gains ?? Array.from({ length: 10 }, () => 0)).map((gain, slot) =>
      slot === index ? value : gain
    );
  }

  function resetGains() {
    eqPreset = "custom";
    gains = Array.from({ length: 10 }, () => 0);
  }

  async function applyGains() {
    if (gains) await device.setEqGains(gains);
  }

  async function saveNormalization() {
    await device.saveNormalization(level);
  }

  async function chooseSpatial(enabled: boolean) {
    if (spatialBusy) return;
    await device.setSpatial(enabled);
  }

  async function toggleNormalization() {
    normalizationOn = !normalizationOn;
    await device.setNormalizationEnabled(normalizationOn);
  }
</script>

<div class="page-head">
  <div>
    <h1>声音</h1>
    <p>拖动曲线上的圆点调整，或直接输入数值。当前频段会同步显示。</p>
  </div>
  <span class="badge" class:ok={available} class:warn={!available}>
    {available ? `THX ${device.thx?.level ?? "—"}` : "THX 未安装"}
  </span>
</div>

<section class="panel output-panel" aria-labelledby="output-title">
  <div class="output-copy">
    <div class="eyebrow">输出模式</div>
    <h2 id="output-title">立体声 / THX 空间音效</h2>
    <p>空间音效由 THX APO 处理，和声音标准化相互独立。</p>
  </div>
  <div class="preset-seg output-seg" role="group" aria-label="输出模式">
    <button
      type="button"
      class:on={spatial === false}
      disabled={spatialBusy}
      aria-pressed={spatial === false}
      onclick={() => chooseSpatial(false)}
    >立体声</button>
    <button
      type="button"
      class:on={spatial === true}
      disabled={spatialBusy}
      aria-pressed={spatial === true}
      onclick={() => chooseSpatial(true)}
    >THX 空间音效</button>
  </div>
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

  {#if gains !== null && gains.length === 10}
    <EqEditor
      {gains}
      {frequencies}
      busy={device.busy}
      oninput={updateGain}
      onapply={applyGains}
      onreset={resetGains}
    />
  {:else}
    <div class="eq-unknown">
      <strong>当前均衡器曲线未知</strong>
      <p>耳机没有返回当前 10 段增益。选择一个预设，或从平直曲线开始；这样不会在写入时覆盖未知的其余频段。</p>
      <button class="btn" type="button" onclick={startFlat} disabled={device.busy}>从平直曲线开始</button>
    </div>
  {/if}

  <p class="inline-status">{device.notice}</p>
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
        {chargingText(device.status?.charging)}
      </span>
    </div>
    <dl class="kv">
      <dt>链路</dt><dd>{device.status?.link === "wired" ? "有线 USB" : device.status?.link === "wireless" ? "无线 2.4G" : "—"}</dd>
      <dt>电量</dt><dd>{fieldText(device.status?.battery, "%")}</dd>
      <dt>固件</dt><dd>{fieldText(device.status?.firmware)}</dd>
    </dl>
    <div class="battery-line">
      <span class="battery-bar"><span style={`width:${Math.min(100, Math.max(0, battery))}%`}></span></span>
      <span class="battery-value">{battery}%</span>
    </div>
  </section>
</div>
