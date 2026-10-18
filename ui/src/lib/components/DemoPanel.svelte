<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { onDestroy } from "svelte";
  import { device } from "$lib/device.svelte";

  let stageEl: HTMLDivElement;
  let audio: AudioContext | null = null;
  let source: AudioBufferSourceNode | null = null;
  let master: GainNode | null = null;
  let stereoBranch: GainNode | null = null;
  let spatialBranch: GainNode | null = null;
  let stereoPanner: StereoPannerNode | null = null;
  let panner: PannerNode | null = null;
  let raf = 0;
  let startedAt = 0;
  let playing = $state(false);
  let elapsed = $state(0);
  let angle = $state(0);
  let volume = $state(45);
  let audioError = $state("");

  let available = $derived(!!device.thx?.installed);
  let spatial = $derived(device.thx?.spatialEnabled ?? false);
  let timeText = $derived(formatTime(elapsed % 30));
  let progress = $derived(((elapsed % 30) / 30) * 100);
  let canToggleSpatial = $derived(available && !device.busy && device.thx?.spatialEnabled !== null);

  function formatTime(value: number): string {
    const total = Math.max(0, Math.floor(value));
    return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
  }

  function makePingBuffer(context: AudioContext): AudioBuffer {
    const seconds = 4;
    const rate = context.sampleRate;
    const buffer = context.createBuffer(1, rate * seconds, rate);
    const data = buffer.getChannelData(0);
    const pings = [
      { at: 0.05, hz: 660 },
      { at: 1.05, hz: 880 },
      { at: 2.05, hz: 740 },
      { at: 3.05, hz: 990 }
    ];
    for (const ping of pings) {
      const start = Math.floor(ping.at * rate);
      const length = Math.floor(0.42 * rate);
      for (let i = 0; i < length && start + i < data.length; i += 1) {
        const t = i / rate;
        const envelope = Math.exp(-t * 10);
        data[start + i] += Math.sin(2 * Math.PI * ping.hz * t) * envelope * 0.16;
      }
    }
    return buffer;
  }

  function ensureAudio(): AudioContext {
    if (audio) return audio;
    const context = new AudioContext();
    const stereoPannerNode = context.createStereoPanner();
    stereoPannerNode.pan.value = 0;
    const stereoLevel = context.createGain();
    stereoLevel.gain.value = spatial ? 0 : 1;
    stereoPannerNode.connect(stereoLevel);

    const spatialPannerNode = context.createPanner();
    spatialPannerNode.panningModel = "HRTF";
    spatialPannerNode.distanceModel = "inverse";
    spatialPannerNode.refDistance = 1;
    spatialPannerNode.maxDistance = 10;
    spatialPannerNode.rolloffFactor = 1.1;
    const spatialLevel = context.createGain();
    spatialLevel.gain.value = spatial ? 1 : 0;
    spatialPannerNode.connect(spatialLevel);

    const output = context.createGain();
    output.gain.value = (volume / 100) * 0.45;
    stereoLevel.connect(output);
    spatialLevel.connect(output);
    output.connect(context.destination);

    audio = context;
    master = output;
    stereoBranch = stereoLevel;
    spatialBranch = spatialLevel;
    stereoPanner = stereoPannerNode;
    panner = spatialPannerNode;
    return context;
  }

  function applyMode() {
    if (!audio || !stereoBranch || !spatialBranch) return;
    const now = audio.currentTime;
    stereoBranch.gain.setTargetAtTime(spatial ? 0 : 1, now, 0.02);
    spatialBranch.gain.setTargetAtTime(spatial ? 1 : 0, now, 0.02);
  }

  function applyVolume() {
    if (!audio || !master) return;
    master.gain.setTargetAtTime((volume / 100) * 0.45, audio.currentTime, 0.02);
  }

  function tick() {
    if (!playing || !audio) return;
    elapsed = Math.max(0, audio.currentTime - startedAt);
    angle = (elapsed * 62) % 360;
    if (panner) {
      const radians = (angle * Math.PI) / 180;
      panner.setPosition(Math.cos(radians) * 2.2, 0, Math.sin(radians) * 2.2);
    }
    raf = requestAnimationFrame(tick);
  }

  async function play() {
    try {
      const context = ensureAudio();
      await context.resume();
      if (!source) {
        const next = context.createBufferSource();
        next.buffer = makePingBuffer(context);
        next.loop = true;
        const input = context.createGain();
        // The branch gains already carry the mode; this node is the shared mono
        // source for both the stereo and HRTF paths.
        next.connect(input);
        input.connect(stereoPanner as StereoPannerNode);
        input.connect(panner as PannerNode);
        next.start();
        source = next;
        startedAt = context.currentTime;
      }
      playing = true;
      applyMode();
      cancelAnimationFrame(raf);
      tick();
    } catch (error) {
      audioError = error instanceof Error ? error.message : String(error);
    }
  }

  async function pause() {
    if (!audio) return;
    await audio.suspend();
    playing = false;
    cancelAnimationFrame(raf);
  }

  function togglePlay() {
    if (playing) void pause();
    else void play();
  }

  async function toggleSpatial(enabled: boolean) {
    if (!canToggleSpatial) return;
    await device.setSpatial(enabled);
  }

  async function toggleFullscreen() {
    if (!stageEl) return;
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await stageEl.requestFullscreen();
    } catch {
      /* Fullscreen can be blocked by the host; the demo still works inline. */
    }
  }

  function restart() {
    if (source) {
      try {
        source.stop();
      } catch {
        /* already stopped */
      }
      source = null;
    }
    startedAt = audio?.currentTime ?? 0;
    elapsed = 0;
    angle = 0;
    if (playing) void play();
  }

  $effect(() => {
    // Keep the branch gains and volume in sync without rebuilding the graph.
    applyMode();
    applyVolume();
  });

  onDestroy(() => {
    cancelAnimationFrame(raf);
    if (source) {
      try {
        source.stop();
      } catch {
        /* already stopped */
      }
    }
    void audio?.close();
  });
</script>

<div class="demo-page">
  <div class="page-head">
    <div>
      <h1>演示</h1>
      <p>用一段实时合成的空间音频，听出立体声与 THX 空间音效的区别。</p>
    </div>
    <span class="badge" class:ok={available} class:warn={!available}>
      {available ? "THX 就绪" : "THX 未连接"}
    </span>
  </div>

  <section class="demo-controls" aria-labelledby="demo-mode-title">
    <div>
      <div class="eyebrow">体验差异</div>
      <h2 id="demo-mode-title">立体声 / THX 空间音效</h2>
      <p>切换会同时改变耳机输出模式和演示音频的空间化方式。</p>
    </div>
    <div class="preset-seg demo-seg" role="group" aria-label="演示输出模式">
      <button
        type="button"
        class:on={!spatial}
        disabled={!canToggleSpatial}
        aria-pressed={!spatial}
        onclick={() => toggleSpatial(false)}
      >立体声</button>
      <button
        type="button"
        class:on={spatial}
        disabled={!canToggleSpatial}
        aria-pressed={spatial}
        onclick={() => toggleSpatial(true)}
      >THX SPATIAL AUDIO 空间音效</button>
    </div>
  </section>

  <div class="demo-stage" class:playing bind:this={stageEl}>
    <div class="demo-stage-top">
      <span>THX SPATIAL AUDIO</span>
      <span class="demo-live"><i></i> LIVE SYNTH</span>
    </div>

    <svg class="demo-radar" viewBox="0 0 600 400" role="img" aria-label="空间音频演示雷达">
      <defs>
        <radialGradient id="demo-glow" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stop-color="#5dff7a" stop-opacity=".18" />
          <stop offset="100%" stop-color="#5dff7a" stop-opacity="0" />
        </radialGradient>
        <filter id="demo-blur">
          <feGaussianBlur stdDeviation="3" />
        </filter>
      </defs>
      <rect width="600" height="400" fill="#080b09" />
      <circle cx="300" cy="200" r="188" fill="url(#demo-glow)" />
      <g fill="none" stroke="#2c7a43" stroke-width="1" stroke-dasharray="2 8" opacity=".7">
        <circle cx="300" cy="200" r="55" />
        <circle cx="300" cy="200" r="100" />
        <circle cx="300" cy="200" r="145" />
        <circle cx="300" cy="200" r="188" />
        <path d="M300 10v380M10 200h580" opacity=".45" />
        <path d="M125 25 475 375M475 25 125 375" opacity=".2" />
      </g>
      <g class="demo-orbit" transform={`rotate(${angle} 300 200)`}>
        <circle cx="430" cy="200" r="7" fill="#5dff7a" filter="url(#demo-blur)" opacity=".7" />
        <circle cx="430" cy="200" r="4" fill="#b9ffc4" />
        <circle cx="300" cy="100" r="4" fill="#5dff7a" opacity=".8" />
        <circle cx="175" cy="255" r="3" fill="#5dff7a" opacity=".6" />
      </g>
      <g fill="none" stroke="#b7f7c2" stroke-width="3" stroke-linecap="round" opacity=".9">
        <path d="M277 196v-9a23 23 0 0 1 46 0v9" />
        <path d="M270 196h12v28h-7a5 5 0 0 1-5-5zM318 196h12v23a5 5 0 0 1-5 5h-7z" />
        <path d="M291 224h18" opacity=".4" />
      </g>
      <g fill="#5dff7a" opacity=".85">
        <circle cx="295" cy="250" r="3" />
        <circle cx="305" cy="250" r="3" />
      </g>
      <text x="300" y="76" text-anchor="middle" fill="#e8f5ea" font-size="25" font-family="Segoe UI, sans-serif" letter-spacing="2">EXPERIENCE SUPERIOR</text>
      <text x="300" y="103" text-anchor="middle" fill="#8dbb96" font-size="16" font-family="Segoe UI, sans-serif" letter-spacing="6">POSITIONAL</text>
      <text x="300" y="374" text-anchor="middle" fill="#5d7f63" font-size="10" font-family="Segoe UI, sans-serif" letter-spacing="2">SYNTHESIZED DEMO · NO RAZER MEDIA</text>
    </svg>

    <div class="demo-toolbar">
      <button class="demo-play" type="button" aria-label={playing ? "暂停演示" : "播放演示"} onclick={togglePlay}>
        {#if playing}
          <span class="demo-pause-icon"></span>
        {:else}
          <span class="demo-play-icon"></span>
        {/if}
      </button>
      <span class="demo-time">{timeText}</span>
      <div class="demo-progress" aria-hidden="true"><i style={`width:${progress}%`}></i></div>
      <span class="demo-time">0:30</span>
      <button class="demo-icon-btn" type="button" aria-label="重新开始演示" onclick={restart}>↺</button>
      <label class="demo-volume">
        <span aria-hidden="true">VOL</span>
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          value={volume}
          aria-label="演示音量"
          oninput={(event) => (volume = Number(event.currentTarget.value))}
        />
      </label>
      <button class="demo-icon-btn" type="button" aria-label="全屏演示" onclick={toggleFullscreen}>⛶</button>
    </div>
  </div>

  {#if audioError}
    <p class="inline-status error">音频启动失败：{audioError}</p>
  {/if}
  <p class="demo-footnote">
    演示音频由 Web Audio 实时合成，雷达动画由 SVG 绘制；不包含雷蛇/THX 的视频或音频文件。
    播放时会经过当前默认播放设备，因此 THX 空间音效开关会真实影响输出。
  </p>
</div>
