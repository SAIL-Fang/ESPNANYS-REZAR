<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { device } from "$lib/device.svelte";
  import { chargingText, fieldText } from "$lib/types";
  import type { DeviceSummary, ModelDescriptor } from "$lib/types";

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

  function title(model: ModelDescriptor): string {
    if (model.capabilities.eqViaThx) return "BlackShark V2";
    return model.displayName;
  }

  function subtitle(summary: DeviceSummary): string {
    if (summary.model.capabilities.eqViaThx) {
      return "Razer USB Sound Card · 3.5mm";
    }
    if (summary.link === "wireless") return "2.4 GHz 无线";
    if (summary.link === "wired") return "USB 有线";
    return "无线接收器";
  }

  function stateText(summary: DeviceSummary): string {
    if (!summary.connected) {
      return summary.link === "standby" ? "耳机未开机" : "未连接";
    }
    return summary.currentOutput ? "当前输出" : "输出未指向";
  }

  function powerText(summary: DeviceSummary): string {
    if (!summary.model.capabilities.battery) return "无电池遥测";
    if (!summary.connected) return "—";
    const battery = fieldText(summary.battery, "%");
    const charging = chargingText(summary.charging);
    return `${battery} · ${charging}`;
  }
</script>

<div class="device-hub">
  <header class="hub-head">
    <div>
      <div class="brand hub-brand">ESPNANYS-REZAR</div>
      <h1>设备库</h1>
      <p>选择一个已连接的雷蛇耳机进入控制。选择设备不会自动更改 Windows 默认输出。</p>
    </div>
    <button class="btn" type="button" onclick={() => void device.refreshSummaries()}>
      刷新设备
    </button>
  </header>

  {#if device.summaries.length === 0 && device.loading}
    <section class="hub-empty">
      <strong>正在读取设备…</strong>
      <span>正在识别已连接的雷蛇耳机和输出状态。</span>
    </section>
  {:else if device.summaries.length === 0}
    <section class="hub-empty">
      <strong>未检测到受支持的雷蛇耳机</strong>
      <span>请连接 BlackShark V2 HyperSpeed 接收器，或 Razer USB Sound Card。</span>
    </section>
  {:else}
    <section class="hub-grid" aria-label="已连接设备">
      {#each device.summaries as summary (summary.model.id)}
        <button
          class="device-card"
          class:active={summary.active}
          type="button"
          onclick={() => onEnter(summary.model.id)}
          aria-label={`进入 ${title(summary.model)}`}
        >
          <div class="device-card-top">
            <div>
              <h2>{title(summary.model)}</h2>
              <p>{subtitle(summary)}</p>
            </div>
            {#if summary.active}
              <span class="badge ok">当前控制</span>
            {/if}
          </div>

          <div class="device-card-status">
            <span class="dot" class:on={summary.connected} class:standby={summary.link === "standby"}></span>
            <strong>{stateText(summary)}</strong>
          </div>

          <dl class="device-card-facts">
            <div>
              <dt>电量</dt>
              <dd>{powerText(summary)}</dd>
            </div>
            <div>
              <dt>THX</dt>
              <dd>{summary.thxCapable ? "可用" : "不支持"}</dd>
            </div>
            <div>
              <dt>输出</dt>
              <dd>{summary.currentOutput ? "当前输出" : "未指向"}</dd>
            </div>
            <div>
              <dt>类型</dt>
              <dd>{summary.model.capabilities.eqViaThx ? "USB 声卡" : "无线耳机"}</dd>
            </div>
          </dl>

          <span class="device-card-enter">进入设备 →</span>
        </button>
      {/each}
    </section>
  {/if}

  {#if device.error}
    <p class="inline-status error">{device.error}</p>
  {/if}
</div>
