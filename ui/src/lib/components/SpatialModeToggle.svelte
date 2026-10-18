<!--
  ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
  Author: SAIL
  Copyright (c) 2026 SAIL
  SPDX-License-Identifier: MIT
-->
<script lang="ts">
  interface Props {
    value: boolean;
    disabled?: boolean;
    label?: string;
    spatialLabel?: string;
    onchange: (enabled: boolean) => void;
  }

  let {
    value,
    disabled = false,
    label = "输出模式",
    spatialLabel = "THX 空间音效",
    onchange
  }: Props = $props();
</script>

<div class="preset-seg spatial-toggle" role="group" aria-label={label}>
  <button
    type="button"
    class:on={!value}
    disabled={disabled}
    aria-pressed={!value}
    onclick={() => onchange(false)}
  >立体声</button>
  <button
    type="button"
    class:on={value}
    disabled={disabled}
    aria-pressed={value}
    onclick={() => onchange(true)}
  >{spatialLabel}</button>
</div>
