<#
.SYNOPSIS
Compare two release executables on the same disposable Windows CI runner.
.NOTES
Offline, disconnected UI only; this does not benchmark Spotify playback.
Requires a fresh GitHub runner with no saved Spotify session. Results include
warm filesystem cache and shared-runner noise; they are not hardware promises.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$BaselineExe,
    [Parameter(Mandatory = $true)][string]$CandidateExe,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or $env:GITHUB_ACTIONS -ne 'true') {
    throw 'Run this comparison only on a disposable Windows GitHub runner with no Spotify credentials.'
}
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
[IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
$scratch = Join-Path ([IO.Path]::GetTempPath()) ('Spotiamp-perf-' + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($scratch) | Out-Null
$executables = [ordered]@{
    baseline = (Resolve-Path -LiteralPath $BaselineExe).Path
    candidate = (Resolve-Path -LiteralPath $CandidateExe).Path
}
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class SpotiampPerformanceWindow {
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] private static extern IntPtr GetDC(IntPtr window);
    [DllImport("user32.dll")] private static extern int ReleaseDC(IntPtr window, IntPtr dc);
    [DllImport("gdi32.dll")] private static extern uint GetPixel(IntPtr dc, int x, int y);
    [DllImport("user32.dll", EntryPoint="PostMessageW")]
    public static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    public static bool Painted(IntPtr window) {
        if (window == IntPtr.Zero || !IsWindowVisible(window)) return false;
        var dc = GetDC(window);
        if (dc == IntPtr.Zero) return false;
        try {
            // Unchanging client pixels: skin background and empty playlist.
            // COLORREF is BGR. Visibility alone can precede the first paint.
            return GetPixel(dc, 10, 200) == 0x003d2829 && GetPixel(dc, 20, 260) == 0x00050805;
        } finally { ReleaseDC(window, dc); }
    }
}
'@

function Median($Values) {
    $sorted = @($Values | Sort-Object)
    if ($sorted.Count -eq 0) { throw 'No samples' }
    $mid = [int][Math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2) { return $sorted[$mid] }
    return ($sorted[$mid - 1] + $sorted[$mid]) / 2
}
function Close-OwnedProcess([Diagnostics.Process]$Process) {
    if (-not $Process.HasExited) {
        $Process.Refresh()
        if (-not [SpotiampPerformanceWindow]::PostMessage($Process.MainWindowHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
            throw 'Cannot close benchmark window'
        }
        if (-not $Process.WaitForExit(10000)) { throw 'Benchmark process did not close normally' }
    }
    if ($Process.ExitCode -ne 0) { throw "Benchmark process exit code: $($Process.ExitCode)" }
}
$report = [ordered]@{
    schema_version = 1
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    platform = [Environment]::OSVersion.VersionString
    logical_processors = [Environment]::ProcessorCount
    scope = 'Disconnected native UI, no playback, no update check, no Spotify account; one process at a time'
    startup_note = 'Launch to visible and painted client, 5ms polling, warm filesystem cache; shared CI runner'
    memory_note = 'Working Set includes shared resident pages; Private Bytes is private committed memory. Do not add them.'
    success = $false
    executables = @{}
    startups = @()
    idle = @()
}
try {
    foreach ($label in $executables.Keys) {
        $path = $executables[$label]
        $report.executables[$label] = @{
            bytes = (Get-Item -LiteralPath $path).Length
            sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
    # Five startup pairs, then two idle pairs. Alternate order to reduce cache
    # and scheduling bias; each launch receives empty private configuration.
    for ($round = 0; $round -lt 7; $round++) {
        $order = if ($round % 2) { @('candidate', 'baseline') } else { @('baseline', 'candidate') }
        foreach ($label in $order) {
            $run = "$label-$round"
            $config = Join-Path $scratch $run
            [IO.Directory]::CreateDirectory($config) | Out-Null
            $info = [Diagnostics.ProcessStartInfo]::new()
            $info.FileName = $executables[$label]
            $info.WorkingDirectory = $config
            $info.UseShellExecute = $false
            $info.ArgumentList.Add('--no-update-check')
            $info.Environment['APPDATA'] = $config
            $info.Environment['LOCALAPPDATA'] = $config
            $process = $null
            try {
                $clock = [Diagnostics.Stopwatch]::StartNew()
                $process = [Diagnostics.Process]::Start($info)
                [void]$process.Handle
                $painted = $false
                while ($clock.Elapsed.TotalSeconds -lt 15) {
                    $process.Refresh()
                    if ($process.HasExited) { throw 'Reader exited before first paint' }
                    if ([SpotiampPerformanceWindow]::Painted($process.MainWindowHandle)) { $painted = $true; break }
                    Start-Sleep -Milliseconds 5
                }
                if (-not $painted) { throw 'No painted reader observed within 15 seconds' }
                $startup = $clock.Elapsed.TotalMilliseconds
                if ($round -lt 5) {
                    $report.startups += @{ label = $label; round = $round; milliseconds = $startup }
                } else {
                    $result = Join-Path $OutputDirectory "$run-idle.json"
                    & (Join-Path $PSScriptRoot 'measure-windows.ps1') -TargetProcessId $process.Id -Scenario idle -Seconds 30 -WarmupSeconds 10 -IntervalMs 500 -OutputPath $result
                    $idle = Get-Content -LiteralPath $result -Raw | ConvertFrom-Json
                    if ($idle.completion -ne 'duration_elapsed' -or $idle.sample_count -lt 55) { throw 'Incomplete idle measurement' }
                    $first = $idle.samples[0]; $last = $idle.samples[-1]
                    $report.idle += @{
                        label = $label; round = $round; samples = $idle.sample_count
                        working_set_median_bytes = $idle.summary.working_set_bytes.median_bytes
                        private_median_bytes = $idle.summary.private_bytes.median_bytes
                        cpu_seconds = $last.cpu_seconds - $first.cpu_seconds
                        measured_seconds = $last.elapsed_seconds - $first.elapsed_seconds
                        threads = @($idle.samples.threads | Sort-Object -Unique)
                        raw_file = [IO.Path]::GetFileName($result)
                    }
                }
                Close-OwnedProcess $process
            } finally {
                if ($process) {
                    if (-not $process.HasExited) { $process.Kill($true); [void]$process.WaitForExit(5000) }
                    $process.Dispose()
                }
            }
        }
    }
    $report.startup_medians_ms = @{}
    foreach ($label in $executables.Keys) {
        $report.startup_medians_ms[$label] = Median @($report.startups | Where-Object label -eq $label | ForEach-Object milliseconds)
    }
    $report.success = $true
} catch {
    $report.error = $_.Exception.Message
    $report.error_location = $_.ScriptStackTrace
    throw
} finally {
    $report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'comparison.json') -Encoding utf8
    Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction Continue
}
$report | ConvertTo-Json -Depth 10
