# ADR-0007: Device hub is the landing surface

- Status: Accepted
- Date: 2026-10-25

## Context

The first two-device implementation put a `<select>` in the sidebar and
switched the Windows default output when the selection changed. That conflated
"which device is being controlled" with "which endpoint Windows should use",
and the global route/power gates could block the only surface that would let
the user choose the other device.

Razer Synapse's dashboard uses connected-device summaries and a device-library
view before entering a product page. It does not use a global output modal to
gate the library itself.

## Decision

- The app always opens the **device hub**, even with one device.
- The hub lists only connected, supported headsets. Cards show identity,
  connection/standby, supported battery/charging, THX capability, active marker,
  and whether the current output points at that device.
- Hub summaries are lightweight and never switch the Windows output or a THX
  container. They refresh when the hub is visible and after device changes.
- Selecting a card sets the active device and enters its workspace, but does
  **not** change the Windows default output.
- The workspace contains the output gate. Its target endpoint is matched to the
  active model, not the first headset endpoint. The gate offers set-default,
  open-sound-settings, and back-to-hub.
- The power gate runs before the output gate; both offer back-to-hub. Neither
  gate is rendered in the hub.
- The selected model is remembered, but startup still shows the hub. With no
  previous selection, the first present supported device is the active one.
- A card can show "output not pointed" without changing the output.

## Consequences

- Hub remains usable when the active headset is off or the output is wrong.
- Selecting a device no longer silently changes system audio.
- The output prompt is model-specific, so 0529 gets the USB Sound Card endpoint
  and 0565 gets the BlackShark 2.4 endpoint.
- The UI needs a per-device summary API in addition to the active snapshot.
