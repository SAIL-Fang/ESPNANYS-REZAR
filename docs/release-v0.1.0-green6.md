# ESPNANYS-REZAR v0.1.0 green6

Windows x64 green build. No installer, no admin rights, no Razer/THX binaries.

Requires WebView2 and a locally installed Razer THX APO.

## Changes since green5

### Sound normalization survives a restart

- The sound-normalization level (THX DRC) was only stored in the WebView
  `localStorage` profile. When that LevelDB stopped replaying new writes it kept
  returning an old value of `1`, and the app dutifully pushed `1` back to THX
  every time a headset workspace opened — the reported "set 8, reopen, it is 1
  again" bug. Helper-process tracing confirmed the app itself ran
  `sail_thx.exe level 1` at workspace entry.
- `%APPDATA%\SAIL\ESPNANYS-REZAR\eq-presets.json` is now `version: 2` and stores
  `normalizationLevel` and `normalizationEnabled` per model next to the preset
  overrides. The file is the source of truth; `localStorage` is only a cache and
  is used as a fallback for files written before version 2.
- The first run after the upgrade seeds the current effective level into the
  file, so existing users keep the value they already had.
- Version-1 configuration files load unchanged; the new fields are optional and
  omitted when unset.
- The Tauri command `save_normalization_setting` validates `1..=100`; out-of-
  range values are dropped on load as well.

### Wired headset no longer reports "not connected"

- `set_active_device` used to bind a headset to `model.wireless_pid` first. A
  headset plugged in over its USB cable (`0x056E`) therefore left every later
  command talking to the absent receiver PID `0x0565`, and the workspace showed
  the "耳机未连接" lock screen even though the device was online.
- The active PID is now resolved from the transport that is actually present
  (`sail_app::present_pid_for`), falling back to the descriptor's nominal PIDs
  only when nothing is enumerated.

### Documentation

- New `docs/adr/0010-normalization-persistence.md`; ADR-0008 cross-references it.
- README and `docs/audio-features.md` describe the config file as the source of
  truth instead of `localStorage`.

## Assets

- `ESPNANYS-REZAR.exe` — one-file UI; extracts its own THX helper on first run.
- `ESPNANYS-REZAR-v0.1.0-win-x64-green6.zip` — one-file UI + update-switch
  scripts + README.
- `.sha256` for both assets.

## SHA256

- `ESPNANYS-REZAR.exe`: `fec75530fe113474caed4eddfa89cf6c859080422cff9eb5a661ca6d90f4ec0f`
- `ESPNANYS-REZAR-v0.1.0-win-x64-green6.zip`: `dcd6a4250a76c43f5cd7cf6a2159464deae5ff35a2bc325d589951097c4c2e98`

## Verification

- `cargo test -p sail_app --lib` — 22 passed (including the new
  `normalization_round_trips_and_invalid_level_is_dropped` and
  `version_one_file_still_loads`).
- `npm run check` — 0 errors, 0 warnings; `npm run test:ui` — 12 passed.
- Hardware: level forced back to `1` with `sail_thx.exe level 1`, then the
  packaged one-file exe was started and the headset workspace opened; THX
  reported `level: 8` again, read from the JSON configuration.

## Notes

- No Razer or THX binaries are bundled or redistributed.
- The THX APO must already be installed on the target PC.
- Windows 10/11 x64 + WebView2 Runtime.
