<script lang="ts">
  import { device } from "$lib/device.svelte";

  let sidetone = $state(false);

  async function toggleSidetone() {
    sidetone = !sidetone;
    await device.setSidetone(sidetone);
  }
</script>

<div class="page-head">
  <div>
    <h1>麦克风</h1>
    <p>侧音走耳机 HID；降噪、噪声门、标准化、亮丽人声由 THX APO 处理。</p>
  </div>
  <span class="badge" class:ok={device.thx?.installed} class:warn={!device.thx?.installed}>
    {device.thx?.installed ? "THX 桥已连接" : "THX 未安装"}
  </span>
</div>

<div class="grid-mic">
  <section class="panel">
    <h2>输入</h2>
    <div class="row">
      <div class="label"><span>侧音</span><small>硬件侧音，不经 THX</small></div>
      <button
        class="switch"
        class:on={sidetone}
        onclick={toggleSidetone}
        disabled={device.busy}
        aria-label="切换侧音"
        aria-pressed={sidetone}
      ></button>
    </div>
    <div class="row placeholder-row">
      <div class="label"><span>麦克风音量</span><small>下一阶段接入 Core Audio</small></div>
      <span class="faint">—</span>
    </div>
    <div class="row placeholder-row">
      <div class="label"><span>麦克风测试</span><small>下一阶段接入 THX 预览</small></div>
      <span class="faint">—</span>
    </div>
    <p class="inline-status">{device.notice}</p>
  </section>

  <section class="panel">
    <h2>THX 增强</h2>
    <div class="row placeholder-row">
      <div class="label"><span>噪声门</span><small>1–5 / -60…-20 dB</small></div>
      <span class="badge">下一阶段</span>
    </div>
    <div class="row placeholder-row">
      <div class="label"><span>降噪</span><small>0–100</small></div>
      <span class="badge">下一阶段</span>
    </div>
    <div class="row placeholder-row">
      <div class="label"><span>音量标准化</span><small>0–100</small></div>
      <span class="badge">下一阶段</span>
    </div>
    <div class="row placeholder-row">
      <div class="label"><span>亮丽人声</span><small>0–100</small></div>
      <span class="badge">下一阶段</span>
    </div>
    <p class="muted" style="font-size:12px">
      THX 桥已经能读写声音标准化；麦克风参数会在下一阶段按同样的 bridge 模式接入。
    </p>
  </section>
</div>
