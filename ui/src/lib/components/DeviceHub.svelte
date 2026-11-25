<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { device } from "$lib/device.svelte";
  import { HUB_COPY, deriveHubCard, modelSubtitle, modelTitle } from "$lib/presentation";
  import type { HubCard } from "$lib/presentation";

  interface Props {
    onEnter: (modelId: string) => void;
  }

  let { onEnter }: Props = $props();

  onMount(() => {
    const timer = setInterval(() => {
      void device.refreshSummaries();
    }, 10_000);
    return () => clearInterval(timer);
  });
</script>

{#snippet cardBody(card: HubCard)}
  <div class="device-card-top">
    <div>
      <h2>{card.title}</h2>
      <p>{card.subtitle}</p>
    </div>
    {#if card.active}
      <span class="badge ok">当前控制</span>
    {/if}
  </div>

  <div class="device-card-status">
    <span class="dot" class:on={card.connected} class:standby={card.standby}></span>
    <strong>{card.stateText}</strong>
  </div>

  <dl class="device-card-facts">
    <div>
      <dt>电量</dt>
      <dd>{card.batteryText}</dd>
    </div>
    {#if card.thxText !== null}
      <div>
        <dt>THX</dt>
        <dd>{card.thxText}</dd>
      </div>
    {/if}
    {#if card.outputText !== null}
      <div>
        <dt>输出</dt>
        <dd>{card.outputText}</dd>
      </div>
    {/if}
    <div>
      <dt>类型</dt>
      <dd>{card.typeText}</dd>
    </div>
  </dl>

  <span class="device-card-enter">{card.actionText}</span>
{/snippet}

<div class="device-hub">
  <header class="hub-head">
    <div>
      <div class="brand hub-brand">ESPNANYS-REZAR</div>
      <h1>设备库</h1>
      <p>{HUB_COPY.headline}</p>
    </div>
    <button class="btn" type="button" onclick={() => void device.refreshSummaries()}>
      刷新设备
    </button>
  </header>

  {#if device.summaries.length > 0}
    <section class="hub-grid" aria-label="已连接设备">
      {#each device.summaries as summary (summary.model.id)}
        {@const card = deriveHubCard(summary)}
        {#if card.interactive}
          <button
            class="device-card"
            class:active={card.active}
            type="button"
            onclick={() => onEnter(summary.model.id)}
            aria-label={`进入 ${card.title}`}
          >
            {@render cardBody(card)}
          </button>
        {:else}
          <div class="device-card telemetry-card" class:active={card.active}>
            {@render cardBody(card)}
          </div>
        {/if}
      {/each}
    </section>
  {:else if device.devices.length > 0 && device.loading}
    <section class="hub-grid" aria-label="正在读取设备">
      {#each device.devices as model (model.id)}
        <div class="device-card loading-card" aria-busy="true">
          <div class="device-card-top">
            <div>
              <h2>{modelTitle(model)}</h2>
              <p>{modelSubtitle(model)}</p>
            </div>
          </div>
          <div class="device-card-status">
            <span class="dot"></span>
            <strong>正在读取状态…</strong>
          </div>
          <span class="device-card-enter">请稍候</span>
        </div>
      {/each}
    </section>
  {:else if device.loading}
    <section class="hub-empty">
      <strong>{HUB_COPY.scanningTitle}</strong>
      <span>{HUB_COPY.scanningDetail}</span>
    </section>
  {:else}
    <section class="hub-empty">
      <strong>{HUB_COPY.emptyTitle}</strong>
      <span>{HUB_COPY.emptyDetail}</span>
    </section>
  {/if}

  {#if device.error}
    <p class="inline-status error">{device.error}</p>
  {/if}
</div>
