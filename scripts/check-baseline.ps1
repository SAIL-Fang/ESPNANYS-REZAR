# ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
# Author: SAIL
# Copyright (c) 2026 SAIL
# SPDX-License-Identifier: MIT
param(
    [string]$Baseline = ""
)

$ErrorActionPreference = "Continue"

if (-not $Baseline) {
    $Baseline = Join-Path (Split-Path $PSScriptRoot -Parent) "docs\thx-baseline.json"
}
if (-not (Test-Path $Baseline)) {
    Write-Error "Baseline not found: $Baseline"
    exit 2
}

$expected = Get-Content $Baseline -Raw | ConvertFrom-Json
$script:failed = 0
$script:passed = 0
$script:unknown = 0

function Result($name, $ok, $detail) {
    if ($ok -eq $true) {
        $script:passed++
        Write-Host ("PASS  {0}: {1}" -f $name, $detail)
    } elseif ($ok -eq $false) {
        $script:failed++
        Write-Host ("FAIL  {0}: {1}" -f $name, $detail) -ForegroundColor Red
    } else {
        $script:unknown++
        Write-Host ("WARN  {0}: {1}" -f $name, $detail) -ForegroundColor Yellow
    }
}

Write-Host ("SAIL baseline: {0}" -f $Baseline)
Write-Host ("Frozen at:     {0}" -f $expected.frozen_at)
Write-Host ""

# 1) THX DLL
$dllDir = Join-Path $env:LOCALAPPDATA "Razer\RazerAppEngine\User Data\Apps\Common\thxv3common"
$dlls = @(Get-ChildItem $dllDir -Filter "ThxV3Native_v*.dll" -ErrorAction SilentlyContinue)
if ($dlls.Count -eq 0) {
    Result "THX DLL" $false "no ThxV3Native_v*.dll under $dllDir"
} else {
    $match = $null
    foreach ($dll in $dlls) {
        $hash = (Get-FileHash $dll.FullName -Algorithm SHA256).Hash
        if ($hash -eq $expected.thx.dll_sha256) { $match = $dll; break }
    }
    if ($match) {
        Result "THX DLL" $true ("{0} sha256 matches" -f $match.Name)
    } else {
        $names = ($dlls | ForEach-Object { $_.Name }) -join ", "
        Result "THX DLL" $false ("expected {0}; found {1}" -f $expected.thx.dll_name, $names)
    }
}

# 2) VSSrv
$vssrv = "C:\Windows\System32\VSSrv.exe"
if (Test-Path $vssrv) {
    $hash = (Get-FileHash $vssrv -Algorithm SHA256).Hash
    $version = (Get-Item $vssrv).VersionInfo.FileVersion
    Result "VSSrv" ($hash -eq $expected.thx.vssrv_sha256) ("version {0}; sha256 {1}" -f $version, $hash)
} else {
    Result "VSSrv" $false "$vssrv not found"
}

# 3) Driver packages
$enum = (& pnputil.exe /enum-drivers) -join "`n"
foreach ($pkg in $expected.thx.driver_packages) {
    $present = $enum -match [regex]::Escape($pkg.original)
    Result ("driver " + $pkg.original) $present ("expected {0}" -f $pkg.published)
}

# 4) Firmware (read-only HID)
$cliCandidates = @(
    (Join-Path $env:LOCALAPPDATA "SAIL\bin\sail_cli.exe"),
    (Join-Path (Split-Path $PSScriptRoot -Parent) "target\debug\sail_cli.exe"),
    (Join-Path (Split-Path $PSScriptRoot -Parent) "target\release\sail_cli.exe")
)
$cli = $cliCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $cli) {
    Result "firmware" $null "sail_cli not staged; skip HID check"
} else {
    $status = (& $cli status 2>&1) -join "`n"
    if ($status -match "firmware:\s*(\S+)") {
        $fw = $Matches[1]
        Result "firmware" ($fw -eq $expected.device.firmware) ("expected {0}; got {1}" -f $expected.device.firmware, $fw)
    } else {
        Result "firmware" $null "could not read firmware from sail_cli output"
    }
}

# 5) Dependency locks (do not run cargo: rustup may try to sync here)
$root = Split-Path $PSScriptRoot -Parent
foreach ($lock in @("Cargo.lock", "ui\package-lock.json", "ui\src-tauri\Cargo.lock")) {
    $path = Join-Path $root $lock
    Result ("lockfile " + $lock) (Test-Path $path) ("expected rust " + $expected.toolchain.rust)
}

Write-Host ""
Write-Host ("Summary: {0} passed, {1} failed, {2} unknown" -f $script:passed, $script:failed, $script:unknown)
if ($script:failed -gt 0) { exit 1 }
exit 0
