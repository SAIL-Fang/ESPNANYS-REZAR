# ADR-0003: One presentation interface

- Status: Accepted
- Date: 2026-10-20

## Context

`UiState` was the single state derivation, but `StatusStrip`, `+page`, `HeadsetPowerGate`, `AudioRouteGate`, and `DevicePanel` still recombined state and telemetry into their own labels. This allowed copy and visibility to drift from the state machine.

## Decision

- `ui/src/lib/presentation.ts` is the single presentation interface.
- It accepts `DeviceSnapshot`, `ThxSnapshot`, `UiState`, and `loading`, and returns a typed view model.
- It owns connection/transport labels, state badge, battery/charge text, THX label, power-gate copy, output-route warning copy, and route target/current labels.
- `UiState` remains state-only; power copy and labels move out of it.
- UI modules render the presentation fields and do not recombine state flags.
- The presentation module is pure and covered by Node tests.

## Consequences

- One place to change copy or decide which label belongs to a state.
- State and presentation remain separate concepts; presentation can evolve without growing the state machine.
- Components become thinner, but must receive `loading` explicitly.
