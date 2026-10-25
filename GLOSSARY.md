# Glossary

## Active device

The one supported Razer model the app is currently reading and writing. When
several supported devices are present, the user can select one explicitly;
the selection determines which model profile and preset set apply.

## Model profile

The product-specific description of a supported device: identity, transport,
available controls, EQ bands, and preset defaults. A model profile is not the
same thing as a hardware protocol; it may delegate to HID, THX, or both.

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
