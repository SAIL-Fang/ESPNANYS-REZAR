# SAIL Design System

> 本文是最终设计源（source of truth）。`design-system/sail/MASTER.md` 是 ui-ux-pro-max 的原始输出，仅作为输入；其中“游戏营销页 / 3D / 霓虹紫”的方案已被否决。

## 0. 命名约定

- `razer_*`：Razer / THX 自带，**不是我们的代码，不打包**。
- `sail_*`：我们自己的组件、模块、服务、UI。
- 设备固件、THX APO、Razer 服务都不加 `sail_` 前缀。

## 1. 产品与模式

- 产品：**SAIL**，一个低占用的 Windows 耳机控制台。
- 设备：Razer BlackShark V2 HyperSpeed（0x0565 / 0x056E）。
- 模式：**Operate**（完成任务，不是营销页）。
- 用户：PC 玩家、主播、需要麦克风增强但不想开 Synapse 的人。
- 核心任务：看状态、调 EQ、开关声音标准化、调麦克风增强、切配置。
- 非目标：RGB 灯效、游戏内 overlay、THX 空间/房间深度定制、固件刷新。

## 2. 视觉世界：录音棚机架，不是游戏外设

关键词：**机架、表头、刻度、琥珀色信号、冷灰、精确、安静**。

- 一个记忆点：顶部 **signal strip** + 主区域 **EQ 仪表**。
- 其余全部克制：无渐变背景、无发光边框、无 RGB、无 3D 耳机渲染。
- 信息密度高，但每块只有一个主动作。
- 数字用等宽字体，像表头读数；标签用 sentence case，不用全大写。

## 3. Color tokens

```css
--sail-bg:          #0B0D10;  /* 近黑，不是纯黑 */
--sail-surface:     #12161B;  /* 主面板 */
--sail-surface-2:   #171D24;  /* 次级面板 / hover */
--sail-line:        #2A333D;  /* 1px 分隔线 */
--sail-line-strong: #3A4652;
--sail-text:        #E7ECF1;
--sail-muted:       #8D9AA8;
--sail-faint:       #5E6A77;
--sail-accent:      #E8A33D;  /* 琥珀：主操作 / 当前值 */
--sail-accent-soft: #3A2A16;  /* 琥珀底色 */
--sail-accent-2:    #5BC8C4;  /* 冰青：测量 / 次要状态 */
--sail-danger:      #E06055;
--sail-success:     #86C28B;
--sail-focus:       #F2C14E;
```

规则：

- 主操作/当前值用琥珀；冰青只用于“测量/链路”类信息，不抢主操作。
- 不用 Razer 绿（#44D62C 一类），不用紫色渐变。
- 纯色块 + 1px 线；不使用大面积阴影。
- 文本对比度 ≥ 4.5:1； muted 只用于辅助信息。

## 4. Typography

| 角色 | 字体 | 用法 |
|---|---|---|
| UI / 正文 | **IBM Plex Sans** + **Noto Sans SC** | 中文回退；正文 13px |
| 数字 / 读数 | **IBM Plex Mono** | `font-variant-numeric: tabular-nums` |
| 标题 | IBM Plex Sans 600 | 页面 24px，区块 15px |

Type scale：

```text
11 / 12 / 13 / 15 / 18 / 24 / 32
body 13/1.5 · label 12/1.4 · page title 24/1.2 · readout 13 mono
```

规则：

- 不用 Russo One / Chakra Petch（电竞海报感太重）。
- 不用全大写标签；中文标签不加字距。
- 标题不靠“一个词换色/斜体”制造重点。
- 正文行宽 ≤ 72 字符。

## 5. Layout

- 默认窗口 `1180 × 760`；最小 `1040 × 680`。
- 左侧 rail 68px（图标 + 文字 tooltip），顶部 status strip 56px。
- 内容区 padding 24/32；12 列网格，gutter 16。
- 主面板不是“卡片汤”：同一表面 + 1px 分隔线；只有 4/6px 两种圆角。
- 需要强调的区块用“内嵌仪表盘”而不是浮起卡片。

```text
┌─────────────────────────────────────────────────────────────┐
│ SAIL   ● Connected    ▮▮▮▮▯ 78%    THX ready    ⚙          │ 56
├──────┬──────────────────────────────────────────────────────┤
│ 设备 │ 页面标题                          [主操作]           │
│ 声音 │                                                      │
│ 麦克风│  内容：左设置 / 中仪表 / 右读数                      │
│ 设置 │                                                      │
└──────┴──────────────────────────────────────────────────────┘
```

## 6. Components

| 名称 | 说明 |
|---|---|
| `sail_status_strip` | 连接点、电量分段、THX 状态、设置入口；高度 56 |
| `sail_signal_meter` | 电量/麦克风电平的分段条；不是进度条 |
| `sail_eq_curve` | SVG 10 段曲线；琥珀线、冷灰网格、当前点高亮 |
| `sail_band_slider` | 竖向推子；44px 命中；键盘可调 |
| `sail_toggle_row` | 标签 + 一句说明 + 开关；禁用时说明原因 |
| `sail_preset_tabs` | 分段控件，不是卡片 |
| `sail_value_readout` | 等宽数字 + 单位；不滚动 |
| `sail_thx_badge` | “THX 增强”状态；可点开说明/安装引导 |
| `sail_inline_status` | 写入中/失败/已回读；不做 toast-only |
| `sail_missing_thx` | THX 缺失面板：解释 + 官方安装入口 |

## 7. Motion

- 只做“状态改变”的动效：toggle 120ms、slider 160ms、面板展开 180ms。
- 不做页面载入动画、不做 scroll reveal、不做弹跳。
- EQ 点跟随指针；释放后回读。
- `prefers-reduced-motion: reduce` 时全部瞬时。

## 8. States

- **Disconnected**：status strip 变暗；控件 disabled；显示“重新连接”；保留上次值并标 `上次值`。
- **THX missing**：`sail_missing_thx` 面板；麦克风增强/声音标准化 disabled；HID 功能仍可用。
- **THX crashed**：`sail_thx_bridge` 自动重启；UI 显示 `THX 暂不可用`，不假装成功。
- **Write pending**：控件旁 `写入中`；失败显示原因 + 重试；成功回读后去掉。
- **Device busy**：命令队列；不弹模态。

## 9. Accessibility

- 所有交互有可见 focus ring（2px `--sail-focus`），不被 sticky 元素遮挡。
- 滑杆：原生 `input[type=range]` 优先；`aria-valuemin=1`、`aria-valuemax=100`、`aria-valuetext="10 级"`。
- 键盘：方向键 ±1，PageUp/Down ±10，Home/End 到边界；`Tab` 顺序与视觉一致。
- 命中区 ≥ 44×44；图标按钮有 `aria-label`。
- 不用 emoji 当图标；用 Lucide SVG。
- EQ 提供“数值表”替代操作，键盘用户可直接输入每段 dB。

## 10. Anti-patterns

- 不用 Razer 绿、RGB 渐变、发光边框。
- 不用“每块都是圆角卡片 + 同样阴影”。
- 不用全大写 eyebrow、`A · B · C` 元信息、`→` 结尾按钮。
- 不用 3D 耳机大图占据首屏。
- 不用 hover-only 操作。
- 不用只有颜色表达状态（同时给文字/图标）。
