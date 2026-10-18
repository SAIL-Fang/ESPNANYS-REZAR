# ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
# Author: SAIL
# Copyright (c) 2026 SAIL
# SPDX-License-Identifier: MIT
param(
    [switch]$Release,
    [switch]$Stage
)

$ErrorActionPreference = "Stop"

$vswhere = "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"
$install = $null
if (Test-Path $vswhere) {
    $install = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
}
if (-not $install) {
    $install = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools"
}
$vcvars = Join-Path $install "VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path $vcvars)) {
    throw "vcvars64.bat not found. Install the Visual Studio C++ workload first."
}

# Import the MSVC environment into this PowerShell process.
cmd /c "call `"$vcvars`" >nul 2>&1 && set" | ForEach-Object {
    if ($_ -match '^([^=]+)=(.*)$') {
        Set-Item -Path "env:$($matches[1])" -Value $matches[2]
    }
}

$env:CRATE_CC_NO_DEFAULTS = "1"
$env:CFLAGS = "/MD /O2"

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
$cargoArgs = @("build")
if ($Release) { $cargoArgs += "--release" }
& $cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if ($Stage) {
    $profile = if ($Release) { "release" } else { "debug" }
    $stageDir = Join-Path $env:LOCALAPPDATA "SAIL\bin"
    New-Item -ItemType Directory -Force -Path $stageDir | Out-Null
    Copy-Item "target\$profile\sail_cli.exe" $stageDir -Force
    Copy-Item "target\$profile\sail_thx.exe" $stageDir -Force
    Write-Host "staged: $stageDir\sail_cli.exe"
    Write-Host "staged: $stageDir\sail_thx.exe"
    Write-Host "Use SAIL_THX_HOST=$stageDir\sail_thx.exe for the UI."
}
