# ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
# Author: SAIL
# Copyright (c) 2026 SAIL
# SPDX-License-Identifier: MIT
[CmdletBinding()]
param(
    [string]$Backup = "",
    [string]$AppSource = "",
    [string]$InstallDir = "$env:ProgramFiles\SAIL",
    [switch]$SkipThx,
    [switch]$DryRun,
    [switch]$NoShortcut
)

$ErrorActionPreference = "Stop"

function Say($message) { Write-Host "==> $message" }
function Warn($message) { Write-Warning $message }
function Fail($message) { throw $message }

function Test-Admin {
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Find-LatestBackup {
    $base = Join-Path $env:USERPROFILE "Documents\SAIL-Backup"
    if (-not (Test-Path $base)) { return $null }
    Get-ChildItem $base -Directory -Filter "THX-*" -ErrorAction SilentlyContinue |
        Where-Object { Test-Path (Join-Path $_.FullName "manifest.json") } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}

function Find-AppPayload {
    $root = Split-Path $PSScriptRoot -Parent
    $candidates = @(
        (Join-Path $PSScriptRoot "payload"),
        (Join-Path $root "dist\SAIL\payload")
    )
    foreach ($dir in $candidates) {
        if ((Test-Path (Join-Path $dir "sail_ui.exe")) -and
            (Test-Path (Join-Path $dir "sail_cli.exe")) -and
            (Test-Path (Join-Path $dir "sail_thx.exe"))) {
            return $dir
        }
    }
    # Development fallback: the two cargo target directories.
    $ui = Join-Path $root "ui\src-tauri\target\debug\sail_ui.exe"
    $cli = Join-Path $root "target\debug\sail_cli.exe"
    $thx = Join-Path $root "target\debug\sail_thx.exe"
    if ((Test-Path $ui) -and (Test-Path $cli) -and (Test-Path $thx)) {
        return @{ ui = $ui; cli = $cli; thx = $thx }
    }
    $ui = Join-Path $root "ui\src-tauri\target\release\sail_ui.exe"
    $cli = Join-Path $root "target\release\sail_cli.exe"
    $thx = Join-Path $root "target\release\sail_thx.exe"
    if ((Test-Path $ui) -and (Test-Path $cli) -and (Test-Path $thx)) {
        return @{ ui = $ui; cli = $cli; thx = $thx }
    }
    return $null
}

function Resolve-AppFiles($source) {
    if ($source -is [hashtable]) { return $source }
    return @{
        ui = Join-Path $source "sail_ui.exe"
        cli = Join-Path $source "sail_cli.exe"
        thx = Join-Path $source "sail_thx.exe"
    }
}

if (-not $Backup) {
    $Backup = Find-LatestBackup
}
if (-not $SkipThx) {
    if (-not $Backup) {
        Fail "No THX backup found. Run scripts\backup-thx.cmd on the source PC, copy the THX-* folder here, or pass -Backup <dir>."
    }
    if (-not (Test-Path (Join-Path $Backup "manifest.json"))) {
        Fail "Backup folder is missing manifest.json: $Backup"
    }
}

if ($AppSource) {
    $app = Resolve-AppFiles $AppSource
} else {
    $found = Find-AppPayload
    if (-not $found) {
        Fail "SAIL app binaries not found. Run scripts\package-sail.cmd first, or pass -AppSource <payload-dir>."
    }
    $app = Resolve-AppFiles $found
}
if (-not $app) {
    Fail "SAIL app binaries not found. Run scripts\package-sail.cmd first, or pass -AppSource <payload-dir>."
}
foreach ($key in @("ui", "cli", "thx")) {
    if (-not (Test-Path $app[$key])) { Fail "Missing app binary: $($app[$key])" }
}

if (-not $DryRun -and -not (Test-Admin)) {
    Fail "Run this installer from an elevated prompt (right-click install-sail.cmd -> Run as administrator)."
}

Say "Backup:      $Backup"
Say "App payload: $($app.ui) | $($app.cli) | $($app.thx)"
Say "Install dir: $InstallDir"
Say "THX restore: $(if ($SkipThx) { 'skipped' } else { 'enabled' })"

if ($DryRun) {
    Say "DRY RUN: no files, drivers, services or registry keys were changed."
    return
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$rollback = Join-Path $env:ProgramData "SAIL\rollback\$stamp"
New-Item -ItemType Directory -Force -Path $rollback | Out-Null
Say "Rollback snapshot: $rollback"

# Snapshot the target before touching drivers/services.
& reg.exe export "HKLM\SOFTWARE\THX" (Join-Path $rollback "HKLM-SOFTWARE-THX.reg") /y 2>$null | Out-Null
& reg.exe export "HKCU\SOFTWARE\THX" (Join-Path $rollback "HKCU-SOFTWARE-THX.reg") /y 2>$null | Out-Null
& reg.exe export "HKLM\SYSTEM\CurrentControlSet\Services\VSSrv" (Join-Path $rollback "VSSrv-service.reg") /y 2>$null | Out-Null
& pnputil.exe /enum-drivers 2>$null | Out-File (Join-Path $rollback "drivers-before.txt") -Encoding utf8
Copy-Item "C:\Windows\System32\VSSrv.exe" (Join-Path $rollback "VSSrv.exe") -Force -ErrorAction SilentlyContinue

Say "Install SAIL app"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item $app.ui (Join-Path $InstallDir "sail_ui.exe") -Force
Copy-Item $app.cli (Join-Path $InstallDir "sail_cli.exe") -Force
Copy-Item $app.thx (Join-Path $InstallDir "sail_thx.exe") -Force

# The UI looks for the THX helper in LocalAppData; keep that contract.
$localBin = Join-Path $env:LOCALAPPDATA "SAIL\bin"
New-Item -ItemType Directory -Force -Path $localBin | Out-Null
Copy-Item $app.cli (Join-Path $localBin "sail_cli.exe") -Force
Copy-Item $app.thx (Join-Path $localBin "sail_thx.exe") -Force

if (-not $NoShortcut) {
    Say "Create Start Menu shortcut"
    $startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
    New-Item -ItemType Directory -Force -Path $startMenu | Out-Null
    $ws = New-Object -ComObject WScript.Shell
    $lnk = $ws.CreateShortcut((Join-Path $startMenu "SAIL.lnk"))
    $lnk.TargetPath = Join-Path $InstallDir "sail_ui.exe"
    $lnk.WorkingDirectory = $InstallDir
    $lnk.Description = "SAIL headset control"
    $lnk.Save()
}

if (-not $SkipThx) {
    Say "Restore THX driver packages"
    $infs = Get-ChildItem (Join-Path $Backup "driver-store") -Recurse -Filter "*.inf" -ErrorAction SilentlyContinue
    if (-not $infs) { Warn "No .inf files in backup driver-store; skipping driver restore." }
    foreach ($inf in $infs) {
        Write-Host "  pnputil /add-driver $($inf.FullName) /install"
        & pnputil.exe /add-driver $inf.FullName /install | Out-Host
        if ($LASTEXITCODE -ne 0) {
            Warn "pnputil returned $LASTEXITCODE for $($inf.Name); continuing (may already be installed)."
        }
    }

    Say "Restore VSSrv service"
    $serviceExe = Join-Path $Backup "service\VSSrv.exe"
    if (Test-Path $serviceExe) {
        Copy-Item $serviceExe "C:\Windows\System32\VSSrv.exe" -Force
    }
    $serviceReg = Join-Path $Backup "service\VSSrv-service.reg"
    if (Test-Path $serviceReg) {
        & reg.exe import $serviceReg | Out-Host
    }
    & sc.exe start VSSrv 2>$null | Out-Host

    Say "Restore ThxV3Native.dll"
    $dll = Get-ChildItem (Join-Path $Backup "dll") -Filter "ThxV3Native*.dll" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($dll) {
        $thxDir = Join-Path $env:LOCALAPPDATA "Razer\RazerAppEngine\User Data\Apps\Common\thxv3common"
        New-Item -ItemType Directory -Force -Path $thxDir | Out-Null
        Copy-Item $dll.FullName (Join-Path $thxDir $dll.Name) -Force
    } else {
        Warn "No ThxV3Native DLL in backup."
    }

    Say "Restore THX registry and profiles"
    foreach ($reg in @("HKLM-SOFTWARE-THX.reg", "HKCU-Software-THX.reg")) {
        $path = Join-Path $Backup "registry\$reg"
        if (Test-Path $path) { & reg.exe import $path | Out-Host }
    }
    $profiles = Join-Path $Backup "programdata-thx\THX"
    if (Test-Path $profiles) {
        New-Item -ItemType Directory -Force -Path "C:\ProgramData\THX" | Out-Null
        Copy-Item (Join-Path $profiles "*") "C:\ProgramData\THX" -Recurse -Force
    }
}

Say "Self-check: HID"
& (Join-Path $InstallDir "sail_cli.exe") status
if ($LASTEXITCODE -ne 0) { Warn "HID status failed; check the headset/dongle." }

if (-not $SkipThx) {
    Say "Self-check: THX"
    & (Join-Path $InstallDir "sail_thx.exe") status
    if ($LASTEXITCODE -ne 0) {
        Warn "THX status failed. Reboot first; if it still fails, restore the official THX/Razer installer or use the rollback snapshot."
    }
}

Write-Host ""
Write-Host "SAIL installed to: $InstallDir"
Write-Host "Rollback snapshot: $rollback"
Write-Host "A reboot is recommended before relying on the restored THX APO."
Write-Host "Run 'SAIL' from the Start Menu, or: $InstallDir\sail_ui.exe"
