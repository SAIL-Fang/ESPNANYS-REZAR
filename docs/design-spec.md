# SAIL 设计文档

> 按图片里的三段式流程出：**定方向（ui-ux-pro-max）→ 定基调（frontend-design）→ 评审打磨（impeccable）**。  
> 设计系统最终源在 [DESIGN.md](../DESIGN.md)；`design-system/sail/MASTER.md` 只是工具原始输出，已被本文件覆盖。

## 0. 决策摘要

| 决策 | 结论 |
|---|---|
| HID 设备控制 | **自己实现**，`sail_hid` / `sail_core`，不调用 Razer 服务 |
| 麦克风增强 / 声音标准化 | **复用已安装的 THX APO**，由 `sail_thx_bridge` 控制；不重写 DSP |
| THX 是否打包进仓库 | **默认不打包**；只做检测 + 官方安装引导；可选“用户主动下载官方安装包” |
| 前端 | **Tauri 2 + Svelte 5 + TypeScript**（已确认） |
| 声音标准化范围 | UI **1–100**；THX 层按 0–100 double 处理，回读校正 |
| 实施顺序 | 按你确认的：HID Core → THX Host → Tauri UI → Productization |
| 设计模式 | **Operate**（工具/控制台），不是营销页 |

## 1. 产品定义

**SAIL** 是一个低占用的 Windows 耳机控制台，面向不想常开 Synapse 的 PC 玩家/主播。

核心任务：

1. 看设备状态：连接、电量、固件、当前 EQ/侧音。
2. 调扬声器 EQ：游戏/音乐/电影/自定义，10 段。
3. 开关声音标准化，并调到 **1–100**（Synapse 只能 10–100）。
4. 调麦克风：噪声门、降噪、音量标准化、亮丽人声、麦克风 EQ、侧音、测试。
5. 切配置、开机启动、最小化到托盘。

非目标：

- RGB/灯效、游戏内 overlay、THX 空间/房间深度定制、固件刷新、虚拟 7.1 驱动。
- 不把 THX DLL/安装包放进 MIT/Apache 仓库。

## 2. 归属与命名：哪些是 Razer 的，哪些是 `sail_`

### 2.1 命名规则

- `razer_*`：Razer / THX 自带的运行时、固件、DLL、APO；**不是我们的代码，默认不打包**。
- `sail_*`：我们自己的代码、模块、UI 组件、数据类型。
- 设备固件本身不叫 `sail_*`，也不能被我们修改。

### 2.2 功能归属表

| 功能 | 实际执行者 | 我们的模块 | 是否打包 |
|---|---|---|---|
| HID 64 字节协议 | `razer_device_firmware`（耳机/接收器） | `sail_hid` | 否 |
| 电量/固件/序列号 | `razer_device_firmware` | `sail_hid` 读取 | 否 |
| 扬声器 EQ（10 段） | `razer_device_dsp`（耳机硬件） | `sail_eq` 写 `0x93/0x95` | 否 |
| 侧音硬件开关/音量 | `razer_device_dsp` | `sail_mic` 写 `0x98/0x99` | 否 |
| 声音标准化（DRC） | `razer_thx_apo`（THX） | `sail_normalization` → `sail_thx_bridge` | 否 |
| 噪声门 | `razer_thx_apo` | `sail_mic` | 否 |
| 麦克风降噪 | `razer_thx_apo` | `sail_mic` | 否 |
| 音量标准化（麦克风） | `razer_thx_apo` | `sail_mic` | 否 |
| 亮丽人声 | `razer_thx_apo` | `sail_mic` | 否 |
| 麦克风 EQ | `razer_thx_apo` | `sail_mic` | 否 |
| 麦克风测试 | `razer_thx_apo` | `sail_mic` | 否 |
| THX 空间/房间/31 段 | `razer_thx_apo` | 暂不做 | 否 |
| 主 UI | 我们 | `sail_ui` | 是 |
| 托盘/配置/诊断 | 我们 | `sail_tray` / `sail_config` / `sail_diagnostics` | 是 |
| THX 安装引导/下载器 | 我们（只下官方包） | `sail_thx_bootstrap` | 是（代码） |
| THX DLL / APO 二进制 | THX / Razer | 不放进仓库 | **否** |

### 2.3 模块命名

```text
sail_core           # 协议帧、seq、checksum、状态机（无 UI）
sail_hid            # HID 枚举/读写/事件
sail_device         # 设备门面
sail_eq             # 扬声器 EQ 预设/10 段
sail_normalization  # 声音标准化 1–100 领域逻辑
sail_mic            # 麦克风增强编排（调用 THX）
sail_thx_bridge     # THX Host 的 IPC 客户端
sail_thx_bootstrap  # 检测/引导/可选官方下载
sail_config         # 配置 schema/持久化
sail_tray           # 托盘/开机启动
sail_ui             # Tauri + Svelte
sail_design         # tokens + 组件
sail_cli            # 命令行
sail_diagnostics    # 日志/导出/恢复
```

UI 组件也用 `sail_`：`sail_status_strip`、`sail_eq_curve`、`sail_band_slider`、`sail_toggle_row`、`sail_thx_badge`、`sail_missing_thx`。

## 3. THX：能不能打包？下载方式怎么处理？

### 3.1 结论

**默认不打包。** 原因不是技术，是法律 + 工程：

- `razer_thx_apo` / `ThxV3Native.dll` 是 THX/Razer 的**专有签名二进制**，不是 MIT/Apache 可再分发代码。
- APO 不是单个 DLL：还有 INF、驱动包、签名、按设备（0565/056E）和版本安装的注册表；静默安装需要管理员权限，可能要求重启。
- 打包后你要承担更新、签名、兼容性、安全责任；THX 版本一变，行为就可能变。
- GitHub 开源仓库若包含这些二进制，会破坏“纯开源”的许可边界。

### 3.2 推荐做法：检测 + 引导，不重打包

```text
sail_thx_bridge.detect()
  ├─ 查 HKCU/HKLM THX APO 安装键
  ├─ 查 ThxV3Native DLL 是否存在
  ├─ 查设备 0565/056E 对应的 APO 版本
  └─ 返回 sail_thx_capabilities { installed, version, micFeatures, normalization }
```

- 已安装：直接复用，UI 显示 `THX 就绪`。
- 未安装：显示 `sail_missing_thx` 面板：
  - 「打开官方下载页」→ 打开 Razer 官方支持页/Synapse 下载页；
  - 「我已安装，重新检测」；
  - 不静默下载、不静默安装。
- 可选 `sail_thx_bootstrap`：用户点击后，**下载官方安装包**（不是我们仓库里的二进制），校验 Authenticode 签名，再让用户运行安装。默认构建可关闭此功能。

### 3.3 “自动匹配驱动下载”的逆向

技术上可以继续挖 Razer 安装器的下载清单/版本匹配（设备 PID、APO 版本、INF），但建议：

- 只做**官方安装包**的下载/校验/引导，不做重打包、不绕过授权；
- 下载前明确告知来源、版本、大小、签名者；
- 不做静默安装；
- 先做法律/ToS 评估，再决定是否进默认构建；
- 如果要做“免安装体验”，更稳的是**检测用户已装的 Synapse/THX**，而不是自己分发。

### 3.4 如果一定要打包

只有一条合法路径：**拿到 THX/Razer 的书面再分发许可**，并做独立的、非 MIT 的分发渠道；不要放在开源仓库的默认构建里。

## 4. 设计流程

### 4.1 定方向：ui-ux-pro-max

工具原始输出（`design-system/sail/MASTER.md`）：

- 建议 Dark Mode (OLED)、密度 8、Inter；
- 但同时给了 “Product Demo + Features” 营销页结构、3D/Hyperrealism、霓虹紫 + 亮绿。

**采纳**：暗色、低光、密度高、技术感、低性能成本。  
**否决**：营销页结构、3D 大图、紫/绿霓虹、scroll reveal。理由：这是 Operate 工具，不是游戏宣传页；紫/绿也容易落回“AI 默认”或 Razer 的视觉。

最终方向：**录音棚机架 / 广播调音台**，信息密度高、刻度精确、材质是金属和纸面，不是 RGB。

### 4.2 定基调：frontend-design

- **主体**：音频硬件控制台；用户在工作/游戏前快速调音，不需要“酷炫”。
- **一个记忆点**：顶部 signal strip（连接/电量/THX）+ 主区域 EQ 仪表。
- **字体**：IBM Plex Sans + Noto Sans SC；数字 IBM Plex Mono。拒绝 Russo One/Chakra Petch 的电竞海报感。
- **颜色**：石墨黑 + 琥珀（当前值/主操作）+ 冰青（测量）+ 信号红（错误）。**不用 Razer 绿**。
- **布局**：左侧 rail + 顶部状态条 + 连续面板；不是卡片汤。
- **文案**：sentence case、动词开头、说明“会发生什么”；不写“Submit”。

### 4.3 评审打磨：impeccable

按 **Operate** 模式评审：

- 可扫读、可预测、native 期望优先；品牌只放在精确细节里。
- 一个大胆元素（EQ/信号条），其余安静。
- 每个控件有 hover/focus/disabled/pending/error 状态。
- 不用 emoji 图标；Lucide SVG。
- 键盘全可达；焦点不被 sticky 元素遮挡。
- 动效只表达状态变化；尊重 `prefers-reduced-motion`。
- 交付前按第 8 节验收。

## 5. 信息架构

```text
SAIL
├─ 设备
│  ├─ 连接/电量/固件/序列号
│  ├─ 自动关机 / Dongle LED
│  └─ 重新连接 / 诊断
├─ 声音
│  ├─ EQ 预设（游戏/音乐/电影/自定义）
│  ├─ 10 段 EQ
│  ├─ 声音标准化 1–100
│  └─ 低音增强 / 人声清晰度
├─ 麦克风
│  ├─ 麦克风音量
│  ├─ 噪声门
│  ├─ 降噪 / 音量标准化 / 亮丽人声
│  ├─ 麦克风 EQ
│  ├─ 侧音
│  └─ 麦克风测试
└─ 设置
   ├─ 开机启动 / 托盘 / 语言
   ├─ THX 状态 / 安装引导
   └─ 日志/导出/恢复默认
```

## 6. 页面规格

### 6.1 主壳

```text
┌─────────────────────────────────────────────────────────────┐
│ SAIL   ● 已连接   ▮▮▮▮▯ 78%   THX 就绪        ⚙           │ 56
├──────┬──────────────────────────────────────────────────────┤
│ 设备 │ 声音                                   [应用/回读]   │
│ 声音 │                                                      │
│ 麦克风│  ┌─────────┬──────────────────────┬──────────────┐  │
│ 设置 │  │ 预设     │ 10 段 EQ 仪表        │ 标准化 1–100 │  │
│      │  │ 游戏     │                      │ 低音增强      │  │
│      │  │ 音乐     │                      │ 人声清晰度    │  │
│      │  └─────────┴──────────────────────┴──────────────┘  │
└──────┴──────────────────────────────────────────────────────┘
```

- 顶部 status strip：连接点、电量分段、THX badge、设置。
- 左下角显示版本/诊断入口；不放大段说明。
- 主内容最大宽度 1200；窗口缩放时保持 EQ 仪表优先。

### 6.2 声音页

- 预设：分段控件（`sail_preset_tabs`），不是 4 张卡片。
- EQ：`sail_eq_curve`（SVG）+ 10 个 `sail_band_slider`；右侧 +12/+6/0/-6/-12 刻度。
- 频段：31/63/125/250/500/1k/2k/4k/8k/16k；值用 mono。
- 声音标准化：开关 + 1–100 滑杆；显示“当前 10 / 范围 1–100”；不显示“仅 10–100”。
- 低音增强、人声清晰度：同一行式，开关 + 滑杆。
- 写入：拖动只改本地；`pointerup` 写设备；显示 `写入中 → 已回读`；失败回滚。

### 6.3 麦克风页

```text
┌───────────────────────┬───────────────────────┬──────────────────────┐
│ 输入                   │ 增强                   │ 麦克风 EQ            │
│ 音量 100               │ 噪声门 [ ] 阈值 1–5    │ 预设：默认/增强/直播 │
│ 电平表                 │ 降噪 [x] 60            │ 10 段仪表            │
│ 麦克风测试             │ 音量标准化 [ ]         │                      │
│ 侧音 [ ] 50            │ 亮丽人声 [ ]           │                      │
└───────────────────────┴───────────────────────┴──────────────────────┘
```

- 噪声门：开关 + 阈值滑杆；阈值显示 `1–5`，旁边小字 `-60…-20 dB`；不要只显示 dB 让用户困惑。
- 降噪：开关 + 0–100；默认开/60。
- 音量标准化、亮丽人声：开关 + 滑杆；未启用时滑杆 disabled。
- 麦克风 EQ：5 个预设（默认/麦克风增强/直播/会议/自定义）+ 10 段。
- 侧音：开关 + 0–100；显示“硬件侧音”小标签；与 THX 采集链分开说明。
- 麦克风测试：按住/点击开始；按钮变琥珀；显示电平；再点停止。
- 如果 THX 缺失：整块显示 `sail_missing_thx`，HID 音量/侧音仍可用。

### 6.4 设备页

- 固件、Edition、序列号、Dongle LED、自动关机。
- 只读项用 mono；自动关机用分段选择（15/30/60/关）。
- “重新连接”只重连 HID，不重启应用。
- 诊断：复制设备信息、导出日志；不显示原始 hex（debug 模式例外）。

### 6.5 设置页

- 开机启动、最小化到托盘、语言（简体中文/English）、主题（MVP 仅暗色）。
- THX 状态：`razer_thx_apo` 版本、设备匹配、`重新检测`、`打开官方下载页`。
- 危险区：恢复默认（需二次确认），不提供“卸载 THX”。

## 7. 数据契约（`sail_` 命名）

```ts
type sail_device_state = {
  connected: boolean;
  battery: number | null;       // 0..100
  charging: boolean;
  firmware: string | null;
  serial: string | null;
  autoPowerOffMin: number | null;
};

type sail_eq_state = {
  preset: "game" | "music" | "cinema" | "custom";
  gains: number[];              // 10, -12..12
  dirty: boolean;
};

type sail_thx_capabilities = {
  installed: boolean;
  version: string | null;
  normalization: boolean;
  mic: boolean;
};

type sail_normalization_state = {
  enabled: boolean;
  level: number;                // UI 1..100
};

type sail_mic_state = {
  volume: number;               // 0..100
  noiseGate: { enabled: boolean; level: number };       // 1..5
  noiseReduction: { enabled: boolean; level: number };  // 0..100
  normalization: { enabled: boolean; level: number };   // 0..100
  vocalClarity: { enabled: boolean; level: number };    // 0..100
  sidetone: { enabled: boolean; level: number };        // 0..100
};
```

UI 不直接调用 DLL；只通过 `sail_thx_bridge`。HID 只通过 `sail_hid`。

## 8. 验收 / 评审清单

### 8.1 功能

- [ ] 未装 THX 时，设备/扬声器 EQ/侧音仍可用；麦克风增强显示缺失面板。
- [ ] 声音标准化滑杆为 **1–100**；写入后回读一致；切换预设回到对应值。
- [ ] 噪声门 1–5 与 -60…-20 dB 同步显示；关闭后不重置。
- [ ] 降噪默认开/60；关闭后滑杆隐藏/禁用；重新打开保持 60。
- [ ] 麦克风 EQ 5 个预设值正确；自定义可编辑。
- [ ] 侧音 HID + THX 双写；关闭后 UI 与设备一致。
- [ ] 设备断开/重连不崩溃；写入失败有回滚。

### 8.2 设计（impeccable / Operate）

- [ ] 1180×760、1040×680、125% 缩放下无横向滚动、无遮挡。
- [ ] 焦点可见；键盘可完成所有滑杆/开关/预设；焦点不被 sticky 遮挡。
- [ ] 对比度 ≥ 4.5:1；状态不只靠颜色。
- [ ] 命中区 ≥ 44×44；无 emoji 图标。
- [ ] `prefers-reduced-motion` 下无动效。
- [ ] 只有一个记忆点（signal strip / EQ）；其余无装饰。
- [ ] 空/错误/缺失/ pending 状态都有文案和下一步。

## 9. 实施顺序（按你确认的）

### 阶段 1：HID Core（纯自研）

- `sail_core`：帧、seq、checksum、响应匹配。
- `sail_hid`：枚举 0xFF14、阻塞读、事件、重连。
- `sail_cli`：`status`、`battery`、`sidetone`、`eq-preset`、`eq-gains`。
- 先不做 UI；用 CLI 验证。

### 阶段 2：THX Host（复用 APO）

- `sail_thx_bridge`：先做 **独立 helper 进程** `sail_thx.exe`；UI 每次调用 spawn 一次，解析 stdout。
- 后续再升级成常驻 Named Pipe；MVP 不做常驻服务。
- `DeviceInitialize`、`GetRenderDevice`、`SetDefaultRenderDevice`、`Get/SetRenderNormalizationLevel`。
- `sail_normalization`：UI 1–100；回读；预设切换。
- 已验证 1–9 被 THX 接受：`verify` 写 1 → 回读 1 → 恢复原值。
- 再扩 `sail_mic`：噪声门/降噪/音量标准化/亮丽人声/麦克风 EQ/测试/侧音。

### 阶段 3：Tauri UI

- `sail_ui`：设备/声音/麦克风/设置四页。
- `sail_design`：tokens + 组件。
- 先只读，再写入；每个写入有回读。
- 声音标准化滑杆从第一天就是 1–100。

### 阶段 4：Productization

- `sail_tray`：托盘/开机启动/单实例。
- `sail_config`：schema/迁移/导入导出。
- `sail_thx_bootstrap`：检测/官方安装引导（可选下载）。
- `sail_diagnostics`：日志/崩溃恢复/设备信息导出。

## 10. 未决问题

1. ~~THX 1–9 是否实际生效~~：已用 `sail_thx_bridge` 验证：写入 1 → 回读 1 → 恢复 10。
2. THX 官方安装包是否有稳定公开 URL；`sail_thx_bootstrap` 是否进默认构建需法律确认。
3. 侧音 0–100 → HID 0–15 的完整映射（目前 50→7）。
4. 有线 0x056E 是否支持 HID 麦克风 EQ（`0x96`）；当前无线不支持。
5. MVP 是否只做暗色；浅色主题延后。

## 11. 实现状态（2026-10-18）

| 阶段 | 状态 | 证据 |
|---|---|---|
| 阶段 1 `sail_core` / `sail_hid` / `sail_cli` | ✅ | 真实设备读到电量/固件/序列号；mock 测试 6 个通过 |
| 阶段 2 `sail_thx_bridge` | ✅ | `sail_thx verify`：before 10 → set 1 → readback 1 → restore 10 |
| 阶段 3 `sail_ui` | ✅ 首版 | Tauri 2 + SvelteKit；声音页 1–100、EQ、设备/麦克风/设置 |
| 阶段 4 Productization | ⏳ | 托盘/配置/THX 安装引导/诊断未做 |

### 构建命令

```powershell
scripts\build.cmd -Stage
%LOCALAPPDATA%\SAIL\bin\sail_thx.exe verify
scripts\ui.cmd build --debug --no-bundle
scripts\run-ui.cmd
```

### 已知环境限制

- THX DLL 在很深的开发路径下加载会失败（栈溢出/access denied）；必须 staging 到短路径（`%LOCALAPPDATA%\SAIL\bin`）。
- Tauri 的 WebView2 在深层项目路径 + 提升权限下也可能 access denied；`run-ui.cmd` 会复制到短路径后启动。
- 生产安装到 `C:\Program Files\SAIL` 时路径短，不受影响。
