#requires -Version 5.1
<#
.SYNOPSIS
    SAIL · THX / 声音标准化 全链路诊断。

.DESCRIPTION
    按真实调用链逐环检查，指出断点，并给出可执行的修复命令：

        Windows 音频栈 (Audiosrv / AudioEndpointBuilder)
          -> THX 服务 VSSrv.exe        (Razer UI 与 APO 之间的控制通道, TCP 127.0.0.1:1024/1025)
            -> THX APO 组件            (SWD\DRIVERENUM\THXAPOCOMPONENT_*)
              -> 端点 FX 绑定          (MMDevices\...\FxProperties + PKEY_FX_Association)
                -> THX 控制 DLL        (ThxV3Native_v1.1.42.1.dll)
                  -> Synapse 前端      (RazerAppEngine.exe)

    默认只读诊断，不修改设备状态。
    -Verify  额外做一次 写 1 -> 读回 -> 恢复 的写入验证（会短暂改变标准化等级）。
    -Fix     管理员权限下自动执行对应断点的修复（失败时回退为打印手工命令）。

.PARAMETER ThxHost
    sail_thx.exe 的路径。默认在 %LOCALAPPDATA%\SAIL\bin、C:\sailtest、target\debug 中查找。

.PARAMETER Verify
    运行 sail_thx verify，验证标准化等级 1..100 真实可写。
    注意：THX 写等级前必须先开启标准化；当前 verify 会恢复原等级和原开关状态。

.PARAMETER Fix
    自动修复。需要管理员权限（脚本会请求提权）。

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\diagnose-thx.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\diagnose-thx.ps1 -Verify

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\diagnose-thx.ps1 -Fix
#>
[CmdletBinding()]
param(
    [string]$ThxHost,
    [switch]$Verify,
    [switch]$Fix
)

$ErrorActionPreference = 'Continue'
$script:Container = '{FEC3BF0F-2180-5CED-AEEB-DFB45C54FB17}'
$script:MmRender = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render'
$script:DllRelative = 'Razer\RazerAppEngine\User Data\Apps\Common\thxv3common\ThxV3Native_v1.1.42.1.dll'

# PKEY 常量（PROPERTYKEY 在注册表里就是 "{guid},pid" 形式的名称）
$PKEY_Device_FriendlyName = '{b3f8fa53-0004-438e-9003-51a46e139bfc},6'
$PKEY_FX_Association      = '{8c7ed206-3f8a-4827-b3ab-ae9e1faefc6c},2'
$PKEY_FX_PreMixClsid      = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},3'
$PKEY_FX_PostMixClsid     = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},13'
$PKEY_FX_StreamClsid      = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},14'

# ---------------------------------------------------------------------------
# 输出辅助
# ---------------------------------------------------------------------------

function Write-Section([string]$Text) {
    Write-Host ''
    Write-Host $Text -ForegroundColor Cyan
    Write-Host ('-' * 72) -ForegroundColor DarkCyan
}

function New-Result([string]$Id, [string]$Title, [string]$Status, [string]$Detail, [string]$Warn) {
    [pscustomobject]@{
        Id     = $Id
        Title  = $Title
        Status = $Status
        Detail = $Detail
        Warn   = $Warn
    }
}

function Show-Result($Result) {
    $tag = switch ($Result.Status) {
        'PASS' { '[ OK ]' }
        'WARN' { '[WARN]' }
        'FAIL' { '[FAIL]' }
        'SKIP' { '[SKIP]' }
        default { '[INFO]' }
    }
    $color = switch ($Result.Status) {
        'PASS' { 'Green' }
        'WARN' { 'Yellow' }
        'FAIL' { 'Red' }
        'SKIP' { 'DarkGray' }
        default { 'Gray' }
    }
    Write-Host ('{0} {1,-18} {2}' -f $tag, $Result.Id, $Result.Title) -ForegroundColor $color
    if ($Result.Detail) {
        Write-Host ('       {0}' -f $Result.Detail) -ForegroundColor DarkGray
    }
    if ($Result.Warn) {
        Write-Host ('       -> {0}' -f $Result.Warn) -ForegroundColor Yellow
    }
}

# ---------------------------------------------------------------------------
# 通用探测
# ---------------------------------------------------------------------------

function Test-Elevated {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-ServiceState([string]$Name) {
    $svc = Get-Service -Name $Name -ErrorAction SilentlyContinue
    if (-not $svc) { return $null }
    return $svc
}

function Test-LoopbackPort([int]$Port, [int]$TimeoutMs = 700) {
    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $async = $client.BeginConnect('127.0.0.1', $Port, $null, $null)
        if (-not $async.AsyncWaitHandle.WaitOne($TimeoutMs)) { return $false }
        $client.EndConnect($async)
        return $client.Connected
    } catch {
        return $false
    } finally {
        $client.Close()
    }
}

function Get-ListenPorts([int[]]$Ports) {
    $found = @()
    try {
        $conns = Get-NetTCPConnection -State Listen -ErrorAction Stop
        $found = $conns | Where-Object { $Ports -contains $_.LocalPort }
        return $found
    } catch {
        # 没有 NetTCPIP 模块时退回 netstat 解析（不依赖列名，只看端口和 PID）
        $lines = & netstat.exe -ano 2>$null
        $out = @()
        foreach ($line in $lines) {
            if ($line -notmatch 'LISTENING') { continue }
            $parts = ($line.Trim() -split '\s+')
            if ($parts.Length -lt 5) { continue }
            $local = $parts[1]
            $port = 0
            if (-not [int]::TryParse(($local -split ':')[-1], [ref]$port)) { continue }
            if ($Ports -notcontains $port) { continue }
            $out += [pscustomobject]@{ LocalPort = $port; OwningProcess = [int]$parts[-1] }
        }
        return $out
    }
}

function Get-ThxChannelState {
    # VSSrv 每次启动都会挑选新的本地端口，并把它们写进 THX\Discovery。
    # 因此不能硬编码 1024/1025：必须读这里发布的地址。
    $state = [pscustomobject]@{
        Published     = @()
        Ports         = @()
        Connected     = @()
        Listening     = @()
        ForeignOwner  = @()
        DiscoverySeen = $false
    }

    $discovery = Get-ItemProperty 'HKLM:\SOFTWARE\THX\Discovery' -ErrorAction SilentlyContinue
    if (-not $discovery) { return $state }
    $state.DiscoverySeen = $true

    foreach ($name in 'thx:sa:status:publisher', 'thx:sa:service') {
        $raw = $null
        try { $raw = $discovery.$name } catch { $raw = $null }
        $port = $null
        if ($raw -and ("$raw" -match ':(\d+)\s*$')) { $port = [int]$Matches[1] }
        $state.Published += [pscustomobject]@{ Name = $name; Address = "$raw"; Port = $port }
        if ($port) { $state.Ports += $port }
    }

    if ($state.Ports.Count -eq 0) { return $state }

    $listen = @(Get-ListenPorts $state.Ports)
    $state.Listening = @($listen | ForEach-Object { $_.LocalPort } | Sort-Object -Unique)
    $state.ForeignOwner = @($listen | Where-Object {
            $owner = $null
            try { $owner = (Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).Name } catch { }
            $owner -ne 'VSSrv'
        } | ForEach-Object { $_.LocalPort })

    foreach ($p in $state.Ports) {
        if (Test-LoopbackPort $p) { $state.Connected += $p }
    }
    return $state
}

function Test-ThxChannelReady($State) {
    return ($State.Ports.Count -gt 0 -and $State.Connected.Count -eq $State.Ports.Count)
}

function ConvertFrom-PropVariantGuid([byte[]]$Bytes) {
    # PROPERTYKEY 的 VT_CLSID 值：8 字节头 + 16 字节 GUID（小端序）
    if ($null -eq $Bytes) { return $null }
    if ($Bytes.Length -lt 24) { return $null }
    $guidBytes = New-Object byte[] 16
    [Array]::Copy($Bytes, 8, $guidBytes, 0, 16)
    try {
        $guid = New-Object System.Guid -ArgumentList (, $guidBytes)
        return $guid.ToString('B').ToUpperInvariant()
    } catch {
        return $null
    }
}

function Get-RegValue($Key, [string]$Name) {
    if (-not $Key) { return $null }
    try { return $Key.GetValue($Name) } catch { return $null }
}

# ---------------------------------------------------------------------------
# 上下文：端点、DLL、sail_thx
# ---------------------------------------------------------------------------

function Resolve-Context {
    $ctx = [pscustomobject]@{
        Elevated       = Test-Elevated
        EndpointKey    = $null   # {5f0b2ea6-...}
        EndpointId     = $null   # {0.0.0.00000000}.{5f0b2ea6-...}
        EndpointName   = $null
        EndpointState  = $null
        EndpointFx     = $null   # FxProperties 注册表项
        EndpointProps  = $null   # Properties 注册表项
        Association    = $null
        Candidates     = @()
        DllPath        = $null
        ThxHost        = $null
        ThxHostOrigin  = $null
        ThxHostUsable  = $null
        VssrvPid       = $null
    }

    # --- 音频渲染端点 ---
    $keys = Get-ChildItem $script:MmRender -ErrorAction SilentlyContinue
    foreach ($key in $keys) {
        $props = Get-Item ("{0}\Properties" -f $key.PSPath) -ErrorAction SilentlyContinue
        $name = Get-RegValue $props $PKEY_Device_FriendlyName
        $state = Get-RegValue (Get-Item $key.PSPath -ErrorAction SilentlyContinue) 'DeviceState'
        $assoc = ConvertFrom-PropVariantGuid (Get-RegValue $props $PKEY_FX_Association)
        $ctx.Candidates += [pscustomobject]@{
            Key   = $key.PSChildName
            Name  = $name
            State = $state
            Assoc = $assoc
        }
    }

    $matched = $ctx.Candidates | Where-Object { $_.Name -match 'BlackShark|雷蛇耳机' }
    if (-not $matched) {
        # 退而求其次：按 THX 容器绑定来找
        $matched = $ctx.Candidates | Where-Object { $_.Assoc -eq $script:Container }
    }
    $pick = $matched | Where-Object { $_.State -eq 1 } | Select-Object -First 1
    if (-not $pick) { $pick = $matched | Select-Object -First 1 }

    if ($pick) {
        $ctx.EndpointKey = $pick.Key
        $ctx.EndpointName = $pick.Name
        $ctx.EndpointState = $pick.State
        $ctx.Association = $pick.Assoc
        $ctx.EndpointId = '{0.0.0.00000000}.' + $pick.Key
        $ctx.EndpointProps = Get-Item ("{0}\{1}\Properties" -f $script:MmRender, $pick.Key) -ErrorAction SilentlyContinue
        $ctx.EndpointFx = Get-Item ("{0}\{1}\FxProperties" -f $script:MmRender, $pick.Key) -ErrorAction SilentlyContinue
    }

    # --- THX 控制 DLL ---
    $dllOverride = [Environment]::GetEnvironmentVariable('SAIL_THX_DLL')
    if ($dllOverride) {
        $ctx.DllPath = $dllOverride
    } elseif ($env:LOCALAPPDATA) {
        $ctx.DllPath = Join-Path $env:LOCALAPPDATA $script:DllRelative
    }

    # --- sail_thx.exe ---
    $candidates = New-Object System.Collections.ArrayList
    if ($ThxHost) { [void]$candidates.Add([pscustomobject]@{ Path = $ThxHost; Origin = '参数 -ThxHost' }) }
    if ($env:LOCALAPPDATA) {
        [void]$candidates.Add([pscustomobject]@{ Path = (Join-Path $env:LOCALAPPDATA 'SAIL\bin\sail_thx.exe'); Origin = '已暂存' })
    }
    [void]$candidates.Add([pscustomobject]@{ Path = 'C:\sailtest\sail_thx.exe'; Origin = '临时目录' })
    [void]$candidates.Add([pscustomobject]@{ Path = (Join-Path (Get-Location) 'target\debug\sail_thx.exe'); Origin = '构建输出' })
    [void]$candidates.Add([pscustomobject]@{ Path = (Join-Path $PSScriptRoot '..\target\debug\sail_thx.exe'); Origin = '构建输出' })
    [void]$candidates.Add([pscustomobject]@{ Path = (Join-Path $PSScriptRoot '..\target\release\sail_thx.exe'); Origin = '构建输出' })

    foreach ($c in $candidates) {
        if ($c.Path -and (Test-Path -LiteralPath $c.Path -PathType Leaf)) {
            $ctx.ThxHost = (Resolve-Path -LiteralPath $c.Path).Path
            $ctx.ThxHostOrigin = $c.Origin
            break
        }
    }

    if ($ctx.ThxHost) {
        $ctx.ThxHostUsable = $ctx.ThxHost
        # THX DLL 在超长路径下会栈溢出；项目构建目录路径过长，先复制到短路径再跑。
        if ($ctx.ThxHost.Length -gt 90 -or $ctx.ThxHost -match 'razer-blackshark-v2-hyperspeed') {
            $short = Join-Path $env:TEMP 'sail_thx_probe.exe'
            try {
                Copy-Item -LiteralPath $ctx.ThxHost -Destination $short -Force
                $ctx.ThxHostUsable = $short
            } catch {
                $ctx.ThxHostUsable = $null
            }
        }
    }

    $vssrv = Get-Process -Name 'VSSrv' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($vssrv) { $ctx.VssrvPid = $vssrv.Id }

    return $ctx
}

function Invoke-ThxHost([string]$Exe, [string[]]$Arguments) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Exe
    $psi.Arguments = ($Arguments -join ' ')
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $psi.WorkingDirectory = $env:TEMP

    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    try {
        [void]$proc.Start()
        $stdout = $proc.StandardOutput.ReadToEnd()
        $stderr = $proc.StandardError.ReadToEnd()
        if (-not $proc.WaitForExit(8000)) {
            try { $proc.Kill() } catch { }
            return [pscustomobject]@{ ExitCode = -1; StdOut = $stdout; StdErr = 'timeout'; TimedOut = $true }
        }
        return [pscustomobject]@{ ExitCode = $proc.ExitCode; StdOut = $stdout; StdErr = $stderr; TimedOut = $false }
    } catch {
        return [pscustomobject]@{ ExitCode = -1; StdOut = ''; StdErr = $_.Exception.Message; TimedOut = $false }
    } finally {
        $proc.Dispose()
    }
}

function Get-StatusField([string]$Text, [string]$Name) {
    $m = [regex]::Match($Text, ('(?m)^' + [regex]::Escape($Name) + ':\s*(.*)$'))
    if ($m.Success) { return $m.Groups[1].Value.Trim() }
    return $null
}

# ---------------------------------------------------------------------------
# 逐环检查
# ---------------------------------------------------------------------------

function Invoke-Checks($ctx) {
    $results = New-Object System.Collections.ArrayList

    # 0) VSSrv 历史异常（先取，供恢复策略判断用） ------------------------------
    try {
        $events = @(Get-WinEvent -FilterHashtable @{ LogName = 'System'; Id = 7034, 7000; StartTime = (Get-Date).AddDays(-30) } -ErrorAction Stop |
            Where-Object { $_.Message -match 'VSSrv' })
    } catch { $events = @() }

    $crashes = @($events | Where-Object { $_.Id -eq 7034 })
    $startFails = @($events | Where-Object { $_.Id -eq 7000 })
    $lastCrash = $null
    if ($crashes.Count -gt 0) {
        $lastCrash = ($crashes | Sort-Object TimeCreated -Descending | Select-Object -First 1).TimeCreated
    }

    # 1) Windows 音频栈 -------------------------------------------------------
    $down = @()
    foreach ($name in 'Audiosrv', 'AudioEndpointBuilder') {
        $svc = Get-ServiceState $name
        if (-not $svc) { $down += "$name=缺失" }
        elseif ($svc.Status -ne 'Running') { $down += "$name=$($svc.Status)" }
    }
    if ($down.Count -eq 0) {
        [void]$results.Add((New-Result 'audio.services' 'Windows 音频服务' 'PASS' 'Audiosrv + AudioEndpointBuilder 均在运行' $null))
    } else {
        [void]$results.Add((New-Result 'audio.services' 'Windows 音频服务' 'FAIL' ($down -join '  ') '音频引擎未运行，THX APO 不会加载'))
    }

    # 2) THX 服务 VSSrv -------------------------------------------------------
    $vssrvSvc = Get-ServiceState 'VSSrv'
    $vssrvExe = 'C:\WINDOWS\System32\VSSrv.exe'
    if (-not $vssrvSvc) {
        [void]$results.Add((New-Result 'thx.service' 'THX 服务 VSSrv' 'FAIL' '服务未注册' 'THX APO 未安装或安装损坏'))
    } elseif ($vssrvSvc.Status -ne 'Running') {
        [void]$results.Add((New-Result 'thx.service' 'THX 服务 VSSrv' 'FAIL' ("状态={0}" -f $vssrvSvc.Status) 'Synapse 的声音标准化开关依赖此服务'))
    } elseif (-not (Test-Path -LiteralPath $vssrvExe)) {
        [void]$results.Add((New-Result 'thx.service' 'THX 服务 VSSrv' 'FAIL' ("运行中但文件不存在: {0}" -f $vssrvExe) '服务二进制被清理/杀软隔离，需重装 THX APO'))
    } else {
        [void]$results.Add((New-Result 'thx.service' 'THX 服务 VSSrv' 'PASS' ("运行中 (PID {0}, Auto, LocalSystem)" -f $ctx.VssrvPid) $null))
    }

    # 2b) VSSrv 崩溃自动恢复 --------------------------------------------------
    #    没有 FailureActions 时：历史上真崩过 -> FAIL（已证实会导致失灵）；
    #    没崩过 -> WARN（潜在风险，尚未发作）。
    $svcKey = Get-Item 'HKLM:\SYSTEM\CurrentControlSet\Services\VSSrv' -ErrorAction SilentlyContinue
    $failure = Get-RegValue $svcKey 'FailureActions'
    $recoveryMissing = ($null -eq $failure -or $failure.Length -lt 16)
    if ($null -ne $failure -and $failure.Length -ge 16) {
        $recoveryMissing = ([BitConverter]::ToUInt32($failure, 12) -eq 0)
    }

    if (-not $vssrvSvc) {
        [void]$results.Add((New-Result 'thx.recovery' 'VSSrv 失败自动恢复' 'SKIP' '服务不存在' $null))
    } elseif ($recoveryMissing -and $crashes.Count -gt 0) {
        [void]$results.Add((New-Result 'thx.recovery' 'VSSrv 失败自动恢复' 'FAIL' ("未配置 FailureActions，且近 30 天已崩溃 {0} 次" -f $crashes.Count) '服务崩溃后不会自动拉起，这就是"有时失灵"的原因'))
    } elseif ($recoveryMissing) {
        [void]$results.Add((New-Result 'thx.recovery' 'VSSrv 失败自动恢复' 'WARN' '未配置 FailureActions' '目前没崩过；一旦崩溃不会自愈，建议提前配置'))
    } else {
        $reset = [BitConverter]::ToUInt32($failure, 0)
        $count = [BitConverter]::ToUInt32($failure, 12)
        [void]$results.Add((New-Result 'thx.recovery' 'VSSrv 失败自动恢复' 'PASS' ("reset={0}s, actions={1}" -f $reset, $count) $null))
    }

    # 3) THX 本地控制通道（端口由 VSSrv 动态选择并发布到 THX\Discovery） -----
    $channel = Get-ThxChannelState
    if (-not $channel.DiscoverySeen) {
        [void]$results.Add((New-Result 'thx.channel' 'THX 控制通道' 'FAIL' '缺少 HKLM\SOFTWARE\THX\Discovery' 'THX APO 未安装或安装损坏'))
    } elseif ($channel.Ports.Count -eq 0) {
        [void]$results.Add((New-Result 'thx.channel' 'THX 控制通道' 'FAIL' 'Discovery 中未发布 tcp 端点' 'VSSrv 没有正常初始化'))
    } elseif (Test-ThxChannelReady $channel) {
        $detail = "Discovery {0}，VSSrv 监听且可连接" -f ($channel.Ports -join '/')
        [void]$results.Add((New-Result 'thx.channel' 'THX 控制通道' 'PASS' $detail $null))
    } elseif ($channel.Connected.Count -gt 0) {
        [void]$results.Add((New-Result 'thx.channel' 'THX 控制通道' 'WARN' ("{0}，仅 {1} 可连接" -f ($channel.Ports -join '/'), ($channel.Connected -join ',')) '通道不完整，Synapse 可能间歇性失败'))
    } elseif ($channel.Listening.Count -gt 0) {
        [void]$results.Add((New-Result 'thx.channel' 'THX 控制通道' 'FAIL' ("Discovery {0}，端口在监听但不接受连接" -f ($channel.Ports -join '/')) 'VSSrv 已挂起，重启服务即可'))
    } else {
        $detail = "Discovery 发布 {0}，但没有进程在监听" -f ($channel.Ports -join '/')
        [void]$results.Add((New-Result 'thx.channel' 'THX 控制通道' 'FAIL' $detail 'VSSrv 崩溃后未重新发布端口，或客户端缓存了旧端口'))
    }

    # 4) THX APO 安装痕迹 -----------------------------------------------------
    $markers = @(Get-ChildItem 'HKLM:\SOFTWARE\THX\THXSpatialAudio' -ErrorAction SilentlyContinue |
        Where-Object { $_.PSChildName -match 'THXUSBAPOInstaller' })
    if ($markers.Count -eq 0) {
        [void]$results.Add((New-Result 'thx.install' 'THX APO 安装记录' 'FAIL' 'HKLM\SOFTWARE\THX\THXSpatialAudio 无安装标记' 'THX APO 未安装，需运行雷蛇官方安装包'))
    } else {
        $missingInf = @()
        $infSet = @()
        foreach ($m in $markers) {
            $kv = Get-Item $m.PSPath -ErrorAction SilentlyContinue
            foreach ($n in @($kv.GetValueNames())) {
                if ($n -notmatch '\.inf$') { continue }
                $infName = [string](Get-RegValue $kv $n)
                if (-not $infName) { continue }
                $infSet += $infName
                if (-not (Test-Path (Join-Path 'C:\Windows\INF' $infName))) { $missingInf += $infName }
            }
        }
        $names = @($markers | ForEach-Object { $_.PSChildName })
        $uniqueInf = @($infSet | Sort-Object -Unique)
        if ($missingInf.Count -eq 0) {
            [void]$results.Add((New-Result 'thx.install' 'THX APO 安装记录' 'PASS' ("{0} 个 INF 全部在位 ({1})" -f $uniqueInf.Count, ($uniqueInf -join ', ')) $null))
        } else {
            [void]$results.Add((New-Result 'thx.install' 'THX APO 安装记录' 'FAIL' ("缺少 INF: {0}" -f (($missingInf | Sort-Object -Unique) -join ', ')) 'INF 被清理，APO 无法绑定'))
        }
    }

    # 5) THX APO 组件设备 -----------------------------------------------------
    $apo = @()
    try {
        if (Get-Command Get-PnpDevice -ErrorAction SilentlyContinue) {
            $apo = @(Get-PnpDevice -Class AudioProcessingObject -ErrorAction SilentlyContinue |
                Where-Object { $_.FriendlyName -match 'THX' })
        } else {
            $apo = @(Get-CimInstance Win32_PnPEntity -Filter "PNPClass='AudioProcessingObject'" -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -match 'THX' } |
                ForEach-Object { [pscustomobject]@{ Status = $_.Status; FriendlyName = $_.Name; InstanceId = $_.DeviceID } })
        }
    } catch { $apo = @() }

    $apoOk = @($apo | Where-Object { $_.Status -eq 'OK' })
    if ($apoOk.Count -gt 0) {
        [void]$results.Add((New-Result 'thx.apo.component' 'THX APO 组件' 'PASS' ("{0} 个实例状态 OK" -f $apoOk.Count) $null))
    } elseif ($apo.Count -gt 0) {
        [void]$results.Add((New-Result 'thx.apo.component' 'THX APO 组件' 'FAIL' ("{0} 个实例，但无 OK（状态: {1}）" -f $apo.Count, (($apo | Select-Object -First 3 | ForEach-Object { $_.Status }) -join ',')) 'APO 组件未启动，音频链里没有 THX'))
    } else {
        [void]$results.Add((New-Result 'thx.apo.component' 'THX APO 组件' 'FAIL' '未找到 AudioProcessingObject 类 THX 设备' 'THX APO 未正确安装'))
    }

    # 6) 渲染端点 -------------------------------------------------------------
    if (-not $ctx.EndpointKey) {
        [void]$results.Add((New-Result 'audio.endpoint' '渲染端点' 'FAIL' '未找到 BlackShark 渲染端点' '耳机未连接/未枚举；插拔接收器或检查设备管理器'))
    } elseif ($ctx.EndpointState -ne 1) {
        [void]$results.Add((New-Result 'audio.endpoint' '渲染端点' 'FAIL' ("{0} 状态={1}" -f $ctx.EndpointId, $ctx.EndpointState) '端点被禁用或未插入'))
    } else {
        [void]$results.Add((New-Result 'audio.endpoint' '渲染端点' 'PASS' ("{0}  ({1})" -f $ctx.EndpointId, $ctx.EndpointName) $null))
    }

    # 7) 端点 FX 绑定 ---------------------------------------------------------
    if (-not $ctx.EndpointFx) {
        [void]$results.Add((New-Result 'audio.fx.binding' '端点 FX 绑定' 'SKIP' '无端点或端点无 FxProperties' $null))
    } else {
        $clsidPre = [string](Get-RegValue $ctx.EndpointFx $PKEY_FX_PreMixClsid)
        $clsidPost = @(Get-RegValue $ctx.EndpointFx $PKEY_FX_PostMixClsid)
        $assoc = $ctx.Association
        $hasApo = ($clsidPre -or ($clsidPost -and $clsidPost.Count -gt 0))
        $assocOk = ($assoc -eq $script:Container)
        if ($hasApo -and $assocOk) {
            [void]$results.Add((New-Result 'audio.fx.binding' '端点 FX 绑定' 'PASS' ("APO CLSID 已绑定，FX 关联 = THX 容器") $null))
        } elseif ($hasApo -and -not $assocOk) {
            [void]$results.Add((New-Result 'audio.fx.binding' '端点 FX 绑定' 'WARN' ("有 APO，但 FX 关联={0}" -f ($(if ($assoc) { $assoc } else { '无' }))) '端点没有指向 THX 容器，效果可能不生效'))
        } else {
            [void]$results.Add((New-Result 'audio.fx.binding' '端点 FX 绑定' 'FAIL' 'FxProperties 中无 THX APO CLSID' 'APO 未挂到该端点，需重新安装 THX APO'))
        }
    }

    # 8) THX 控制 DLL ---------------------------------------------------------
    if (-not $ctx.DllPath) {
        [void]$results.Add((New-Result 'thx.dll' 'THX 控制 DLL' 'FAIL' '无法确定 LOCALAPPDATA' '环境异常'))
    } elseif (-not (Test-Path -LiteralPath $ctx.DllPath -PathType Leaf)) {
        [void]$results.Add((New-Result 'thx.dll' 'THX 控制 DLL' 'FAIL' ("不存在: {0}" -f $ctx.DllPath) 'THX APO 安装不完整，需重装'))
    } else {
        $ver = $null
        try { $ver = (Get-Item -LiteralPath $ctx.DllPath).VersionInfo.FileVersion } catch { }
        [void]$results.Add((New-Result 'thx.dll' 'THX 控制 DLL' 'PASS' ("{0}  ({1})" -f $ver, (Split-Path -Leaf $ctx.DllPath)) $null))
    }

    # 9) DLL API：端点 / 开关 / 等级 -----------------------------------------
    if (-not $ctx.ThxHost -or -not $ctx.ThxHostUsable) {
        [void]$results.Add((New-Result 'thx.api' 'THX API (sail_thx)' 'SKIP' '未找到可用的 sail_thx.exe' '先构建或暂存：scripts\build.cmd -Stage'))
    } else {
        $run = Invoke-ThxHost $ctx.ThxHostUsable @('probe')
        if ($run.ExitCode -ne 0) {
            [void]$results.Add((New-Result 'thx.api' 'THX API (sail_thx)' 'FAIL' ("退出码 {0}: {1}" -f $run.ExitCode, ($run.StdErr.Trim() -replace '\s+', ' ')) 'DLL 无法加载或导出缺失，链路在此断开'))
        } else {
            $ep = Get-StatusField $run.StdOut 'endpoint'
            $en = Get-StatusField $run.StdOut 'enabled'
            $lv = Get-StatusField $run.StdOut 'level'
            if (-not $ep -or $ep -eq '(none)') {
                [void]$results.Add((New-Result 'thx.api' 'THX API (sail_thx)' 'FAIL' 'GetRenderDevice 未返回端点' 'THX 未选中渲染设备'))
            } else {
                $detail = "endpoint={0}  enabled={1}  level={2}" -f $ep, $en, $lv
                [void]$results.Add((New-Result 'thx.api' 'THX API (sail_thx)' 'PASS' $detail $null))
                if ($lv -match '^-?\d+$' -and [int]$lv -lt 1) {
                    [void]$results.Add((New-Result 'thx.level' '标准化等级范围' 'WARN' ("当前等级 {0} 超出 1..100" -f $lv) 'THX 状态异常，建议 set 回 10'))
                }
            }
        }
    }

    # 10) 写入验证（可选） ----------------------------------------------------
    if ($Verify) {
        if (-not $ctx.ThxHostUsable) {
            [void]$results.Add((New-Result 'thx.write' '标准化写入验证' 'SKIP' '缺少 sail_thx.exe' $null))
        } else {
            $run = Invoke-ThxHost $ctx.ThxHostUsable @('verify')
            if ($run.ExitCode -eq 0) {
                $flat = ($run.StdOut.Trim() -replace '\r?\n', '  |  ')
                [void]$results.Add((New-Result 'thx.write' '标准化写入验证' 'PASS' $flat $null))
            } else {
                [void]$results.Add((New-Result 'thx.write' '标准化写入验证' 'FAIL' ($run.StdErr.Trim() -replace '\s+', ' ') 'API 可读但不可写，DLL 处于半失效状态'))
            }
        }
    }

    # 11) Synapse 前端 --------------------------------------------------------
    $engine = @(Get-Process -Name 'RazerAppEngine' -ErrorAction SilentlyContinue)
    if ($engine.Count -gt 0) {
        [void]$results.Add((New-Result 'razer.appengine' 'Synapse 前端' 'PASS' ("{0} 个 RazerAppEngine 进程" -f $engine.Count) $null))
    } else {
        [void]$results.Add((New-Result 'razer.appengine' 'Synapse 前端' 'WARN' 'RazerAppEngine 未运行' 'Synapse 界面里的开关自然无响应；直接用它启动即可'))
    }

    # 11b) Synapse 是否卡在已失效的 THX 端口 -----------------------------------
    #      VSSrv 重启后端口会变；客户端若缓存旧端口，就会一直 SYN_SENT，
    #      界面上的开关随之失效 —— 但 APO 和 DLL 其实都是好的。
    $stale = @()
    try {
        $stale = @(Get-NetTCPConnection -ErrorAction Stop | Where-Object {
                $_.State -eq 'SynSent' -and
                $_.RemoteAddress -eq '127.0.0.1' -and
                ($channel.Ports -notcontains $_.RemotePort) -and
                (Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).Name -eq 'RazerAppEngine'
            })
    } catch { $stale = @() }

    if ($stale.Count -eq 0) {
        [void]$results.Add((New-Result 'synapse.stale' 'Synapse 端口缓存' 'PASS' '未发现卡在失效端口的连接' $null))
    } else {
        $deadPorts = @($stale | ForEach-Object { $_.RemotePort } | Sort-Object -Unique)
        $thxKnown = @(1024, 1025) + @($channel.Ports)
        $toThx = @($deadPorts | Where-Object { $thxKnown -contains $_ })
        if ($toThx.Count -gt 0) {
            $detail = "RazerAppEngine 正卡在 {0}（THX 当前监听 {1}）" -f ($toThx -join '/'), ($(if ($channel.Ports) { $channel.Ports -join '/' } else { '无' }))
            [void]$results.Add((New-Result 'synapse.stale' 'Synapse 端口缓存' 'FAIL' $detail 'Synapse 缓存了旧端口：重启前端即可，APO 无罪'))
        } else {
            [void]$results.Add((New-Result 'synapse.stale' 'Synapse 端口缓存' 'WARN' ("RazerAppEngine 有 {0} 条连接卡在 {1}" -f $stale.Count, ($deadPorts -join '/')) '可能连的是其它本地服务，留意即可'))
        }
    }

    # 12) THX 状态文件 --------------------------------------------------------
    $conf = 'C:\ProgramData\THX\usb\1532\0565\thx_spatial.conf.json'
    if (-not (Test-Path -LiteralPath $conf)) {
        [void]$results.Add((New-Result 'thx.config' 'THX 状态文件' 'WARN' ("不存在: {0}" -f $conf) 'APO 会用默认值，个别设置可能不落盘'))
    } else {
        $writable = $false
        try {
            $fs = [System.IO.File]::Open($conf, 'Open', 'Write', 'ReadWrite')
            $fs.Close()
            $writable = $true
        } catch { $writable = $false }
        if ($writable) {
            [void]$results.Add((New-Result 'thx.config' 'THX 状态文件' 'PASS' '存在且可写' $null))
        } else {
            [void]$results.Add((New-Result 'thx.config' 'THX 状态文件' 'FAIL' '存在但不可写' '设置无法保存，检查权限/只读/杀软锁定'))
        }
    }

    # 13) 历史崩溃证据（数据在步骤 0 已取） -----------------------------------
    if ($events.Count -eq 0) {
        [void]$results.Add((New-Result 'thx.events' 'VSSrv 历史异常' 'PASS' '近 30 天无崩溃/启动失败记录' $null))
    } else {
        $detail = "崩溃 {0} 次 / 启动失败 {1} 次" -f $crashes.Count, $startFails.Count
        if ($lastCrash) { $detail += ("，最近崩溃 {0:yyyy-MM-dd HH:mm}" -f $lastCrash) }
        [void]$results.Add((New-Result 'thx.events' 'VSSrv 历史异常' 'WARN' $detail '这是"有时无法调节"的历史证据；配置自动恢复后可自愈'))
    }

    return $results
}

# ---------------------------------------------------------------------------
# 结论与修复
# ---------------------------------------------------------------------------

function Get-Break($results) {
    foreach ($r in $results) {
        if ($r.Status -eq 'FAIL') { return $r }
    }
    return $null
}

function Show-Verdict($results, $ctx) {
    $break = Get-Break $results

    Write-Section '结论'
    if (-not $break) {
        Write-Host '链路完整：音频栈 -> VSSrv -> APO 组件 -> 端点 FX 绑定 -> THX DLL/API 全部正常。' -ForegroundColor Green
        Write-Host ''
        Write-Host '如果 Synapse 此刻仍然不能开关/拖动声音标准化，那么：' -ForegroundColor Gray
        Write-Host '  · APO 没有坏（组件 OK、FX 绑定在、DLL 可读可写）；' -ForegroundColor Gray
        Write-Host '  · 断点在 Synapse 前端 RazerAppEngine 的进程内状态，重启它即可，不需要动音频栈。' -ForegroundColor Gray
        $engineWarn = $results | Where-Object { $_.Id -eq 'razer.appengine' -and $_.Status -eq 'WARN' }
        if ($engineWarn) {
            Write-Host ''
            Write-Host '注意：RazerAppEngine 当前未运行，Synapse 的开关必然无响应。' -ForegroundColor Yellow
        }
        return
    }

    if ($break.Id -eq 'thx.recovery') {
        Write-Host ('间歇性断点：{0}（{1}）' -f $break.Id, $break.Title) -ForegroundColor Yellow
        Write-Host '  服务此刻在线，所以现在能调；一旦崩溃就再也调不动，直到手动重启或重启系统。' -ForegroundColor DarkGray
    } else {
        Write-Host ('断点：{0}（{1}）' -f $break.Id, $break.Title) -ForegroundColor Red
    }
    if ($break.Detail) { Write-Host ('  ' + $break.Detail) -ForegroundColor DarkGray }
    if ($break.Warn) { Write-Host ('  ' + $break.Warn) -ForegroundColor Yellow }
    Write-Host ''

    switch ($break.Id) {
        'audio.services' {
            Write-Host '音频引擎没起来，THX APO 根本不会被加载。先修 Windows 音频栈，再谈 Synapse。' -ForegroundColor Gray
        }
        { $_ -in 'thx.service', 'thx.recovery', 'thx.channel' } {
            Write-Host '这是"有时无法调节声音标准化"最常见的真凶：' -ForegroundColor Gray
            Write-Host '  VSSrv.exe 是 THX Ltd. 的本地服务，每次启动都会随机挑一对本地端口，' -ForegroundColor Gray
            Write-Host '  再把地址写进 HKLM\SOFTWARE\THX\Discovery；Synapse 与 THX DLL 读该注册表后连接。' -ForegroundColor Gray
            Write-Host '  它默认没有配置失败自动恢复，崩溃后就一直死着：音频照常出声，' -ForegroundColor Gray
            Write-Host '  但 Synapse 的标准化开关和滑杆全部失效 —— 症状与你描述的完全一致。' -ForegroundColor Gray
            if ($break.Id -eq 'thx.channel') {
                Write-Host '  另一种情况：服务重启后端口变了，而 RazerAppEngine 还连着旧端口，' -ForegroundColor Gray
                Write-Host '  同样表现为开关无效 —— 重启 Synapse 前端即可。' -ForegroundColor Gray
            }
        }
        'thx.install' {
            Write-Host 'THX APO 的 INF 被清理（常见于系统优化/清理工具），需要重装 THX APO。' -ForegroundColor Gray
        }
        'thx.apo.component' {
            Write-Host 'APO 组件设备没有起来。重装 THX APO 后仍失败，多半是签名/驱动缓存问题。' -ForegroundColor Gray
        }
        'audio.endpoint' {
            Write-Host '端点没有枚举出来：耳机没连上，或端点被禁用。' -ForegroundColor Gray
        }
        'audio.fx.binding' {
            Write-Host 'APO 存在但没有绑定到这个端点，效果链里没有 THX。' -ForegroundColor Gray
        }
        { $_ -in 'thx.dll', 'thx.api' } {
            Write-Host '控制 DLL 缺失或 API 调不通。注意 THX DLL 在超长路径下会栈溢出，' -ForegroundColor Gray
            Write-Host '请把 sail_thx.exe 放在短路径（如 %LOCALAPPDATA%\SAIL\bin）再运行。' -ForegroundColor Gray
        }
        'thx.config' {
            Write-Host 'THX 配置文件被锁/只读，设置无法保存。' -ForegroundColor Gray
        }
        'synapse.stale' {
            Write-Host 'APO、端点绑定、THX DLL 都是好的 —— 坏的是"客户端连错了端口"：' -ForegroundColor Gray
            Write-Host '  VSSrv 每次重启都会换一对本地端口并更新 THX\Discovery，' -ForegroundColor Gray
            Write-Host '  而 Synapse（RazerAppEngine）还在重试旧端口，于是开关和滑杆点了没反应。' -ForegroundColor Gray
            Write-Host '  重启 Synapse 前端即可，不需要动音频栈，也不用重装 APO。' -ForegroundColor Gray
        }
        default {
            Write-Host '按下方命令逐条修复，然后重新运行本脚本确认。' -ForegroundColor Gray
        }
    }
}

function Get-RepairSteps([string]$BreakId) {
    $steps = New-Object System.Collections.ArrayList

    switch ($BreakId) {
        'audio.services' {
            [void]$steps.Add('net stop Audiosrv /y')
            [void]$steps.Add('net start AudioEndpointBuilder')
            [void]$steps.Add('net start Audiosrv')
        }
        { $_ -in 'thx.service', 'thx.recovery', 'thx.channel' } {
            [void]$steps.Add('Restart-Service VSSrv -Force')
            [void]$steps.Add('sc.exe failure VSSrv reset= 86400 actions= restart/5000/restart/10000/restart/30000')
            [void]$steps.Add('sc.exe failureflag VSSrv 1')
            [void]$steps.Add('# 若服务起不来，先重启音频栈再试：')
            [void]$steps.Add('Restart-Service AudioEndpointBuilder -Force ; Start-Service Audiosrv ; Start-Service VSSrv')
            [void]$steps.Add('# 复查：')
            [void]$steps.Add('powershell -ExecutionPolicy Bypass -File scripts\diagnose-thx.ps1')
        }
        { $_ -in 'thx.install', 'thx.apo.component', 'audio.fx.binding' } {
            [void]$steps.Add('# 重装 THX APO（雷蛇官方安装包，管理员）：')
            [void]$steps.Add('C:\WINDOWS\Installer\Razer\THX-0565-BlackSharkV2HyperSpeed-329437\THXAPO-USB_BlackSharkV2HyperSpeed-0565-odessa-t2w_v3.2.3.0-a1194-mssigned_20240702-w27_i1036.exe')
            [void]$steps.Add('pnputil /enum-drivers | Select-String -Pattern "oem(40|45|48|49)\.inf"')
            [void]$steps.Add('# 或让 Synapse 重新检测设备：拔插接收器')
        }
        'audio.endpoint' {
            [void]$steps.Add('# 设置 -> 系统 -> 声音 -> 输出：确认 Razer BlackShark V2 HS 2.4 已启用')
            [void]$steps.Add('Get-PnpDevice -Class AudioEndpoint | Where-Object { $_.FriendlyName -match "BlackShark" }')
        }
        { $_ -in 'thx.dll', 'thx.api' } {
            [void]$steps.Add('explorer "%LOCALAPPDATA%\Razer\RazerAppEngine\User Data\Apps\Common\thxv3common"')
            [void]$steps.Add('# 确认 ThxV3Native_v1.1.42.1.dll 存在；不在就重装 THX APO')
            [void]$steps.Add('scripts\build.cmd -Stage')
        }
        'razer.appengine' {
            [void]$steps.Add('Start-Process "C:\Program Files\Razer\RazerAppEngine\RazerAppEngine.exe" -ArgumentList "--url-params=apps=synapse --launch-force-hidden=synapse"')
        }
        'synapse.stale' {
            [void]$steps.Add('Get-Process RazerAppEngine -ErrorAction SilentlyContinue | Stop-Process -Force')
            [void]$steps.Add('Start-Process "C:\Program Files\Razer\RazerAppEngine\RazerAppEngine.exe" -ArgumentList "--url-params=apps=synapse --launch-force-hidden=synapse"')
            [void]$steps.Add('# 只重启前端；APO / 音频栈 / VSSrv 都不需要动。')
        }
        'thx.config' {
            [void]$steps.Add('icacls "C:\ProgramData\THX\usb\1532\0565\thx_spatial.conf.json" /grant "Users:(M)"')
        }
        default {
            [void]$steps.Add('# 重启音频栈 + THX 服务')
            [void]$steps.Add('net stop Audiosrv /y ; net start AudioEndpointBuilder ; net start Audiosrv ; net start VSSrv')
        }
    }
    return $steps
}

function Invoke-Repair([string]$BreakId, $ctx) {
    Write-Section '自动修复'

    if (-not $ctx.Elevated) {
        Write-Host '需要管理员权限才能重启服务。请用管理员 PowerShell 运行：' -ForegroundColor Yellow
        Write-Host ('  powershell -ExecutionPolicy Bypass -File "{0}" -Fix' -f $PSCommandPath) -ForegroundColor Yellow
        return $false
    }

    $changed = $false
    switch ($BreakId) {
        { $_ -in 'thx.service', 'thx.recovery', 'thx.channel' } {
            Write-Host '重启 THX 服务 VSSrv ...' -ForegroundColor Gray
            try {
                Restart-Service -Name 'VSSrv' -Force -ErrorAction Stop
                Start-Sleep -Milliseconds 800
                $svc = Get-Service -Name 'VSSrv' -ErrorAction SilentlyContinue
                if ($svc -and $svc.Status -eq 'Running') {
                    Write-Host '  VSSrv 已重启。' -ForegroundColor Green
                    $changed = $true
                } else {
                    Write-Host '  VSSrv 未能启动，尝试重启音频栈 ...' -ForegroundColor Yellow
                    Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction SilentlyContinue
                    Start-Service -Name 'Audiosrv' -ErrorAction SilentlyContinue
                    Start-Sleep -Milliseconds 1200
                    Start-Service -Name 'VSSrv' -ErrorAction SilentlyContinue
                    Start-Sleep -Milliseconds 800
                    $svc = Get-Service -Name 'VSSrv' -ErrorAction SilentlyContinue
                    $changed = ($svc -and $svc.Status -eq 'Running')
                }
            } catch {
                Write-Host ('  重启失败: {0}' -f $_.Exception.Message) -ForegroundColor Red
            }

            Write-Host '配置崩溃自动恢复 ...' -ForegroundColor Gray
            & sc.exe failure VSSrv reset= 86400 actions= restart/5000/restart/10000/restart/30000 | Out-Null
            & sc.exe failureflag VSSrv 1 | Out-Null
            if ($LASTEXITCODE -eq 0) {
                Write-Host '  已配置：崩溃后 5s/10s/30s 自动重启，窗口 24h。' -ForegroundColor Green
                $changed = $true
            } else {
                Write-Host '  自动恢复配置失败（可能需要更高权限）。' -ForegroundColor Yellow
            }

            # 服务重启后端口会变，等它把新地址发布出来并开始监听。
            Write-Host '等待控制通道重新发布 ...' -ForegroundColor Gray
            $ready = $false
            for ($i = 0; $i -lt 10; $i++) {
                Start-Sleep -Milliseconds 600
                $ch = Get-ThxChannelState
                if (Test-ThxChannelReady $ch) { $ready = $true; break }
            }
            if ($ready) {
                Write-Host ('  通道就绪: {0}' -f ($ch.Ports -join '/')) -ForegroundColor Green
            } else {
                Write-Host '  通道未就绪，重启 Synapse 前端让它重新读取 Discovery ...' -ForegroundColor Yellow
                Get-Process -Name 'RazerAppEngine' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
                Start-Sleep -Milliseconds 1200
                $engineExe = 'C:\Program Files\Razer\RazerAppEngine\RazerAppEngine.exe'
                if (Test-Path -LiteralPath $engineExe) {
                    Start-Process -FilePath $engineExe -ArgumentList '--url-params=apps=synapse --launch-force-hidden=synapse' -ErrorAction SilentlyContinue
                    $changed = $true
                }
            }
        }
        'audio.services' {
            Write-Host '重启 Windows 音频栈 ...' -ForegroundColor Gray
            try {
                Restart-Service -Name 'AudioEndpointBuilder' -Force -ErrorAction Stop
                Start-Service -Name 'Audiosrv' -ErrorAction Stop
                Write-Host '  音频栈已重启。' -ForegroundColor Green
                $changed = $true
            } catch {
                Write-Host ('  失败: {0}' -f $_.Exception.Message) -ForegroundColor Red
            }
        }
        'razer.appengine' {
            Write-Host '启动 RazerAppEngine ...' -ForegroundColor Gray
            $engine = 'C:\Program Files\Razer\RazerAppEngine\RazerAppEngine.exe'
            if (Test-Path -LiteralPath $engine) {
                try {
                    Start-Process -FilePath $engine -ArgumentList '--url-params=apps=synapse --launch-force-hidden=synapse'
                    Write-Host '  已启动。' -ForegroundColor Green
                    $changed = $true
                } catch {
                    Write-Host ('  启动失败: {0}' -f $_.Exception.Message) -ForegroundColor Red
                }
            } else {
                Write-Host '  未找到 RazerAppEngine.exe。' -ForegroundColor Yellow
            }
        }
        'synapse.stale' {
            Write-Host '重启 Synapse 前端（RazerAppEngine）以重新读取 Discovery ...' -ForegroundColor Gray
            Get-Process -Name 'RazerAppEngine' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
            Start-Sleep -Milliseconds 1500
            $engine = 'C:\Program Files\Razer\RazerAppEngine\RazerAppEngine.exe'
            if (Test-Path -LiteralPath $engine) {
                try {
                    Start-Process -FilePath $engine -ArgumentList '--url-params=apps=synapse --launch-force-hidden=synapse'
                    Start-Sleep -Milliseconds 1500
                    Write-Host '  已重启，等待前端重新连接。' -ForegroundColor Green
                    $changed = $true
                } catch {
                    Write-Host ('  启动失败: {0}' -f $_.Exception.Message) -ForegroundColor Red
                }
            } else {
                Write-Host '  未找到 RazerAppEngine.exe。' -ForegroundColor Yellow
            }
        }
        { $_ -in 'thx.install', 'thx.apo.component', 'audio.fx.binding', 'audio.endpoint', 'thx.dll', 'thx.config' } {
            Write-Host '这一环需要人工处理（重装 THX APO / 启用端点 / 修权限），不自动执行。' -ForegroundColor Yellow
        }
        default {
            Write-Host '没有可自动执行的修复动作。' -ForegroundColor Yellow
        }
    }
    return $changed
}

# ---------------------------------------------------------------------------
# 主体
# ---------------------------------------------------------------------------

Write-Host ''
Write-Host 'SAIL · THX / 声音标准化 链路诊断' -ForegroundColor White
Write-Host ('时间 {0:yyyy-MM-dd HH:mm:ss}   管理员 {1}' -f (Get-Date), $(if (Test-Elevated) { '是' } else { '否' })) -ForegroundColor DarkGray

$ctx = Resolve-Context

Write-Host ''
Write-Host '调用链：音频栈 -> VSSrv(THX 服务) -> THX APO -> 端点 FX -> THX DLL/API -> Synapse' -ForegroundColor DarkGray
if ($ctx.EndpointId) {
    Write-Host ('端点    {0}' -f $ctx.EndpointId) -ForegroundColor DarkGray
    Write-Host ('名称    {0}' -f $ctx.EndpointName) -ForegroundColor DarkGray
} else {
    Write-Host '端点    未找到' -ForegroundColor Yellow
}
if ($ctx.ThxHost) {
    Write-Host ('探针    {0}  ({1})' -f $ctx.ThxHost, $ctx.ThxHostOrigin) -ForegroundColor DarkGray
    Write-Host '        只读探针（probe），不会开启或改动声音标准化。' -ForegroundColor DarkGray
}

Write-Section '逐环检查'
$results = Invoke-Checks $ctx
foreach ($r in $results) { Show-Result $r }

Show-Verdict $results $ctx

$break = Get-Break $results

if ($break) {
    $steps = Get-RepairSteps $break.Id
    if ($steps.Count -gt 0) {
        Write-Section '修复命令（管理员 PowerShell）'
        foreach ($s in $steps) {
            if ($s.StartsWith('#')) {
                Write-Host $s -ForegroundColor DarkGray
            } else {
                Write-Host ('  ' + $s) -ForegroundColor White
            }
        }
    }
}

if ($Fix) {
    if (-not $break) {
        Write-Host ''
        Write-Host '链路完整，无需修复。' -ForegroundColor Green
    } elseif (-not (Test-Elevated)) {
        Write-Host ''
        Write-Host '提权后重跑以自动修复：' -ForegroundColor Yellow
        $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', ('"{0}"' -f $PSCommandPath), '-Fix')
        if ($Verify) { $argList += '-Verify' }
        try {
            Start-Process -FilePath 'powershell.exe' -Verb RunAs -ArgumentList $argList | Out-Null
            Write-Host '已请求管理员权限，请在新窗口查看结果。' -ForegroundColor Gray
        } catch {
            Write-Host ('提权失败: {0}' -f $_.Exception.Message) -ForegroundColor Red
        }
    } else {
        $changed = Invoke-Repair $break.Id $ctx
        if ($changed) {
            Write-Host ''
            Write-Host '修复后复查：' -ForegroundColor Gray
            $ctx2 = Resolve-Context
            $results2 = Invoke-Checks $ctx2
            foreach ($r in $results2) { Show-Result $r }
            $break2 = Get-Break $results2
            Write-Host ''
            if ($break2) {
                Write-Host ('仍有断点：{0} —— 请按上方手工命令处理。' -f $break2.Id) -ForegroundColor Yellow
                $results = $results2
                $break = $break2
            } else {
                Write-Host '已恢复：链路完整。' -ForegroundColor Green
                $results = $results2
                $break = $null
            }
        }
    }
}

# 退出码：0 = 链路完整；1 = 仍有断点
if ($break) { exit 1 }
exit 0
