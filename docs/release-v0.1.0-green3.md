# ESPNANYS-REZAR v0.1.0 green3

Windows x64 green build. No installer, no admin rights, no Razer/THX binaries.

Requires WebView2 and a locally installed Razer THX APO.

## Changes

### Device hub

- Starts in a device library instead of a sidebar selector.
- Shows connected supported headsets with connection/standby, battery when supported, THX capability, active marker, and whether the current output points at that device.
- Selecting a device no longer changes the Windows default output.
- The output prompt is scoped to the device workspace and targets the correct endpoint:
  - `0529` → `Razer USB Sound Card`
  - `0565` → `BlackShark V2 HS 2.4`
- Power and output gates no longer block the hub; both can return to the library.
- Card shells appear while summaries load.

### Second headset: BlackShark V2 + Razer USB Sound Card (`1532:0529`)

- HID enumeration on vendor usage page `FFA0`.
- THX EQ, spatial, normalization and microphone supported.
- No fake battery, charging, firmware, serial or hardware sidetone.
- THX capture device selected as `麦克风 (Razer USB Sound Card)`.
- Capability parity is explicit; unsupported fields show as unavailable.

### EQ

- Factory game/music/movie tables are per model.
- Custom default is `[1, -5, -3, 0, 1, 1, 1, 1, 1, -3]` on both models.
- Editing a preset keeps that preset selected; no automatic jump to Custom and no “modified” badge.
- Reset restores the currently selected preset’s factory curve.
- Preset overrides and the last selected preset persist in:

  ```text
  %APPDATA%\SAIL\ESPNANYS-REZAR\eq-presets.json
  ```

- Missing config files are lazily initialized; corrupt files are backed up; writes happen after the hardware write succeeds.
- On the next start, the selected preset and curve are written back after the active model’s output is correct.

## Assets

- `ESPNANYS-REZAR.exe` — one-file UI; extracts its own THX helper on first run.
- `ESPNANYS-REZAR-v0.1.0-win-x64-green3.zip` — UI + helper + CLI + README.
- `.sha256` for both assets.

## SHA256

- `ESPNANYS-REZAR.exe`: `b345ad705cb6b26b8765780f693d45b1dbb90ad7c0ddc0a7bb118a70322c05ce`
- `ESPNANYS-REZAR-v0.1.0-win-x64-green3.zip`: `da34c0d8468c878612c9b71cd9c7d61d7822c3fe909c579c7ad2f687f7360ddc`

## Notes

- No Razer or THX binaries are bundled or redistributed.
- The THX APO must already be installed on the target PC.
- Windows 10/11 x64 + WebView2 Runtime.
