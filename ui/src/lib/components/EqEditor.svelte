<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  interface Props {
    gains: number[];
    frequencies: string[];
    busy: boolean;
    oninput: (index: number, value: number) => void;
    onapply: () => void;
    onreset: () => void;
  }

  let { gains, frequencies, busy, oninput, onapply, onreset }: Props = $props();

  let active = $state(3);
  let graphEl: HTMLDivElement;
  let dragIndex = $state<number | null>(null);
  let editing = $state<number | null>(null);
  let drafts = $state<string[]>([]);

  $effect(() => {
    // Keep non-focused fields in sync with drag/preset changes. While editing,
    // preserve the raw string so "-" and a trailing decimal can be typed.
    if (editing === null) {
      drafts = gains.map(formatGain);
    }
  });

  function formatGain(gain: number): string {
    return gain > 0 ? `+${gain}` : `${gain}`;
  }

  function beginEdit(index: number) {
    active = index;
    editing = index;
    drafts[index] = formatGain(gains[index] ?? 0);
  }

  function updateDraft(index: number, raw: string) {
    // Keep the raw string local. Applying on each keystroke made Escape
    // unable to undo the edit and broke intermediate values like "-".
    drafts[index] = raw;
  }

  function commitDraft(index: number) {
    const raw = drafts[index] ?? "";
    editing = null;
    if (raw.trim() === "") {
      drafts[index] = formatGain(gains[index] ?? 0);
      return;
    }
    const value = Number(raw);
    if (Number.isFinite(value)) {
      oninput(index, clamp(value));
    } else {
      drafts[index] = formatGain(gains[index] ?? 0);
    }
  }

  function onKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "Enter") {
      event.preventDefault();
      commitDraft(index);
      if (!busy) onapply();
    } else if (event.key === "Escape") {
      event.preventDefault();
      editing = null;
      drafts[index] = formatGain(gains[index] ?? 0);
    }
  }

  let points = $derived(
    gains
      .map((gain, index) => `${(index / Math.max(1, gains.length - 1)) * 100},${50 - gain * 3.5}`)
      .join(" ")
  );

  function clamp(value: number): number {
    return Math.max(-12, Math.min(12, Math.round(value)));
  }

  function setGain(index: number, raw: string | number) {
    const value = clamp(Number(raw));
    if (Number.isFinite(value)) {
      active = index;
      oninput(index, value);
    }
  }

  function step(index: number, delta: number) {
    active = index;
    oninput(index, clamp((gains[index] ?? 0) + delta));
  }

  function gainAt(clientY: number): number {
    const rect = graphEl.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (clientY - rect.top) / rect.height));
    return clamp((50 - ratio * 100) / 3.5);
  }

  function startDrag(event: PointerEvent, index: number) {
    event.preventDefault();
    event.stopPropagation();
    active = index;
    dragIndex = index;
    const dot = event.currentTarget as HTMLButtonElement;
    dot.focus();
    dot.setPointerCapture(event.pointerId);
    oninput(index, gainAt(event.clientY));
  }

  function startGraphDrag(event: PointerEvent) {
    if ((event.target as HTMLElement).closest(".eq-dot")) return;
    const rect = graphEl.getBoundingClientRect();
    const ratioX = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    const index = Math.round(ratioX * (gains.length - 1));
    active = index;
    dragIndex = index;
    graphEl.setPointerCapture(event.pointerId);
    oninput(index, gainAt(event.clientY));
  }

  function moveGraphDrag(event: PointerEvent) {
    if (dragIndex === null) return;
    oninput(dragIndex, gainAt(event.clientY));
  }

  function endGraphDrag(event: PointerEvent) {
    if (dragIndex === null) return;
    dragIndex = null;
    if (graphEl.hasPointerCapture(event.pointerId)) {
      graphEl.releasePointerCapture(event.pointerId);
    }
  }

  function moveDrag(event: PointerEvent, index: number) {
    if (dragIndex === index) {
      oninput(index, gainAt(event.clientY));
    }
  }

  function endDrag(event: PointerEvent, index: number) {
    if (dragIndex !== index) return;
    dragIndex = null;
    const dot = event.currentTarget as HTMLButtonElement;
    if (dot.hasPointerCapture(event.pointerId)) {
      dot.releasePointerCapture(event.pointerId);
    }
  }

  function dotKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      step(index, event.key === "ArrowUp" ? 1 : -1);
    }
  }
</script>

<div
  class="eq-graph"
  role="group"
  aria-label="均衡器曲线"
  bind:this={graphEl}
  onpointerdown={startGraphDrag}
  onpointermove={moveGraphDrag}
  onpointerup={endGraphDrag}
  onpointercancel={endGraphDrag}
>
  <span class="eq-unit">−12 dB / +12 dB</span>
  <span class="eq-axis" style="top:8%">+12</span>
  <span class="eq-axis" style="top:29%">+6</span>
  <span class="eq-axis" style="top:50%">0</span>
  <span class="eq-axis" style="top:71%">−6</span>
  <span class="eq-axis" style="top:92%">−12</span>
  <svg viewBox="0 0 100 100" preserveAspectRatio="none" role="img" aria-label="当前均衡器曲线">
    <defs>
      <linearGradient id="eq-wash" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#4e85ff" stop-opacity=".19" />
        <stop offset="1" stop-color="#4e85ff" stop-opacity="0" />
      </linearGradient>
    </defs>
    <g stroke="rgba(21,24,30,.045)" stroke-width=".3" vector-effect="non-scaling-stroke">
      {#each frequencies as _, index}
        <line x1={(index / Math.max(1, frequencies.length - 1)) * 100} y1="0" x2={(index / Math.max(1, frequencies.length - 1)) * 100} y2="100" />
      {/each}
    </g>
    <g stroke="rgba(21,24,30,.075)" stroke-width=".35" vector-effect="non-scaling-stroke">
      <line x1="0" y1="8" x2="100" y2="8" />
      <line x1="0" y1="29" x2="100" y2="29" />
      <line x1="0" y1="50" x2="100" y2="50" stroke="rgba(21,24,30,.15)" />
      <line x1="0" y1="71" x2="100" y2="71" />
      <line x1="0" y1="92" x2="100" y2="92" />
    </g>
    <polygon points={`${points} 100,100 0,100`} fill="url(#eq-wash)" />
    <polyline
      points={points}
      fill="none"
      stroke="#2f6bff"
      stroke-width="1.7"
      vector-effect="non-scaling-stroke"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
  <div class="eq-dots">
    {#each gains as gain, index}
      <button
        type="button"
        class="eq-dot"
        class:active={index === active}
        style={`left:${(index / Math.max(1, gains.length - 1)) * 100}%;top:${50 - gain * 3.5}%`}
        aria-label={`${frequencies[index]} Hz 当前 ${gain} dB，拖动或方向键调整`}
        title={`${frequencies[index]} Hz · ${gain > 0 ? "+" : ""}${gain} dB`}
        onpointerdown={(event) => startDrag(event, index)}
        onpointermove={(event) => moveDrag(event, index)}
        onpointerup={(event) => endDrag(event, index)}
        onpointercancel={(event) => endDrag(event, index)}
        onkeydown={(event) => dotKeydown(event, index)}
      ></button>
    {/each}
  </div>
</div>

<div class="eq-bands">
  {#each frequencies as frequency, index}
    <div class="eq-band" class:active={index === active}>
      <div class="gain-control">
        <button type="button" aria-label={`${frequency} Hz 减 1 dB`} onclick={() => step(index, -1)}>−</button>
        <input
          type="text"
          inputmode="decimal"
          autocomplete="off"
          value={drafts[index] ?? formatGain(gains[index] ?? 0)}
          aria-label={`${frequency} Hz 增益`}
          onfocus={() => beginEdit(index)}
          oninput={(event) => updateDraft(index, event.currentTarget.value)}
          onblur={() => commitDraft(index)}
          onkeydown={(event) => onKeydown(event, index)}
        />
        <button type="button" aria-label={`${frequency} Hz 加 1 dB`} onclick={() => step(index, 1)}>+</button>
      </div>
      <span class="freq">{frequency}<span>Hz</span></span>
    </div>
  {/each}
</div>

<div class="eq-actions">
  <div class="eq-hint">
    拖动曲线或圆点调整 <span>·</span> 也可直接输入 dB <span>·</span> 输入后按 <kbd>Enter</kbd> 写入 <span>·</span> 范围 −12 到 +12 dB
  </div>
  <div class="actions">
    <button class="btn" type="button" onclick={onreset} disabled={busy}>恢复平直</button>
    <button class="btn primary" type="button" onclick={onapply} disabled={busy}>写入耳机</button>
  </div>
</div>
