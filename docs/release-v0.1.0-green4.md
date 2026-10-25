# ESPNANYS-REZAR v0.1.0 green4

Windows x64 green build. No installer, no admin rights, no Razer/THX binaries.

Requires WebView2 and a locally installed Razer THX APO.

## Changes since green3

### App icon

- Final DeepSeek-chan bust icon: head-and-shoulders crop, no pointing hand,
  deep-blue vignette and cyan ring.
- Replaced the Tauri icon set (`ico`, `icns`, PNG, Square, StoreLogo) and the
  in-app favicon.
- The application executable and window/taskbar icon now use the bust artwork.

### Razer auto-update switch

- `razer-update-off.cmd` / `razer-update-on.cmd` now own the per-process
  firewall block as well as the hosts entries.
- One double-click no longer leaves firewall cleanup to a separate
  `block-razer-network.ps1 -Apply/-Remove` call.
- `-Disable` applies the firewall rule with `RazerInstaller.exe` as an explicit
  extra target; `-Enable` removes it.
- `-NoFirewall` keeps hosts-only behavior.
- Fixed `block-razer-network.ps1` dropping `-ExtraPath` targets in the default
  non-`-All` mode.
- `-Status` and the post-run verification report the firewall rule count.

## Assets

- `ESPNANYS-REZAR.exe` — one-file UI; extracts its own THX helper on first run.
- `ESPNANYS-REZAR-v0.1.0-win-x64-green4.zip` — one-file UI + update-switch
  scripts + README.
- `.sha256` for both assets.

## SHA256

- `ESPNANYS-REZAR.exe`: `0e1d5c99e35c8706b7a39ac882a9bc6a1e9a7068615238bd1319df9940fb8253`
- `ESPNANYS-REZAR-v0.1.0-win-x64-green4.zip`: `291613b52e41b684647eb0a4f8e68d47f95c5d71000826286405dd9e58fef7d2`

## Notes

- No Razer or THX binaries are bundled or redistributed.
- The THX APO must already be installed on the target PC.
- Windows 10/11 x64 + WebView2 Runtime.
