# ADR-0008: Persist per-model preset overrides

- Status: Accepted
- Date: 2026-10-25

## Context

Editing the Game curve used to set `eqPreset = "custom"`, so the next Reset
reset Custom rather than Game. The user wants Game/Music/Movie to remain the
selected mode while their curve is edited, and wants the last selected preset
and its curve to survive an application restart.

Editing a factory preset is not the same thing as selecting the Custom preset.
The selected preset is a mode; an override is a curve delta from that mode's
factory curve. They must be stored separately.

## Decision

- Editing a preset keeps that preset selected. There is no "modified" badge;
  Reset is the explicit way to restore the factory curve.
- The configuration is a human-readable JSON file at
  `%APPDATA%\SAIL\ESPNANYS-REZAR\eq-presets.json`.
- The schema is sparse and versioned:

  ```json
  {
    "version": 1,
    "models": {
      "razer.blackshark_v2_usb_sound_card": {
        "selected": "game",
        "presets": {
          "game": [-3, -3, -4, 0, 5, 5, 4, 1, 0, -1]
        }
      }
    }
  }
  ```

- A missing preset key means "factory curve"; Reset removes the override. A
  `selected` key records the last explicit preset for that model.
- A missing file is an empty configuration; the file is created lazily on the
  first preset selection, edit, or Reset. A corrupt file is copied to
  `eq-presets.corrupt-<unix>.json` and the app starts with an empty
  configuration and a visible notice.
- The selected preset and its effective curve are written to the hardware on
  the next start after the audio output points at the active model. This
  avoids writing a THX curve into the wrong endpoint while the output gate is
  active.
- The configuration is written only after the hardware write succeeds. Writes
  use a temporary file followed by replacement; a write failure is shown and
  the in-memory curve remains available for a later retry.
- Preset selection, editing and Reset all update `selected` for the active
  model. Config is scoped by the stable model id, so both headsets keep
  independent curves.

## Consequences

- Game/Music/Movie edits persist without changing the selected mode.
- Reset restores the current mode's factory curve and removes only that
  override.
- The app has one user-editable config file outside the install directory.
- A corrupt or hand-edited file can never prevent startup.
