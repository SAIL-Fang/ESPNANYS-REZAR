# 组件归属：哪些是雷蛇/THX 的，哪些是 sail_ 自研

## 命名规则

- **没有 `sail_` 前缀**：雷蛇硬件/固件、雷蛇软件、THX 组件，或第三方系统组件。
- **有 `sail_` 前缀**：我们自己写、要开源、可修改的代码。
- `razer_` / `thx_` 只用于描述设备/协议/第三方接口，不用作我们自己的模块名。

> 一句话：看到 `sail_` 就是我们的；看到 `Razer*`、`THX*` 就不是我们的。

## 归属总表

| 名称 | 归属 | 运行位置 | 我们是否修改/分发 |
|---|---|---|---|
| Razer BlackShark V2 HyperSpeed 耳机 | **雷蛇硬件** | 硬件 | ❌ 只通过 HID 控制 |
| 耳机固件（1.6.11.0） | **雷蛇** | 耳机 MCU | ❌ 不刷写、不逆向固件 |
| HID 协议（64 字节帧） | **雷蛇设备接口** | 耳机/接收器 | 我们只写文档和客户端 |
| `RazerAppEngine` / Synapse | **雷蛇软件** | Windows | ❌ 不使用、不修改 |
| `Razer Central Service` | **雷蛇软件** | Windows 服务 | ❌ 不使用 |
| `ThxV3Native_v1.1.42.1.dll` | **THX（雷蛇安装包携带）** | Windows 用户态 | ❌ 不打包、不分发；运行时加载用户已安装的 |
| THX Spatial Audio APO | **THX（雷蛇安装包安装）** | Windows 音频引擎 | ❌ 不重写；复用其 DSP |
| `sail_hid` | **我们（sail_）** | 用户态 | ✅ 开源 |
| `sail_hid_service`（可选托盘/服务） | **我们（sail_）** | 用户态 | ✅ 开源 |
| `sail_thx_bridge` | **我们（sail_）** | 独立用户态进程 | ✅ 开源；仅调用用户已安装的 THX |
| `sail_ui` | **我们（sail_）** | Tauri/WebView2 | ✅ 开源 |
| `sail_cli` | **我们（sail_）** | 命令行 | ✅ 开源 |
| `sail_config` | **我们（sail_）** | JSON/SQLite | ✅ 开源 |
| `sail_ipc` | **我们（sail_）** | Named Pipe | ✅ 开源 |
| `sail_protocol` | **我们（sail_）** | 文档/测试 | ✅ 开源（clean-room 文档） |
| `sail_installer` | **我们（sail_）** | Windows 安装包 | ✅ 开源 |

## 数据流（带归属）

```text
┌─────────────────────────────────────────────────────────────┐
│ sail_ui（我们的 Tauri 前端）                                 │
│   invoke / events                                            │
└───────────────────────┬─────────────────────────────────────┘
                        │ sail_ipc（我们的 Named Pipe）
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ sail_hid_service / sail_daemon（我们的后台）                 │
│   ├─ sail_hid（我们的 64-byte HID 协议）                     │
│   │     └─ HID interface 3 / usage 0xFF14                   │
│   │           └─ Razer 耳机/接收器（雷蛇硬件）               │
│   └─ sail_thx_bridge（我们的独立进程）                       │
│         └─ ThxV3Native.dll（THX，雷蛇安装包携带）            │
│               └─ THX APO（THX，Windows 音频引擎）            │
│                     └─ Razer 耳机 endpoint（雷蛇硬件）        │
└─────────────────────────────────────────────────────────────┘
```

## 为什么 `sail_thx_bridge` 要单独存在

- `ThxV3Native.dll` 不是我们的代码，不能打包、不能改；
- 它是原生 C++/FFI，崩溃会拖垮主进程；
- 只在用户打开“声音/麦克风”页时启动，关闭后退出；
- 没有安装 THX 时，`sail_thx_bridge` 不启动，`sail_ui` 把麦克风增强页置灰。

## 术语替换

| 之前文档里的名字 | 现在的名字 | 归属 |
|---|---|---|
| HID Core | `sail_hid` | 我们 |
| RazerHidService | `sail_hid_service` / `sail_daemon` | 我们 |
| THX Host / ThxHost | `sail_thx_bridge` | 我们 |
| Tauri UI | `sail_ui` | 我们 |
| CLI / razerctl | `sail_cli` | 我们 |
| `src/razer_hid.py` | `src/sail_hid.py` | 我们 |
| `tools/razerctl.py` | `tools/sail_cli.py` | 我们 |
| `tools/collect.py` | `tools/sail_collect.py` | 我们 |

## 开源发布检查

- 仓库里只出现 `sail_*` 源码、协议文档、测试；
- 不提交 `ThxV3Native.dll`、THX APO 安装包、Razer 安装包；
- README 写清楚：`sail_*` 是自研，THX 是可选第三方运行时；
- 发布前清理 captures 里的序列号、container GUID、用户名路径。
