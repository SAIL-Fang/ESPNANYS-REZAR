# ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
# Author: SAIL
# Copyright (c) 2026 SAIL
# SPDX-License-Identifier: MIT
#requires -Version 5.1
<#
.SYNOPSIS
    找到本机 Razer 网络客户端，并按可执行文件创建 Windows 防火墙阻断规则。

.DESCRIPTION
    这个脚本做的是“禁止 Razer 软件联网”，不是关闭 Windows 防火墙。
    默认只处理 RazerAppEngine.exe（Synapse 的网络客户端），避免误伤安装器、
    卸载器、驱动安装程序和本地服务；
    规则只属于本脚本的组 "SAIL Razer Network Block"，不会修改其它防火墙规则，
    也不会改动 hosts、服务、计划任务或 Razer 文件。

    默认只发现并打印计划，不改系统；-Apply 才创建规则。
    应用/移除需要管理员权限；脚本会自动请求提权，-NoElevate 可禁用自动提权。

.PARAMETER List
    列出发现的 Razer 可执行文件和已有 SAIL 规则；不改系统。

.PARAMETER Apply
    创建入站/出站阻断规则。需要管理员权限。

.PARAMETER Remove
    只删除本脚本创建的规则；不影响其它防火墙规则。

.PARAMETER NoElevate
    不自动提权；需要管理员权限的操作在非管理员下直接报错。

.PARAMETER ExtraPath
    额外加入的 .exe 路径；用于自动发现漏掉的 Razer 组件。

.PARAMETER All
    扩展到所有 Razer 可执行文件（安装器、卸载器、服务、工具）。
    默认只阻断 RazerAppEngine.exe；不要和默认模式混用。

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\block-razer-network.ps1 -List

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\block-razer-network.ps1 -Apply

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\block-razer-network.ps1 -Remove
#>
[CmdletBinding()]
param(
    [switch]$List,
    [switch]$Apply,
    [switch]$Remove,
    [switch]$NoElevate,
    [switch]$All,
    [string[]]$ExtraPath = @()
)

$ErrorActionPreference = 'Stop'

$script:Group = 'SAIL Razer Network Block'
$script:RulePrefix = 'SAIL Razer block'
$script:All = [bool]$All
$script:NetworkClient = 'RazerAppEngine.exe'

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Write-Title([string]$Text) {
    Write-Host ''
    Write-Host "SAIL · $Text" -ForegroundColor White
    Write-Host ('-' * 64) -ForegroundColor DarkCyan
}

function Write-Ok([string]$Text) { Write-Host "  [ OK ] $Text" -ForegroundColor Green }
function Write-Warn([string]$Text) { Write-Host "  [WARN] $Text" -ForegroundColor Yellow }
function Write-Info([string]$Text) { Write-Host "  $Text" -ForegroundColor Gray }

function Get-ExecutableFromCommandLine([string]$CommandLine) {
    if ([string]::IsNullOrWhiteSpace($CommandLine)) { return $null }

    # Quoted service path: "C:\Program Files\Razer\foo.exe" -arg
    if ($CommandLine -match '^\s*"([^"]+\.exe)"') {
        return $Matches[1]
    }

    # Unquoted service path: C:\Program Files\Razer\foo.exe -arg
    if ($CommandLine -match '^\s*(.+?\.exe)(?:\s|$)') {
        return $Matches[1].Trim('"')
    }

    return $null
}

function Get-RazerExecutables {
    $found = [System.Collections.Generic.HashSet[string]]::new(
        [System.StringComparer]::OrdinalIgnoreCase
    )

    function Add-Candidate([string]$Path) {
        if ([string]::IsNullOrWhiteSpace($Path)) { return }
        if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return }

        $full = (Resolve-Path -LiteralPath $Path -ErrorAction SilentlyContinue).Path
        if ([string]::IsNullOrWhiteSpace($full)) { return }
        if ($full -notmatch '(?i)\.exe$') { return }

        # Do not accidentally block our own app if it lives under a Razer-like path.
        if ($full -match '(?i)ESPNANYS-REZAR|\\sail_[^\\]+\.exe$') { return }

        # Default mode: one network client application. RazerAppEngine spawns
        # its renderer/utility children from the same executable path.
        if (-not $script:All -and [IO.Path]::GetFileName($full) -ne $script:NetworkClient) {
            return
        }

        $found.Add($full) | Out-Null
    }

    # 1. Running processes.
    foreach ($process in @(Get-Process -ErrorAction SilentlyContinue)) {
        try {
            if ($process.Path -and $process.Path -match '(?i)\\Razer\\|Razer') {
                Add-Candidate $process.Path
            }
        } catch {
            # Access denied for a protected process; skip it.
        }
    }

    # 2. Windows services whose binary path mentions Razer.
    try {
        foreach ($service in @(Get-CimInstance Win32_Service -ErrorAction SilentlyContinue)) {
            if ($service.PathName -and $service.PathName -match '(?i)Razer') {
                Add-Candidate (Get-ExecutableFromCommandLine $service.PathName)
            }
        }
    } catch {
        Write-Warn "读取服务列表失败：$($_.Exception.Message)"
    }

    # 3. Known install/data roots, including per-user Razer AppEngine data.
    $roots = @(
        (Join-Path $env:ProgramFiles 'Razer'),
        (Join-Path ${env:ProgramFiles(x86)} 'Razer'),
        (Join-Path $env:ProgramData 'Razer'),
        (Join-Path $env:LOCALAPPDATA 'Razer'),
        (Join-Path $env:APPDATA 'Razer')
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Container) }

    foreach ($root in $roots) {
        foreach ($file in @(Get-ChildItem -LiteralPath $root -Recurse -File -Filter '*.exe' -ErrorAction SilentlyContinue)) {
            Add-Candidate $file.FullName
        }
    }

    # 4. Uninstall entries. InstallLocation is the useful bit.
    $uninstallRoots = @(
        'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
    )
    foreach ($uninstallRoot in $uninstallRoots) {
        try {
            $entries = Get-ItemProperty -Path $uninstallRoot -ErrorAction SilentlyContinue
            foreach ($entry in @($entries)) {
                if ($entry.DisplayName -notmatch '(?i)Razer|Synapse') { continue }
                if ($entry.InstallLocation) {
                    $location = [Environment]::ExpandEnvironmentVariables($entry.InstallLocation.Trim('"'))
                    if (Test-Path -LiteralPath $location -PathType Container) {
                        foreach ($file in @(Get-ChildItem -LiteralPath $location -Recurse -File -Filter '*.exe' -ErrorAction SilentlyContinue)) {
                            Add-Candidate $file.FullName
                        }
                    }
                }
            }
        } catch {
            # Uninstall keys are best-effort and version-dependent.
        }
    }

    # 5. Explicit paths supplied by the user.
    foreach ($path in $ExtraPath) {
        Add-Candidate $path
    }

    return @($found | Sort-Object)
}

function Get-SailRules {
    try {
        return @(Get-NetFirewallRule -Group $script:Group -ErrorAction SilentlyContinue)
    } catch {
        return @()
    }
}

function Remove-SailRules {
    $rules = Get-SailRules
    if ($rules.Count -eq 0) {
        Write-Info "没有找到 $($script:Group) 规则。"
        return
    }
    $rules | Remove-NetFirewallRule -ErrorAction Stop
    Write-Ok "已删除 $($rules.Count) 条 $($script:Group) 规则。"
}

function Install-SailRules([string[]]$Executables) {
    if ($Executables.Count -eq 0) {
        Write-Warn '没有发现 Razer 可执行文件；未创建规则。'
        return
    }

    Remove-SailRules
    $created = 0
    $index = 0
    foreach ($exe in $Executables) {
        $index += 1
        $leaf = [IO.Path]::GetFileName($exe)
        foreach ($direction in @('Outbound', 'Inbound')) {
            $displayName = "$($script:RulePrefix) $direction - $leaf ($index)"
            New-NetFirewallRule `
                -DisplayName $displayName `
                -Group $script:Group `
                -Direction $direction `
                -Program $exe `
                -Action Block `
                -Profile Any `
                -Enabled True `
                -Description "SAIL: block Razer software network access ($exe)" `
                -ErrorAction Stop | Out-Null
            $created += 1
        }
        Write-Info "blocked: $exe"
    }
    Write-Ok "已创建 $created 条规则（$index 个可执行文件，入站 + 出站）。"
}

function Show-RazerList([string[]]$Executables) {
    Write-Title 'Razer 网络客户端'
    if ($Executables.Count -eq 0) {
        Write-Warn '没有发现 Razer 可执行文件。'
    } else {
        foreach ($exe in $Executables) {
            Write-Info $exe
        }
    }

    Write-Title '现有 SAIL 防火墙规则'
    $rules = Get-SailRules
    if ($rules.Count -eq 0) {
        Write-Info "没有 $($script:Group) 规则。"
    } else {
        foreach ($rule in $rules) {
            $direction = (Get-NetFirewallRule -Name $rule.Name).Direction
            $enabled = (Get-NetFirewallRule -Name $rule.Name).Enabled
            Write-Info "$($rule.DisplayName) | $direction | enabled=$enabled"
        }
    }
}

# --- main -----------------------------------------------------------------

if (($Apply -and $Remove) -or ($List -and ($Apply -or $Remove))) {
    throw '请只选择 -List、-Apply、-Remove 之一。'
}

$needsAdmin = $Apply -or $Remove
if ($needsAdmin -and -not (Test-Admin)) {
    if ($NoElevate) {
        throw '需要管理员权限；请用管理员 PowerShell 运行，或去掉 -NoElevate。'
    }

    $arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -NoElevate"
    if ($Apply) { $arguments += ' -Apply' }
    if ($Remove) { $arguments += ' -Remove' }
    if ($All) { $arguments += ' -All' }

    Write-Warn '需要管理员权限，正在请求提权...'
    $process = Start-Process -FilePath (Get-Command powershell.exe).Source `
        -ArgumentList $arguments -Verb RunAs -Wait -PassThru
    if ($null -ne $process) { exit $process.ExitCode }
    exit 1
}

$executables = @(Get-RazerExecutables)

if ($List) {
    Show-RazerList $executables
    exit 0
}

if ($Remove) {
    Write-Title '移除 Razer 防火墙规则'
    Remove-SailRules
    exit 0
}

if ($Apply) {
    Write-Title '应用 Razer 防火墙规则'
    Install-SailRules $executables
    exit 0
}

# Default: safe preview. No system changes.
Write-Title 'Razer 网络隔离预览（未修改系统）'
if (-not $script:All) {
    Write-Info '模式：仅 RazerAppEngine.exe（Synapse 网络客户端）；全部 Razer 组件请用 -All。'
} else {
    Write-Info '模式：全部 Razer 可执行文件（-All）。'
}
if ($executables.Count -eq 0) {
    Write-Warn '没有发现 Razer 可执行文件。'
} else {
    Write-Info "发现 $($executables.Count) 个 Razer 可执行文件："
    foreach ($exe in $executables) {
        Write-Info "  $exe"
    }
}

Write-Host ''
Write-Host '这是预览，没有创建/删除任何防火墙规则。' -ForegroundColor Yellow
Write-Host '应用： powershell -ExecutionPolicy Bypass -File scripts\block-razer-network.ps1 -Apply' -ForegroundColor Gray
Write-Host '撤销： powershell -ExecutionPolicy Bypass -File scripts\block-razer-network.ps1 -Remove' -ForegroundColor Gray
Write-Host '说明：只阻断 Razer 软件联网，不关闭 Windows 防火墙，也不影响耳机 HID。' -ForegroundColor DarkGray
