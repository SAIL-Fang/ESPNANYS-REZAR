# ADR-0002: One foreground connection lifecycle

- Status: Accepted
- Date: 2026-10-20

## Context

`HeadsetPowerGate` owned a 5-second reconnect timer plus focus/visibility listeners; `StatusStrip` owned a 10-second status timer and checked focus itself. The policy was pure and tested, but the timer and listener wiring was split across two rendering modules.

## Decision

- `ui/src/lib/connection-lifecycle.ts` owns the single timer, foreground checks, initial refresh, offline reconnect, and online status cadence.
- One 5-second tick:
  - offline: attempt reconnect every tick;
  - online: refresh status every second tick (10 seconds).
- Focus/visibility events trigger an immediate foreground check.
- A successful HID reconnect performs the existing two-stage refresh: HID first, THX second.
- `HeadsetPowerGate` and `StatusStrip` only render.
- Tests inject scheduler, foreground state, event subscription, and ports; no real timer or DOM is required.

## Consequences

- One place to change cadence or foreground rules.
- The components no longer duplicate lifecycle wiring.
- The first online status refresh remains 10 seconds after startup; the initial full refresh happens once at start.
