<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { device } from "$lib/device.svelte";
  import SpatialModeToggle from "$lib/components/SpatialModeToggle.svelte";

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
  let startPending = false;
  let demoSpatial = $state(false);
  let initialized = false;
  let outputs = $state<MediaDeviceInfo[]>([]);
  let sinkId = $state("default");

  let available = $derived(!!device.thx?.installed);
  let timeText = $derived(formatTime(elapsed % 30));
  let progress = $derived(((elapsed % 30) / 30) * 100);

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
    stereoLevel.gain.value = demoSpatial ? 0 : 1;
    // A short right-channel delay gives the stereo audition a real L/R width
    // instead of a single mono image. The spatial path uses HRTF below.
    const stereoWidth = context.createDelay(0.05);
    stereoWidth.delayTime.value = 0.012;
    const splitter = context.createChannelSplitter(2);
    const merger = context.createChannelMerger(2);
    stereoPannerNode.connect(splitter);
    splitter.connect(merger, 0, 0);
    splitter.connect(stereoWidth, 0, 0);
    stereoWidth.connect(merger, 0, 1);
    merger.connect(stereoLevel);

    const spatialPannerNode = context.createPanner();
    spatialPannerNode.panningModel = "HRTF";
    spatialPannerNode.distanceModel = "inverse";
    spatialPannerNode.refDistance = 1;
    spatialPannerNode.maxDistance = 10;
    spatialPannerNode.rolloffFactor = 1.1;
    const spatialLevel = context.createGain();
    spatialLevel.gain.value = demoSpatial ? 1 : 0;
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
    stereoBranch.gain.setTargetAtTime(demoSpatial ? 0 : 1, now, 0.02);
    spatialBranch.gain.setTargetAtTime(demoSpatial ? 1 : 0, now, 0.02);
  }

  function applyVolume() {
    if (!audio || !master) return;
    master.gain.setTargetAtTime((volume / 100) * 0.45, audio.currentTime, 0.02);
  }

  function tick() {
    if (!playing || !audio) return;
    elapsed = Math.max(0, audio.currentTime - startedAt);
    // The stereo audition stays centered; only the spatial path orbits, so the
    // visual difference matches the audible A/B.
    angle = demoSpatial ? (elapsed * 62) % 360 : 0;
    if (panner) {
      const radians = (angle * Math.PI) / 180;
      panner.setPosition(Math.cos(radians) * 2.2, 0, Math.sin(radians) * 2.2);
    }
    raf = requestAnimationFrame(tick);
  }

  async function play() {
    if (startPending) return;
    startPending = true;
    audioError = "";
    try {
      const context = ensureAudio();
      await context.resume();
      try {
        await refreshOutputs();
        await applySink(sinkId);
      } catch {
        // Device selection is best-effort; keep the system default audible.
        sinkId = "default";
      }
      if (!source) {
        const next = context.createBufferSource();
        next.buffer = makePingBuffer(context);
        next.loop = true;
        const input = context.createGain();
        // This shared mono input feeds both the fixed stereo panner and the
        // animated HRTF panner; the branch gains select the audible path.
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
    } finally {
      startPending = false;
    }
  }

  async function pause() {
    if (!audio) return;
    await audio.suspend();
    playing = false;
    cancelAnimationFrame(raf);
  }

  function togglePlay() {
    if (startPending) return;
    if (playing) void pause();
    else void play();
  }

  async function applySink(id: string) {
    if (!audio) {
      sinkId = id;
      return;
    }
    const sinkable = audio as AudioContext & {
      setSinkId?: (value: string) => Promise<void>;
    };
    if (!sinkable.setSinkId) {
      if (id !== "default") throw new Error("当前 WebView 不支持选择输出设备");
      return;
    }
    await sinkable.setSinkId(id);
    sinkId = id;
  }

  async function refreshOutputs() {
    try {
      const devices = await navigator.mediaDevices.enumerateDevices();
      outputs = devices.filter((device) => device.kind === "audiooutput");
      // Best-effort: if the browser exposes labels, prefer the headset output.
      if (sinkId === "default") {
        const match = outputs.find((device) => /blackshark|razer/i.test(device.label));
        if (match) sinkId = match.deviceId;
      }
    } catch {
      outputs = [];
    }
  }

  async function chooseOutput(id: string) {
    try {
      await applySink(id);
      if (playing) await audio?.resume();
    } catch (error) {
      audioError = error instanceof Error ? error.message : String(error);
    }
  }

  function toggleSpatial(enabled: boolean) {
    // Audition-only switch: it must work without THX installed and must not
    // mutate the system output mode. The real switch lives on the Sound page.
    demoSpatial = enabled;
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

  function onVisibilityChange() {
    if (document.hidden && playing) void pause();
  }

  onMount(() => {
    document.addEventListener("visibilitychange", onVisibilityChange);
  });

  $effect(() => {
    // Seed the audition mode from the system once, then keep the demo local.
    if (!initialized && device.thx) {
      demoSpatial = device.thx.spatialEnabled ?? false;
      initialized = true;
    }
  });

  $effect(() => {
    // Keep the branch gains and volume in sync without rebuilding the graph.
    demoSpatial;
    volume;
    applyMode();
    applyVolume();
  });

  onDestroy(() => {
    cancelAnimationFrame(raf);
    document.removeEventListener("visibilitychange", onVisibilityChange);
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
      <h2 id="demo-mode-title">立体声 / 空间音效试听</h2>
      <p>仅改变演示试听；系统输出模式仍在「声音」页切换。</p>
    </div>
    <SpatialModeToggle
      value={demoSpatial}
      label="演示试听模式"
      spatialLabel="空间音效"
      onchange={toggleSpatial}
    />
  </section>

  <div class="demo-output-row">
    <label for="demo-output">演示输出设备</label>
    <select
      id="demo-output"
      value={sinkId}
      onchange={(event) => chooseOutput(event.currentTarget.value)}
    >
      <option value="default">系统默认</option>
      {#each outputs as output, index}
        <option value={output.deviceId}>{output.label || `输出设备 ${index + 1}`}</option>
      {/each}
    </select>
    <span>可手动指向 Razer 耳机；浏览器未暴露名称时只显示编号。</span>
  </div>

  <div class="demo-stage" class:playing bind:this={stageEl}>
    <div class="demo-stage-top">
      <span>THX 空间音效</span>
      <span class="demo-live"><i></i> 实时合成</span>
    </div>

    <svg class="demo-radar" viewBox="0 0 600 400" role="img" aria-label="空间音频演示雷达">
      <rect width="600" height="400" fill="#0d1118" />
      <g fill="none" stroke="rgba(157, 192, 255, 0.26)" stroke-width="1" stroke-dasharray="2 8">
        <circle cx="300" cy="200" r="55" />
        <circle cx="300" cy="200" r="100" />
        <circle cx="300" cy="200" r="145" />
        <circle cx="300" cy="200" r="188" />
        <path d="M300 10v380M10 200h580" opacity=".45" />
        <path d="M125 25 475 375M475 25 125 375" opacity=".2" />
      </g>
      <g class="demo-orbit" transform={`rotate(${angle} 300 200)`}>
        <circle cx="430" cy="200" r="5" fill="var(--sail-blue)" opacity=".65" />
        <circle cx="430" cy="200" r="3" fill="#dce8ff" />
        <circle cx="300" cy="100" r="3" fill="var(--sail-blue)" opacity=".8" />
        <circle cx="175" cy="255" r="2.5" fill="var(--sail-blue)" opacity=".55" />
      </g>
      <g fill="none" stroke="#b8cdf5" stroke-width="3" stroke-linecap="round" opacity=".9">
        <path d="M277 196v-9a23 23 0 0 1 46 0v9" />
        <path d="M270 196h12v28h-7a5 5 0 0 1-5-5zM318 196h12v23a5 5 0 0 1-5 5h-7z" />
        <path d="M291 224h18" opacity=".4" />
      </g>
      <g fill="#9dc0ff" opacity=".8">
        <circle cx="295" cy="250" r="3" />
        <circle cx="305" cy="250" r="3" />
      </g>
      <text x="300" y="76" text-anchor="middle" fill="#e7ecf1" font-size="22" font-family="Segoe UI, sans-serif">更清晰的空间定位</text>
      <text x="300" y="103" text-anchor="middle" fill="#8da4c4" font-size="14" font-family="Segoe UI, sans-serif">感受方向与距离</text>
      <text x="300" y="374" text-anchor="middle" fill="#6f8198" font-size="10" font-family="Segoe UI, sans-serif">实时合成演示，不含雷蛇素材</text>
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
        <span aria-hidden="true">音量</span>
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
    试听开关只改变演示音频，不写入系统状态；系统模式在「声音」页切换。音频走当前默认播放设备，
    如果默认设备是 THX endpoint，系统空间处理才会叠加在试听上。
  </p>
</div>
