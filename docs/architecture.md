# 架构：Synapse 原来怎么做的，以及低占用上位机怎么做

## 1. Synapse 的实际架构（逆向观察）

```text
Razer Synapse 窗口 (Chromium/Electron)
  ├─ 远程页面 https://apps.razer.com/synapse/products/1390/mw/index.html
  │    ├─ 设备控制 UI（声音/增强/麦克风/电源/演示）
  │    └─ THX 控制 UI
  ├─ IPC
  ▼
RazerAppEngine.exe（Electron main + 多个 renderer/GPU/utility 进程）
  ├─ node-rz-hid / HID.node（native addon）
  │    └─ HID interface 3, usage 0xFF14, report ID 2
  ├─ THXDLL_V3CommonSubProcess
  │    └─ ThxV3Native_v1.1.42.1.dll
  │         └─ Windows 音频 APO（耳机 endpoint）
  ├─ 设备/配置/云同步/遥测
  └─ Razer Central Service（系统服务）

THX APO 本身运行在 audiodg.exe / 音频引擎，不在 RazerAppEngine 进程里。
```

实测 `RazerAppEngine.exe` 同时有 **13 个进程**（main、GPU、多个 renderer、utility、THX FFI 等），再加远程 Web 页面。这就是它“重”的原因：

- Chromium/Electron 多进程；
- 远程 `apps.razer.com` 页面（网络、缓存、JS）；
- 页面与 main 之间大量 IPC；
- HID/THX 状态轮询与日志；
- 云同步/遥测/商店等与耳机控制无关的模块；
- 每个功能都要经过 “Web UI → IPC → main → native” 多层。

## 2. 推荐的低占用架构

目标：**后台常驻、无窗口/托盘、玩游戏时 <0.1% CPU、10–30 MB 内存、无 GPU、无浏览器**。

```text
┌─────────────────────────────────────────────────────────────┐
│  RazerHidService.exe（Windows 服务 / 托盘后台，Native）      │
│                                                             │
│  ┌──────────────┐   ┌──────────────┐   ┌────────────────┐  │
│  │ HID Worker   │   │ State Cache  │   │ Command Queue  │  │
│  │ 1 thread     │◄──│ battery/EQ/  │◄──│ seq + checksum │  │
│  │ blocking read│   │ sidetone/... │   │ retry/timeout  │  │
│  └──────┬───────┘   └──────┬───────┘   └───────┬────────┘  │
│         │                  │                   │           │
│         └──────────┬───────┴───────────────────┘           │
│                    ▼                                       │
│           Event Bus / Named Pipe Server                    │
│                    │                                       │
│         ┌──────────┴──────────┐                            │
│         ▼                     ▼                            │
│   UI (on demand)       ThxHost.exe (optional)              │
│   WinUI3/WPF/WebView2   native, out-of-proc                │
│         │                     │                            │
│         └──────────┬──────────┘                            │
│                    ▼                                       │
│           ThxV3Native.dll → THX APO                        │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 技术选型

| 层 | 推荐 | 理由 |
|---|---|---|
| 后台服务 | **C# .NET 8 NativeAOT** 或 **Rust** | 单文件、无运行时依赖、内存小、开发快；Rust 更适合极致占用 |
| HID | `hidapi`（C）或 WinUSB + overlapped I/O | 与现有 Python 协议完全一致；Windows 原生异步 |
| THX 桥 | **独立 `ThxHost.exe`（C++/C# P/Invoke）** | THX DLL 是 C++/COM 风格，崩溃隔离；只在用户打开音频页时启动 |
| UI | WinUI 3 / WPF，或 **WebView2 按需加载** | 不用 Electron；UI 退出后服务仍在 |
| IPC | Named Pipe + JSON/MessagePack | 简单、低延迟、无网络端口 |
| 配置 | JSON（`%ProgramData%`）或 SQLite | 无云依赖 |
| 安装 | 单 exe + 可选 MSI；服务自启动 | 不装 Chromium |

> 如果不想碰 THX 的 C++ ABI，可以先用 **C# + P/Invoke** 调 C 导出；`ThxV3Native` 的导出是 `extern "C"` 风格（`DeviceInitialize`、`SetRenderNormalization` 等），比 C++ 类好调。

### 2.2 HID Worker（最关键）

- 打开 `0xFF14` collection，`read()` **阻塞等待** input report；不要 1 ms 轮询。
- 一个设备一个 reader 线程（或 IOCP）；事件解析后丢给 Event Bus。
- 命令走队列，单线程串行，带 `seq`/checksum/超时/重试；避免并发写。
- 只在你需要时写；没有 UI 操作时设备侧几乎零流量。
- 断线重连：监听 `WM_DEVICECHANGE` 或定时（5 s）枚举，不要在游戏时高频枚举。
- 日志：生产环境只保留环形缓冲；调试时再开 hex dump。

### 2.3 THX 桥（可选）

- **按需启动**：用户打开“麦克风/增强”页才启动 `ThxHost.exe`；关闭页面 30 s 后退出。
- 一次初始化后缓存 `containerId`、设备列表、预设列表；不要每次操作重新 `DeviceInitialize`。
- 批量写：EQ/预设用 `SetRenderBatch(true)` → 多次设置 → `SetRenderBatch(false)`。
- THX APO 本身在音频引擎里，**不占你的进程 CPU**；你的进程只做配置。
- 如果 THX 不可用（未安装/被卸载），降级为“仅 HID 功能”，UI 灰掉麦克风增强。

### 2.4 UI

- 托盘菜单 + 按需窗口；窗口关闭后服务继续。
- 不做游戏内 overlay，不注入游戏进程，不挂全局钩子。
- 需要显示电量/连接状态时，用服务推送的事件，不要 UI 自己轮询。

### 2.5 低占用/游戏友好设置

- 进程优先级 `BELOW_NORMAL_PRIORITY_CLASS`；Windows 11 可加 **EcoQoS**。
- 不做 CPU 亲和性绑定（没必要，反而影响调度）。
- 不使用 `SetWindowsHookEx`、不做 D3D overlay、不读游戏内存。
- HID 读是阻塞的，空闲时线程在等待，CPU ≈ 0。
- 音频处理在 `audiodg.exe`（THX APO），你的服务不持有音频流。
- 避免把日志写到游戏所在磁盘的高频路径；用内存环形缓冲 + 异步落盘。

## 3. 建议的实现顺序

### Phase 1：纯 HID MVP（1–2 天）

- 枚举/连接、电量、充电、自动关机、固件/序列号。
- 扬声器 EQ：`0x93` 预设 + `0x95` 10 段。
- 侧音：`0x98` + `0x99`（需要补 0–100 → 0–15 映射）。
- 托盘 UI + Named Pipe；无 THX 依赖。

### Phase 2：THX 只读/配置桥（2–4 天）

- 用 C 导出做 `DeviceInitialize` → `GetRenderDeviceList` → `GetRenderNormalization` 只读验证。
- 再做 `SetRenderNormalization` / `Level`、`SetRenderPreset`、`SetRenderEQGains`。
- 麦克风：`SetCaptureVoiceGate`、`SetCaptureNoiseReduction`、`SetCaptureNormalization`、`SetCaptureVocalClarity`、`SetCaptureEQGains`、`SetCapturePreview`、`SetStartCaptureStatus`。

### Phase 3：产品化

- 配置导入/导出、预设、热键、自动切换、开机自启。
- 崩溃恢复、设备热插拔、单实例、日志/诊断。
- 安装包/签名/自动更新（可选）。

## 4. 为什么不需要内核驱动

- 耳机控制是标准 HID output/input report，用户态 `hidapi`/WinUSB 足够。
- 只有“虚拟音频设备/自定义 APO”才需要内核/音频驱动；这是另一个量级的项目。
- 麦克风降噪/噪声门已经由 THX APO 实现，直接复用比自研 DSP 更稳。

## 5. 关键取舍

| 选择 | 代价 |
|---|---|
| 纯 HID，不碰 THX | 麦克风 EQ/降噪/噪声门/标准化/亮丽人声做不了 |
| 调 THX C 导出 | 需要确认 ABI；THX 升级可能变；要处理未安装 |
| 自研 APO 做降噪 | 工作量大、驱动签名/稳定性风险高 |
| Electron/Web UI | 开发快，但内存/CPU/启动明显高于原生 |
| 原生服务 + 按需 UI | 首版开发稍慢，但长期占用最低 |

## 6. 结论

- **原来**：Electron 多进程 + 远程 Web + native HID + THX FFI 子进程 + APO，功能全但重。
- **建议**：**原生后台服务（C#/Rust）+ HID 阻塞读 + 按需 THX 桥 + 轻量 UI**。
- 先做纯 HID MVP（电量/固件/侧音/扬声器 EQ），再把 THX 作为可选模块；这样即使 THX 不可用，核心控制仍然可用，且玩游戏时几乎零开销。
