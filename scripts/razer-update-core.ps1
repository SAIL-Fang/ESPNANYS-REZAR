# ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
# Author: SAIL
# Copyright (c) 2026 SAIL
# SPDX-License-Identifier: MIT
#requires -Version 5.1
<#
.SYNOPSIS
    SAIL · 控制 Razer Synapse / THX 自动更新。

.DESCRIPTION
    做三件事：
      1. 在 hosts 中把 Razer 更新控制面域名指向 0.0.0.0，按角色分三组：
           - 更新清单：manifest.razerapi.com、manifest3.razerapi.com
           - 安装包：app-assets.razer.com、cdn.razersynapse.com、
             manifest-assets.razersynapse.com
           - 端点发现：discovery.razerapi.com、discovery3.razerapi.com
      2. 禁用计划任务 "Razer Upgrade Elevation Service"。
      3. 按进程封网：调用同目录 block-razer-network.ps1，给 RazerAppEngine.exe
         和 RazerInstaller.exe 建入站/出站 Block 规则，规则组
         "SAIL Razer Network Block"。hosts 会被雷神加速器之类的工具改写，
         这一层是兜底；不需要时用 -NoFirewall 跳过。

    不会修改：
      - apps.razer.com 的 DNS 解析（hosts 不挡它）。但第 3 步封的是进程：
        Synapse 面板本身就是从 apps.razer.com 拉远程 UI，所以关闭期间面板/
        登录/云同步不可用是预期代价，不是脚本出错。
      - VSSrv / THX APO / 本地回环控制通道（Windows 防火墙不过滤 127.0.0.1）
      - Razer 的服务与驱动、其它防火墙规则（只增删本脚本组内的规则）

    代价：关闭期间无法在线安装 Synapse 组件/驱动（cdn.razersynapse.com 被挡），
    Synapse 面板与云功能也不可用；要装组件或恢复面板，先 -Enable。

    默认只读；-Disable / -Enable 需要管理员权限，脚本会自动请求提权。

.PARAMETER Status
    只读查看当前状态（默认）。

.PARAMETER Disable
    挡更新域名 + 禁用升级任务 + 按进程封网。

.PARAMETER Enable
    恢复 hosts + 启用升级任务 + 移除按进程封网规则。

.PARAMETER NoFirewall
    只做 hosts + 计划任务，不创建/移除防火墙规则（保留 Synapse 面板与云功能）。

.PARAMETER NoElevate
    不自动提权；非管理员时直接报错。用于自动化。

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\razer-update-core.ps1 -Status

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\razer-update-core.ps1 -Disable

.EXAMPLE
    # 只挡域名、保留 Synapse 面板与云功能
    powershell -ExecutionPolicy Bypass -File scripts\razer-update-core.ps1 -Disable -NoFirewall

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\razer-update-core.ps1 -Enable
#>
[CmdletBinding()]
param(
    [switch]$Status,
    [switch]$Disable,
    [switch]$Enable,
    [switch]$NoFirewall,
    [switch]$NoElevate
)

$ErrorActionPreference = 'Stop'

$script:HostsPath = Join-Path $env:WINDIR 'System32\drivers\etc\hosts'

# Razer 更新控制面，按角色分组。来源：本机 Razer 日志（2026-10）
#   installer[engineUpgrade].log -> apps.razer.com/installer（清单）、
#                                   app-assets.razer.com/files/synapse/RazerAppEngineUpgrade-*.exe
#   lwi2_*.log                   -> discovery3.razerapi.com、manifest3.razerapi.com、
#                                   manifest-assets.razersynapse.com
#   RazerInstaller*.log          -> cdn.razersynapse.com、manifest.razerapi.com
#   Razer Central Service.log    -> discovery.razerapi.com、manifest.razerapi.com
$script:DomainGroups = [ordered]@{
    '更新清单' = @('manifest.razerapi.com', 'manifest3.razerapi.com')
    '安装包'   = @('app-assets.razer.com', 'cdn.razersynapse.com', 'manifest-assets.razersynapse.com')
    '端点发现' = @('discovery.razerapi.com', 'discovery3.razerapi.com')
}
$script:Domains = @($script:DomainGroups.Values | ForEach-Object { $_ })
$script:Marker = '# SAIL: block Razer update endpoints'
$script:TaskName = 'Razer Upgrade Elevation Service'

# 按进程封网：复用同目录的 block-razer-network.ps1，规则只属于这个组。
$script:NetworkScript = Join-Path $PSScriptRoot 'block-razer-network.ps1'
$script:NetworkGroup = 'SAIL Razer Network Block'
$script:BlockExe = @(
    # 安装器/更新器：不封它的话，手动跑安装器仍可下载更新包
    (Join-Path $env:ProgramData 'Razer\Installer\App\RazerInstaller.exe')
)

$script:HostLines = @($script:Domains | ForEach-Object { "0.0.0.0 $_ $($script:Marker)" })
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
    try {
        $process = Start-Process -FilePath $ps -Verb RunAs -ArgumentList $argList -Wait -PassThru
        if ($null -ne $process) {
            $script:ExitCode = $process.ExitCode
        }
    } catch {
        $script:ExitCode = 1
        throw "提权失败或已取消：$($_.Exception.Message)"
    }
    Write-Host '管理员窗口已结束。' -ForegroundColor Gray
}

function Get-HostsLines {
    if (-not (Test-Path -LiteralPath $script:HostsPath)) {
        throw "找不到 hosts 文件：$($script:HostsPath)"
    }
    return @(Get-Content -LiteralPath $script:HostsPath -ErrorAction Stop)
}

function Get-BlockLines {
    $lines = Get-HostsLines
    $domainPattern = (@($script:Domains | ForEach-Object { [regex]::Escape($_) }) -join '|')
    return @($lines | Where-Object {
            $_ -match $domainPattern -or $_ -match [regex]::Escape($script:Marker)
        })
}

function Get-Task {
    try {
        return Get-ScheduledTask -TaskName $script:TaskName -ErrorAction Stop
    } catch {
        return $null
    }
}

function Get-NetworkRules {
    return @(Get-NetFirewallRule -Group $script:NetworkGroup -ErrorAction SilentlyContinue)
}

# 调用同目录的 block-razer-network.ps1；宿主已提权，用 -NoElevate 避免二次弹窗。
function Invoke-NetworkRules([bool]$Blocked) {
    if (-not (Test-Path -LiteralPath $script:NetworkScript)) {
        Write-Warn "找不到 $($script:NetworkScript)，跳过防火墙步骤"
        return $false
    }

    $ps = (Get-Command powershell.exe).Source
    $arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$($script:NetworkScript)`" -NoElevate"

    if ($Blocked) {
        $arguments += ' -Apply'
        $extra = @($script:BlockExe | Where-Object { Test-Path -LiteralPath $_ })
        if ($extra.Count -gt 0) {
            $arguments += ' -ExtraPath ' + (($extra | ForEach-Object { '"' + $_ + '"' }) -join ',')
        }
    } else {
        $arguments += ' -Remove'
    }

    $process = Start-Process -FilePath $ps -ArgumentList $arguments -Wait -PassThru -NoNewWindow
    if ($null -eq $process) {
        Write-Warn '防火墙规则操作没有启动'
        return $false
    }
    if ($process.ExitCode -ne 0) {
        Write-Warn "防火墙规则操作失败（exit $($process.ExitCode)）"
        return $false
    }
    return $true
}

function Get-DnsAddresses([string]$Domain) {
    try {
        return @([System.Net.Dns]::GetHostAddresses($Domain) |
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
    # Remove only SAIL's marked lines. Unrelated user mappings must survive.
    $kept = @($lines | Where-Object {
            $_ -notmatch '(?i)# SAIL: block Razer'
        })

    if ($Blocked) {
        # 去掉文件末尾多余空行，再追加每个更新域名的屏蔽项
        while ($kept.Count -gt 0 -and [string]::IsNullOrWhiteSpace($kept[$kept.Count - 1])) {
            $kept = @($kept | Select-Object -First ($kept.Count - 1))
        }
        $kept += $script:HostLines
    }

    # Write a sibling file first, then replace. An interrupted/AV-locked write
    # must not leave the system hosts file truncated.
    $temp = "$($script:HostsPath).sail.tmp"
    # Match Get-Content's default (ANSI on Windows PowerShell 5.1) so existing
    # localized comments are not turned into mojibake.
    $encoding = [System.Text.Encoding]::Default
    [System.IO.File]::WriteAllLines($temp, [string[]]$kept, $encoding)
    Move-Item -LiteralPath $temp -Destination $script:HostsPath -Force
}

function Show-Status {
    Write-Title 'Razer 自动更新状态'

    $admin = Test-Admin
    Write-Info ("管理员: " + $(if ($admin) { '是' } else { '否' }))
    Write-Info "hosts: $($script:HostsPath)"

    $block = @(Get-BlockLines)
    if ($block.Count -gt 0) {
        Write-Ok "hosts 已包含 $($block.Count) 条 Razer 更新屏蔽项"
    } else {
        Write-Warn "hosts 没有屏蔽任何 Razer 更新域名"
    }

    foreach ($domain in $script:Domains) {
        $dns = @(Get-DnsAddresses $domain)
        if ($dns.Count -eq 0) {
            if ($block.Count -gt 0) {
                Write-Ok "DNS 已无法解析 $domain（hosts 已挡）"
            } else {
                Write-Warn "DNS 当前解析不到 $domain（可能是网络/DNS问题，不代表已封锁）"
            }
        } elseif ($dns -contains '0.0.0.0') {
            Write-Ok "$domain -> 0.0.0.0（更新被挡）"
        } else {
            Write-Warn "$domain -> $($dns -join ', ')（仍可访问）"
        }
    }

    $task = Get-Task
    if (-not $task) {
        Write-Warn "计划任务不存在：$($script:TaskName)"
    } elseif ($task.State -eq 'Disabled') {
        Write-Ok "计划任务已禁用：$($script:TaskName)"
    } else {
        Write-Warn "计划任务仍启用：$($script:TaskName) ($($task.State))"
    }

    $netRules = @(Get-NetworkRules)
    if ($netRules.Count -gt 0) {
        Write-Ok "防火墙：$($netRules.Count) 条按进程阻断规则（$($script:NetworkGroup)）"
    } else {
        Write-Warn '防火墙：没有 SAIL 按进程阻断规则（只靠 hosts）'
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
    Write-Host '  · 本机 HID、设备设置、声音（走 127.0.0.1，防火墙不过滤回环）' -ForegroundColor Gray
    Write-Host '  · VSSrv / THX APO / 127.0.0.1 控制通道' -ForegroundColor Gray
    Write-Host '会受影响（按进程封网的代价）：' -ForegroundColor Gray
    Write-Host '  · RazerAppEngine.exe 的全部外网访问：Synapse 面板、登录、云同步' -ForegroundColor Gray
    Write-Host '会被阻止：' -ForegroundColor Gray
    foreach ($group in $script:DomainGroups.GetEnumerator()) {
        Write-Host ("  · {0}：{1}" -f $group.Key, ($group.Value -join ' / ')) -ForegroundColor Gray
    }
    Write-Host "  · 计划任务 $($script:TaskName)" -ForegroundColor Gray
    Write-Host "  · 防火墙规则组 $($script:NetworkGroup)（RazerAppEngine.exe / RazerInstaller.exe）" -ForegroundColor Gray
}

function Show-Verification([string]$Mode) {
    Write-Title '执行后验证'

    $failed = $false
    $block = @(Get-BlockLines)
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

        foreach ($domain in $script:Domains) {
            $dns = @(Get-DnsAddresses $domain)
            if ($dns.Count -eq 0 -or $dns -contains '0.0.0.0') {
                Write-Ok "DNS: $domain 已不可达"
            } else {
                Write-Warn "DNS: $domain 仍解析到 $($dns -join ', ')（hosts 已写入，但可能被 DNS 缓存/代理影响）"
            }
        }

        if ($NoFirewall) {
            Write-Info '防火墙: 已跳过（-NoFirewall）'
        } else {
            $netRules = @(Get-NetworkRules)
            if ($netRules.Count -gt 0) {
                Write-Ok "防火墙: $($netRules.Count) 条按进程阻断规则"
            } else {
                Write-Fail '防火墙: 没有按进程阻断规则'
                $failed = $true
            }
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

        $netRules = @(Get-NetworkRules)
        if ($netRules.Count -eq 0) {
            Write-Ok '防火墙: 按进程阻断规则已移除'
        } else {
            Write-Fail "防火墙: 仍有 $($netRules.Count) 条规则"
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
        if ($NoFirewall) { Invoke-Elevated '-Disable -NoFirewall' } else { Invoke-Elevated '-Disable' }
        return
    }

    Write-Title '关闭 Razer 自动更新'
    [void](Backup-Hosts)
    Set-HostsBlocked $true
    Write-Ok "hosts: 已写入 $($script:Domains.Count) 个 Razer 更新域名屏蔽项"
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

    if ($NoFirewall) {
        Write-Info '防火墙：已跳过（-NoFirewall）'
    } elseif (Invoke-NetworkRules $true) {
        $rules = @(Get-NetworkRules)
        Write-Ok "防火墙：已创建 $($rules.Count) 条按进程阻断规则（$($script:NetworkGroup)）"
        Write-Warn 'RazerAppEngine.exe 已断网：Synapse 面板/登录/云同步不可用（HID、THX、回环通道不受影响）'
    }

    $done = '完成：更新域名已挡，升级任务已禁用'
    if (-not $NoFirewall) { $done += '，RazerAppEngine 已断网' }
    Write-Host ''
    Write-Ok ($done + '。')
    Write-Info 'THX 控制仍走 127.0.0.1 -> VSSrv，不受影响。'
    Write-Info '恢复：运行 razer-update-on.cmd（会一并移除防火墙规则）。'
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
    Write-Ok "hosts: 已移除 $($script:Domains.Count) 个 Razer 更新域名屏蔽项"
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

    if (Invoke-NetworkRules $false) {
        Write-Ok '防火墙：已移除 SAIL 按进程阻断规则'
    }

    Write-Host ''
    Write-Ok '完成：hosts、升级任务与防火墙规则均已恢复。'
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
