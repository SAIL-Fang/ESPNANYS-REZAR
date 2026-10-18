# HID 协议 — 64 字节 Razer 帧

本文只描述 **Razer BlackShark V2 HyperSpeed**（PID `0x0565` 无线 / `0x056E` 有线）在 interface 3、usage page `0xFF14` 上的 64 字节 output/input report 协议。它和 Razer 鼠标常见的 90/91 字节 Protocol 2.5 不是同一套。

## 1. HID 集合选择

`hid.enumerate(0x1532, 0x0565)` 会看到 interface 3 下的 4 个集合：

| 集合 | usage_page | usage | 报告 | 用途 | 可写 |
|---|---|---|---|---|---|
| Col01 | `0xFF13` | 1 | output 6 / input 7，61 字节 | 另一套 vendor 协议 | ❌ `Cannot write to hid device` |
| Col02 | `0x000C` | 1 | input 12，1 字节 | Consumer Control | ❌ |
| Col03 | `0x000B` | 5 | input/output 5 | Headset/Telephony | ❌ |
| **Col04** | **`0xFF14`** | **1** | **input 2 / output 2，63+1 字节** | **本协议** | ✅ |

Synapse 自己会依次尝试 Col01→Col04，前三个报 `Cannot write to hid device`，最后落在 **Col04**（`main.log` 可见）。

```text
path = \\?\HID#VID_1532&PID_0565&MI_03&Col04#...&0003#{4d1e55b2-f16f-11cf-88cb-001111000030}
```

Python：

```python
import hid
for d in hid.enumerate(0x1532, 0x0565):
    if d["interface_number"] == 3 and d["usage_page"] == 0xFF14:
        dev = hid.device()
        dev.open_path(d["path"])
        dev.set_nonblocking(1)
        break
```

> 打开后先 `read(64)` 排空队列，再发命令；否则可能先读到旧事件。

## 2. 帧格式

固定 64 字节，包含 report ID `0x02`。

### 请求（host → device）

| 偏移 | 长度 | 值 / 含义 |
|---|---|---|
| 0 | 1 | `0x02` report ID |
| 1 | 1 | `0x00` 请求 |
| 2 | 1 | sequence（建议从 `0x60` 开始递增） |
| 3–5 | 3 | `0x00` |
| 6–7 | 2 | class：`04 00` get / `05 00` set1 / `0E 00` set10 |
| 8 | 1 | `0x00` |
| 9 | 1 | 有线 056E get `0x00`；无线 0565 get `0x80`；set1 / set10 `0x80`。响应固定 `0x80`，不要照抄响应 |
| 10 | 1 | command / target |
| 11 | 1 | subcommand（get 为 `0x00`） |
| 12 | 1 | payload 长度 |
| 13… | n | payload |
| 62 | 1 | checksum |
| 63 | 1 | `0x00` |

### 响应（device → host）

| 偏移 | 长度 | 值 / 含义 |
|---|---|---|
| 0 | 1 | `0x02` |
| 1 | 1 | `0x02` 响应 |
| 2 | 1 | 回显 sequence |
| 3–5 | 3 | `0x00` |
| 6–7 | 2 | class **不固定**：低字节 ≈ `4 + data 长度`（1 字节→`05`，4 字节→`08`，15 字节→`13`） |
| 8–9 | 2 | `80 80` |
| 10 | 1 | command |
| 11 | 1 | status（`1` = success，`5` = not supported） |
| 12 | 1 | data 长度 |
| 13… | n | data |
| 62 | 1 | checksum |
| 63 | 1 | `0x00` |

> 不要把响应 class 当作固定值；只按 `type == 0x02`、`seq`、`cmd` 匹配。

### 事件（device → host）

| 偏移 | 长度 | 值 / 含义 |
|---|---|---|
| 0 | 1 | `0x02` |
| 1 | 1 | `0x0A`（偶见 `0x09`） |
| 2 | 1 | sequence / 计数器 |
| 6–7 | 2 | `05 00` |
| 8–9 | 2 | `80 80` |
| 10 | 1 | record ID |
| 11 | 1 | `0x02` |
| 12 | 1 | 长度 |
| 13… | n | 值 |
| 62 | 1 | checksum |
| 63 | 1 | `0x00` |

事件示例（电量 46%）：

```text
02 0A 60 05 00 00 05 00 80 80 21 02 01 2E ... CHK 00
```

## 3. 校验和

对 **请求和响应** 实测：

```python
checksum = XOR(frame[2:62]) ^ 0x02
```

Python：

```python
def checksum(f):
    x = 0
    for b in f[2:62]:
        x ^= b
    return (x ^ 0x02) & 0xFF
```

> 事件帧在部分抓包里使用变体（多一个 `0x08` 位）；解析事件时不需要校验。

## 4. Sequence 规则

- 设备会回显 sequence。
- **实测 `0x41`–`0x52` 的请求没有响应**；从 `0x60` 开始（首帧 `0x61`）全部正常。
- 建议：`seq = 0x60` 起步，每帧 +1，`0xFF` 后回到 `0x01`。
- 不要用 `0x00`。

## 5. 命令表

`cmd`、`sub` 均按协议字节；`kind` 对应请求 class。

| 名称 | cmd | sub | kind | 实测返回 |
|---|---|---|---|---|
| Serial | `0x00` | `0x00` | get | 15 字节 ASCII：`<SERIAL>`（已脱敏） |
| Edition | `0x01` | `0x00` | get | `[0]` |
| Firmware | `0x02` | `0x00` | get | `[1,6,11,0]` → **1.6.11.0** |
| Speaker Preset EQ | `0x1E` | `0x00` | get | `[0]` |
| Battery | `0x21` | `0x00` | get | `[39]`（39%） |
| Charging | `0x2A` | `0x00` | get | `[0]`（未充电） |
| Auto Power Off | `0x2C` | `0x00` | get | `[15]`（15 分钟） |
| Dongle LED | `0x66` | `0x00` | get | `[2]`（flags9=0x00） |
| Set Preset EQ Index | `0x93` | `0x00` | set1 | 写 `7` = Game |
| Set Customer EQ Band | `0x95` | `0x00` | set10 | 10 个 signed byte |
| Set Mic Preset EQ Index | `0x96` | `0x00` | set1 | 0x0565 上 status=5（不支持） |
| Set Sidetone | `0x98` | `0x00` | set1 | `0/1` |
| Set Sidetone Volume | `0x99` | `0x00` | set1 | UI 50 → 设备 `7` |
| Set Speaker Preset EQ Status | `0x9E` | `0x00` | set1 | `[0]` |

### 5.1 读固件

```text
02 00 61 00 00 00 04 00 00 80 02 00 00 ... CHK 00
```

响应：

```text
02 02 61 00 00 00 08 00 80 80 02 01 04 01 06 0B 00 ... 60 00
```

`data = [1,6,11,0]`，即 `1.6.11.0`。

### 5.2 读序列号

```text
02 00 63 00 00 00 04 00 00 80 00 00 00 ... CHK 00
```

响应 `data` 为 15 字节 ASCII，解码为 `<SERIAL>`（已脱敏）。

### 5.3 读电量

```text
02 00 61 00 00 00 04 00 00 80 21 00 00 ... CHK 00
02 02 61 00 00 00 05 00 80 80 21 01 01 27 ... 60 00
```

`data[0] = 0x27 = 39` → 39%。

### 5.4 开关侧音

```text
开: 02 00 61 00 00 00 05 00 00 80 98 00 01 01 ... CHK 00
关: 02 00 61 00 00 00 05 00 00 80 98 00 01 00 ... CHK 00
```

响应：

```text
02 02 61 00 00 00 05 00 80 80 98 01 01 00 ... CHK 00
```

Synapse 开侧音时还会紧跟一条 `0x99 00 01 07`（侧音音量）。

### 5.5 写 EQ 预设

```text
02 00 61 00 00 00 05 00 00 80 93 00 01 07 ... CHK 00
```

响应 status=1。`7` 在日志里对应 `eqPresetEnum: "Game"`。

### 5.6 写 10 段 EQ

class `0x0E`，payload 10 个 signed byte：

```text
02 00 61 00 00 00 0E 00 00 80 95 00 0A 01 FB FD 00 01 01 01 01 01 FD ... CHK 00
```

对应十进制 `[1,-5,-3,0,1,1,1,1,1,-3]`。这和 THX 的 `SetRenderEQGains` 是同一组 10 段值。

## 6. 事件

| record ID | 含义 | 值示例 |
|---|---|---|
| `0x20` | Wireless connection status | 0/… |
| `0x21` | Battery status | `0x2E` = 46% |
| `0x27` | Do Not Disturb | 0/1 |
| `0x55` | Mic status | 0/2 |

事件解析：

```python
rid = resp[10]
value = resp[13:13 + resp[12]]
```

## 7. 实现注意事项

1. **只打开 Col04**；其他集合写入会失败。
2. **seq 从 0x60 开始**；低 seq 会被忽略。
3. 每次发送前可先排空输入队列；事件和响应共用 input report。
4. 响应按 `(type == 0x02, seq, cmd)` 匹配，不能只按 cmd（可能有旧事件/上一条响应）。
5. Synapse 运行时读操作可并发（实测），但写操作可能与页面轮询竞争；做控制软件时建议先退出 Synapse，或做互斥。
6. 设备在无线接收器上工作；有线 PID `0x056E` 的 HID 集合/协议可能略有差异，需要单独枚举验证。
7. `0x96` 在 0x0565 上不支持；不要把它当作通用命令。

## 8. 最小实现

见 `src/razer_hid.py`：

```python
with RazerHeadset() as hs:
    print(hs.status())
    hs.set_sidetone(False)
```

CLI：

```powershell
python tools/razerctl.py status
python tools/razerctl.py sidetone off
```
