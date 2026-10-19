# ADR-0004: The model descriptor is real; the behavior trait waits

- Status: Accepted
- Date: 2026-10-20

## Context

`ModelDescriptor` already carried identity, EQ bands, and capability flags, but the UI still hardcoded the 10-band labels and the 0–15 sidetone range. The app also reported THX ready even if a model declared `thx: false`. At the same time there is still only one adapter; a behavior-bearing `HeadsetProfile` trait with no second implementation would be a speculative seam.

## Decision

- The descriptor is the source of truth for product identity, EQ band labels, sidetone maximum, and feature capabilities.
- Consume it now:
  - `deriveUiState` only reports `thxReady` when the model declares `thx`.
  - the presentation layer reports `THX 不支持` for a model without THX.
  - EQ labels come from `model.eqBands`.
  - the sidetone control uses `model.sidetoneMax` and the model's `sidetone` capability.
  - `SoundPanel` and `MicPanel` gate their THX controls on the model capability.
- Do **not** introduce the behavior-bearing `HeadsetProfile` trait or a model-specific THX container profile until a second adapter or a concrete protocol difference exists.
- Backend capability enforcement beyond the existing online gate is deferred; the current Razer descriptor remains the only production model.

## Consequences

- Adding metadata for a second model no longer requires editing the UI's fixed EQ/sidetone assumptions.
- The descriptor is a data interface, not a full protocol adapter; command mapping is still Razer-specific.
- The next real model should add an adapter and capability tests, not a generic framework.
