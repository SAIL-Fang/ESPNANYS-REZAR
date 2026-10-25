# Second headset + per-model EQ defaults — 2026-10-25

## What was decided

- Factory EQ presets are per model, not global.
- S.A.I.L. custom default is `[1, -5, -3, 0, 1, 1, 1, 1, 1, -3]` on both models.
- The second device is the analog **BlackShark V2** through the **Razer USB Sound Card** (`1532:0529`).
- Its available controls are THX EQ, spatial, normalization and microphone; it has no battery, charging, firmware, serial, dongle-LED or hardware sidetone.
- The app has an explicit device selector. Selecting a model switches the Windows default output to that model's endpoint so the THX container binds to the right device.
- Full parity means capability parity; missing fields are shown as unsupported, never simulated.

## Factory curves (reverse-engineered from Razer Synapse logs)

```text
BlackShark V2 HyperSpeed (0565)
game   [-7, -5, -3, -3, 3, 4, -2, -3, -3, -3]
music  [ 3,  3,  2,  0, 1, 1,  1,  1,  1, -3]
movie  [ 4,  4,  3, -4,-6,-3,  1,  3,  1, -1]
custom [ 1, -5, -3,  0, 1, 1,  1,  1,  1, -3]  (S.A.I.L.)

BlackShark V2 + USB Sound Card (0529)
game   [-3, -3, -4, 0, 5, 5, 4, 1, 0, -1]
music  [ 2,  2,  1, 1, 2, 3, 3, 3, 1,  0]
movie  [ 4,  4,  3, 0,-3,-1, 3, 5, 2,  1]
custom [ 1, -5, -3, 0, 1, 1, 1, 1, 1, -3]  (S.A.I.L.)
```

## Implementation

- `ModelDescriptor` gained the `0529` profile and `eq_via_thx`.
- `SpeakerEqGains` carries both factory tables; `eq_presets(model)` chooses the active model's table.
- `SailApp::set_eq_gains` sends the 0529 curve through THX `SetRenderEQGains` (JSON array); `set_eq_preset` is a no-op on that path.
- THX helper accepts `SAIL_THX_MODEL`, chooses the USB Sound Card container, and runs from its own directory.
- Tauri exposes `get_devices` / `set_active_device`; the UI has a device selector and automatically switches the default audio output for the selected model.
- `wiredNotReady` only checks telemetry that the model claims to support.

## Verification

- UI state tests: 16 pass; lifecycle: 6; presentation: 5.
- `svelte-check`: 0 errors / 0 warnings.
- `cargo test --workspace`: pass; `cargo check`/clippy pass.
- Manual debug UI: both models appear; selecting USB Sound Card switches output and reaches THX ready; the 0529 game curve `[-3,-3,-4,0,5,5,4,1,0,-1]` was written and read back in the UI.
- The 0529 microphone page selected `麦克风 (Razer USB Sound Card)` and exposed the THX capture controls; hardware sidetone is disabled because the model declares no sidetone.
- THX `SetRenderEQGains` was called successfully on the real DLL for both container paths.

## Boundary

The behavior-bearing `HeadsetProfile` trait is still deferred; this is a data descriptor plus capability branches for two real models. The release is not republished from this state.
