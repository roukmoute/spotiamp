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
using System.Text;
using System.Collections.Generic;
public static class SpotiampPerformanceWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    private delegate bool EnumWindowCallback(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] private static extern bool ClientToScreen(IntPtr window, ref Point point);
    [DllImport("user32.dll")] private static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern int GetWindowText(IntPtr window, StringBuilder text, int count);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern int GetClassName(IntPtr window, StringBuilder text, int count);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
    [DllImport("user32.dll", EntryPoint="SendMessageTimeoutW")]
    private static extern IntPtr SendMessageTimeout(IntPtr window, uint message, IntPtr w, IntPtr l, uint flags, uint timeout, out IntPtr result);
    [DllImport("gdi32.dll")] private static extern bool BitBlt(IntPtr target, int x, int y, int width, int height, IntPtr source, int sx, int sy, uint operation);
    public static IntPtr NamedWindow(int processId, bool library) {
        var result = IntPtr.Zero;
        int matchesCount = 0;
        EnumWindows((window, parameter) => {
            uint owner; GetWindowThreadProcessId(window, out owner);
            if (owner != processId || !IsWindowVisible(window)) return true;
            var text = new StringBuilder(256); GetWindowText(window, text, text.Capacity);
            var title = text.ToString();
            bool matches = library ? title == "Spotiamp — Bibliothèque Spotify"
                : title.StartsWith("Spotiamp ", StringComparison.Ordinal)
                    && title.EndsWith(" — lecteur Spotify expérimental", StringComparison.Ordinal);
            if (matches) {
                matchesCount++;
                result = window;
            }
            return true;
        }, IntPtr.Zero);
        if (matchesCount > 1) throw new InvalidOperationException("Ambiguous Spotiamp window title");
        return result;
    }
    private static string Identity(IntPtr window) {
        var title = new StringBuilder(256); var name = new StringBuilder(256); uint pid;
        GetWindowText(window, title, title.Capacity); GetClassName(window, name, name.Capacity);
        GetWindowThreadProcessId(window, out pid);
        return $"HWND={window.ToInt64()} PID={pid} class={name} title={title}";
    }
    public static object Describe(IntPtr window) {
        Rect screen, client; GetWindowRect(window, out screen); GetClientRect(window, out client);
        var result = new Dictionary<string, object> {
            ["window"] = Identity(window), ["visible"] = IsWindowVisible(window), ["minimized"] = IsIconic(window),
            ["screen_rect"] = $"{screen.Left},{screen.Top},{screen.Right},{screen.Bottom}",
            ["client_rect"] = $"{client.Left},{client.Top},{client.Right},{client.Bottom}",
            ["foreground"] = Identity(GetForegroundWindow())
        };
        // Observe the composed screen, never borrow the reader's CS_OWNDC.
        // A private DC may only be used by one thread at a time (GetDC docs).
        var dc = window == IntPtr.Zero ? IntPtr.Zero : GetDC(IntPtr.Zero);
        try {
            foreach (var point in new [] { new Point { X=10, Y=200 }, new Point { X=20, Y=260 } }) {
                var mapped = point; ClientToScreen(window, ref mapped);
                result[$"pixel_{point.X}_{point.Y}"] = dc == IntPtr.Zero ? "no_dc" : $"0x{GetPixel(dc, mapped.X, mapped.Y):X8}";
                result[$"window_at_{point.X}_{point.Y}"] = Identity(WindowFromPoint(mapped));
            }
        } finally { if (dc != IntPtr.Zero) ReleaseDC(IntPtr.Zero, dc); }
        IntPtr response;
        result["responds_to_wm_null"] = window != IntPtr.Zero && SendMessageTimeout(window, 0, IntPtr.Zero, IntPtr.Zero, 0x22, 1000, out response) != IntPtr.Zero;
        return result;
    }
    public static bool CopyClient(IntPtr window, IntPtr target, int width, int height) {
        var origin = new Point();
        if (!ClientToScreen(window, ref origin)) return false;
        var dc = GetDC(IntPtr.Zero);
        try { return dc != IntPtr.Zero && BitBlt(target, 0, 0, width, height, dc, origin.X, origin.Y, 0x00CC0020); }
        finally { if (dc != IntPtr.Zero) ReleaseDC(IntPtr.Zero, dc); }
    }

    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] private static extern IntPtr GetDC(IntPtr window);
    [DllImport("user32.dll")] private static extern int ReleaseDC(IntPtr window, IntPtr dc);
    [DllImport("gdi32.dll")] private static extern uint GetPixel(IntPtr dc, int x, int y);
    [DllImport("user32.dll", EntryPoint="PostMessageW")]
    public static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    public static bool Painted(IntPtr window) {
        return PaintedColors(window, 0x003d2829, 0x00050805);
    }
    public static bool PaintedColors(IntPtr window, uint backgroundColor, uint playlistColor) {
        if (window == IntPtr.Zero || !IsWindowVisible(window) || IsIconic(window)) return false;
        var background = new Point { X=10, Y=200 };
        var playlist = new Point { X=20, Y=260 };
        if (!ClientToScreen(window, ref background) || !ClientToScreen(window, ref playlist)) return false;
        if (WindowFromPoint(background) != window || WindowFromPoint(playlist) != window) return false;
        var dc = GetDC(IntPtr.Zero);
        if (dc == IntPtr.Zero) return false;
        try {
            // Unchanging client pixels mapped to the screen, with occlusion checked.
            // Do not call GetDC(window): minifb retains its private CS_OWNDC and
            // an external thread must not use that same DC while it is drawing.
            // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdc
            // COLORREF is BGR. Visibility alone can precede the first paint.
            return GetPixel(dc, background.X, background.Y) == backgroundColor && GetPixel(dc, playlist.X, playlist.Y) == playlistColor;
        } finally { ReleaseDC(IntPtr.Zero, dc); }
    }
}
'@

function Save-WindowDiagnostic([IntPtr]$Window, [string]$Name) {
    $details = [SpotiampPerformanceWindow]::Describe($Window)
    $details | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $OutputDirectory "$Name-window.json") -Encoding utf8
    if ($Window -eq [IntPtr]::Zero) { return }
    $rect = [SpotiampPerformanceWindow+Rect]::new()
    if (-not [SpotiampPerformanceWindow]::GetWindowRect($Window, [ref]$rect)) { return }
    $width = $rect.Right - $rect.Left; $height = $rect.Bottom - $rect.Top
    if ($width -lt 1 -or $height -lt 1 -or $width -gt 4096 -or $height -gt 4096) { return }
    $bitmap = [Drawing.Bitmap]::new($width, $height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $dc = $graphics.GetHdc()
        try { [void][SpotiampPerformanceWindow]::CopyClient($Window, $dc, $width, $height) }
        finally { $graphics.ReleaseHdc($dc) }
        $bitmap.Save((Join-Path $OutputDirectory "$Name-window.png"), [Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
}

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
        $player = [SpotiampPerformanceWindow]::NamedWindow($Process.Id, $false)
        if ($player -eq [IntPtr]::Zero -or -not [SpotiampPerformanceWindow]::PostMessage($player, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
            throw 'Cannot close benchmark window'
        }
        if (-not $Process.WaitForExit(10000)) { throw 'Benchmark process did not close normally' }
    }
    if ($Process.ExitCode -ne 0) { throw "Benchmark process exit code: $($Process.ExitCode)" }
}
function New-SkinProcessInfo([string]$Skin, [string]$Configuration) {
    [IO.Directory]::CreateDirectory($Configuration) | Out-Null
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $executables.candidate
    $info.WorkingDirectory = $Configuration
    $info.UseShellExecute = $false
    $info.ArgumentList.Add('--no-update-check')
    $info.ArgumentList.Add('--skin')
    $info.ArgumentList.Add($Skin)
    $info.Environment['APPDATA'] = $Configuration
    $info.Environment['LOCALAPPDATA'] = $Configuration
    return $info
}
function Get-SkinReference([string]$Skin) {
    # The production renderer supplies the expected pixels before timing starts.
    # This avoids encoding a second copy of either skin's palette in the test.
    $path = Join-Path $OutputDirectory "skin-$Skin-reference.ppm"
    $info = New-SkinProcessInfo $Skin (Join-Path $scratch "reference-$Skin")
    $info.ArgumentList.Add('--screenshot')
    $info.ArgumentList.Add($path)
    $process = [Diagnostics.Process]::Start($info)
    try {
        [void]$process.Handle
        if (-not $process.WaitForExit(15000)) { throw "Timed out rendering $Skin reference" }
        if ($process.ExitCode -ne 0) { throw "Cannot render $Skin reference: exit $($process.ExitCode)" }
    } finally {
        if (-not $process.HasExited) { $process.Kill($true); [void]$process.WaitForExit(5000) }
        $process.Dispose()
    }
    $bytes = [IO.File]::ReadAllBytes($path)
    $header = "P6`n560 470`n255`n"
    if ($bytes.Length -ne $header.Length + 560 * 470 * 3 -or [Text.Encoding]::ASCII.GetString($bytes, 0, $header.Length) -ne $header) {
        throw "Unexpected $Skin reference dimensions/format"
    }
    foreach ($point in @(@(10, 200), @(20, 260))) {
        $offset = $header.Length + ($point[1] * 560 + $point[0]) * 3
        # PPM is RGB; Win32 GetPixel returns a BGR COLORREF.
        [uint32]$bytes[$offset] -bor ([uint32]$bytes[$offset + 1] -shl 8) -bor ([uint32]$bytes[$offset + 2] -shl 16)
    }
}
$report = [ordered]@{
    schema_version = 3
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    platform = [Environment]::OSVersion.VersionString
    logical_processors = [Environment]::ProcessorCount
    scope = 'Comparable reader-only startup/idle pairs; separate candidate default two-window idle and Classic/Modern comparison. Disconnected native UI, no playback, no update check, no Spotify account; one process at a time'
    startup_note = 'Launch to painted client observed on the screen with occlusion checked; screen DC only, 5ms polling, warm filesystem cache; shared CI runner'
    memory_note = 'Working Set includes shared resident pages; Private Bytes is private committed memory. Do not add them.'
    success = $false
    executables = @{}
    startups = @()
    idle = @()
    default_layout_idle = $null
    skin_comparison = @()
}
try {
    foreach ($label in $executables.Keys) {
        $path = $executables[$label]
        $report.executables[$label] = @{
            bytes = (Get-Item -LiteralPath $path).Length
            sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
            comparison_arguments = if ($label -eq 'candidate') { @('--no-update-check', '--no-library') } else { @('--no-update-check') }
        }
    }
    # Five startup pairs, then two idle pairs. Alternate order to reduce cache
    # and scheduling bias; each launch receives empty private configuration.
    # The final candidate-only run measures its default two-window layout
    # separately, without changing the reader-only comparison above.
    for ($round = 0; $round -lt 8; $round++) {
        $defaultLayout = $round -eq 7
        $order = if ($defaultLayout) { @('candidate') } elseif ($round % 2) { @('candidate', 'baseline') } else { @('baseline', 'candidate') }
        foreach ($label in $order) {
            $run = if ($defaultLayout) { 'candidate-default-layout' } else { "$label-$round" }
            $config = Join-Path $scratch $run
            [IO.Directory]::CreateDirectory($config) | Out-Null
            $info = [Diagnostics.ProcessStartInfo]::new()
            $info.FileName = $executables[$label]
            $info.WorkingDirectory = $config
            $info.UseShellExecute = $false
            $info.ArgumentList.Add('--no-update-check')
            if ($label -eq 'candidate' -and -not $defaultLayout) { $info.ArgumentList.Add('--no-library') }
            $info.Environment['APPDATA'] = $config
            $info.Environment['LOCALAPPDATA'] = $config
            $process = $null
            try {
                $clock = [Diagnostics.Stopwatch]::StartNew()
                $process = [Diagnostics.Process]::Start($info)
                [void]$process.Handle
                $painted = $false
                $player = [IntPtr]::Zero
                while ($clock.Elapsed.TotalSeconds -lt 15) {
                    $process.Refresh()
                    if ($process.HasExited) { throw 'Reader exited before first paint' }
                    if ($player -eq [IntPtr]::Zero) { $player = [SpotiampPerformanceWindow]::NamedWindow($process.Id, $false) }
                    if ([SpotiampPerformanceWindow]::Painted($player)) { $painted = $true; break }
                    Start-Sleep -Milliseconds 5
                }
                if (-not $painted) {
                    Save-WindowDiagnostic $player "$run-before-activation"
                    # Diagnostic only: an activated window never rescues a failed measurement.
                    [void][SpotiampPerformanceWindow]::SetForegroundWindow($player)
                    Start-Sleep -Milliseconds 300
                    Save-WindowDiagnostic $player "$run-after-activation"
                    throw "No painted reader observed within 15 seconds: $run"
                }
                $startup = $clock.Elapsed.TotalMilliseconds
                if ($round -eq 0 -or $defaultLayout) { Save-WindowDiagnostic $player "$run-painted" }
                if ($defaultLayout) {
                    $library = [IntPtr]::Zero
                    $libraryClock = [Diagnostics.Stopwatch]::StartNew()
                    while ($libraryClock.Elapsed.TotalSeconds -lt 15) {
                        $process.Refresh()
                        if ($process.HasExited) { throw 'Reader exited before opening its default library' }
                        $library = [SpotiampPerformanceWindow]::NamedWindow($process.Id, $true)
                        if ($library -ne [IntPtr]::Zero) { break }
                        Start-Sleep -Milliseconds 20
                    }
                    if ($library -eq [IntPtr]::Zero) { throw 'Default Spotify library did not open' }
                    Save-WindowDiagnostic $library "$run-library"
                }
                if ($round -lt 5) {
                    $report.startups += @{ label = $label; round = $round; milliseconds = $startup }
                } else {
                    $result = Join-Path $OutputDirectory "$run-idle.json"
                    $seconds = if ($defaultLayout) { 20 } else { 30 }
                    & (Join-Path $PSScriptRoot 'measure-windows.ps1') -TargetProcessId $process.Id -Scenario idle -Seconds $seconds -WarmupSeconds 10 -IntervalMs 500 -OutputPath $result
                    $idle = Get-Content -LiteralPath $result -Raw | ConvertFrom-Json
                    if ($idle.completion -ne 'duration_elapsed' -or $idle.sample_count -lt ($seconds * 2 - 5)) { throw 'Incomplete idle measurement' }
                    $first = $idle.samples[0]; $last = $idle.samples[-1]
                    $measurement = @{
                        label = $label; round = $round; samples = $idle.sample_count
                        working_set_median_bytes = $idle.summary.working_set_bytes.median_bytes
                        private_median_bytes = $idle.summary.private_bytes.median_bytes
                        cpu_seconds = $last.cpu_seconds - $first.cpu_seconds
                        measured_seconds = $last.elapsed_seconds - $first.elapsed_seconds
                        threads = @($idle.samples.threads | Sort-Object -Unique)
                        raw_file = [IO.Path]::GetFileName($result)
                    }
                    if ($defaultLayout) {
                        if ([SpotiampPerformanceWindow]::NamedWindow($process.Id, $true) -ne $library) { throw 'Default library disappeared during idle measurement' }
                        $measurement.arguments = @('--no-update-check')
                        $measurement.window_count = 2
                        $report.default_layout_idle = $measurement
                    } else {
                        if ([SpotiampPerformanceWindow]::NamedWindow($process.Id, $true) -ne [IntPtr]::Zero) { throw 'Reader-only comparison unexpectedly opened the library' }
                        $report.idle += $measurement
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

    # Compare skins in this exact candidate, with the library open in both.
    # Keep these runs out of the existing baseline/reader-only result fields.
    $skinRuns = [ordered]@{}
    foreach ($skin in @('classic', 'modern')) {
        [uint32[]]$colors = @(Get-SkinReference $skin)
        if ($colors.Count -ne 2) { throw "Invalid $skin reference colors" }
        $skinRuns[$skin] = @{
            skin = $skin
            executable_sha256 = $report.executables.candidate.sha256
            arguments = @('--no-update-check', '--skin', $skin)
            window_count = 2
            reference_ppm = "skin-$skin-reference.ppm"
            reference_pixels_bgr = $colors
            captures = @("skin-$skin-0-player-window.png", "skin-$skin-0-library-window.png")
            startup_milliseconds = @()
            startup_note = 'Player painted in the selected skin and library visible; 5ms polling, three alternating runs'
            idle = $null
        }
    }
    if ($skinRuns.classic.reference_pixels_bgr[0] -eq $skinRuns.modern.reference_pixels_bgr[0] -and
        $skinRuns.classic.reference_pixels_bgr[1] -eq $skinRuns.modern.reference_pixels_bgr[1]) {
        throw 'Skin references must differ at the observed pixels; cannot verify Modern activation'
    }
    for ($round = 0; $round -lt 3; $round++) {
        $order = if ($round % 2) { @('modern', 'classic') } else { @('classic', 'modern') }
        foreach ($skin in $order) {
            $run = "skin-$skin-$round"
            $configuration = Join-Path $scratch $run
            $info = New-SkinProcessInfo $skin $configuration
            $colors = $skinRuns[$skin].reference_pixels_bgr
            $process = $null
            try {
                $clock = [Diagnostics.Stopwatch]::StartNew()
                $process = [Diagnostics.Process]::Start($info)
                [void]$process.Handle
                $player = [IntPtr]::Zero; $library = [IntPtr]::Zero; $painted = $false
                while ($clock.Elapsed.TotalSeconds -lt 15) {
                    $process.Refresh()
                    if ($process.HasExited) { throw "$run exited before first paint" }
                    if ($player -eq [IntPtr]::Zero) { $player = [SpotiampPerformanceWindow]::NamedWindow($process.Id, $false) }
                    if ($library -eq [IntPtr]::Zero) { $library = [SpotiampPerformanceWindow]::NamedWindow($process.Id, $true) }
                    if ($library -ne [IntPtr]::Zero -and [SpotiampPerformanceWindow]::PaintedColors($player, $colors[0], $colors[1])) {
                        $painted = $true; break
                    }
                    Start-Sleep -Milliseconds 5
                }
                if (-not $painted) {
                    Save-WindowDiagnostic $player "$run-failed-player"
                    Save-WindowDiagnostic $library "$run-failed-library"
                    throw "No painted $skin reader and visible library within 15 seconds"
                }
                $skinRuns[$skin].startup_milliseconds += $clock.Elapsed.TotalMilliseconds
                if ($round -eq 0) {
                    # Capture after the measured boundary so diagnostic overhead
                    # and the library's first paint do not distort launch timing.
                    Start-Sleep -Milliseconds 100
                    Save-WindowDiagnostic $player "$run-player"
                    Save-WindowDiagnostic $library "$run-library"
                }
                if ($round -eq 2) {
                    $result = Join-Path $OutputDirectory "$run-idle.json"
                    & (Join-Path $PSScriptRoot 'measure-windows.ps1') -TargetProcessId $process.Id -Scenario idle -Seconds 20 -WarmupSeconds 10 -IntervalMs 500 -OutputPath $result
                    $idle = Get-Content -LiteralPath $result -Raw | ConvertFrom-Json
                    if ($idle.completion -ne 'duration_elapsed' -or $idle.sample_count -lt 35) { throw "Incomplete $skin idle measurement" }
                    if ([SpotiampPerformanceWindow]::NamedWindow($process.Id, $true) -ne $library -or
                        -not [SpotiampPerformanceWindow]::PaintedColors($player, $colors[0], $colors[1])) {
                        throw "$skin windows changed during idle measurement"
                    }
                    $first = $idle.samples[0]; $last = $idle.samples[-1]
                    $skinRuns[$skin].idle = @{
                        samples = $idle.sample_count
                        working_set_median_bytes = $idle.summary.working_set_bytes.median_bytes
                        private_median_bytes = $idle.summary.private_bytes.median_bytes
                        cpu_seconds = $last.cpu_seconds - $first.cpu_seconds
                        measured_seconds = $last.elapsed_seconds - $first.elapsed_seconds
                        threads = @($idle.samples.threads | Sort-Object -Unique)
                        raw_file = [IO.Path]::GetFileName($result)
                    }
                }
                Close-OwnedProcess $process
                if (Test-Path -LiteralPath (Join-Path $configuration 'Spotiamp/skin')) {
                    throw "CLI $skin override unexpectedly saved a preference"
                }
            } finally {
                if ($process) {
                    if (-not $process.HasExited) { $process.Kill($true); [void]$process.WaitForExit(5000) }
                    $process.Dispose()
                }
            }
        }
    }
    foreach ($skin in $skinRuns.Keys) {
        $skinRuns[$skin].startup_median_ms = Median $skinRuns[$skin].startup_milliseconds
        $report.skin_comparison += $skinRuns[$skin]
    }
    $report.success = $true
} catch {
    $report.error = $_.Exception.Message
    $report.error_location = $_.ScriptStackTrace
    foreach ($label in $executables.Keys) {
        $render = [Diagnostics.ProcessStartInfo]::new()
        $render.FileName = $executables[$label]; $render.UseShellExecute = $false
        $render.ArgumentList.Add('--screenshot')
        $render.ArgumentList.Add((Join-Path $OutputDirectory "$label-framebuffer.ppm"))
        $render.Environment['APPDATA'] = $scratch; $render.Environment['LOCALAPPDATA'] = $scratch
        $child = [Diagnostics.Process]::Start($render)
        try { if (-not $child.WaitForExit(10000)) { $child.Kill($true) } } finally { $child.Dispose() }
    }
    throw
} finally {
    $report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'comparison.json') -Encoding utf8
    Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction Continue
}
$report | ConvertTo-Json -Depth 10
