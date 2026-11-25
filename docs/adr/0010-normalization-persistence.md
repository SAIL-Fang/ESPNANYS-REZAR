# ADR-0010: Persist the sound-normalization setting in the config file

- Status: Accepted
- Date: 2026-11-25

## Context

The sound-normalization level (THX DRC) was the only user setting that lived
exclusively in the WebView `localStorage` profile at
`%LOCALAPPDATA%\com.sail.control\EBWebView\Default\Local Storage\leveldb`.

That copy is not durable in two ways:

- A hard kill during a write left the LevelDB log in a state where Chromium
  replays an old snapshot and silently ignores every later `setItem`. The key
  stayed at `1` while the UI happily reported success.
- The value is invisible to the Rust side, so the file-based EQ configuration
  and the in-browser cache could disagree with no way to reconcile them.

The visible symptom: the user set the level to 8, reopened the app, entered the
headset workspace, and THX was back at 1. Helper-spawn tracing showed the app
itself running `sail_thx.exe level 1` at workspace entry — it was restoring a
stale preference, not losing a fresh one.

Note that the restore only runs when a workspace opens; sitting on the device
hub never writes to THX. That made the bug look like "the driver forgets my
setting on restart" when it was really "the driver re-applies an old value".

## Decision

- Store the per-model sound-normalization state in the same versioned JSON file
  as the EQ presets (`%APPDATA%\SAIL\ESPNANYS-REZAR\eq-presets.json`), which
  becomes `version: 2`:

  ```json
  {
    "version": 2,
    "models": {
      "razer.blackshark_v2_hyperspeed": {
        "selected": "custom",
        "presets": { "game": [1, -5, -3, -3, 3, 4, -2, -3, -3, -3] },
        "normalizationLevel": 8,
        "normalizationEnabled": true
      }
    }
  }
  ```

- The file is the source of truth. `desiredNormalizationLevel()` reads the
  config first and falls back to `localStorage` only for files written before
  version 2. `localStorage` is demoted to a cache that mirrors what the app last
  applied, so a corrupt WebView profile can no longer override the user.
- A version-1 file loads unchanged; the new fields are optional and omitted
  when unset, so existing installations do not rewrite their presets.
- On the first run after the upgrade (`normalizationLevel` absent) the current
  effective value is seeded into the file, so an existing user keeps the level
  they already had.
- Writes happen after the THX write succeeds, through the existing
  `save_atomic` path, exactly like preset overrides.
- Out-of-range levels are dropped on load and rejected by the Tauri command
  (`normalization level must be 1..=100`).

## Consequences

- The sound-normalization level and switch now survive restarts, WebView profile
  corruption, and a wiped `localStorage`.
- One file holds all user-visible preferences; it stays human-readable and
  hand-editable.
- The app no longer trusts a value it cannot verify against the file, so a
  stale browser store only affects pre-version-2 installations.
- `localStorage` remains as a compatibility cache and can be deleted at any
  time.
