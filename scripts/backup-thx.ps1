param(
    [string]$Destination = "",
    [switch]$SkipProfiles
)

$ErrorActionPreference = "Stop"

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Run this script from an elevated PowerShell (driver export needs admin)."
}

if (-not $Destination) {
    $Destination = Join-Path $env:USERPROFILE ("Documents\SAIL-Backup\THX-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
}
$root = $Destination
New-Item -ItemType Directory -Force -Path $root | Out-Null

function Write-Step($message) {
    Write-Host "==> $message"
}

function Copy-Tree($source, $target) {
    if (-not (Test-Path -LiteralPath $source)) { return }
    New-Item -ItemType Directory -Force -Path $target | Out-Null
    Copy-Item -LiteralPath $source -Destination $target -Recurse -Force -ErrorAction SilentlyContinue
}

$manifest = [ordered]@{
    created_at = (Get-Date).ToString("o")
    computer = $env:COMPUTERNAME
    user = $env:USERNAME
    windows = (Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber)
    thx_dll = $null
    thx_dll_sha256 = $null
    vssrv = $null
    vssrv_version = $null
    firmware = $null
    driver_packages = @()
    notes = @(
        "Personal backup for interoperability/recovery only.",
        "Do not commit or redistribute THX/Razer binaries.",
        "Prefer the official THX/Razer installer for deployment to another PC."
    )
}

Write-Step "THX driver packages"
$driverDir = Join-Path $root "driver-store"
New-Item -ItemType Directory -Force -Path $driverDir | Out-Null
$enum = (& pnputil.exe /enum-drivers) -join "`n"
$blocks = $enum -split "(`r?`n){2,}"
$exported = @()
foreach ($block in $blocks) {
    if ($block -notmatch 'Published Name:\s*(\S+)') { continue }
    $published = $Matches[1]
    if ($block -notmatch 'Original Name:\s*(\S+)') { continue }
    $original = $Matches[1]
    if ($original -notmatch 'thx') { continue }
    $target = Join-Path $driverDir ([IO.Path]::GetFileNameWithoutExtension($published))
    New-Item -ItemType Directory -Force -Path $target | Out-Null
    Write-Host "  export $published ($original)"
    & pnputil.exe /export-driver $published $target | Out-Host
    if ($LASTEXITCODE -eq 0) {
        $exported += [ordered]@{ published = $published; original = $original; directory = $target }
    }
}
$manifest.driver_packages = $exported

# Fallback: known package names from the captured install state.
if ($exported.Count -eq 0) {
    foreach ($inf in @("oem40.inf", "oem45.inf", "oem46.inf", "oem48.inf", "oem49.inf")) {
        $target = Join-Path $driverDir ([IO.Path]::GetFileNameWithoutExtension($inf))
        New-Item -ItemType Directory -Force -Path $target | Out-Null
        & pnputil.exe /export-driver $inf $target | Out-Host
        if ($LASTEXITCODE -eq 0) {
            $exported += [ordered]@{ published = $inf; original = ""; directory = $target }
        }
    }
    $manifest.driver_packages = $exported
}

Write-Step "VSSrv service"
$serviceDir = Join-Path $root "service"
New-Item -ItemType Directory -Force -Path $serviceDir | Out-Null
Copy-Item "C:\Windows\System32\VSSrv.exe" (Join-Path $serviceDir "VSSrv.exe") -Force -ErrorAction SilentlyContinue
& sc.exe qc VSSrv | Out-File (Join-Path $serviceDir "sc-qc.txt") -Encoding utf8
& reg.exe export "HKLM\SYSTEM\CurrentControlSet\Services\VSSrv" (Join-Path $serviceDir "VSSrv-service.reg") /y | Out-Null
$manifest.vssrv = "C:\Windows\System32\VSSrv.exe"
$manifest.vssrv_version = (Get-Item "C:\Windows\System32\VSSrv.exe").VersionInfo.FileVersion

Write-Step "ThxV3Native.dll"
$dll = Join-Path $env:LOCALAPPDATA "Razer\RazerAppEngine\User Data\Apps\Common\thxv3common\ThxV3Native_v1.1.42.1.dll"
$dllDir = Join-Path $root "dll"
New-Item -ItemType Directory -Force -Path $dllDir | Out-Null
Copy-Item -LiteralPath $dll (Join-Path $dllDir (Split-Path $dll -Leaf)) -Force -ErrorAction SilentlyContinue
$manifest.thx_dll = $dll
if (Test-Path $dll) {
    $manifest.thx_dll_sha256 = (Get-FileHash $dll -Algorithm SHA256).Hash
}

Write-Step "Registry"
$regDir = Join-Path $root "registry"
New-Item -ItemType Directory -Force -Path $regDir | Out-Null
& reg.exe export "HKLM\SOFTWARE\THX" (Join-Path $regDir "HKLM-SOFTWARE-THX.reg") /y | Out-Null
& reg.exe export "HKCU\Software\THX" (Join-Path $regDir "HKCU-Software-THX.reg") /y | Out-Null

Write-Step "THX profiles"
if (-not $SkipProfiles) {
    Copy-Tree "C:\ProgramData\THX" (Join-Path $root "programdata-thx")
}

Write-Step "Firmware metadata (not a flashable image)"
$fwDir = Join-Path $root "firmware"
New-Item -ItemType Directory -Force -Path $fwDir | Out-Null
$cli = Join-Path $env:LOCALAPPDATA "SAIL\bin\sail_cli.exe"
if (Test-Path $cli) {
    $status = & $cli status 2>&1 | Out-String
    $status | Out-File (Join-Path $fwDir "device-status.txt") -Encoding utf8
    $manifest.firmware = ($status -split "`r?`n" | Where-Object { $_ -match '^firmware:' }) -join ""
} else {
    "sail_cli not staged; run scripts\build.cmd -Stage first." |
        Out-File (Join-Path $fwDir "device-status.txt") -Encoding utf8
}
Copy-Tree (Join-Path $env:LOCALAPPDATA "Razer\RazerAppEngine\User Data\Apps\Common\FWUpdate") $fwDir
@"
Firmware is not stored as a plain .bin in the Synapse cache on this machine.
This folder contains the updater SDK and the current firmware version only.
Use the official Razer updater to change firmware; do not flash a copied image.
"@ | Out-File (Join-Path $fwDir "README.txt") -Encoding utf8

Write-Step "Manifest"
$manifest | ConvertTo-Json -Depth 6 | Out-File (Join-Path $root "manifest.json") -Encoding utf8

@"
SAIL / THX private backup
=========================

Created: $($manifest.created_at)
This folder contains proprietary THX/Razer binaries and registry exports.
Keep it private. Do not commit it to the SAIL repository or redistribute it.

Restore (test in a VM first):
1. Install the official THX/Razer package if you have it. That is safer than
   restoring driver-store packages by hand.
2. Otherwise:
   pnputil /add-driver driver-store\oem40\*.inf /install
   (repeat for oem41, oem45, oem46, oem48, oem49)
3. Copy service\VSSrv.exe to C:\Windows\System32, import
   service\VSSrv-service.reg, then: sc start VSSrv
4. Copy dll\ThxV3Native_v1.1.42.1.dll back to the Razer app-data path.
5. Import registry\HKLM-SOFTWARE-THX.reg and HKCU-Software-THX.reg.
6. Reboot and test with:
   %LOCALAPPDATA%\SAIL\bin\sail_thx.exe status

Firmware:
- The current version is recorded in firmware\device-status.txt.
- No flashable firmware image was present in the Synapse cache.
- Use the official Razer updater for firmware changes.
"@ | Out-File (Join-Path $root "RESTORE.txt") -Encoding utf8

Write-Host ""
Write-Host "Backup written to: $root"
Write-Host "Keep it private; do not add it to the open-source repository."
