# ADR-0009: Telemetry-only mouse cards

- Status: Accepted
- Date: 2026-11-25

Extends ADR-0007 (the hub lists connected supported devices; only a headset
workspace is enterable).

## Context

The Viper V3 Pro shares Razer's vendor ID (`0x1532`) but speaks the 90-byte
Razer Protocol 2.5 over HID feature reports, not the headset's 64-byte
output/input frames. Its only useful telemetry is battery level and charging
state. ADR-0004 keeps the model descriptor as the single source of device
identity and capabilities; ADR-0006 requires capability parity, so fields the
mouse does not expose must read as unavailable rather than be fabricated.

Product scope agreed for the first version: show the mouse in the device hub
with battery and charging only. Read-only, no mouse workspace, no DPI, polling
rate, RGB or buttons.

## Decision

- `DeviceKind` (`headset` | `mouse`) becomes part of the model descriptor. The
  Viper V3 Pro descriptor declares wired PID `0x00C0`, wireless PID `0x00C1`,
  usage page `0x0001`, usage `0x0002`, interface 0, and only battery/charging
  capabilities.
- Protocol 2.5 lives in its own `sail_core::mouse` module: 90-byte frame,
  `FeatureTransport` seam, read commands `0x07/0x80` (battery) and `0x07/0x84`
  (charging), transaction id `0x1F`, XOR `[2..88]` checksum, bounded retries.
  It does not reuse the headset `SailDevice` transport.
- The HID feature exchange waits between SET_REPORT and GET_REPORT — 31 ms for
  the wired mouse, 60 ms for the wireless receiver (OpenRazer's receiver
  constants). Without the wait a receiver answers `status = 0x01` (busy) with
  empty arguments even though the request was accepted.- `sail_hid` adds `MouseHidTransport` and `open_mouse_device`. The mouse HID
  collection is chosen by device kind, never by the headset's `0xFF14` vendor
  usage.
- `sail_app` builds ordinary `DeviceSnapshot` / `DeviceSummary` values for a
  mouse: battery and charging are real fields, every headset-only field is
  `Unsupported`. A mouse that answers neither query is not reported as online:
  a wireless mouse behind a live dongle is `standby` (the dongle is present,
  the mouse did not wake) and a wired mouse that went silent keeps its identity
  with `link = wired` and timeout fields.
- Summary ordering is headset first, telemetry cards last (one shared
  `sail_hid::profile_rank`). Audio-output matching is false for a mouse.
  `sail_app::set_active_pid` rejects a telemetry-only PID and the Tauri
  `set_active_device` command delegates to it, so the invariant is owned once
  by the app layer (ADR-0001). The headset transport filters candidates by
  `DeviceKind`, so it can never open a mouse collection, and the write gate
  still refuses a mouse PID defensively.
- The UI presentation layer owns `deriveHubCard`. A mouse card shows battery,
  charging, link state, a link-derived type ("有线鼠标" / "无线鼠标") and the
  action text "仅显示电量"; it hides the THX and output rows and is not
  interactive. Headset card rows are unchanged; the hub's empty-state copy is
  now model-agnostic.

## Consequences

- Battery parity with Synapse for the mouse without a control surface that the
  hardware scope does not cover.
- The two wire protocols stay isolated: selection and opening are filtered by
  `DeviceKind`, so no 64-byte frame can reach a mouse interface and no 90-byte
  feature report can reach the headset.
- Supporting another mouse means adding a descriptor and, only if its wire
  format differs, a protocol adapter; no plugin framework or generic device
  behaviour trait is introduced.
- Hub copy that refers to cards says "设备"; copy about entering a workspace
  still says "耳机".
