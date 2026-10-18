# ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
# Author: SAIL
# Copyright (c) 2026 SAIL
# SPDX-License-Identifier: MIT
param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$profile = if ($Configuration -eq "Release") { "release" } else { "debug" }

if (-not $SkipBuild) {
    Write-Host "==> Building SAIL ($Configuration)"
    $buildArgs = @()
    if ($Configuration -eq "Release") { $buildArgs += "-Release" }
    & cmd.exe /c "`"$root\scripts\build.cmd`" $($buildArgs -join ' ')"
    if ($LASTEXITCODE -ne 0) { throw "build failed" }

    $uiArgs = if ($Configuration -eq "Release") { "build --no-bundle" } else { "build --debug --no-bundle" }
    & cmd.exe /c "`"$root\scripts\ui.cmd`" $uiArgs"
    if ($LASTEXITCODE -ne 0) { throw "UI build failed" }
}

$appDir = Join-Path $root "dist\SAIL"
$payload = Join-Path $appDir "payload"
New-Item -ItemType Directory -Force -Path $payload | Out-Null

$files = @(
    @{ src = Join-Path $root "target\$profile\sail_cli.exe"; name = "sail_cli.exe" },
    @{ src = Join-Path $root "target\$profile\sail_thx.exe"; name = "sail_thx.exe" },
    @{ src = Join-Path $root "ui\src-tauri\target\$profile\sail_ui.exe"; name = "sail_ui.exe" }
)
foreach ($file in $files) {
    if (-not (Test-Path $file.src)) { throw "Missing build output: $($file.src)" }
    Copy-Item $file.src (Join-Path $payload $file.name) -Force
}

Copy-Item (Join-Path $PSScriptRoot "install-sail.cmd") $appDir -Force
Copy-Item (Join-Path $PSScriptRoot "install-sail.ps1") $appDir -Force

@"
SAIL portable package
=====================

1. On the source PC, make a private THX backup:
   scripts\backup-thx.cmd -Destination D:\Backup\THX

2. Copy the resulting THX-* folder next to install-sail.cmd, or put it at:
   %USERPROFILE%\Documents\SAIL-Backup\THX-*

3. On the target PC, double-click install-sail.cmd.
   It installs SAIL, restores the THX APO/driver/service, and runs status checks.

The package intentionally does NOT contain THX/Razer binaries.
Do not redistribute THX/Razer files.
"@ | Out-File (Join-Path $appDir "README.txt") -Encoding utf8

$zip = Join-Path $root ("dist\SAIL-portable-" + $Configuration.ToLower() + ".zip")
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $appDir "*") -DestinationPath $zip -Force
Write-Host ""
Write-Host "Portable directory: $appDir"
Write-Host "Zip:                $zip"
