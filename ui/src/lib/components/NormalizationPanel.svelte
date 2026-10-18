<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  interface Props {
    level: number;
    enabled: boolean;
    available: boolean;
    busy: boolean;
    thxError: string;
    onlevel: (level: number) => void;
    ontoggle: () => void;
    onsave: () => void;
  }

  let {
    level,
    enabled,
    available,
    busy,
    thxError,
    onlevel,
    ontoggle,
    onsave
  }: Props = $props();
</script>

<section class="panel norm-panel" aria-labelledby="norm-title">
  <div class="norm-top">
    <div>
      <div class="eyebrow">THX DRC</div>
      <h2 id="norm-title">声音标准化</h2>
      <p>把不同游戏的响度拉回同一条线。</p>
    </div>
    <button
      class="switch"
      class:on={enabled}
      onclick={ontoggle}
      disabled={!available || busy}
      aria-label="切换声音标准化"
      aria-pressed={enabled}
    ></button>
  </div>

  <div class="level-row">
    <span class="level">{level}</span>
    <span class="level-unit">/ 100</span>
  </div>
  <input
    class="norm-level-range"
    type="range"
    min="1"
    max="100"
    step="1"
    value={level}
    disabled={!available || busy}
    aria-label="声音标准化等级"
    aria-valuetext={`${level} 级，范围 1 到 100`}
    oninput={(event) => onlevel(Number(event.currentTarget.value))}
    onchange={onsave}
  />
  <div class="range-scale"><span>1</span><span>100</span></div>

  <p class="muted" style="margin-top:8px;font-size:10.5px">
    {#if available}
      THX 已连接，SAIL 直接写入 1–100。
    {:else}
      未检测到 THX；均衡器仍可用。
    {/if}
  </p>
  {#if thxError}
    <p class="inline-status error">{thxError}</p>
  {/if}
</section>
