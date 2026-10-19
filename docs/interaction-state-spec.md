# 交互状态规格（Interaction State Spec）

> 目的：结束“按截图逐个打补丁”的状态逻辑。先定义唯一的设备/界面状态模型，再改代码。
> 状态：Draft 2，待确认后按 TDD 逐条实现。
>
> 2026-10-19 决策：`no_receiver` / `headset_off` 使用**全屏遮罩 + 背景虚化 + 居中卡片**，
> 不再使用页面内联提示条；背景页面进入 `inert`，视觉上完全被遮住。

## 0. 责任与范围

这是实现问题，不是用户操作问题。之前缺少统一状态模型，导致：

- THX 可用但耳机未开机时，UI 仍显示“THX 就绪”，却又允许操作；
- 同一时刻可能同时出现输出设备弹窗、电源锁提示、普通错误条；
- UI 锁了，但某些 Tauri 写命令没有统一检查；
- 默认 EQ 的写入条件和“设备在线”条件分散在不同层。

本规格只约束**设备状态 → 可用性 → 可写性 → 提示层**，不重新设计视觉风格。

## 1. 单一状态模型

界面最终只依赖一个 `UiState`。`UiState` 是纯函数，输入是快照与 THX 快照，输出是唯一状态、提示级别和可写性。

```text
UiState = derive(snapshot, thx)
```

输入：

| 输入 | 含义 | 来源 |
|---|---|---|
| `receiverPresent` | 能打开 Razer HID 接口；无线接收器/有线设备存在 | `snapshot.connected` / `link` |
| `headsetOnline` | 无线：battery 或 charging 命令有响应；有线：接口可用 | `snapshot.connected` + `link` |
| `link` | `disconnected / standby / wireless / wired / unknown` | `snapshot.link` |
| `audioIsHeadset` | 当前 Windows 默认输出是 Razer 耳机 | `snapshot.audioOutput.isHeadset` |
| `thxInstalled` | 本机 THX DLL/端点可用 | `thx.installed` |
| `thxReady` | THX 已绑定当前耳机端点，可写入 | `thx.installed && thx.endpoint` |
| `previewActive` | 麦克风 preview helper 正在运行 | `capture.preview` / UI 本地确认 |
| `busy` | 有一个写操作正在进行 | store `busy` |

## 2. 状态与优先级

同一时刻只能有一个主状态。优先级从上到下：

| 优先级 | 状态 | 条件 | 主交互 |
|---|---|---|---|
| 1 | `booting` | 首次快照尚未返回 | 全页 loading；不写 |
| 2 | `no_receiver` | 无法打开 HID 接口 | 全屏虚化遮罩：未检测到耳机/接收器 |
| 3 | `headset_off` | 接口存在，但 `headsetOnline == false` | 全屏虚化遮罩：耳机未开机 |
| 4 | `output_wrong` | 耳机在线，但 `audioIsHeadset == false` | 输出设备阻断弹窗 |
| 5 | `thx_unavailable` | 耳机在线、输出正确，THX 不完整 | 页面可看，THX 区域禁用 |
| 6 | `ready` | 以上全部满足 | 全部可用 |
| 7 | `busy` | 写操作进行中 | 控件禁用，但不改变主状态 |

规则：

- `headset_off` 优先于 `output_wrong`：耳机没开机时，不弹输出设备窗口。
- `no_receiver` 和 `headset_off` 使用**同一个锁定组件**，只是文案不同。
- `thxReady` 与 `headsetOnline` 是两个独立维度；“THX 就绪”不能推导成“设备在线”。
- `busy` 不是主状态，不能覆盖 `headset_off`。

## 3. 状态转换

| 事件 | 结果 |
|---|---|
| 启动，快照未返回 | `booting` |
| 快照返回 `link=standby` | `headset_off` |
| 快照返回 `link=disconnected` | `no_receiver` |
| 快照返回在线，输出不是耳机 | `output_wrong` |
| 快照返回在线，输出是耳机，THX 不可用 | `thx_unavailable` |
| 快照返回在线，输出是耳机，THX 可用 | `ready` |
| 耳机开机，握手成功 | 按输出/THX 进入 `output_wrong` / `thx_unavailable` / `ready` |
| 耳机在设置过程中关机 | 下一次刷新进入 `headset_off`，并取消/拒绝排队写 |
| 处于 `headset_off`，应用在前台 | 每 5 s 自动重连；开机后自动解除锁定 |
| 处于 `headset_off`，应用失焦/最小化/托盘 | 暂停自动重连；恢复前台时立即检查一次 |
| 用户在锁定态点“重新检测” | 只刷新快照；不执行写 |
| 用户切换默认输出成功 | 重新 `derive`；可能从 `output_wrong` 到 `ready` |

## 4. 每个状态的行为契约

### `booting`

- 显示 loading；页面内容不可写。
- 不写默认 EQ、不写声音标准化。
- 不显示“THX 未安装”这种结论，直到 THX 状态返回。

### `no_receiver` / `headset_off`

- 顶部状态条：红点，`耳机未开机` / `未检测到耳机`。
- 全屏遮罩 + 背景虚化 + 居中卡片，背景页面进入 `inert`。
- 卡片只提供「退出」；不提供手动「重新检测」/设置入口。
- 不显示输出设备弹窗；同一时刻只有一个阻塞层。
- 所有设置写操作禁止：
  - HID：EQ、侧音、EQ 预设、设备开关
  - THX：空间音效、声音标准化、麦克风、preview
- **默认 EQ 不写**；声音标准化默认值也不写。
- 前台且可见时每 5 秒自动做一次 HID 握手；耳机开机后自动解除锁定。
- 应用失焦、最小化、隐藏到托盘或 `document.hidden` 时**停止自动重连**，不产生后台 HID 轮询。
- 窗口重新获得焦点/可见时立即补一次检查。
- 如果 preview 正在运行：停止 preview，或卡片提供「停止测试」；不能把停止按钮也锁死。

### `output_wrong`

- 使用阻断弹窗，提供「设为默认输出」「打开声音设置」「退出」。
- 页面其余控件不可操作。
- 如果耳机同时关机，优先显示 `headset_off`，不显示此弹窗。

### `thx_unavailable`

- 页面可浏览；HID 设置仍可用。
- THX 相关区域显示禁用与原因，不显示“THX 就绪”。
- 允许 HID 写入（EQ、侧音）如果耳机在线。

### `ready`

- 全部控件可用。
- 默认 EQ 只在**首次进入 ready**时写一次。
- 用户手动改 EQ 后，不因为刷新覆盖用户值。

## 5. 写操作契约

UI 的 `inert` 只是体验层，**不是权限边界**。所有设置写命令必须在 Tauri 边界再次检查：

```text
require_online():
  no HID transport        -> reject
  wireless + battery/charging timeout -> reject
  wired available         -> allow
```

被拒绝时返回统一错误：

```text
耳机未开机，设置已锁定；请先打开耳机电源
```

以下命令必须经过检查：

- `set_eq_preset`
- `set_eq_gains`
- `set_sidetone`
- `set_sidetone_volume`
- `set_normalization`
- `set_normalization_enabled`
- `set_spatial`
- `verify_thx`
- `mic_command`
- `start_mic_preview`

不检查：`get_status`、`get_thx_status`、`set_default_audio_output`、`open_sound_settings`、`quit`。

## 6. 默认写入契约

- 默认扬声器 EQ：仅当 `headsetOnline && audioIsHeadset && thxReady` 时写入一次。
- 声音标准化默认值：仅当耳机在线且标准化开关已开启时应用；关机状态绝不写。
- 麦克风默认值：不自动写；只由用户操作触发。
- 每次进入 `ready` 后最多执行一次自动写；失败后允许下一次显式刷新重试。
- 自动写不能阻塞首屏；必须显示“默认 EQ 已写入”或明确失败。

## 7. 测试矩阵（先确认 seam，再写测试）

Seam：

1. `derive_ui_state(snapshot, thx) -> UiState`（纯函数）
2. `SailApp::require_online() -> Result<(), SailError>`
3. Tauri 写命令边界
4. `device.applyDefaultEqOnce()` 的调用条件

验收用例：

| # | 场景 | 期望 |
|---|---|---|
| 1 | 启动中，无快照 | booting；无写 |
| 2 | 无接收器 | no_receiver；锁定；无默认写 |
| 3 | 接收器在，耳机未开机 | headset_off；全屏虚化遮罩；无默认写 |
| 4 | 耳机未开机，点 EQ 预设 | 遮罩阻止点击；后端若收到写则拒绝 |
| 5 | 耳机未开机，启动应用 | 不出现“默认 EQ 已写入” |
| 6 | 耳机开机，输出不是耳机 | output_wrong；只显示输出弹窗 |
| 7 | 耳机开机，输出正确，THX 可用 | ready；默认 EQ 写一次 |
| 8 | 耳机使用中关机 | 下一次刷新锁定；排队写被拒绝 |
| 9 | 耳机重新开机 | 前台 5 s 内自动解锁；失焦时不轮询，回到前台立即检查 |
| 10 | 耳机未开机，preview 仍在 | preview 被停止，或锁定条可停止 |
| 11 | THX 未安装但耳机在线 | thx_unavailable；HID 仍可用 |
| 12 | 锁定态 | 没有手动重新检测按钮；只等待自动恢复或退出 |

## 8. 实施顺序

1. 新增 `derive_ui_state` 纯函数与单元测试（先红后绿）。
2. UI 只消费 `UiState`，删除分散的 `link === ...` 判断。
3. 统一锁定组件（`no_receiver` / `headset_off`）。
4. 输出弹窗只在 `output_wrong` 渲染。
5. Tauri 写边界统一 `require_online`。
6. 实机验证：开机/关机/拔接收器/输出切换/THX 缺失。

## 9. 非目标

- 不把 THX DLL 并入 UI 进程。
- 不改变现有 THX capture 绑定逻辑。
- 不重新设计视觉主题。

## 10. 实现状态

- `ui/src/lib/ui-state.ts`：已实现 `deriveUiState`，页面、电源遮罩、输出弹窗、状态条统一消费。
- `ui/src/lib/ui-state.test.ts`：12 条 Node 测试，覆盖分类优先级、`canWrite`、`wiredNotReady` 和前台重连策略。
- `HeadsetPowerGate`：全屏虚化遮罩，只有「退出」；前台每 5 秒静默检测，失焦/隐藏不轮询。
- `StatusStrip`：THX 徽标由 `UiState.thxReady` 推导，不再单看 `installed && audioOk`。
- 已实测输出矩阵：音响 → `output_wrong` 阻断窗 → 点「设为默认输出」→ 耳机 → `ready`，THX 徽标同步恢复。
- 已实测 `headset_off`：全屏遮罩、只有「退出」、显示「正在等待耳机开机…」、设置控件不可用，且没有默认 EQ 写入。
- 已实测后端写保护：关机状态下直接调用 `set_eq_preset` / `set_normalization` 均返回「耳机未开机，设置已锁定」。
- 已实测关机→开机：无需点击，电源遮罩自动消失；因 Windows 把默认输出切回音响，随后进入 `output_wrong`，点「设为默认输出」后恢复 `ready`、THX 就绪、默认 EQ 已写入。
- 已实测拔接收器：`no_receiver`，遮罩标题「耳机未连接」，接收器显示「未连接」，设置写入锁定，只有「退出」。
- 后端写命令：已统一 `require_online`。
- 尚待真实硬件验证：开机自动解锁、拔接收器、失焦后台不轮询；debug 稳定前不发布。
