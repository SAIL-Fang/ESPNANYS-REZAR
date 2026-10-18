# 音频功能逆向：EQ、声音标准化、麦克风增强

本文把 Synapse 里 **声音** / **增强** / **麦克风** 三个页面的功能映射到实际调用。数据来自 2026-10-18 的实时操作 + THX/HID 日志。

机器可读版本：`captures/audio-features.json`

## 1. 总览：哪些走 HID，哪些走 THX

| 功能 | UI 位置 | 硬件 HID | THX APO | 备注 |
|---|---|---|---|---|
| 扬声器 EQ 预设/10 段 | 声音 → 音频均衡器 | ✅ `0x93` + `0x95` | ✅ `SetRenderEQGains` | 两边都写 |
| 空间音效 / 立体声 | 声音 → 输出模式 | ❌ | ✅ `Get/SetRenderSpatialProcessing` | 与 DRC 独立 |
| 麦克风 EQ 预设/10 段 | 麦克风 → 麦克风均衡器 | ❌ `0x96` status=5 | ✅ `SetCaptureEQGains` | 无线版不支持 HID 麦克风 EQ |
| 声音标准化 | 增强 → 声音标准化 | ❌ | ✅ `SetRenderNormalization(Level)` | 即 THX DRC |
| 低音增强 | 增强 → 低音增强 | ❌ | ✅ `SetRenderBassBoost(Level)` | |
| 人声清晰度（扬声器） | 增强 → 人声清晰度 | ❌ | ✅ `SetRenderVocalClarity(Level)` | |
| 噪声门 | 麦克风 → 噪声门 | ❌ | ✅ `SetCaptureVoiceGate(Level)` | UI 1–5 → -60…-20 dB |
| 麦克风降噪 | 麦克风 → 增强 → 麦克风降噪 | ❌ | ✅ `SetCaptureNoiseReduction(Level)` | 0–100 1:1 |
| 音量标准化（麦克风） | 麦克风 → 增强 → 音量标准化 | ❌ | ✅ `SetCaptureNormalization` | 默认关/55 |
| 亮丽人声（麦克风） | 麦克风 → 增强 → 亮丽人声 | ❌ | ✅ `SetCaptureVocalClarity` | 默认关/55 |
| 麦克风音量 | 麦克风 → 麦克风音量 | ❌ | ✅ `SetCaptureVolumeBySystem` | 0–100 1:1 |
| 麦克风测试 | 麦克风 → 麦克风测试 | ❌ | ✅ `SetCapturePreview` | 开始/停止 |
| 麦克风监听（侧音） | 麦克风 → 侧音 | ✅ `0x98`/`0x99` | ✅ `SetStartCaptureStatus` + `SetCaptureSidetoneLevel` | 双链路 |
| 勿扰 | 增强 → 勿扰 | ✅ record `0x27` | ❌ | 本轮未实时抓 |

> 关键结论：**扬声器 EQ 有硬件寄存器**，可以完全脱离 Synapse/THX 写；**麦克风 EQ、噪声门、降噪、标准化、亮丽人声全部是 THX APO**，HID 只负责侧音开关/音量。

## 2. 扬声器 EQ（声音 → 音频均衡器）

### 2.1 参数

- 10 段：`31, 63, 125, 250, 500, 1k, 2k, 4k, 8k, 16k Hz`
- 范围：`-12 … +12 dB`
- HID payload：10 个 **signed byte**（`0xFB = -5`）
- UI 预设：游戏 / 电影 / 音乐 / 自定义 / 重置

### 2.2 预设映射（实测）

| UI | HID `0x93` | THX preset | 10 段值（HID `0x95`） |
|---|---:|---|---|
| 游戏 | `7` | `Game Mode` | `[1,-5,-3,0,1,1,1,1,1,-3]` |
| 音乐 | `8` | `Music Mode` | `[3,3,2,0,1,1,1,1,1,-3]` |
| 电影 | `9` | `Cinema Mode` | `[4,4,3,-4,-6,-3,1,3,1,-1]` |
| 自定义 | `255` (`0xFF`) | `Custom` | `[3,3,2,0,1,1,1,1,1,-1]` |

### 2.3 实际调用序列（以电影为例）

```text
HID:
  02 00 6A ... 05 00 00 80 93 00 01 09 ... CHK 00
  02 00 6B ... 0E 00 00 80 95 00 0A 04 04 03 FC FA FD 01 03 01 FF ... CHK 00

THX:
  SetRenderPreset(containerId, 'Cinema Mode')
  SetRenderBatch(containerId, true)
  SetRenderEQGains(containerId, [4,4,3,-4,-6,-3,1,3,1,-1])
  SetRenderBatch(containerId, false)
```

- `0x93` 只切预设索引。
- `0x95` 写完整 10 段曲线；单段拖动没有单独抓，但预设证明 payload 是完整数组。
- THX 侧用 `SetRenderBatch(true/false)` 包住 `SetRenderEQGains`，避免多次重载。
- `0x9E` 是 Speaker Preset EQ Status（本轮读到 0）。

## 3. 麦克风 EQ（麦克风 → 麦克风均衡器）

### 3.1 预设映射（实测）

| UI | 10 段值（THX `SetCaptureEQGains`） |
|---|---|
| 默认 | `[0,0,0,0,0,0,0,0,0,0]` |
| 麦克风增强 | `[2,4,6,7,7,7,7,6,4,2]` |
| 直播 | `[4,4,4,3,-2,-7,-4,-2,-3,-5]` |
| 会议 | `[-8,-6,-5,-3,-1,1,3,2,1,0]` |
| 自定义 | `[0,0,0,0,0,0,0,0,0,0]` |

- 频段标签同样是 `31Hz … 16kHz`，范围 `±12 dB`。
- 只调用 `SetCaptureEQGains(containerId, [10 floats])`；没有 `SetCapturePreset` 调用。
- HID `0x96 Set Mic Preset EQ Index` 在 0x0565 上返回 **status=5（Command Not Support）**，所以麦克风 EQ 必须走 THX。
- 单段拖动未抓；预设数组已确认是 10 个值。

## 4. 麦克风增强（重点：噪声门 / 降噪）

### 4.1 噪声门（Noise Gate）

UI：麦克风页 “噪声门”，勾选后出现阈值滑杆。

| UI 值 | THX `SetCaptureVoiceGateLevel` |
|---:|---:|
| 1 | -60 dB |
| 2 | -50 dB |
| 3 | -40 dB |
| 4 | -30 dB |
| 5 | -20 dB |

调用：

```text
开启: SetCaptureVoiceGate(containerId, true)
      SetCaptureVoiceGateLevel(containerId, -60)
调节: SetCaptureVoiceGateLevel(containerId, -40)   # UI 3
      SetCaptureVoiceGateLevel(containerId, -20)   # UI 5
关闭: SetCaptureVoiceGate(containerId, false)
```

- 默认：关闭，level = `-60`。
- 开启时先写 `true`，再写当前 level。
- 关闭时只写 `false`，不重置 level。
- 公式：`dB = -70 + 10 * UI`（UI 1–5）。

### 4.2 麦克风降噪（Noise Reduction）

UI：增强 → 麦克风降噪，默认开启，滑杆 60。

调用：

```text
开启: SetCaptureNoiseReduction(containerId, true)
调节: SetCaptureNoiseReductionLevel(containerId, 60)
关闭: SetCaptureNoiseReduction(containerId, false)
```

- 强度 **0–100 直接映射**：UI 90 → 90；历史值还有 15 / 40 / 60 / 90。
- 关闭时滑杆隐藏，只发 `false`；重新开启不重发 level（THX 记住）。
- 默认：开启 / 60。

### 4.3 音量标准化（麦克风）

- 开关：`SetCaptureNormalization(containerId, true/false)`
- 等级：`SetCaptureNormalizationLevel(containerId, 55)`（默认 55，本轮开关未重发 level）
- 默认：关闭 / 55

### 4.4 亮丽人声（麦克风）

- 开关：`SetCaptureVocalClarity(containerId, true/false)`
- 等级：`SetCaptureVocalClarityLevel(containerId, 55)`（默认 55）
- 默认：关闭 / 55
- 打开后 UI 出现 55 的滑杆；本轮拖动没有触发 level 调用，API 名称/默认值已确认。

### 4.5 麦克风音量

- UI 滑杆走 **`SetCaptureVolumeBySystem(containerId, 0..100)`**，1:1。
  - UI 73 → 73
  - UI 100 → 100
- `SetCaptureVolume(containerId, 10)` 是 THX 内部增益，不是 UI 音量；启动/麦克风测试时出现。

### 4.6 麦克风测试

```text
开始: SetCapturePreview(containerId, true)
      SetCaptureVolume(containerId, 10)
结束: SetCapturePreview(containerId, false)
```

- 必须在一个**常驻 helper 进程**里调用 `SetCapturePreview(true)`。短命进程启动 preview 后立即卸载 DLL 会触发 `0xC0000005`（音频实际上已启动，但 UI 会看到空错误且没有停止按钮）。
- 本项目用 `sail_thx mic-preview-hold` 持有 preview，另一个 `sail_thx mic-preview off` 负责停止；hold 进程检测到 preview=false 后退出。
- 按钮激活时变为「停止测试」；关闭后恢复「开始测试」。

### 4.7 麦克风监听（侧音）

双链路：

```text
开启:
  THX: SetStartCaptureStatus(containerId, true)
       SetCaptureSidetoneLevel(containerId, 1582)
  HID: 0x98 01  (enable)
       0x99 07  (volume; UI 50 -> 7)

关闭:
  THX: SetStartCaptureStatus(containerId, false)
  HID: 0x98 00
```

- 没有观察到 `SetCaptureSidetone` action；THX 用 `SetStartCaptureStatus` 启停采集链。
- THX level `1582` 是内部标量，不等于 UI 50。
- HID `0x99` 的值 7 对应 UI 50；0–100 → 0–15 的精确映射还差一个采样点（建议测 0/100）。
- 默认：关闭 / UI 50。

## 5. 声音标准化（增强页）

- UI：增强 → 声音标准化，默认开，滑杆 10。
- **ESPNANYS-REZAR 默认等级改为 5**（Synapse 组件默认仍是 10）。首次启动写 5；用户改过以后把最后值存到 WebView `localStorage`，下次启动优先恢复该值，没有记录才用 5。
- 开关：`SetRenderNormalization(containerId, true/false)`
- 等级：`SetRenderNormalizationLevel(containerId, 0..100)`
- 实测 UI 54 → 54，1:1。
- 历史值：10（Game）/ 100（Music、Cinema、Custom、PUBG）。
- **不发任何 HID 包**；完全是 THX DRC。

### 5.1 为什么 Synapse 只有 10–100，以及能不能做 1–100

Synapse 页面里的转换代码（从页面缓存提取）：

```js
convertToTHXLevel(t) { return t / 100 * thx_setting_steps + thx_setting_min }
convertToUILevel(t) { return Math.round((t - thx_setting_min) / thx_setting_steps * 100) }
```

实测 `10 → 10`、`54 → 54`、`100 → 100`，说明这个设备上 THX 层是 **1:1 的 0–100**。  
Synapse 滑杆的 **10 是前端组件的最小值**，不是 THX API 的下限。

- `SetRenderNormalizationLevel` 的参数类型是 `double`。
- 直接调用 THX setter 传 `1`，返回 `true`。
- 但 Synapse 页面有缓存，直接改 THX 后切页仍显示 10；需要用独立的 THX 桥/刷新页面来确认 1–9 的实际听感。
- 结论：**1–100 在 API 层可行**，实现时做一次 A/B 回读即可；不要在前端把最小值写死 10。

> 如果最终发现 THX 内部对 1–9 做了静默 clamp，前端仍可显示 1–100，但后端要按实际回读值校正；这一点需要在 THX 桥里验证。

## 6. 其他增强项

| UI | 开启 | 等级 |
|---|---|---|
| 低音增强 | `SetRenderBassBoost(containerId, true)` | `SetRenderBassBoostLevel(containerId, 0..100)`，默认 55 |
| 人声清晰度（扬声器） | `SetRenderVocalClarity(containerId, true)` | `SetRenderVocalClarityLevel(containerId, 0..100)`，默认 55 |
| 勿扰 | HID record `0x27` | — |

## 7. 对“自己写控制软件”的直接结论

1. **扬声器 EQ 可以纯 HID 实现**：`0x93` + `0x95`，不需要 THX；这是最低占用、最稳的部分。
2. **麦克风 EQ / 噪声门 / 降噪 / 标准化 / 亮丽人声必须控制 THX APO**；HID 没有对应命令（`0x96` 还不支持）。
3. **侧音是双写**：HID 控制硬件侧音，THX 控制采集链；只写一边可能“看起来开了但没声音”或“有声音但 UI 状态不对”。
4. **音量/静音优先走 Windows Core Audio**（`SetCaptureVolumeBySystem` 本质也是系统音量），不要自己维持音频流。
5. 如果只做“低占用后台 + 不影响游戏”，建议 MVP 先做：
   - HID：电量、固件、侧音、扬声器 EQ、自动关机；
   - THX：只做按需配置（用户打开麦克风页时才初始化），不做常驻音频处理。

## 8. 已实现 / 未闭环

- **已实现**：`SetCaptureEQGains` 的 10 段拖动会写成 JSON 字符串 `[v0,...,v9]`，并先调用 `SetCaptureEQ(true)`；否则只写 gains 会被 THX 旁路。
- **已实现**：噪声门（float dB）、降噪、标准化、亮丽人声、麦克风音量、`SetCapturePreview`、`SetCaptureSidetone` 都已通过真实 DLL 回读验证。
- **已实现**：启动/读写前通过 `GetCaptureDevice` + `GetCaptureDeviceList` 绑定精确的 Razer BlackShark 麦克风，避免 Windows 默认麦克风不是耳机时写错设备。
- 侧音 HID 音量 0–100 → 0–15 仍是线性换算（当前 50→7），精确曲线没有更多采样点。
- 勿扰的 HID 写入帧（record `0x27` 已知，未实时抓写方向）。
