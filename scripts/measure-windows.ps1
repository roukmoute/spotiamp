<#
.SYNOPSIS
Sample a single process's real working set and private committed bytes.
.EXAMPLE
.\scripts\measure-windows.ps1 -ExePath .\spotiamp.exe -Scenario idle -Seconds 60
.EXAMPLE
.\scripts\measure-windows.ps1 -TargetProcessId 1234 -Scenario playing -Seconds 120
.NOTES
Start playback yourself before using -Scenario playing. The label does not verify playback.
The script leaves the process running. Child processes are excluded.
#>
[CmdletBinding(DefaultParameterSetName = 'Launch')]
param(
    [Parameter(ParameterSetName = 'Launch', Mandatory = $true)]
    [string]$ExePath,
    [Parameter(ParameterSetName = 'Launch')]
    [string[]]$ExecutableArguments = @(),
    [Parameter(ParameterSetName = 'Attach', Mandatory = $true)]
    [ValidateRange(1, 2147483647)]
    [int]$TargetProcessId,
    [Parameter(Mandatory = $true)]
    [ValidateSet('idle', 'playing')]
    [string]$Scenario,
    [ValidateRange(1, 86400)]
    [int]$Seconds = 60,
    [ValidateRange(10, 60000)]
    [int]$IntervalMs = 1000,
    [ValidateRange(0, 3600)]
    [int]$WarmupSeconds = 5,
    [Parameter(ParameterSetName = 'Launch')]
    [switch]$MeasureStartup,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'This sampler requires Windows.'
}
if (-not $OutputPath) { $OutputPath = "memory-windows-$Scenario.json" }

function Get-Distribution($Values) {
    $ordered = @($Values | Sort-Object)
    if ($ordered.Count -eq 0) { return $null }
    $middle = [int][Math]::Floor($ordered.Count / 2)
    $median = $ordered[$middle]
    if ($ordered.Count % 2 -eq 0) {
        $median = ($ordered[$middle - 1] + $ordered[$middle]) / 2
    }
    return [ordered]@{
        min_bytes = $ordered[0]
        median_bytes = $median
        p95_bytes = $ordered[[Math]::Max(0, [Math]::Ceiling($ordered.Count * 0.95) - 1)]
        max_bytes = $ordered[-1]
    }
}

if ($MeasureStartup -and -not ('SpotiampWindowProbe' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class SpotiampWindowProbe {
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindowVisible(IntPtr hWnd);
}
'@
}

$stopwatch = [Diagnostics.Stopwatch]::StartNew()
$startupMs = $null
$startupNote = 'Not measured; use -MeasureStartup with -ExePath for a best-effort window-visibility observation.'
if ($PSCmdlet.ParameterSetName -eq 'Launch') {
    $resolvedExe = (Resolve-Path -LiteralPath $ExePath).Path
    $startOptions = @{ FilePath = $resolvedExe; PassThru = $true }
    if ($ExecutableArguments.Count -gt 0) { $startOptions.ArgumentList = $ExecutableArguments }
    $process = Start-Process @startOptions
    if ($MeasureStartup) {
        $startupNote = 'Observed launch-to-visible main window, polled every 10 ms; includes launch overhead. This does not establish first paint or audio readiness.'
        while ($stopwatch.ElapsedMilliseconds -lt 10000) {
            $process.Refresh()
            if ($process.HasExited) { break }
            if ($process.MainWindowHandle -ne [IntPtr]::Zero -and [SpotiampWindowProbe]::IsWindowVisible($process.MainWindowHandle)) {
                $startupMs = $stopwatch.Elapsed.TotalMilliseconds
                break
            }
            Start-Sleep -Milliseconds 10
        }
        if ($null -eq $startupMs) { $startupNote += ' No visible main window observed before exit or the 10-second timeout.' }
    }
} else {
    $process = Get-Process -Id $TargetProcessId
    $resolvedExe = $process.Path
}

$digest = $null
if ($resolvedExe) { $digest = (Get-FileHash -LiteralPath $resolvedExe -Algorithm SHA256).Hash.ToLowerInvariant() }
$samples = New-Object 'System.Collections.Generic.List[object]'
$report = [ordered]@{
    schema_version = 1
    platform = [Environment]::OSVersion.VersionString
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    pid = $process.Id
    executable = $resolvedExe
    executable_sha256 = $digest
    scenario = $Scenario
    scenario_verified = $false
    measurement_scope = 'single process; children excluded'
    metric_note = 'Private Bytes is private committed memory. Working Set is resident memory and can include shared pages. Values may differ from Task Manager columns.'
    startup_to_visible_ms = $startupMs
    startup_note = $startupNote
    requested_seconds = $Seconds
    interval_ms = $IntervalMs
    warmup_seconds = $WarmupSeconds
    completion = 'duration_elapsed'
}

try {
    Start-Sleep -Seconds $WarmupSeconds
    $deadline = $stopwatch.Elapsed.TotalSeconds + $Seconds
    while ($stopwatch.Elapsed.TotalSeconds -lt $deadline) {
        $process.Refresh()
        if ($process.HasExited) {
            $report.completion = 'process_exited'
            break
        }
        $samples.Add([ordered]@{
            elapsed_seconds = $stopwatch.Elapsed.TotalSeconds
            private_bytes = $process.PrivateMemorySize64
            working_set_bytes = $process.WorkingSet64
            peak_working_set_bytes = $process.PeakWorkingSet64
            cpu_seconds = $process.TotalProcessorTime.TotalSeconds
            threads = $process.Threads.Count
        })
        $remainingMs = [Math]::Max(0, ($deadline - $stopwatch.Elapsed.TotalSeconds) * 1000)
        Start-Sleep -Milliseconds ([int][Math]::Min($IntervalMs, $remainingMs))
    }
} catch {
    $report.completion = 'sampling_error'
    $report.error = $_.Exception.Message
} finally {
    $report.elapsed_seconds = $stopwatch.Elapsed.TotalSeconds
    $report.sample_count = $samples.Count
    $report.summary = [ordered]@{
        private_bytes = Get-Distribution @($samples | ForEach-Object { $_.private_bytes })
        working_set_bytes = Get-Distribution @($samples | ForEach-Object { $_.working_set_bytes })
    }
    $report.samples = @($samples.ToArray())
    $fullOutput = [IO.Path]::GetFullPath($OutputPath)
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($fullOutput)) | Out-Null
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $fullOutput -Encoding UTF8
    Write-Host "Saved $($samples.Count) samples to $fullOutput"
}
if ($samples.Count -eq 0 -or $report.completion -eq 'sampling_error') { exit 1 }
