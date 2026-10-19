# ADR-0001: Put the online write gate behind SailApp

- Status: Accepted
- Date: 2026-10-20

## Context

Tauri's `blocking_write` checked `require_online`, but `SailApp`'s public HID setters and some `sail_cli` commands opened HID directly. The write invariant was therefore owned by one caller, and the CLI could bypass it. The check and the write also opened separate HID sessions, leaving a race between the handshake and the command.

## Decision

- Every state-changing write crosses `SailApp`'s public write interface.
- HID writes first open one transport, prove the headset is awake, and write through that same session under one HID lock.
- THX and microphone writes perform the online check immediately before delegating to the helper.
- Tauri and the CLI are adapters; neither performs its own online check.
- `NoReceiver` and `HeadsetOff` are separate errors. The UI keeps its existing unified copy.
- The gate has an injectable test seam; offline tests prove the writer is never called.

## Consequences

- The CLI can no longer write while the receiver is absent or the headset is off.
- The Tauri adapter no longer performs a duplicate handshake.
- `SailApp` owns one more invariant, but callers get a smaller interface.
- THX writes retain a narrow race: the headset can power off after the handshake, because the THX helper is out-of-process.
