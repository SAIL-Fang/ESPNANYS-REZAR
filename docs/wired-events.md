# 有线 0x056E：主动读取与异步事件

2026-10-18 实测结论：**有线也有主动读取命令**，之前 timeout 的原因是请求帧的 flag 写错了。

## 根因：第 9 字节

主动读电量/充电的请求：

```text
[02 00 7c 00 00 00 04 00 00 00 21 00 00 ...]
                          ^^
                          offset 9 = 0x00（有线 056E）
```

设备响应：

```text
[02 02 7c 00 00 00 05 00 80 80 21 01 01 62 ...]
```

- **有线 056E get 请求**：offset 9 = `0x00`
- **无线 0565 get 请求**：offset 9 = `0x80`
- **set1 / set10 请求**：offset 9 = `0x80`
- **响应**：offset 9 固定 `0x80`

之前 SAIL 把响应里的 `0x80` 抄进了 get 请求，所以设备直接忽略，表现为 timeout。  
修正后有线主动读取正常：

```text
battery: 98
charging: true
firmware: 1.6.11.0
serial: ...
```

`seq` 不需要跟事件计数器同步；`0x60` 起步即可。响应会回显 seq。

## 异步事件仍然存在

设备也会主动推事件：

```text
record 33  BATTERY_STATUS   payload [98]
record 42  CHARGING_STATUS  payload [1]   // InCharging
```

事件帧：

```text
[02 0a seq 00 00 00 05 00 80 80 2a 02 01 01 ...]
```

设备还会把最后一条事件缓存在 **feature report 2**，可以作为主动命令失败时的兜底：

```text
get_feature_report(2, 64)
02 0a 60 3f 01 00 05 00 80 80 2a 02 01 01 ...
```

## SAIL 当前策略

1. 先读 feature report 2，记住最近的 charging/battery（兜底）；
2. 用正确的 `get` flag 主动读 battery/charging/firmware/serial；
3. 主动读成功的字段优先，事件缓存只补 timeout 的字段。

## 备注

- `record 42` 约 60 秒一次；`record 33` 约 3–5 分钟一次。
- 主动读取命令还会返回当前电量，所以不必再等 `record 33`。
- 事件缓存会被下一条命令响应覆盖，因此必须在发命令前读。
