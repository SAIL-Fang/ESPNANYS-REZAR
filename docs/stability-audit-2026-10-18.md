# ESPNANYS-REZAR stability audit — 2026-10-18

Scope: Rust `sail_core` / `sail_hid` / `sail_app` / `sail_thx_bridge`, the Tauri/Svelte UI, and Windows scripts.
No hardware soak was performed. Hardware-marked tests remain `#[ignore]`.

## Baseline

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace --all-targets` | pass; 13 core, 3 HID mock, 3 app; 2 HID hardware tests ignored |
| `npm run check` (ui) | 0 errors / 0 warnings |
| `npm run build` (ui) | pass |
| `npm audit --omit=dev --audit-level=high` | 0 vulnerabilities |
| PowerShell 5.1 parse + `razer-update-core.ps1 -Status` | pass after UTF-8 BOM fix |
| CI | no workflow in repo yet |

## Findings

### P0 — fixed

**EQ editor could overwrite an unknown curve.** The UI used `device.presets[0]` as if it were the headset's current 10-band state. There is no HID/THX getter for the current gains, so launching with a custom curve and writing one band would overwrite the other nine.

Fix (updated per user request): the sound page now starts with an explicit **default starting curve** `[1, -5, -3, 0, 1, 1, 1, 1, 1, -3]`, and writes it once on startup and after each reconnect (the headset can reset its EQ on reboot). Curve edits and preset tabs write automatically (350 ms debounce for drags); the separate 「写入耳机」 button is gone. 「重置」 restores that same default curve and writes it immediately. The tradeoff is deliberate: convenient default, but applying it can overwrite an unknown custom curve.

### P1 — fixed

- **Request `flags9` was wrong for every transaction.** `transact()` always used the transport's GET flag. Dongle LED GETs on wireless used `0x80` instead of `0x00`; wired set1/set10 used `0x00` instead of `0x80`. Fixed with the documented policy and two mock tests (`dongle_led_get_uses_zero_flag_on_wireless`, `wired_get_and_set_use_expected_flags`).
- **Short HID reads were treated as full 64-byte frames.** Fixed: reject non-zero reads that are not exactly 64 bytes.
- **Tauri commands ran blocking HID/THX work on the main thread.** All commands now use `spawn_blocking`; the window no longer waits synchronously for a snapshot or THX helper.
- **PowerShell scripts were UTF-8 without BOM but launched by Windows PowerShell 5.1.** This produced actual syntax errors in `razer-update-core.ps1` and `diagnose-thx.ps1` under the system ANSI code page. Fixed: all `.ps1` files are UTF-8 with BOM and CRLF; parser and `-Status` now pass.
- **UAC elevation returned success before doing anything.** `Start-Process -Verb RunAs` now uses `-Wait -PassThru` and propagates the child exit code.
- **Hosts rewrite could delete user mappings, truncate on failure, or loop.** It now removes only SAIL's marked line, writes a sibling temp file and replaces it, preserves the default encoding, and handles a single blank line without an infinite loop. Full historical restore is still not implemented.
- **THX `verify` could never restore normalization off.** It sampled state through the activating constructor, so `before_enabled` was always true. It now samples with a passive bridge first, requires a valid current level, and explicitly restores the prior enabled/level state.
- **Non-ASCII DLL paths used `LoadLibraryA`.** Switched to `LoadLibraryW` with a UTF-16 path.
- **`open(None)` could select a non-headset Razer collection.** The selector now requires PID `0x0565`/`0x056E`, prefers interface 3, then wireless; three unit tests cover decoys/preference.
- **No HID serialization.** `SailApp` now serializes snapshot and setters through a global HID mutex, so polling and writes cannot interleave sequence numbers/handles.
- **THX spatial mode was missing.** Added `GetRenderSpatialProcessing` / `SetRenderSpatialProcessing`, helper readback, `spatialEnabled` in the snapshot, and the stereo / THX SPATIAL AUDIO switch. Tested off → on against the installed DLL and through a CDP-driven UI click.
- **UI poll and write state could race.** `refresh()` now has an epoch (stale responses are dropped), skips polling while a write is active, and uses `finally` for `loading`. `busy` is a reference count for overlapping writes, and Enter is gated on `busy`.
- **Numeric EQ input needed a draft state.** The field now keeps a raw string while focused, ignores empty input, supports `-`/decimal intermediate states, commits on blur/Enter, and reverts on Escape.
- **Default output was not checked.** `sail_app` now enumerates Windows render endpoints through Core Audio and exposes `audioOutput`; a blocking modal offers a one-click `IPolicyConfig` default switch and an endpoint selector. It is user-mode COM, not a kernel driver. `sail_cli audio-default` exposes the same operation.
- **No tray resident mode.** Added a Tauri tray icon with Show / Quit; closing the window hides to the tray. Tray settings/autostart are still pending.
- **THX helper could hang indefinitely.** `ProcessThxHost::run` now polls with an 8-second deadline and kills the child on timeout.
- **Audio dropped briefly every 10 seconds.** The silent UI poll spawned `sail_thx status --json` every 10s. Each helper open calls `SetDefaultRenderDevice`, which interrupts the active render stream. Fixed: silent polls skip THX entirely (THX refreshes at startup, manual refresh, and after THX writes), and polling pauses while the window is hidden. Reproduced before the fix and confirmed gone after restart.
- **Normalization slider was draggable while normalization was off.** The range input now disables when `!enabled`, and its input/change handlers guard on the enabled state as a second defense.
- **Microphone preview reported a false failure and had no stop button.** `SetCapturePreview(true)` started THX's preview worker, but the short-lived `sail_thx` helper unloaded the DLL and exited with `0xC0000005`; the UI saw an empty stderr and kept the button at 「开始测试」. Preview now runs in a detached `mic-preview-hold` helper, `mic-preview off` stops it, and the UI keeps a real stop state. Also, helper exit errors now include the exit code instead of an empty message.
- **THX container discovery could pick a different BlackShark.** It now accepts only exact BlackShark V2 HyperSpeed / V2 HS names, prefers type 0, and fails with an explicit error instead of falling back to the hardcoded container. `SAIL_THX_CONTAINER` remains the explicit override.
- **Installer only warned on `pnputil` failure.** Missing driver backup and any nonzero `pnputil` result now fail the install; `-SkipThx` remains the explicit opt-out.

### P1 — open

- **Feature-report read has no timeout.** `cached_event()` → `get_feature_report()` is a blocking hidapi call. A wedged dongle can leave `snapshot()` and a `spawn_blocking` thread stuck; the 10-second poll can accumulate. Needs a bounded worker/read path or a total snapshot deadline.
- **THX container name matching is still name-based** because the THX device JSON does not expose the HID PID. Exact V2 HyperSpeed/V2 HS names are required, and no fallback is used; a future DLL that reports a different name needs `SAIL_THX_CONTAINER`.
- **Passive THX binding still calls `SetDefaultRenderDevice`.** Gating it made the DLL fail/crash during open in testing; the call is required before per-endpoint getters. This remains a documented side effect of status/space reads and needs a proper passive binding path.
- **UI last-good state and optimistic writes.** Silent read failures now keep the previous status/THX, and `statusError` / `thxReadError` are separate from action errors. Sidetone and normalization only update their local switch after the write succeeds. The one-shot `synced` latch can still retain a stale THX level.
- **Installer uses elevated per-user paths and does not check signatures.** Over-the-shoulder UAC can put the helper in the admin profile; driver failures now abort, but package signatures are still not verified.
- **CI is not in the repo.** GitHub CLI token lacks `workflow` scope; add the workflow after refreshing scopes.

### P2 — fixed or documented

- Fixed: response data length must end before checksum (`> 62`, not `> 63`).
- Fixed: response checksum is now verified; event checksum remains unvalidated per the documented variant.
- Fixed: sequence generation skips the observed-dead `0x41..=0x52` range after wrapping.
- Fixed: `read_feature_report(0)` no longer panics.
- Fixed: EQ write validation is `-12..=12` at the app boundary.
- Open: global event cache is not keyed by PID and never expires.
- Open: THX helper status overwrites `installed=true` even when per-field calls fail.
- Open: container discovery/JSON parser is hand-rolled and accepts only exact `"id":"` formatting.
- Open: package script does not clear stale `payload/` and omits update scripts; installer rollback dirs accumulate.

## Next three actions

1. Add a `snapshot_with(&mut impl Transport, ...)` seam and tests for timeout, empty data, status 5, and event-cache fallback.
2. Bound the wired feature-report read (wireless already skips it).
3. Add a component test around startup default EQ/preset/apply and restore CI.

> This audit proves the checks above pass and documents known failure modes. It does not prove the absence of bugs, especially on hardware, across driver versions, or under Windows power/sleep transitions.
