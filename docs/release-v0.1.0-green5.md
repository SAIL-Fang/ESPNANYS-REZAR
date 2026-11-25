# ESPNANYS-REZAR v0.1.0 green5

Windows x64 green build. No installer, no admin rights, no Razer/THX binaries.

Requires WebView2 and a locally installed Razer THX APO.

## Changes since green4

### Mouse battery telemetry (Razer Viper V3 Pro)

- The device hub now shows a battery card for the Razer Viper V3 Pro: battery
  level plus charging state, refreshed with the rest of the hub. It is read-only
  and cannot be entered like a headset workspace.
- Both cable and dongle forms are supported and were verified on hardware:
  wired `PID 0x00C0` and wireless receiver `PID 0x00C1`.
- New independent Razer Protocol 2.5 implementation (90-byte HID feature report)
  next to the 64-byte headset frames: class `0x07` with battery `0x80` and
  charging `0x84`, transaction id `0x1F`, CRC `XOR[2..88]`, raw level
  `0..255` scaled as `(raw * 100 + 127) / 255`.
- The receiver needs a wait between `SET_REPORT` and `GET_REPORT` (31 ms wired,
  60 ms wireless); without it the device answers with an empty `busy` frame.
- A sleeping wireless mouse reads as "mouse is off"; a connected mouse that
  stops replying reads as "mouse not responding".

### Safety boundaries

- A mouse can never become the active device: the app layer rejects
  telemetry-only PIDs in `set_active_pid`, and the Tauri command delegates to it.
- Device workspaces, sidetone/EQ writes and audio-output matching stay
  headset-only. Headset and mouse HID transports are selected by device kind, so
  the two protocols cannot cross.
- `DeviceSnapshot::unavailable` keeps the mouse identity when its HID interface
  cannot be opened instead of falling back to an unknown, interactive card.

### Documentation

- `docs/protocol.md` gains section 9: mouse Protocol 2.5 (frames, commands,
  receiver wait, headset-vs-mouse comparison).
- New `docs/adr/0009-mouse-telemetry-card.md`.
- GLOSSARY adds device kind, telemetry-only device and Razer Protocol 2.5.
- README, device identification and the modularity notes list the Viper PIDs and
  the read-only card.

## Assets

- `ESPNANYS-REZAR.exe` — one-file UI; extracts its own THX helper on first run.
- `ESPNANYS-REZAR-v0.1.0-win-x64-green5.zip` — one-file UI + update-switch
  scripts + README.
- `.sha256` for both assets.

## SHA256

- `ESPNANYS-REZAR.exe`: `f70179f7c6577881619beef2822c66ac00180755ee5cf0f80281dec2ca112be2`
- `ESPNANYS-REZAR-v0.1.0-win-x64-green5.zip`: `c12ec04ce9dd68c3725703f55184972d082cc8a22ed267a45722dd72a625d4f9`

## Notes

- No Razer or THX binaries are bundled or redistributed.
- The THX APO must already be installed on the target PC.
- Windows 10/11 x64 + WebView2 Runtime.
