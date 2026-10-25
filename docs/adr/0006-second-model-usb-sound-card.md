# ADR-0006: Second model is the BlackShark V2 over the Razer USB Sound Card

- Status: Accepted
- Date: 2026-10-25

## Context

A second product is connected as `VID_1532&PID_0529`, reported by Windows as
`Razer USB Sound Card`. It is the analog BlackShark V2 connected to that sound
card, not a second wireless HyperSpeed. It exposes a THX container and a vendor
HID collection on usage page `FFA0`, but no headset battery or charging
telemetry. The user wants feature parity where the hardware supports it.

This is the second real adapter, so the behaviour-bearing model profile is no
longer speculative.

## Decision

- Add a real model profile for the USB Sound Card path, separate from the
  HyperSpeed HID profile.
- Detect it by `VID_1532&PID_0529`; do not pretend it is a HyperSpeed.
- Its EQ, spatial, normalization, and microphone controls go through THX when
  the THX container exposes them. It does not use the headset `FF14` HID
  command channel.
- Battery, charging, firmware, serial, dongle-LED, and hardware sidetone are
  unavailable on this path. The UI hides or marks them unavailable; it never
  invents values.
- When several supported models are present, offer an explicit active-device
  selector. Default selection follows the current Windows default output, and
  the last explicit selection is remembered.
- The model profile is data plus capability-driven branching; a separate
  DAW-style plugin framework is not introduced. The behaviour-bearing
  `HeadsetProfile` trait remains deferred.

## Consequences

- Preset values can differ per model, and the UI must reload them on selection.
- THX container discovery must accept a model hint rather than always choosing
  the HyperSpeed container.
- Full parity means capability parity, not fake battery or sidetone controls.
- The 0529 path can be tested without a second headset HID protocol, but its
  THX container must be present.
