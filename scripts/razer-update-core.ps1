# ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
# Author: SAIL
# Copyright (c) 2026 SAIL
# SPDX-License-Identifier: MIT
#requires -Version 5.1
<#
.SYNOPSIS
    SAIL · 控制 Razer Synapse / THX 自动更新。

.DESCRIPTION
    只做两件事：
      1. 在 hosts 中把更新 CDN app-assets.razer.com 指向 0.0.0.0；
      2. 禁用计划任务 "Razer Upgrade Elevation Service"。

    不会修改：
      - apps.razer.com（Synapse 远程 UI）
      - VSSrv / THX APO / 本地回环控制通道
      - Razer AppEngine / Elevation Service / Game Manager 服务
      - Windows 防火墙规则

    默认只读；-Disable / -Enable 需要管理员权限，脚本会自动请求提权。

.PARAMETER Status
    只读查看当前状态（默认）。

.PARAMETER Disable
    挡更新 CDN + 禁用升级任务。

.PARAMETER Enable
    恢复 hosts + 启用升级任务。

.PARAMETER NoElevate
    不自动提权；非管理员时直接报错。用于自动化。

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\razer-update-core.ps1 -Status

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\razer-update-core.ps1 -Disable

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\razer-update-core.ps1 -Enable
#>
[CmdletBinding()]
param(
    [switch]$Status,
    [switch]$Disable,
    [switch]$Enable,
    [switch]$NoElevate
)

$ErrorActionPreference = 'Stop'

$script:HostsPath = Join-Path $env:WINDIR 'System32\drivers\etc\hosts'
$script:Domain = 'app-assets.razer.com'
$script:Marker = '# SAIL: block Razer updater CDN'
$script:TaskName = 'Razer Upgrade Elevation Service'
$script:HostLine = "0.0.0.0 $($script:Domain) $($script:Marker)"
$script:ExitCode = 0

function Test-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Write-Title([string]$Text) {
    Write-Host ''
    Write-Host "SAIL · $Text" -ForegroundColor White
    Write-Host ('-' * 64) -ForegroundColor DarkCyan
}

function Write-Ok([string]$Text) { Write-Host "  [ OK ] $Text" -ForegroundColor Green }
function Write-Warn([string]$Text) { Write-Host "  [WARN] $Text" -ForegroundColor Yellow }
function Write-Fail([string]$Text) { Write-Host "  [FAIL] $Text" -ForegroundColor Red }
function Write-Info([string]$Text) { Write-Host "  $Text" -ForegroundColor Gray }

function Invoke-Elevated([string]$Argument) {
    if ($NoElevate) {
        throw "需要管理员权限；请用管理员 PowerShell 运行，或去掉 -NoElevate。"
    }
    $ps = (Get-Command powershell.exe).Source
    $file = '"' + $PSCommandPath + '"'
    $argList = "-NoProfile -ExecutionPolicy Bypass -NoExit -File $file $Argument"
    Write-Host '需要管理员权限，正在请求提权...' -ForegroundColor Yellow
    Start-Process -FilePath $ps -Verb RunAs -ArgumentList $argList | Out-Null
    Write-Host '已打开管理员窗口；请在那里查看结果。' -ForegroundColor Gray
}

function Get-HostsLines {
    if (-not (Test-Path -LiteralPath $script:HostsPath)) {
        throw "找不到 hosts 文件：$($script:HostsPath)"
    }
    return @(Get-Content -LiteralPath $script:HostsPath -ErrorAction Stop)
}

function Get-BlockLines {
    $lines = Get-HostsLines
    return @($lines | Where-Object {
            $_ -match '(?i)app-assets\.razer\.com' -or $_ -match [regex]::Escape($script:Marker)
        })
}

function Get-Task {
    try {
        return Get-ScheduledTask -TaskName $script:TaskName -ErrorAction Stop
    } catch {
        return $null
    }
}

function Get-DnsAddresses {
    try {
        return @([System.Net.Dns]::GetHostAddresses($script:Domain) |
                ForEach-Object { $_.IPAddressToString } |
                Sort-Object -Unique)
    } catch {
        return @()
    }
}

function Backup-Hosts {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $backup = "$($script:HostsPath).sail-$stamp.bak"
    Copy-Item -LiteralPath $script:HostsPath -Destination $backup -Force
    $fixed = "$($script:HostsPath).sail-backup"
    if (-not (Test-Path -LiteralPath $fixed)) {
        Copy-Item -LiteralPath $script:HostsPath -Destination $fixed -Force
    }
    Write-Ok "hosts 已备份：$backup"
    return $backup
}

function Set-HostsBlocked([bool]$Blocked) {
    $lines = Get-HostsLines
    $kept = @($lines | Where-Object {
            $_ -notmatch '(?i)app-assets\.razer\.com' -and
            $_ -notmatch [regex]::Escape($script:Marker)
        })

    if ($Blocked) {
        # 去掉文件末尾多余空行，再追加一条
        while ($kept.Count -gt 0 -and [string]::IsNullOrWhiteSpace($kept[$kept.Count - 1])) {
            $kept = @($kept[0..($kept.Count - 2)])
        }
        $kept += $script:HostLine
    }

    $encoding = New-Object System.Text.ASCIIEncoding
    [System.IO.File]::WriteAllLines($script:HostsPath, [string[]]$kept, $encoding)
}

function Show-Status {
    Write-Title 'Razer 自动更新状态'

    $admin = Test-Admin
    Write-Info ("管理员: " + $(if ($admin) { '是' } else { '否' }))
    Write-Info "hosts: $($script:HostsPath)"

    $block = @(Get-BlockLines)
    if ($block.Count -gt 0) {
        Write-Ok "hosts 已包含屏蔽项：$($block[0])"
    } else {
        Write-Warn "hosts 没有屏蔽 $($script:Domain)"
    }

    $dns = @(Get-DnsAddresses)
    if ($dns.Count -eq 0) {
        if ($block.Count -gt 0) {
            Write-Ok "DNS 已无法解析 $($script:Domain)（hosts 已挡）"
        } else {
            Write-Warn "DNS 当前解析不到 $($script:Domain)（可能是网络/DNS问题，不代表已封锁）"
        }
    } elseif ($dns -contains '0.0.0.0') {
        Write-Ok "$($script:Domain) -> 0.0.0.0（更新下载被挡）"
    } else {
        Write-Warn "$($script:Domain) -> $($dns -join ', ')（仍可下载更新）"
    }

    $task = Get-Task
    if (-not $task) {
        Write-Warn "计划任务不存在：$($script:TaskName)"
    } elseif ($task.State -eq 'Disabled') {
        Write-Ok "计划任务已禁用：$($script:TaskName)"
    } else {
        Write-Warn "计划任务仍启用：$($script:TaskName) ($($task.State))"
    }

    $vssrv = Get-Service -Name 'VSSrv' -ErrorAction SilentlyContinue
    if ($vssrv) {
        Write-Info "THX VSSrv: $($vssrv.Status) / $($vssrv.StartType)"
    } else {
        Write-Warn '未找到 VSSrv 服务（THX 可能未安装）'
    }

    $thx = Join-Path $env:LOCALAPPDATA 'SAIL\bin\sail_thx.exe'
    if (Test-Path -LiteralPath $thx) {
        Write-Info "THX 探针: $thx"
    } else {
        Write-Info "THX 探针未 staging（不影响本脚本；THX 诊断前可跑 scripts\build.cmd -Stage）"
    }

    Write-Host ''
    Write-Host '不会受影响：' -ForegroundColor Gray
    Write-Host '  · apps.razer.com（Synapse UI）' -ForegroundColor Gray
    Write-Host '  · VSSrv / THX APO / 127.0.0.1 控制通道' -ForegroundColor Gray
    Write-Host '  · HID、设备设置、声音' -ForegroundColor Gray
    Write-Host '会被阻止：' -ForegroundColor Gray
    Write-Host "  · $($script:Domain) 上的 RazerAppEngineUpgrade-*.exe 下载" -ForegroundColor Gray
    Write-Host "  · 计划任务 $($script:TaskName)" -ForegroundColor Gray
}

function Show-Verification([string]$Mode) {
    Write-Title '执行后验证'

    $failed = $false
    $block = @(Get-BlockLines)
    $dns = @(Get-DnsAddresses)
    $task = Get-Task
    $vssrv = Get-Service -Name 'VSSrv' -ErrorAction SilentlyContinue
    $elev = Get-Service -Name 'Razer Elevation Service' -ErrorAction SilentlyContinue

    if ($Mode -eq 'Disable') {
        if ($block.Count -gt 0) {
            Write-Ok 'hosts: 屏蔽项存在'
        } else {
            Write-Fail 'hosts: 没有找到屏蔽项'
            $failed = $true
        }

        if (-not $task) {
            Write-Warn "升级任务: 不存在（$($script:TaskName)）"
        } elseif ($task.State -eq 'Disabled') {
            Write-Ok "升级任务: Disabled（$($script:TaskName)）"
        } else {
            Write-Fail "升级任务: $($task.State)，预期 Disabled"
            $failed = $true
        }

        if ($dns.Count -eq 0 -or $dns -contains '0.0.0.0') {
            Write-Ok "DNS: $($script:Domain) 已不可达"
        } else {
            Write-Warn "DNS: 仍解析到 $($dns -join ', ')（hosts 已写入，但可能被 DNS 缓存/代理影响）"
        }
    } else {
        if ($block.Count -eq 0) {
            Write-Ok 'hosts: 屏蔽项已移除'
        } else {
            Write-Fail 'hosts: 仍存在屏蔽项'
            $failed = $true
        }

        if (-not $task) {
            Write-Warn "升级任务: 不存在（$($script:TaskName)）"
        } elseif ($task.State -ne 'Disabled') {
            Write-Ok "升级任务: $($task.State)（已恢复）"
        } else {
            Write-Fail '升级任务: 仍为 Disabled'
            $failed = $true
        }
    }

    if ($vssrv -and $vssrv.Status -eq 'Running') {
        Write-Ok "THX VSSrv: Running（未被本脚本修改）"
    } else {
        Write-Warn "THX VSSrv: $(if ($vssrv) { $vssrv.Status } else { '未找到' })（请另跑 diagnose-thx.cmd）"
    }

    if ($elev) {
        Write-Info "Razer Elevation Service: $($elev.Status) / $($elev.StartType)（本脚本未修改）"
    } else {
        Write-Warn 'Razer Elevation Service: 未找到（本脚本未修改）'
    }

    if ($failed) {
        Write-Fail '验证失败：请查看上面的 FAIL 项。'
        $script:ExitCode = 1
    } else {
        Write-Ok '验证通过。'
    }
}

function Invoke-Disable {
    if (-not (Test-Admin)) {
        Invoke-Elevated '-Disable'
        return
    }

    Write-Title '关闭 Razer 自动更新'
    [void](Backup-Hosts)
    Set-HostsBlocked $true
    Write-Ok "hosts: $($script:HostLine)"
    [void](ipconfig.exe /flushdns 2>&1)

    $task = Get-Task
    if ($task) {
        try {
            Disable-ScheduledTask -TaskName $script:TaskName -ErrorAction Stop | Out-Null
            Write-Ok "已禁用计划任务：$($script:TaskName)"
        } catch {
            Write-Fail "禁用计划任务失败：$($_.Exception.Message)"
        }
    } else {
        Write-Warn "计划任务不存在：$($script:TaskName)"
    }

    Write-Host ''
    Write-Ok '完成：更新 CDN 已挡，升级任务已禁用。'
    Write-Info 'THX 控制仍走 127.0.0.1 -> VSSrv，不受影响。'
    Write-Info '恢复：运行 razer-update-on.cmd。'
    Write-Host ''
    Show-Verification 'Disable'
    Show-Status
}

function Invoke-Enable {
    if (-not (Test-Admin)) {
        Invoke-Elevated '-Enable'
        return
    }

    Write-Title '恢复 Razer 自动更新'
    [void](Backup-Hosts)
    Set-HostsBlocked $false
    Write-Ok "hosts: 已移除 $($script:Domain) 屏蔽项"
    [void](ipconfig.exe /flushdns 2>&1)

    $task = Get-Task
    if ($task) {
        try {
            Enable-ScheduledTask -TaskName $script:TaskName -ErrorAction Stop | Out-Null
            Write-Ok "已启用计划任务：$($script:TaskName)"
        } catch {
            Write-Fail "启用计划任务失败：$($_.Exception.Message)"
        }
    } else {
        Write-Warn "计划任务不存在：$($script:TaskName)"
    }

    Write-Host ''
    Write-Ok '完成：hosts 与升级任务已恢复。'
    Write-Host ''
    Show-Verification 'Enable'
    Show-Status
}

if ($Disable -and $Enable) {
    throw '不能同时使用 -Disable 和 -Enable。'
}

$mode = if ($Disable) { 'Disable' } elseif ($Enable) { 'Enable' } else { 'Status' }

Write-Host ''
Write-Host 'SAIL · Razer 自动更新控制' -ForegroundColor White
Write-Host ('模式 {0}   管理员 {1}' -f $mode, $(if (Test-Admin) { '是' } else { '否' })) -ForegroundColor DarkGray

switch ($mode) {
    'Disable' { Invoke-Disable }
    'Enable' { Invoke-Enable }
    default { Show-Status }
}

if ($script:ExitCode -ne 0) {
    exit $script:ExitCode
}
