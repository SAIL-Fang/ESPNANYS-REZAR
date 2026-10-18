# Razer 设备识别：VID/PID、产品类型与驱动边界

> 本文记录对 **RazerAppEngine 4.0.827**、Razer Installer、THX APO 安装包和本机 IndexedDB 的静态检查结果。
> 没有把 Razer/THX 二进制或设备数据库复制进仓库；这里只记录识别机制和结论。

## 结论

Razer **确实用 VID/PID 识别设备**，但 PID 在本地主要承担三件事：

1. USB/HID 设备身份；
2. 选择设备页面/配置/驱动的键；
3. 匹配 THX APO、驱动包和安装器资源。

它**不是一个硬编码在驱动里的 `productType` 枚举**。App Engine 的 JS 里没有本地 `productType` / `deviceType` 表；产品类型/型号展示来自按 PID 组织的设备元数据和 Razer 远程配置。

对我们这个独立上位机来说：

- `VID 0x1532 + PID 0x0565/0x056E` 已经足够做**本机型**的强识别；
- 不需要读取 Razer 驱动来判断“这是耳机”；
- 不要把 `0x1532` 单独当作耳机：Razer 的鼠标/键盘也用这个 VID。

## 证据

### 1. App Engine 只按 Razer VID 过滤，PID 继续向下传

`electron/UsbRzDeviceAction.js`（压缩后）里有：

```js
5426 === e.vendorId
```

5426 十进制就是 `0x1532`。USB add/remove 事件里继续携带：

```js
{ productId, vendorId, deviceContainerId, usbInstanceId }
```

同一文件枚举 USB 时打印：

```text
vId:${e.vendorId}, PId:${e.productId}, name:${e.productName},
deviceContainerId:${...}, locationId:${...}
```

代码里没有 `productType` 分支，也没有 `0565 -> headset` 这样的本地表。

### 2. HID 事件按完整元组匹配

`electron/modules/hidHardwareEvents/index.js` 注册 HID 设备时使用：

```js
e.productId === o.productId &&
e.vendorId === o.vendorId &&
e.deviceContainerId === o.containerId &&
e.usage === o.hidConfig?.usageId &&
e.usagePage === o.hidConfig?.usagePage
```

`ffiMappingEngine` 的 key 也是：

```js
createDeviceKey(productId, containerId)
```

所以 Razer App Engine 的“设备身份”是：

```text
VID + PID + containerId + usagePage + usage
```

而不是单独一个 PID 或产品类型字符串。

### 3. Razer 安装器按 PID 分驱动/APO 包

本机可以看到：

```text
C:\Program Files\Razer\PID0565MW
C:\Program Files\Razer\PID056eMW
```

GameManager 日志里也有：

```text
USB\VID_1532&PID_0565&MI_00
THXAPO-USB_BlackSharkV2HyperSpeed-0565-...
THXAPO-USB_BlackSharkV2HyperSpeed-056e-...
THX-0565-Presets-BlackSharkV2HyperSpeed
THX-056E-Presets-BlackSharkV2HyperSpeed
```

这说明 PID 用于选择 **驱动/THX APO/预设安装包**，但安装包目录名本身没有给出通用产品类型枚举。

### 4. Synapse 的远程存储和页面也按十进制 PID 建键

本机 IndexedDB 里有：

```text
pageView: usb_5426_1390_{deviceContainerId}_ui
... /RZR_.../devices/1390/profiles/...
... /RZR_.../devices/1390/deviceMetadatas/main.json
```

其中：

- `1390` 十进制 = `0x056E`（有线）；
- `1381` 十进制 = `0x0565`（无线）。

这进一步说明 PID 是设备页面/配置/元数据的键；具体产品展示信息在 `deviceMetadatas` 中，而不是 App Engine 的本地类型表。

### 5. 内核驱动本身不是产品分类器

`RzComDriver` 的 `RzCommonU.inf` 只绑定：

```text
RAZER\DeviceContol
```

它不是按 `PID 0565/056E` 做产品分类的耳机驱动。耳机主要走：

- 标准 USB HID（我们已实现的 0xFF14 / interface 3）；
- Windows 音频 endpoint；
- 已安装的 THX APO（INF 按 `USB\VID_1532&PID_0565&MI_00` 绑定）。

因此不需要为了“识别产品类型”去写内核驱动。

## 对 SAIL 的建议

当前代码的 `sail_hid::select_device` 已经要求：

```text
vendor_id == 0x1532
product_id == 0x0565 或 0x056E
```

这已经足够锁定 BlackShark V2 HyperSpeed。建议再补一个明确的模型映射，而不是从产品字符串猜：

```rust
pub enum DeviceModel {
    BlackSharkV2HyperSpeedWireless,
    BlackSharkV2HyperSpeedWired,
}

pub fn model_for(vendor_id: u16, product_id: u16) -> Option<DeviceModel> {
    match (vendor_id, product_id) {
        (0x1532, 0x0565) => Some(DeviceModel::BlackSharkV2HyperSpeedWireless),
        (0x1532, 0x056E) => Some(DeviceModel::BlackSharkV2HyperSpeedWired),
        _ => None,
    }
}
```

对于音频 endpoint，Windows 不提供 HID PID，所以仍只能用 endpoint 名称/容器匹配；当前 `audio_route` 和 THX container 的精确名称检查是合理的。

## 限制

- Razer App Engine 是压缩后的 Electron JS，没有完整 source map；结论来自静态字符串/调用点和本机 IndexedDB 缓存。
- `deviceMetadatas/main.json` 的具体字段没有在本地完整展开；可以确定的是它以 PID 为路径键，内容属于远程设备元数据。
- 这是对当前安装版本（4.0.827）的观察，不保证未来版本保持同样路径。
