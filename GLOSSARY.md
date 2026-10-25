# Glossary

## Active device

The one supported Razer model the app is currently reading and writing. When
several supported devices are present, the user can select one explicitly;
the selection determines which model profile and preset set apply.

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
container switch.

## Output target

The Windows render endpoint that belongs to a model profile: the HyperSpeed
2.4 endpoint or the Razer USB Sound Card endpoint. The output target is separate
from the active device; choosing a device does not silently change the output.

## Preset default

The factory curve for a named EQ mode (game, music, movie, custom) for one
model profile. A preset default is not a user setting: the user's current
custom curve must not be overwritten when the application updates its factory
defaults.

## Capability parity

Two model profiles are feature-equivalent only for capabilities their hardware
or host software actually exposes. A missing battery, sidetone, or firmware
reading is shown as unavailable, never simulated.

## USB Sound Card profile

The BlackShark V2 analog headset connected through the Razer USB Sound Card
(`VID_1532&PID_0529`). It has THX audio processing but no headset battery,
charging, or headset HID vendor channel. Its EQ is host-side THX processing,
not a headset HID command.
