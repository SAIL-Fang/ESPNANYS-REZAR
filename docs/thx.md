# 内置 THX 的安装、配置与 API

本文覆盖 Synapse 4.0.827 在 **Razer BlackShark V2 HyperSpeed** 上使用的 THX Spatial Audio：它装在哪、配置存在哪、Synapse 调了哪些函数、以及“声音标准化”到底映射到什么。

## 1. 它是什么

THX 不是耳机固件里的 DSP，而是挂在耳机音频 endpoint 上的 **Windows APO（Audio Processing Object）**，由音频引擎在播放/录音链路里加载。Synapse 只是控制面：它通过 `ThxV3Native_v1.1.42.1.dll` 配置 APO，并把用户状态写进注册表。

```text
Synapse 页面
  → doDLLSubProcessActionAsync({ ffiGroup: "THXDLL_V3Common", action, payload })
  → THXDLL_V3CommonSubProcess
  → ThxV3Native_v1.1.42.1.dll
  → Windows 音频 APO（endpoint: Razer BlackShark V2 HS 2.4）
```

## 2. 安装位置

用户态 DLL：

```text
%LOCALAPPDATA%\Razer\RazerAppEngine\User Data\Apps\Common\thxv3common\
  ThxV3Native_v1.1.42.1.dll
```

APO 驱动/INF（每台设备单独安装）：

```text
HKLM\SOFTWARE\THX\THXSpatialAudio\
  THXUSBAPOInstaller-RazerBlackSharkV2HS-0565-3.2.3.0   (无线 0x0565)
  THXUSBAPOInstaller-RazerBlackSharkV2HS-056E-3.2.3.0   (有线 0x056E)
```

实测的 INF 映射：

| 键 | THXUSBAPO.inf | THXRTAPO.inf | THXRTSVC.inf | THXRTSCU.inf |
|---|---|---|---|---|
| 0565 | `oem49.inf` | `oem40.inf` | `oem45.inf` | `oem48.inf` |
| 056E | `oem46.inf` | `oem40.inf` | `oem45.inf` | `oem41.inf` |

安装包：

```text
C:\WINDOWS\Installer\Razer\THX-0565-BlackSharkV2HyperSpeed-329437\
  THXAPO-USB_BlackSharkV2HyperSpeed-0565-odessa-t2w_v3.2.3.0-a1194-mssigned_20240702-w27_i1036.exe

C:\WINDOWS\Installer\Razer\THX-056E-BlackSharkV2HyperSpeed-300515\
  THXAPO-USB_BlackSharkV2HyperSpeed-056e-odessa-t2w_v3.2.3.0-a1194-mssigned_20240702-w27_i1037.exe
```

版本 **3.2.3.0**，`odessa-t2w` 平台。完整注册表快照见 `captures/thx-install.json`。

## 3. 用户状态：`UserState`

```text
HKCU\Software\THX\SpatialAudio\UserState
```

每个值都是一个 JSON 字符串。命名规则：

```text
CurrentPreset-usb\1532\0565-VSHP
EndpointScope-usb\1532\0565-VSHP
Game Mode-1-Headphones-usb\1532\0565
Music Mode-1-Headphones-usb\1532\0565
Cinema Mode-1-Headphones-usb\1532\0565
Custom-1-Headphones-usb\1532\0565
UserScope
```

- `CurrentPreset-...`：当前预设的 **名字**（例如 `Game Mode-1-Headphones-usb\1532\0565`）。
- `EndpointScope-...`：当前 endpoint 的完整状态。
- `UserScope`：用户级设置（例如最后一次设备/音量）。
- `Game Mode-...` / `Music Mode-...`：每个预设自己的完整参数。

完整导出见 `captures/thx-userstate.json`。

### 字段说明

| 字段 | 含义 | Game Mode 实测 |
|---|---|---|
| `preset_name` | 预设名 | `Game Mode` |
| `spatial_enabled` | THX 空间音效开关 | `true` |
| `spatial_processing_mode` | 空间处理模式 | `Headphones5` |
| `eq_curve` | THX 自己的 31 段曲线 | 31 个数值 |
| `tilt` | 音色倾斜 | `0` |
| `drc_enabled` | **声音标准化开关** | `true` |
| `drc_level` | **声音标准化等级（0–100）** | `10` |
| `bass_boost_enabled` | 低音增强开关 | `false` |
| `bass_boost` | 低音增强值 | `55` |
| `dialog_enhancement_enabled` | 人声清晰度开关 | `false` |
| `dialog_enhancement` | 人声清晰度值 | `55` |
| `emitter_positions` | 5.1 各声道虚拟位置（L/R/C/LFE/Rs/Ls…） | 见 JSON |
| `room` | 房间类型/尺寸/反射率 | 见 JSON |
| `upmix` | 2.0→5.1 / 5.1→5.1.4 上混 | 见 JSON |
| `Headphones5.reverb_tail` | 混响尾音 | 见 JSON |

`Game Mode` 的 `drc_level` 是 **10**；`Music Mode`、`Cinema Mode`、`Custom`、`PUBG_cmm` 是 **100**。这解释了为什么切换预设时“声音标准化”滑杆会跳到不同值。

键名里的 `-1-` / `-0-` 对应空间音效开/关：`Game Mode-1-Headphones-...` 的 `spatial_enabled=true`，`Game Mode-0-Headphones-...` 的 `spatial_enabled=false`。每个预设会同时保存两个变体。汇总见 `captures/thx-presets-summary.json`。

## 4. “声音标准化” = THX DRC

Synapse 界面叫“声音标准化”，THX 内部叫 **DRC（Dynamic Range Control）**。

### 证据链

1. 页面加载时：

```text
AudioEffectsTHXV3.getRenderNormalization() r: true
AudioEffectsTHXV3.getRenderNormalizationLevel() r: 10
```

2. 关闭开关时，THX 日志：

```text
action: 'SetRenderNormalization'
payload: { actionArgs: [ '{FEC3BF0F-...}', false ] }
```

注册表 `UserScope` / `Game Mode-...` → `drc_enabled: false`。

3. 打开开关时：

```text
action: 'SetRenderNormalization'
payload: { actionArgs: [ '{FEC3BF0F-...}', true ] }
r: true
```

注册表 → `drc_enabled: true`。

4. 滑杆写入（历史日志）：

```text
action: 'SetRenderNormalizationLevel'
payload: { actionArgs: [ '{FEC3BF0F-...}', 10 ] }   → r: true
payload: { actionArgs: [ '{FEC3BF0F-...}', 100 ] }  → r: true
```

5. 同期 `protocol.log` 没有新增 1381 HID 报文 → **不走 HID**。

### 对应 API

| 层 | 函数 |
|---|---|
| Synapse FFI action | `SetRenderNormalization` / `SetRenderNormalizationLevel` |
| THX C 导出 | `GetRenderNormalization` / `SetRenderNormalization` / `GetRenderNormalizationLevel` / `SetRenderNormalizationLevel` |
| THX C++ 类 | `THXV3Client::SetDRCEnabled` / `SetDRCLevel` / `GetDRCEnabled` / `GetDRCLevel` |
| 注册表 | `drc_enabled` / `drc_level` |

## 5. Synapse 实际调用的 THX API

`captures/thx-actions.txt` 是去重后的 action 列表（51 个）。核心的：

| Action | 作用 | 典型参数 |
|---|---|---|
| `DeviceInitialize` | 初始化 THX 设备 | device/container |
| `DeviceTerminate` | 释放 | — |
| `GetDLLVersion` | 版本 | — |
| `GetRenderDevice` / `GetRenderDeviceList` | 找播放 endpoint | containerId |
| `SetRenderDevice` / `SetDefaultRenderDevice` | 选择 endpoint | containerId |
| `SetRenderBatch` | 打开/关闭 THX 处理链 | `enabled: 1/0` |
| `SetRenderPreset` | 切预设 | `presetData: "Game Mode"` |
| `SetRenderEQGains` | 10 段 EQ | `[1,-5,-3,0,1,1,1,1,1,-3]` |
| `GetRenderNormalization` / `SetRenderNormalization` | 声音标准化开关 | `true/false` |
| `GetRenderNormalizationLevel` / `SetRenderNormalizationLevel` | 声音标准化等级 | `0..100` |
| `GetRenderBassBoost` / `SetRenderBassBoost` | 低音增强 | `true/false` |
| `GetRenderBassBoostLevel` / `SetRenderBassBoostLevel` | 低音增强值 | `0..100` |
| `GetRenderVocalClarity` / `SetRenderVocalClarity` | 人声清晰度 | `true/false` |
| `GetRenderVocalClarityLevel` / `SetRenderVocalClarityLevel` | 人声清晰度值 | `0..100` |
| `GetRenderSpatialProcessing` / `SetRenderSpatialProcessing` | 空间音效 | `true/false` |
| `GetRenderPresetList` | 预设列表 | — |
| `GetRenderRoomList` / `GetRenderRoom` / `SetRenderRoom` | 房间 | — |
| `GetRenderTilt` / `SetRenderTilt` | 倾斜 | — |
| `SetCaptureSidetoneLevel` | 麦克风侧音电平 | 数值 |
| `SetCaptureNormalization` / `SetCaptureNormalizationLevel` | 麦克风标准化 | `true/false` / 0–100 |
| `SetCaptureEQGains` | 麦克风 10 段 EQ | 10 个 float |
| `SetCaptureVolume` / `SetCaptureMute` | 麦克风音量/静音 | — |
| `SetRenderVolume` / `SetRenderMute` | 播放音量/静音 | — |

> 注意：`SetRenderNormalization` 和 `SetCaptureNormalization` 是两条独立链路；界面上“声音标准化”是 **Render** 那条。
>
> 麦克风页的噪声门、降噪、音量标准化、亮丽人声、麦克风 EQ 的具体参数见 [audio-features.md](audio-features.md)。

### 5.1 页面层的 THX 调用与 1–100

从远程页面的缓存 JS 可读到页面自己的 THX 包装层：

```text
DeviceInitialize(containerId)                  -> bool
GetRenderNormalization(containerId)            -> bool
SetRenderNormalization(containerId, bool)      -> bool
GetRenderNormalizationLevel(containerId)       -> double
SetRenderNormalizationLevel(containerId, double) -> bool
```

以及 UI 数值转换：

```js
convertToTHXLevel(t) { return t / 100 * thx_setting_steps + thx_setting_min }
convertToUILevel(t) { return Math.round((t - thx_setting_min) / thx_setting_steps * 100) }
```

实测该设备 `10 → 10`、`54 → 54`、`100 → 100`，即 THX 层是 0–100 的 1:1。  
Synapse 滑杆最小值 **10** 是前端组件限制；直接调用 setter 传 `1` 返回 `true`。  
所以自己的上位机可以把范围做成 **1–100**（后端仍按 0–100 处理，并回读校正）。

> 注意：直接 `ctypes`/`ffi` 调用 DLL 需要正确的 `DeviceInitialize` + 设备选择；建议做一个独立的 `ThxHost` 进程，复用页面同样的调用序列，不要在主 UI 进程里直接加载 DLL。

## 6. THX DLL 导出（可实现的控制面）

`ThxV3Native_v1.1.42.1.dll` 共 **231 个导出**，完整列表在 `captures/thx-exports.txt`。命名的 C 导出（非 mangled）包括：

```text
DeviceInitialize
DeviceTerminate
GetDLLVersion

GetRenderDevice
GetRenderDeviceList
SetRenderDevice
SetDefaultRenderDevice
SetRenderBatch

GetRenderNormalization
SetRenderNormalization
GetRenderNormalizationLevel
SetRenderNormalizationLevel

GetRenderBassBoost
SetRenderBassBoost
GetRenderBassBoostLevel
SetRenderBassBoostLevel

GetRenderVocalClarity
SetRenderVocalClarity
GetRenderVocalClarityLevel
SetRenderVocalClarityLevel

GetRenderEQGains
SetRenderEQGains
ResetRenderEQGains
GetCurrentModeEQGains
SetCurrentModeEQGains

GetRenderPreset
SetRenderPreset
GetRenderPresetList
ResetRenderPreset

GetRenderRoom
SetRenderRoom
GetRenderRoomList
GetRenderCustomRoomType
SetRenderCustomRoomType

GetRenderSpatialProcessing
SetRenderSpatialProcessing
GetRenderTilt
SetRenderTilt

GetRenderVolume
SetRenderVolume
GetRenderMute
SetRenderMute

Get/SetCapture*（麦克风音量、静音、侧音、标准化、EQ、高通、噪声抑制、VoiceGate、VocalClarity）
```

同时导出了 `THXV3Client` C++ 类（mangled）：

```text
THXV3Client::SetDRCEnabled(int)
THXV3Client::SetDRCLevel(float)
THXV3Client::GetDRCEnabled(int&)
THXV3Client::GetDRCLevel(float&)
THXV3Client::SetPreset(std::wstring)
THXV3Client::SetRoom(...)
THXV3Client::SetSpatialProcessingState(int)
THXV3Client::SetCurrentModeEQGains(std::array<float,10>)
...
```

## 7. 自己实现时的三条路线

### 路线 A：直接调用 THX C 导出（最“正统”，但要确认 ABI）

优点：不依赖 Synapse；能控制 THX 的完整参数。  
风险：C 导出签名没有官方头文件。FFI 日志只给了 **参数形态**（containerId 字符串 + bool/int/float/数组），没有给 C 的精确类型（`wchar_t*`、`GUID`、回调、`std::array` 等）。需要：

1. 用 Ghidra/IDA 反编译 `DeviceInitialize`、`SetRenderNormalization` 等，确认参数宽度/调用约定。
2. 先做只读调用（`DeviceInitialize` → `GetRenderDeviceList` → `GetRenderNormalization`）。
3. 用 `ctypes`/C++ 小程序验证，再写配置。

导出地址见 `captures/thx-exports.txt`。

### 路线 B：读/写 `UserState` 注册表（最简单，但不稳）

- 读：解析 `HKCU\Software\THX\SpatialAudio\UserState` 的 JSON，拿到 `drc_enabled`、`drc_level`、`eq_curve`、`room`、`emitter_positions`。
- 写：改 JSON 后写回，再让 APO 重新加载/重启音频服务。  
  问题：Synapse 会周期性写回，可能覆盖；不是官方接口；不同 THX 版本 schema 可能变化。

适合做 **监控/备份**，不适合做稳定的控制面。

### 路线 C：Windows 音频 + 只做设备级控制

- 音量/静音：用 Core Audio `IAudioEndpointVolume`，不要碰 THX。
- 空间音效/DRC/THX 预设：要么调用 THX，要么让用户在 Synapse 里设置；不要直接改 APO 的二进制/注册表。
- 侧音/EQ 预设/电量：用本项目 HID 客户端，完全独立于 THX。

## 8. 证据与文件

| 文件 | 内容 |
|---|---|
| `captures/thx-userstate.json` | 当前 `UserState` 全部 JSON（Game/Music/Cinema/Custom/…） |
| `captures/thx-install.json` | 0565/056E 的 APO 安装键、INF、安装包路径 |
| `captures/thx-actions.txt` | Synapse 调用的 51 个 THX action 及次数 |
| `captures/thx-exports.txt` | `ThxV3Native` 231 个导出（含地址） |
| `captures/THXDLL_V3CommonSubProcess.log` | FFI action + payload + 返回值 |
| `captures/ThxV3NativeSubProcess.log` | THX 内部函数与设备/预设返回 |

## 9. 法律与安全

- `ThxV3Native_v1.1.42.1.dll`、APO 安装包、Razer 驱动均为第三方签名二进制，**不要重新分发**。
- 仅用于你拥有的设备与互操作性；不要绕过授权/DRM。
- 直接调用未公开 ABI 有蓝屏/音频服务崩溃风险；先在虚拟机或可恢复环境验证。
- 修改 `UserState` 前先备份（`captures/thx-userstate.json` 已备份当前状态）。

## 10. DLL / APO 能不能逆向？

先把三层分开：

```text
游戏/应用音频
  → Windows 音频引擎（audiodg）
      → razer_thx_apo（THX DSP：降噪、噪声门、DRC、空间）
          → 耳机

控制面：
  Synapse / 我们自己的 sail_thx_bridge
      → ThxV3Native.dll（控制库，不处理音频）
          → THX APO 的配置接口
```

### 10.1 能逆向的部分

- `ThxV3Native.dll` 的**导出表**：我们已经拿到 231 个导出。
- Synapse 的 **FFI action 名称、参数形态、调用顺序**：已有 `THX_*` / `SetRender*` / `SetCapture*` 日志。
- 页面层的 **UI 值 → THX 值** 转换：已经拿到 1:1 的证据。
- APO 的**安装信息**：注册表、INF、版本、设备 PID。
- 理论上还能继续挖 DLL → APO 的内部接口（COM/共享内存/属性），但这属于更深的控制面逆向。

### 10.2 不能/不建议逆向的部分

- **APO 的 DSP 算法实现**：这是音频处理代码，不是“一个耳机指令”。要重写降噪/噪声门/DRC，需要做滤波器、动态范围、语音检测等大量 DSP，不是调用一个 HID 命令。
- **签名/许可**：DLL/APO 是 THX/Razer 的签名二进制，逆向用于互操作可以讨论，但**重新分发、改名打包、绕过授权**不行。
- **专利/维护**：THX 的算法可能受专利保护；即使重写，也要长期维护音质/延迟/兼容性。

### 10.3 它不是“控制耳机的指令”

麦克风降噪、噪声门、声音标准化都在 **Windows 音频链路**里执行，不在耳机固件里。  
所以：

- 耳机 HID 只能做硬件级 EQ/侧音/电量；
- THX 功能是 PC 上的 APO；
- 逆向 DLL 得到的是“配置 APO 的 API”，不是“发给耳机的 HID 包”。

### 10.4 推荐策略

- **控制面**：自己写 `sail_thx_bridge`，调用用户已安装的 DLL；这是“逆向接口”，不是“复制实现”。
- **DSP**：复用已安装的 APO；不自研。
- **分发**：不打包 DLL/APO；只做检测 + 官方安装引导。
- 如果未来要完全开源替代：另立 `sail_apo` 项目，做 Windows APO + 自研 DSP，工作量和风险都远大于 HID 控制。

### 10.5 已验证的 C ABI（2026-10-18）

用 `sail_thx_bridge` 对 shipping DLL 实测：

```text
DeviceInitialize(containerId) -> bool
GetRenderDevice(containerId) -> char* JSON
SetDefaultRenderDevice(endpointId) -> bool
GetRenderNormalization(containerId) -> bool
SetRenderNormalization(containerId, bool) -> bool
GetRenderNormalizationLevel(containerId) -> int
SetRenderNormalizationLevel(containerId, int) -> bool
```

关键点：

- **level 是 `int`，不是 `double`**。用 `double` 调用有时返回 true，但不会真正写入。
- 必须先 `DeviceInitialize` + `GetRenderDevice` + `SetDefaultRenderDevice`，再读写 level。
- `SetRenderNormalization(containerId, true)` 是 level 写入的前置状态；读路径用 passive 模式，避免把用户关掉的标准化又打开。
- `sail_thx verify` 实测：`before 10 → set 1 → readback 1 → restore 10`，证明 1–100 不是 Synapse 的 UI 假象。

## 11. 声音标准化“有时调不动”的真实链路

Synapse 的声音标准化并不是“DLL 单独就能生效”，它是一条本地 IPC 链路。实测（2026-10-18）得到完整拓扑：

```text
Synapse UI (RazerAppEngine.exe)
        │  读 HKLM\SOFTWARE\THX\Discovery 得到端口
        ▼
VSSrv.exe  (THX Ltd. Windows Service, C:\WINDOWS\System32\VSSrv.exe)
        │  TCP 127.0.0.1:<动态端口>
        ▼
THX Spatial Audio Effects Component (APO, SWD\DRIVERENUM\THXAPOCOMPONENT_*)
        ▼
渲染端点 {0.0.0.00000000}.{5f0b2ea6-...} 的 FX 链
```

### 11.1 VSSrv 与动态端口

- 服务名 `VSSrv`，显示名同为 `VSSrv`，`Auto` 启动，`LocalSystem`，依赖 `RPCSS`。
- **端口不是固定的**。VSSrv 每次启动都会挑一对新的本地端口，写进：

```text
HKLM\SOFTWARE\THX\Discovery
  thx:sa:status:publisher = tcp://127.0.0.1:2311
  thx:sa:service          = tcp://127.0.0.1:2312
```

  - 开机那次实例用的是 `1024/1025`；
  - 手动重启服务后变成 `2311/2312`。

  所以任何硬编码 `1024/1025` 的检查或客户端都会在服务重启后失效。  
  正确做法：**读 Discovery，再连它发布的端口**。`sail_thx_bridge` 走 DLL，DLL 自己会读 Discovery，因此天然不受影响。

### 11.2 两个真实的失效模式

| 失效 | 现象 | 判断依据 | 修复 |
|---|---|---|---|
| VSSrv 崩溃后没起来 | 音频正常，Synapse 标准化开关/滑杆全部无效 | 服务 `Stopped`；Discovery 里的端口无人监听；事件 `7034/7000` | 启动服务 + 配置自动恢复 |
| 服务重启但前端缓存旧端口 | 同上，但服务是好的 | `RazerAppEngine` 出现指向旧端口的 `SYN_SENT`；Discovery 已是新端口 | 只重启 `RazerAppEngine` |

本机实证：

- 2026-09-27 22:34，`VSSrv` 崩溃两次（事件 `7034`），安装期间还有 4 次 `7000` 启动失败。
- `sc qfailure VSSrv` 显示 `RESET_PERIOD: 0`，注册表里**没有** `FailureActions` —— 即崩溃后永远不会自动拉起。
- 2026-10-18 手动重启 `VSSrv` 后，端口从 `1024/1025` 变为 `2311/2312`，`RazerAppEngine`（PID 31996）随即卡在 `SYN_SENT → 1024/1025`；重启前端后立即恢复并连上 `2311/2312`。

**结论：APO 和 DLL 通常没坏。** 坏的是“服务活着”或“客户端连对端口”这两件事之一。

### 11.3 诊断脚本

```powershell
scripts\diagnose-thx.cmd                  # 只读诊断，逐环 PASS/FAIL 并给出修复命令
scripts\diagnose-thx.cmd -Verify          # 额外验证等级 1..100 可写（会短暂改写后恢复）
scripts\diagnose-thx.cmd -Fix             # 管理员：自动修复断点，并复查
```

检查项：音频服务 → VSSrv → 自动恢复策略 → Discovery 通道 → INF 安装 → APO 组件 → 端点 → FX 绑定 → DLL → API → Synapse 端口缓存 → 状态文件 → 历史崩溃。

退出码：`0` 链路完整，`1` 仍有断点。

### 11.4 修复命令

```powershell
# 服务挂了 / 想立刻恢复
Restart-Service VSSrv -Force

# 永久修复：崩溃后自动拉起（THX 默认没有配）
sc.exe failure VSSrv reset= 86400 actions= restart/5000/restart/10000/restart/30000
sc.exe failureflag VSSrv 1

# 服务重启后端口会变，前端需要重新读 Discovery
Get-Process RazerAppEngine | Stop-Process -Force
Start-Process "C:\Program Files\Razer\RazerAppEngine\RazerAppEngine.exe" `
  -ArgumentList "--url-params=apps=synapse --launch-force-hidden=synapse"
```

不需要动音频栈，也不需要重装 THX APO —— 除非诊断明确指向 INF/APO 组件。

### 11.5 写等级的两个坑

- **写“当前值”会返回 `false`**：THX 把 no-op 当失败。判断成功要看回读，而不是只看返回值。
- **上一次写入未落地时的第二次写会被丢弃**：`set A` 成功后立刻 `set B`，`B` 可能返回 `false` 且不生效。

`sail_thx_bridge` 的处理：写入失败时先回读，命中即视为成功；否则按 80ms 间隔重试最多 5 次。  
读取路径（`status` / `probe`）是**只读**的：不会 `SetRenderNormalization(true)`，也就不会把用户关掉的标准化又打开。

## 12. 备份与版本锁定

### 12.1 THX APO 私有备份

```powershell
scripts\backup-thx.cmd -Destination D:\Backup\THX
```

导出内容：

- `driver-store/oem*/`：THX APO、SCU、服务驱动包（`thxrtapo.inf`、`thxrtscu.inf`、`thxrtsvc.inf`、`thxusbapo.inf`）
- `service/`：`VSSrv.exe` + `sc qc VSSrv` + 服务注册表
- `dll/`：`ThxV3Native_v*.dll`
- `registry/`：`HKLM\SOFTWARE\THX`、`HKCU\Software\THX`
- `programdata-thx/`：THX 游戏/空间配置 JSON
- `firmware/`：当前固件版本 + Razer `FWUpdate` SDK（通常没有可刷写镜像）
- `manifest.json` / `RESTORE.txt`

注意：这是**私有备份**，包含专有二进制；不要提交到开源仓库，也不要分发给别人。  
部署到另一台电脑优先用官方安装包；手工恢复驱动包需要管理员权限、匹配 Windows/设备，并先在 VM 验证。

一键部署：

```powershell
# 开发机：打包应用 + 安装器（不包含 THX 备份）
scripts\package-sail.cmd -Configuration Debug

# 目标机：把 dist\SAIL 和私有 THX-* 备份带过去，双击
dist\SAIL\install-sail.cmd

# 只看路径、不写盘
dist\SAIL\install-sail.cmd -DryRun
```

安装器会复制 `sail_ui.exe` / `sail_cli.exe` / `sail_thx.exe` 到 `C:\Program Files\SAIL`，
恢复 THX 驱动包、`VSSrv`、DLL、注册表和配置，创建开始菜单快捷方式，并跑状态自检。
它会在 `%ProgramData%\SAIL\rollback\<时间>` 留应用/服务/注册表快照；驱动包本身仍以 THX 备份为准。

### 12.2 固件

- 当前 `sail_cli status` 读到 `firmware: 1.6.11.0`。
- Synapse 日志里 `firmwareUpdateInfo: {}`，当前没有待更新信息。
- `%LOCALAPPDATA%\Razer\RazerAppEngine\User Data\Apps\Common\FWUpdate` 只有 updater SDK，没有 `.bin/.dfu` 镜像。
- 因此**无法从当前缓存直接保存可刷写固件**；固件更新必须走官方 updater，不要手动刷来路不明的镜像。
- 备份 `sail_cli status`、HID descriptor、protocol capture，已经足够作为兼容性证据。

### 12.3 防止厂商改接口导致“必须热更新”

- **冻结版本**：保留 `ThxV3Native_v1.1.42.1.dll`、APO 3.2.3.0、`VSSrv`、驱动包；目标机器不要自动更新 Synapse/THX。
- **基线**：`docs/thx-baseline.json` 记录 DLL/VSSrv 的 SHA256、驱动包、固件和工具链；`scripts\check-baseline.cmd` 做只读漂移检测。
- **版本发现**：`sail_thx_bridge` 不硬编码单个 DLL 文件名，自动选 `ThxV3Native_v*` 中数字版本最高的一个；`SAIL_THX_DLL` 可强制指定。
- **适配层**：未来 THX 改 ABI 时，新增一个 `thx_vX` 适配模块，按 DLL 版本选择；旧版本继续可用。
- **自检**：发布前跑 `sail_thx verify`；读取路径用 passive 模式，不改变用户状态。
- **开源仓库**：只放代码、协议、测试和兼容矩阵，不放任何 THX/Razer 二进制；用户带自己的安装包。
