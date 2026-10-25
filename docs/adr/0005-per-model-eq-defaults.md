# ADR-0005: Factory EQ presets are per model

- Status: Accepted
- Date: 2026-10-25

## Context

The two supported products have different factory curves in Razer Synapse. The
old `SpeakerEqGains::default()` used the HyperSpeed custom current curve as its
game preset and never varied by model. The user asked for Razer's actual
game/music/movie defaults and for the custom default to become
`[1, -5, -3, 0, 1, 1, 1, 1, 1, -3]`.

The observed factory defaults are:

```text
BlackShark V2 HyperSpeed (0565)
  game   [-7, -5, -3, -3, 3, 4, -2, -3, -3, -3]
  music  [ 3,  3,  2,  0, 1, 1,  1,  1,  1, -3]
  movie  [ 4,  4,  3, -4,-6,-3,  1,  3,  1, -1]
  custom [ 0,  0,  0,  0, 0, 0,  0,  0,  0,  0]  (factory)
          [ 1, -5, -3,  0, 1, 1,  1,  1,  1, -3]  (S.A.I.L. default)

BlackShark V2 via Razer USB Sound Card (0529)
  game   [-3, -3, -4, 0, 5, 5, 4, 1, 0, -1]
  music  [ 2,  2,  1, 1, 2, 3, 3, 3, 1,  0]
  movie  [ 4,  4,  3, 0,-3,-1, 3, 5, 2,  1]
  custom [ 0,  0,  0, 0, 0, 0, 0, 0, 0,  0]  (factory)
          [ 1, -5, -3, 0, 1, 1, 1, 1, 1, -3]  (S.A.I.L. default)
```

## Decision

- Factory preset values live with the model profile and are exposed by
  `eq_presets()` for the active model.
- The UI does not keep a second table of factory curves.
- S.A.I.L.'s custom default is the same new curve for both models.
- Existing user custom curves are not migrated or overwritten. The new custom
  default applies only when a custom curve is initialized or reset.
- Selecting game/music/movie writes that model's curve; custom remains
  user-controlled.

## Consequences

- A model change must refresh the preset list and the visible curve together.
- `SpeakerEqGains::default()` is no longer a global truth; callers must ask the
  active model profile.
- Tests pin both tables so a future UI refactor cannot silently restore the old
  HyperSpeed game curve.
