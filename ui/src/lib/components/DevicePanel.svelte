<script lang="ts">
  import { device } from "$lib/device.svelte";
  import { chargingText, fieldText } from "$lib/types";

  function linkText(): string {
    switch (device.status?.link) {
      case "wireless":
        return "无线 2.4G";
      case "wired":
        return "有线 USB";
      case "unknown":
        return "未知接口";
      default:
        return "未连接";
    }
  }
</script>

<div class="page-head">
  <div>
    <h1>设备</h1>
    <p>连接、电量、固件和硬件开关。所有 HID 读取都由 SAIL 自己完成。</p>
  </div>
  <span class="badge" class:ok={device.status?.connected}>{device.status?.connected ? "在线" : "离线"}</span>
</div>

<section class="panel">
  <h2>状态</h2>
  {#if device.status}
    <dl class="kv">
      <dt>连接</dt><dd>{linkText()}</dd>
      <dt>电量</dt><dd>{fieldText(device.status.battery, "%")}</dd>
      <dt>充电</dt><dd>{chargingText(device.status.charging)}</dd>
      <dt>固件</dt><dd>{fieldText(device.status.firmware)}</dd>
      <dt>序列号</dt><dd>{fieldText(device.status.serial)}</dd>
      <dt>自动关机</dt><dd>{fieldText(device.status.autoPowerOffMin, " 分钟")}</dd>
      <dt>Dongle LED</dt><dd>{fieldText(device.status.dongleLed)}</dd>
      <dt>均衡器</dt><dd>{fieldText(device.status.speakerPresetEq)}</dd>
      <dt>Edition</dt><dd>{fieldText(device.status.edition)}</dd>
    </dl>
    {#if device.status.issues.length > 0}
      <p class="inline-status error">{device.status.issues.join("；")}</p>
    {/if}
  {:else}
    <p class="muted">未读取到设备。确认耳机已通过 2.4G 接收器或 USB 线连接。</p>
  {/if}
</section>
