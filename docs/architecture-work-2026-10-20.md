# Architecture work — 2026-10-20

The user asked for four deepening candidates to be grilled one at a time, in order, then completed. Because the user left, every remaining grilling question was answered with the recommended option. The decisions are recorded as ADRs, not only here.

## Sequence and commits

| # | Candidate | Decision | Commit |
|---|---|---|---|
| 1 | Write gate | Put the online gate behind `SailApp`; one HID session; Tauri/CLI adapters | `24da730` |
| 2 | Connection lifecycle | One foreground lifecycle module; 5s tick; online status every 2 ticks | `661e849` |
| 3 | State/presentation | One pure presentation interface; UiState remains state-only | `8e4df22` |
| 4 | Model descriptor | Consume descriptor capabilities/bands/sidetone; defer `HeadsetProfile` | `a6206b9` |

ADRs:

- `docs/adr/0001-sailapp-write-gate.md`
- `docs/adr/0002-connection-lifecycle.md`
- `docs/adr/0003-presentation-interface.md`
- `docs/adr/0004-model-descriptor-scope.md`

## Verification

- UI state tests: 15 pass
- Connection lifecycle tests: 6 pass
- Presentation tests: 5 pass
- `svelte-check`: 0 errors / 0 warnings
- `cargo test --workspace`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- Debug UI built and launched after the lifecycle/presentation changes; observed the expected live state and output-wrong dialog.

## Important boundary

This work is **after** the `v0.1.0-green2` release. The GitHub release and desktop one-file/green binaries were not rebuilt or republished with these four changes. The next release should be cut explicitly.

The behavior-bearing `HeadsetProfile` trait and model-specific THX container profile remain deliberately deferred until a second model or a concrete protocol difference exists.
