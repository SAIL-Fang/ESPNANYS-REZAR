# 多型号耳机抽象设计

> 状态：Draft 1。当前只支持 Razer BlackShark V2 HyperSpeed；本文定义后续适配其它型号时的 seam，不改变现有行为。
> 目标：把“协议/传输”和“型号能力”分开，避免 UI、THX、EQ 和 Razer 常量继续耦在一起。

## 0. 当前耦合点

| 位置 | 现状 | 问题 |
|---|---|---|
| `sail_hid` | `RAZER_VID`、`PID_WIRELESS`、`PID_WIRED`、usage page/usage、interface 3 都是常量 | 新型号无法只靠配置加入 |
| `sail_app` | 状态字段、EQ 10 段、预设、默认值、THX 容器假设固定 | 型号差异会散落在 app 编排里 |
| `sail_thx_bridge` | 查找固定的 BlackShark THX 容器 | 其它型号的 THX endpoint/container 不同 |
| `sail_ui` | “BlackShark V2 HS”、EQ 段数、侧音范围等写死 | 型号一变，UI 文案和控件会错 |
| `sail_cli` | 命令按当前型号的字段输出 | 诊断命令需要按能力降级 |

## 1. 目标 seam

```text
sail_core
  ├─ 64-byte 帧、checksum、通用命令原语
  └─ 不包含 VID/PID、产品名、EQ 段数

sail_hid
  ├─ HidSelector / DeviceProfile
  │    VID/PID、usage page/usage、interface、wired/wireless
  └─ 只负责枚举与打开 transport

sail_app
  ├─ HeadsetProfile（型号适配器）
  │    status / eq / sidetone / events / capabilities
  ├─ SailApp
  │    组合 profile + ThxProfile
  └─ DeviceSnapshot
       增加 model/capabilities，UI 不再猜型号

sail_thx_bridge
  └─ ThxProfile
       endpoint/container 解析、capability 探测
```

### 1.1 `HeadsetProfile` 接口草案

```rust
pub trait HeadsetProfile: Send {
    fn descriptor(&self) -> &ModelDescriptor;
    fn read_status(&mut self) -> Result<ModelStatus, Error>;
    fn set_speaker_eq(&mut self, gains: &[i8]) -> Result<(), Error>;
    fn set_eq_preset(&mut self, preset: EqPresetId) -> Result<(), Error>;
    fn set_sidetone(&mut self, enabled: bool, volume: Option<u8>) -> Result<(), Error>;
    fn capabilities(&self) -> Capabilities;
}
```

`ModelDescriptor`：

```rust
pub struct ModelDescriptor {
    pub id: &'static str,              // razer.blackshark_v2_hyperspeed
    pub display_name: &'static str,
    pub vendor_id: u16,
    pub wireless_pid: Option<u16>,
    pub wired_pid: Option<u16>,
    pub eq_bands: &'static [u16],
    pub sidetone_steps: Option<u8>,
    pub has_battery: bool,
    pub has_charging: bool,
    pub has_auto_power_off: bool,
}
```

`Capabilities` 控制 UI，而不是 UI 自己判断 PID：

```text
battery / charging / firmware / serial / auto_off / dongle_led /
speaker_eq / sidetone / thx / mic_capture
```

### 1.2 THX 也要按型号

`ThxProfile` 负责：

- 查找对应型号的 THX container/endpoint
- 判断 normalization/capture 是否可用
- 解析 helper 的型号相关字段

THX 不可用时，`UiState.thxReady = false`，但 HID 能力仍由 `HeadsetProfile` 决定。

## 2. UI 契约

`DeviceSnapshot` 增加：

```text
model: { id, displayName, capabilities }
```

UI 只消费：

- `model.displayName`：侧栏、标题、状态文案
- `capabilities.*`：显示/隐藏或禁用对应控件
- `eqBands`：EQ 横轴
- `uiState`：锁定/输出/THX 状态

禁止：

- 在 Svelte 里写 `BlackShark`、`0x0565`、`10 段`、侧音 `0..15`
- 用 PID 决定是否显示某功能
- 用一个全局 `connected` 推导所有能力

## 3. 型号注册与发现

```text
sail_app::profiles::all() -> &'static [&'static dyn ProfileFactory]
```

启动时：

1. 枚举 HID
2. 用每个 profile 的 selector 匹配
3. 选中唯一 profile
4. 打开 transport
5. 生成 `ModelDescriptor` + `HeadsetProfile`
6. UI/CLI 只读统一 snapshot

匹配冲突（多个 profile 同时匹配）必须显式失败，不做静默猜测。

## 4. 迁移顺序

1. **不改变行为**：把当前 Razer 常量集中到 `RazerBlackSharkV2HyperspeedProfile`，仍由现有代码调用。
2. `sail_hid` 接受 `DeviceProfile` selector，不再直接读常量。
3. `sail_app` 把 `DeviceSnapshot` 增加 `model`，但字段值仍与现在一致。
4. UI 改为读 `model.displayName` / `capabilities` / `eqBands`，去掉硬编码文案。
5. THX 查找改为 `ThxProfile`，按型号解析 container。
6. 新增第二个型号时只增加 profile + tests，不改 UI 主体。

## 5. 测试 seam

- `profile::match_hid(infos) -> ModelDescriptor` 纯函数测试
- `HeadsetProfile` 的协议编解码使用 fixture，不依赖真机
- `DeviceSnapshot` 的 model/capabilities 序列化测试
- `deriveUiState` 继续用纯输入测试，不引入型号分支
- 真机矩阵按型号复用：开关机、接收器拔插、输出切换

## 6. 非目标

- 现在不实现第二个型号
- 不把 Razer/THX 二进制打包
- 不为了“通用”而抽象掉当前协议细节；先把型号边界收干净

## 7. 已落地（第一步）

- `sail_hid::model`：
  - `ModelDescriptor`
  - `ModelCapabilities`
  - `RAZER_BLACKSHARK_V2_HYPERSPEED`
  - `UNKNOWN_MODEL`
  - `model_for_product_id`
- HID selector 改为通过 descriptor 匹配 VID/PID/usage/interface，不再直接读 Razer 常量。
- `DeviceSnapshot.model` 已加入；CLI 输出 model，UI 侧栏使用 `model.displayName`，不再写死 “BlackShark V2 HS”。
- 尚未落地：`HeadsetProfile` trait、按型号命令映射、THX `ThxProfile`、UI capability 驱动。
- 第二步已落地：`sidetone_max` 进入 descriptor；`deriveUiState` 读取 `capabilities.thx`；presentation 输出 `THX 不支持`；Sound/Mic panel 读取 `eqBands`、`sidetoneMax` 和 capability；UI 不再固定 0–15 或 10 段文案。
- 仍明确推迟：行为型 `HeadsetProfile` trait、按型号命令映射、THX profile；见 `docs/adr/0004-model-descriptor-scope.md`。
