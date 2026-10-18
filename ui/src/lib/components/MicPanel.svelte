<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";
  import EqEditor from "./EqEditor.svelte";
  import { frequencies } from "$lib/presets";

  type MicPreset = { id: string; label: string; gains: number[] };
  const MIC_PRESETS: MicPreset[] = [
    { id: "flat", label: "默认", gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
    { id: "boost", label: "麦克风增强", gains: [2, 4, 6, 7, 7, 7, 7, 6, 4, 2] },
    { id: "stream", label: "直播", gains: [4, 4, 4, 3, -2, -7, -4, -2, -3, -5] },
    { id: "meeting", label: "会议", gains: [-8, -6, -5, -3, -1, 1, 3, 2, 1, 0] }
  ];

  let gate = $state(false);
  let gateDb = $state(-60);
  let noiseReduction = $state(true);
  let noiseReductionLevel = $state(60);
  let normalization = $state(false);
  let normalizationLevel = $state(55);
  let vocalClarity = $state(false);
  let vocalClarityLevel = $state(55);
  let volume = $state(100);
  let preview = $state(false);
  let eqEnabled = $state(false);
  let eqGains = $state<number[]>([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
  let eqPreset = $state("flat");
  let sidetone = $state(false);
  let sidetonePct = $state(50);
  let micSynced = $state(false);
  let eqTimer: ReturnType<typeof setTimeout> | null = null;
  let eqSaveEpoch = 0;

  const available = $derived(!!device.thx?.installed && !!device.thx?.capture);

  $effect(() => {
    const capture = device.thx?.capture;
    if (micSynced || !capture) return;
    gate = capture.voiceGate ?? false;
    gateDb = capture.voiceGateDb ?? -60;
    noiseReduction = capture.noiseReduction ?? true;
    noiseReductionLevel = capture.noiseReductionLevel ?? 60;
    normalization = capture.normalization ?? false;
    normalizationLevel = capture.normalizationLevel ?? 55;
    vocalClarity = capture.vocalClarity ?? false;
    vocalClarityLevel = capture.vocalClarityLevel ?? 55;
    volume = capture.volume ?? 100;
    preview = capture.preview ?? false;
    eqEnabled = capture.eqEnabled ?? false;
    if (capture.eqGains?.length === 10) eqGains = [...capture.eqGains];
    sidetone = capture.sidetone ?? false;
    micSynced = true;
  });

  function gateStep(): number {
    return Math.max(1, Math.min(5, Math.round((gateDb + 70) / 10)));
  }

  function setGateStep(value: number) {
    gateDb = -70 + 10 * Math.max(1, Math.min(5, Math.round(value)));
  }

  async function toggleGate() {
    const next = !gate;
    const ok = await device.micCommand(
      next ? ["mic-gate", "on", String(gateDb)] : ["mic-gate", "off"],
      next ? `噪声门 ${gateDb} dB` : "噪声门已关闭"
    );
    if (ok) gate = next;
  }

  async function changeGate() {
    if (!gate) return;
    if (await device.micCommand(["mic-gate", "on", String(gateDb)], `噪声门 ${gateDb} dB`)) {
      gate = true;
    }
  }

  async function toggleNoiseReduction() {
    const next = !noiseReduction;
    const ok = await device.micCommand(
      next ? ["mic-nr", "on", String(noiseReductionLevel)] : ["mic-nr", "off"],
      next ? "麦克风降噪已开启" : "麦克风降噪已关闭"
    );
    if (ok) noiseReduction = next;
  }

  async function changeNoiseReduction() {
    if (!noiseReduction) return;
    if (
      await device.micCommand(
        ["mic-nr", "on", String(noiseReductionLevel)],
        `麦克风降噪 ${noiseReductionLevel}`
      )
    ) {
      noiseReduction = true;
    }
  }

  async function toggleNormalization() {
    const next = !normalization;
    const ok = await device.micCommand(
      next ? ["mic-norm", "on", String(normalizationLevel)] : ["mic-norm", "off"],
      next ? "麦克风音量标准化已开启" : "麦克风音量标准化已关闭"
    );
    if (ok) normalization = next;
  }

  async function changeNormalization() {
    if (!normalization) return;
    if (
      await device.micCommand(
        ["mic-norm", "on", String(normalizationLevel)],
        `麦克风标准化 ${normalizationLevel}`
      )
    ) {
      normalization = true;
    }
  }

  async function toggleVocalClarity() {
    const next = !vocalClarity;
    const ok = await device.micCommand(
      next ? ["mic-clarity", "on", String(vocalClarityLevel)] : ["mic-clarity", "off"],
      next ? "亮丽人声已开启" : "亮丽人声已关闭"
    );
    if (ok) vocalClarity = next;
  }

  async function changeVocalClarity() {
    if (!vocalClarity) return;
    if (
      await device.micCommand(
        ["mic-clarity", "on", String(vocalClarityLevel)],
        `亮丽人声 ${vocalClarityLevel}`
      )
    ) {
      vocalClarity = true;
    }
  }

  async function changeVolume() {
    await device.micCommand(["mic-volume", String(volume)], `麦克风音量 ${volume}`);
  }

  async function togglePreview() {
    if (preview) {
      if (await device.micCommand(["mic-preview", "off"], "麦克风测试已停止")) {
        preview = false;
      }
      return;
    }
    // Preview owns a DLL worker thread. Keep it in a detached helper process
    // instead of a request/response helper, which would crash during unload.
    if (await device.startMicPreview("麦克风测试中")) {
      preview = true;
    }
  }

  async function toggleEq() {
    const next = !eqEnabled;
    if (!next) {
      // A pending debounce must not silently turn EQ back on.
      if (eqTimer) clearTimeout(eqTimer);
      eqSaveEpoch += 1;
    }
    if (
      await device.micCommand(
        ["mic-eq-toggle", next ? "on" : "off"],
        next ? "麦克风 EQ 已开启" : "麦克风 EQ 已关闭"
      )
    ) {
      eqEnabled = next;
    }
  }

  async function applyPreset(preset: MicPreset) {
    if (eqTimer) clearTimeout(eqTimer);
    eqSaveEpoch += 1;
    const ok = await device.micCommand(
      ["mic-eq", ...preset.gains.map(String)],
      `${preset.label}预设已写入`
    );
    if (ok) {
      eqPreset = preset.id;
      eqGains = [...preset.gains];
      eqEnabled = true;
    }
  }

  function scheduleEqSave() {
    eqSaveEpoch += 1;
    const epoch = eqSaveEpoch;
    if (eqTimer) clearTimeout(eqTimer);
    eqTimer = setTimeout(async () => {
      if (epoch !== eqSaveEpoch) return;
      const ok = await device.micCommand(
        ["mic-eq", ...eqGains.map(String)],
        "麦克风 EQ 已写入"
      );
      if (ok) eqEnabled = true;
    }, 350);
  }

  function updateEq(index: number, value: number) {
    const next = [...eqGains];
    next[index] = value;
    eqGains = next;
    eqPreset = "custom";
    scheduleEqSave();
  }

  async function resetEq() {
    if (eqTimer) clearTimeout(eqTimer);
    eqSaveEpoch += 1;
    eqGains = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    eqPreset = "flat";
    if (await device.micCommand(["mic-eq", ...eqGains.map(String)], "麦克风 EQ 已重置")) {
      eqEnabled = true;
    }
  }

  async function toggleSidetone() {
    const next = !sidetone;
    const hid = await device.setSidetone(next);
    if (!hid) return;
    const thx = await device.micCommand(
      ["mic-sidetone", next ? "on" : "off"],
      next ? "侧音已开启" : "侧音已关闭"
    );
    if (thx) {
      sidetone = next;
    } else {
      // Do not leave the hardware sidetone in a state the UI cannot show.
      await device.setSidetone(!next);
    }
  }

  async function changeSidetoneVolume() {
    const raw = Math.max(0, Math.min(15, Math.round((sidetonePct * 15) / 100)));
    await device.setSidetoneVolume(raw);
  }

  const gateMin = $derived(-70 + 10);
  const gateMax = $derived(-70 + 50);
</script>

<div class="page-head">
  <div>
    <h1>麦克风</h1>
    <p>噪声门、降噪、标准化、亮丽人声和 10 段 EQ 由 THX APO 处理；侧音走耳机 HID。</p>
  </div>
  <span class="badge" class:ok={available} class:warn={!available}>
    {#if device.thx?.installed}
      {available ? "THX 麦克风就绪" : "THX 麦克风不可用"}
    {:else}
      THX 未安装
    {/if}
  </span>
</div>

{#if !available}
  <div class="inline-status error">未读取到 THX 麦克风状态；请确认 THX 服务/耳机输出正常。</div>
{/if}

<div class="grid-mic">
  <section class="panel" aria-labelledby="mic-input-title">
    <div class="section-head">
      <div>
        <div class="eyebrow">输入</div>
        <h2 id="mic-input-title">麦克风</h2>
      </div>
      {#if device.thx?.capture?.device}
        <span class="badge">{device.thx.capture.device}</span>
      {/if}
    </div>

    <div class="mic-row">
      <div class="label"><span>麦克风音量</span><small>系统采集音量 0–100</small></div>
      <span class="mono-value">{volume}</span>
    </div>
    <input
      class="norm-level-range"
      type="range"
      min="0"
      max="100"
      step="1"
      bind:value={volume}
      disabled={!available || device.busy}
      aria-label="麦克风音量"
      onchange={changeVolume}
    />

    <div class="row">
      <div class="label"><span>侧音</span><small>HID 硬件 + THX 采集链</small></div>
      <button
        class="switch"
        class:on={sidetone}
        onclick={toggleSidetone}
        disabled={!available || device.busy}
        aria-label="切换侧音"
        aria-pressed={sidetone}
      ></button>
    </div>
    <div class="mic-row">
      <div class="label"><span>侧音音量</span><small>0–100，映射到硬件 0–15</small></div>
      <span class="mono-value">{sidetonePct}%</span>
    </div>
    <input
      class="norm-level-range"
      type="range"
      min="0"
      max="100"
      step="1"
      bind:value={sidetonePct}
      disabled={!sidetone || !available || device.busy}
      aria-label="侧音音量"
      onchange={changeSidetoneVolume}
    />

    <div class="row mic-test-row">
      <div class="label"><span>麦克风测试</span><small>THX capture preview</small></div>
      <button
        class="btn"
        class:active={preview}
        onclick={togglePreview}
        disabled={!available || device.busy}
        aria-pressed={preview}
      >
        {preview ? "停止测试" : "开始测试"}
      </button>
    </div>
    <p class="inline-status">{device.notice}</p>
  </section>

  <section class="panel" aria-labelledby="mic-enhance-title">
    <div class="section-head">
      <div>
        <div class="eyebrow">THX 增强</div>
        <h2 id="mic-enhance-title">语音处理</h2>
      </div>
    </div>

    <div class="mic-feature">
      <div class="row">
        <div class="label"><span>噪声门</span><small>1–5 档，threshold {gateMin}–{gateMax} dB</small></div>
        <div class="mic-control">
          <span class="mono-value">{gateStep()}/5</span>
          <button
            class="switch"
            class:on={gate}
            onclick={toggleGate}
            disabled={!available || device.busy}
            aria-label="切换噪声门"
            aria-pressed={gate}
          ></button>
        </div>
      </div>
      <input
        class="norm-level-range"
        type="range"
        min="1"
        max="5"
        step="1"
        value={gateStep()}
        disabled={!gate || !available || device.busy}
        aria-label="噪声门档位"
        oninput={(event) => setGateStep(Number(event.currentTarget.value))}
        onchange={changeGate}
      />
    </div>

    <div class="mic-feature">
      <div class="row">
        <div class="label"><span>麦克风降噪</span><small>0–100 强度</small></div>
        <div class="mic-control">
          <span class="mono-value">{noiseReductionLevel}</span>
          <button
            class="switch"
            class:on={noiseReduction}
            onclick={toggleNoiseReduction}
            disabled={!available || device.busy}
            aria-label="切换麦克风降噪"
            aria-pressed={noiseReduction}
          ></button>
        </div>
      </div>
      <input
        class="norm-level-range"
        type="range"
        min="0"
        max="100"
        step="1"
        bind:value={noiseReductionLevel}
        disabled={!noiseReduction || !available || device.busy}
        aria-label="麦克风降噪强度"
        onchange={changeNoiseReduction}
      />
    </div>

    <div class="mic-feature">
      <div class="row">
        <div class="label"><span>音量标准化</span><small>把麦克风响度拉平</small></div>
        <div class="mic-control">
          <span class="mono-value">{normalizationLevel}</span>
          <button
            class="switch"
            class:on={normalization}
            onclick={toggleNormalization}
            disabled={!available || device.busy}
            aria-label="切换麦克风音量标准化"
            aria-pressed={normalization}
          ></button>
        </div>
      </div>
      <input
        class="norm-level-range"
        type="range"
        min="1"
        max="100"
        step="1"
        bind:value={normalizationLevel}
        disabled={!normalization || !available || device.busy}
        aria-label="麦克风音量标准化等级"
        onchange={changeNormalization}
      />
    </div>

    <div class="mic-feature">
      <div class="row">
        <div class="label"><span>亮丽人声</span><small>Vocal clarity 0–100</small></div>
        <div class="mic-control">
          <span class="mono-value">{vocalClarityLevel}</span>
          <button
            class="switch"
            class:on={vocalClarity}
            onclick={toggleVocalClarity}
            disabled={!available || device.busy}
            aria-label="切换亮丽人声"
            aria-pressed={vocalClarity}
          ></button>
        </div>
      </div>
      <input
        class="norm-level-range"
        type="range"
        min="0"
        max="100"
        step="1"
        bind:value={vocalClarityLevel}
        disabled={!vocalClarity || !available || device.busy}
        aria-label="亮丽人声等级"
        onchange={changeVocalClarity}
      />
    </div>
  </section>

  <section class="panel mic-eq-panel" aria-labelledby="mic-eq-title">
    <div class="section-head">
      <div>
        <div class="eyebrow">THX capture EQ</div>
        <h2 id="mic-eq-title">麦克风均衡器</h2>
        <p>10 段 ±12 dB；改动会自动写入。</p>
      </div>
      <div class="mic-eq-head-actions">
        <span class="mono-value">{eqEnabled ? "EQ 开" : "EQ 关"}</span>
        <button
          class="switch"
          class:on={eqEnabled}
          onclick={toggleEq}
          disabled={!available || device.busy}
          aria-label="切换麦克风 EQ"
          aria-pressed={eqEnabled}
        ></button>
      </div>
    </div>

    <div class="preset-list" role="group" aria-label="麦克风 EQ 预设">
      {#each MIC_PRESETS as preset}
        <button
          class:active={eqPreset === preset.id}
          onclick={() => applyPreset(preset)}
          disabled={!available || device.busy}
        >{preset.label}</button>
      {/each}
      <button
        class:active={eqPreset === "custom"}
        disabled={!available || device.busy}
        onclick={() => (eqPreset = "custom")}
      >自定义</button>
    </div>

    <EqEditor
      gains={eqGains}
      {frequencies}
      busy={device.busy}
      oninput={updateEq}
      onreset={resetEq}
    />
    <p class="eq-default-note">拖动/输入会自动写入；重置回平直曲线 0 dB。</p>
  </section>
</div>

<p class="inline-status">{device.error || device.thxError}</p>
