# 前端与开源方案

## 0. 先澄清：不依赖 Razer 软件

`RazerHidService` 只是本文档里的**占位名**，不是 Razer 的官方服务，也不会调用 Synapse / Razer Central。

- HID 协议是逆向出来的事实（64 字节帧、命令、校验和），可以完全用自己的代码实现。
- 不需要 Razer 的 DLL、Electron 包、云服务。
- 开源仓库里只放**自己的代码 + 协议文档**，不放 Razer/THX 的二进制、图标、安装包。

建议把项目名改成中性名字，例如：

- `blackshark-control`
- `open-headset`
- `hydra-headset`
- `bs2-toolkit`

README 里写明：**Unofficial, not affiliated with Razer or THX.**

## 1. 前端选型

### 推荐：Tauri 2 + Svelte/React + TypeScript

```text
Tauri 2 (Rust)
  ├─ 系统 WebView2（Windows 10/11 自带，不打包 Chromium）
  ├─ 托盘 + 按需窗口
  ├─ Rust core：HID 协议、命令队列、状态缓存
  └─ 前端：Svelte 5 / React + TS + SVG EQ
```

优点：

- 安装包小（几 MB 到十几 MB），内存远低于 Electron；
- 窗口关闭后 WebView 进程退出，只剩 Rust 托盘进程；
- Rust 直接做 HID，不需要额外服务；
- 前端用 Web 技术，EQ 曲线、滑杆、动画都好做；
- 开源生态成熟，GitHub Actions 可构建 Windows 安装包；
- Tauri 的 `emit`/`listen` 适合把电量、连接状态推给 UI。

代价：

- 依赖系统 WebView2（Win10 较老版本可能需要装 Runtime；Win11 自带）；
- 前端和 Rust 之间要设计命令/事件边界；
- WebView2 本身有内存开销（窗口打开时），但比 Electron 小很多。

### 其他方案对比

| 方案 | 占用 | 开发速度 | EQ 图形 | 适合 |
|---|---|---|---|---|
| **Tauri 2 + Svelte/React** | 低 | 快 | 很好（SVG/Canvas） | 推荐 |
| **WinUI 3 (C#/C++)** | 很低 | 中 | 好 | 纯 Windows、原生 Fluent |
| **Avalonia (C#)** | 低 | 中 | 好 | 跨平台、C# 团队 |
| **WPF** | 中 | 快 | 好 | 老 Windows 项目 |
| **egui / iced (Rust)** | 极低 | 慢 | 一般 | 极简工具、无设计需求 |
| **本地 Web + HTTP** | 低 | 快 | 很好 | 不推荐：端口/鉴权/攻击面 |
| **Electron** | 高 | 快 | 很好 | 不推荐：太重 |

> 如果团队是 C# 背景，选 **Avalonia** 比 WinUI 3 更省心（跨平台、开源、XAML 熟悉）。
> 如果追求单二进制/极低占用，选 **egui/iced**，但中文排版、无障碍、视觉细节要自己补。

## 2. 不建议单独做 Windows 服务（MVP）

单用户桌面场景：

```text
托盘 App（Tauri/Rust）
  └─ 直接持有 HID + 窗口
```

- 窗口关闭 → 隐藏到托盘，Rust 进程继续；
- 不需要 Named Pipe、不需要服务安装、不需要 IPC 鉴权；
- 更少故障点，更少权限问题。

只有这些情况才拆服务：

- 多用户/远程桌面，UI 关闭后仍要控制设备；
- 需要开机即控制、不登录用户；
- 多个前端（CLI/Web/游戏插件）共享一个设备连接。

那时再拆：

```text
RazerHidService（服务，持有 HID）
  └─ Named Pipe + 随机 token
       ├─ Tauri UI
       └─ CLI
```

## 3. 页面结构建议

```text
┌─────────────────────────────────────────────────────┐
│ 顶部：设备名 · 连接状态 · 电量 · 固件 · 设置        │
├──────────┬──────────────────────────────────────────┤
│ 侧栏      │ 内容区                                   │
│ 设备      │  设备：固件/序列号/自动关机/Dongle LED    │
│ 声音      │  声音：EQ 预设 + 10 段曲线 + 标准化       │
│ 麦克风    │  麦克风：音量/噪声门/降噪/EQ/侧音/测试    │
│ 电源      │  电源：自动关机、电量阈值                 │
│ 设置      │  设置：开机启动/托盘/语言/主题/日志       │
└──────────┴──────────────────────────────────────────┘
```

### 声音页

- 预设卡片：游戏 / 音乐 / 电影 / 自定义；
- 10 段 EQ 曲线：SVG，拖点改值，显示 `+12…-12 dB`；
- 声音标准化：开关 + 0–100 滑杆；
- 低音增强、人声清晰度：开关 + 滑杆；
- 重置按钮。

### 麦克风页

- 麦克风音量：0–100；
- 噪声门：开关 + 阈值（UI 1–5，映射 -60…-20 dB）；
- 麦克风降噪：开关 + 0–100；
- 音量标准化、亮丽人声：开关 + 滑杆；
- 麦克风 EQ：5 个预设 + 10 段曲线；
- 侧音：开关 + 0–100（HID 0x99 需要补映射）；
- 麦克风测试：按住/点击开始，显示电平条（可选）。

### 设备/设置页

- 固件、序列号、Edition、Dongle LED；
- 自动关机分钟数；
- 开机启动、最小化到托盘、语言（中/英）、主题；
- 调试：原始 HID 帧、日志导出、恢复默认。

## 4. Tauri 数据流

```text
Frontend (Svelte/React)
  ├─ invoke("get_status") -> Rust
  ├─ invoke("set_speaker_eq", {preset, gains}) -> Rust
  ├─ invoke("set_mic_noise_gate", {enabled, level}) -> ThxHost (optional)
  └─ listen("device-status") <- Rust
```

Rust 侧：

```text
src-tauri/
  ├─ main.rs             # 托盘、窗口、命令注册
  ├─ hid/                # 64-byte 协议、校验、seq
  ├─ state.rs            # 电量/连接/EQ 缓存
  ├─ thx/                # 可选：调用 ThxHost
  └─ commands.rs         # Tauri commands
```

前端侧：

```text
src/
  ├─ lib/eq.ts           # 10 段曲线、预设数组
  ├─ lib/api.ts          # invoke 封装
  ├─ routes/Sound.svelte
  ├─ routes/Mic.svelte
  └─ routes/Device.svelte
```

### EQ 曲线实现建议

- 用 SVG `polyline` + 10 个 `circle`；
- 指针事件 `pointerdown/move/up`，按 y 映射 -12…+12；
- 拖动时只更新本地状态，`pointerup` 才写设备；
- 连续拖动可 50 ms debounce，避免 HID 写太多；
- 预设切换时先 `0x93`，再 `0x95`；
- THX 侧用 `SetRenderBatch(true/false)` 包住 `SetRenderEQGains`。

## 5. 开源仓库结构

```text
blackshark-control/
├─ README.md
├─ LICENSE                 # MIT 或 Apache-2.0
├─ docs/
│  ├─ protocol.md          # 64-byte HID 协议
│  ├─ audio-features.md    # EQ/标准化/麦克风映射
│  └─ architecture.md
├─ crates/
│  ├─ razer-hid/           # 纯协议库，无 UI
│  │  ├─ src/lib.rs
│  │  └─ tests/            # mock HID / 帧构造测试
│  └─ razer-cli/           # 命令行
├─ apps/
│  └─ desktop/             # Tauri + Svelte/React
├─ .github/workflows/      # Windows build + test
└─ scripts/                # 采集/诊断脚本
```

C# 版本：

```text
src/
  ├─ BlackShark.Core/      # HID 协议
  ├─ BlackShark.Cli/
  └─ BlackShark.App/       # Avalonia
```

### 开源注意

- **不要提交** Razer/THX 的 DLL、安装包、图标、截图（可以放协议文档和自己的 UI 截图）。
- README 写清楚 “unofficial / not affiliated”。
- 不要用 Razer 商标做 logo；项目名用中性词。
- 协议文档写“clean-room observation”，不要贴反编译源码。
- 测试用 mock HID，不要求 CI 有耳机。
- License 推荐 **MIT**（宽松、易用）或 **Apache-2.0**（专利条款）。
- 如果以后接 THX，做成**可选 feature**，运行时加载用户自己安装的 DLL，不打包、不分发。

## 6. THX 依赖决策（重要）

如果目标是“完全不用他家任何东西”，要区分：

| 功能 | 能否纯自研 |
|---|---|
| 电量/固件/序列号/自动关机 | ✅ 纯 HID |
| 侧音开关/音量 | ✅ 纯 HID |
| 扬声器 EQ | ✅ 纯 HID（0x93/0x95） |
| 声音标准化 | ❌ 是 THX DRC；需 THX 或自研 DSP |
| 麦克风 EQ | ❌ 是 THX `SetCaptureEQGains`；0x96 不支持 |
| 噪声门/降噪/标准化/亮丽人声 | ❌ 是 THX APO |

三条路线：

1. **MVP 不含 THX**：只做 HID 功能；麦克风增强页显示“需要 THX APO”，不可用。  
   最容易开源、最少法律/依赖问题。
2. **可选 THX 适配器**：用户自己安装 Synapse/THX；程序运行时加载 `ThxV3Native.dll`。  
   仓库不含二进制；但 ABI 可能变，且要处理未安装。  
3. **自研 APO/DSP**：自己写 Windows APO + 虚拟麦克风，实现降噪/噪声门/EQ。  
   工作量大、需要驱动签名、兼容性风险高；不建议作为第一版。

> 如果“开源”是硬要求，建议先走路线 1；THX 作为后续可选插件，而不是核心依赖。

## 7. 推荐结论

- **前端**：Tauri 2 + Svelte 5/React + TypeScript + SVG EQ。
- **核心**：Rust `razer-hid` crate；托盘 App 直接持有 HID，MVP 不拆服务。
- **麦克风 THX 功能**：先标记为“可选/需 THX”，不要打进核心；开源仓库不打包 THX。
- **名字**：不要叫 `RazerHidService`，用中性项目名；README 加非官方声明。
- **第一版范围**：设备状态 + 侧音 + 扬声器 EQ + 自动关机 + 托盘/设置；麦克风页先做 UI 骨架和“THX 未安装”提示。
