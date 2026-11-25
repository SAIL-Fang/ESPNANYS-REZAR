# Glossary

## Active device

The one supported Razer **headset** the app is currently reading and writing.
When several supported headsets are present, the user can select one
explicitly; the selection determines which model profile and preset set apply.
A telemetry-only device is never the active device.

## Device kind

The product class of a model profile: `headset` or `mouse`. A headset has a
workspace and writable controls; a mouse is telemetry-only. Device kind is part
of the model profile, not inferred from a transport or PID at the call site.

## Telemetry-only device

A supported device whose hub card shows live readings (for example battery and
charging) but that has no workspace and accepts no writes. The Viper V3 Pro
mouse is the first telemetry-only profile: its card is read-only and cannot be
entered.

## Razer Protocol 2.5

The 90-byte HID feature-report protocol that supported Razer mice use for
telemetry (battery, charging). It is a different protocol from the headset's
64-byte output/input frames; the two must never share a transport.

## Model profile

The product-specific description of a supported device: identity, transport,
available controls, EQ bands, and preset defaults. A model profile is not the
same thing as a hardware protocol; it may delegate to HID, THX, or both.

## Device hub

The landing view that lists every connected supported device as a card. The
hub is independent of the active device's power and audio-route gates: those
gates apply only after the user enters a device workspace.

## Device workspace

The per-device view containing the sound, microphone, device and settings
modules. A workspace has exactly one active device and may contain the power
gate and the audio-output gate.

## Device summary

A lightweight, per-device card record used by the hub: identity, connection or
standby state, supported power fields, THX capability, and active marker. A
summary is not a full device snapshot and must not force an audio-output or THX
container switch. Telemetry-only summaries carry battery/charging plus link
state, and nothing they cannot read.

## Output target

The Windows render endpoint that belongs to a model profile: the HyperSpeed
2.4 endpoint or the Razer USB Sound Card endpoint. The output target is separate
from the active device; choosing a device does not silently change the output.

## Preset default

The factory curve for a named EQ mode (game, music, movie, custom) for one
model profile. A preset default is not a user setting: the user's current
custom curve must not be overwritten when the application updates its factory
defaults.

## Preset override

A user-edited curve stored under a selected preset mode. The mode remains
Game/Music/Movie/Custom; the override is the delta from that mode's factory
curve. Reset removes the override and restores the factory curve.

## Selected preset

The last preset explicitly selected or edited for a model. It is remembered in
the EQ configuration and applied to the hardware after the model's output is
active on the next start.

## EQ configuration

The versioned JSON file containing per-model selected presets and sparse preset
overrides. A missing entry means the factory curve; a missing file is an empty
configuration and is created lazily.

## Capability parity

Two model profiles are feature-equivalent only for capabilities their hardware
or host software actually exposes. A missing battery, sidetone, or firmware
reading is shown as unavailable, never simulated.

## USB Sound Card profile

The BlackShark V2 analog headset connected through the Razer USB Sound Card
(`VID_1532&PID_0529`). It has THX audio processing but no headset battery,
charging, or headset HID vendor channel. Its EQ is host-side THX processing,
not a headset HID command.
