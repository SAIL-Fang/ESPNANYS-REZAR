<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  import { device } from "$lib/device.svelte";
</script>

<div class="page-head">
  <div>
    <h1>设置</h1>
    <p>THX 是可选运行时依赖；ESPNANYS-REZAR 不打包、不重分发 THX 二进制。</p>
  </div>
</div>

<section class="panel" style="margin-bottom:16px">
  <h2>THX 桥</h2>
  <dl class="kv">
    <dt>状态</dt><dd>{device.thx?.installed ? "已连接" : "未连接"}</dd>
    <dt>端点</dt><dd>{device.thx?.endpoint ?? "—"}</dd>
    <dt>标准化</dt><dd>{device.thx?.enabled ? "开" : "关"} · {device.thx?.level ?? "—"}</dd>
    <dt>DLL</dt><dd>{device.thx?.dll ?? "—"}</dd>
    <dt>DLL 版本</dt><dd>{device.thx?.dllVersion ?? "—"}</dd>
    <dt>ABI</dt><dd>{device.thx?.abi ?? "—"}</dd>
  </dl>
  <div class="row">
    <span class="inline-status">{device.verifyOutput || device.thxError}</span>
    <button class="btn primary" onclick={() => device.verifyThx()} disabled={device.busy}>验证 1–100</button>
  </div>
</section>

<section class="panel">
  <h2>关于</h2>
  <dl class="kv">
    <dt>项目</dt><dd>ESPNANYS-REZAR</dd>
    <dt>作者</dt><dd>SAIL</dd>
    <dt>许可证</dt><dd>MIT</dd>
  </dl>
  <p class="muted">
    ESPNANYS-REZAR 是独立的非官方控制台。HID 协议由本项目实现；声音处理复用用户已安装的 THX APO。
    不包含 Razer/THX 的二进制或资产。
  </p>
  <p class="mono faint">sail_core · sail_hid · sail_app · sail_thx_bridge · sail_ui</p>
</section>
