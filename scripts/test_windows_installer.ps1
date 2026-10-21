<#
.SYNOPSIS
Validate a real per-user Inno install, native update handoff, reinstall and uninstall in Windows CI.
.NOTES
The old install is an explicit fixture: install the current setup, overlay the pinned
published alpha.8 package while keeping Inno's uninstaller, and set its registered
version to alpha.8. The actual alpha.8 helper then replaces the running old reader
with the current package and starts the new reader. This does not test GitHub release
discovery, network download, or Spotify playback. No user credentials are accessed.
Requires PowerShell 7 and RUNNER_TEMP. Refuses an existing Spotiamp installation.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$SetupExe,
    [Parameter(Mandatory = $true)][string]$PortableZip,
    [Parameter(Mandatory = $true)][string]$BaselineZip,
    [string]$ExpectedVersion = '0.1.0',
    [string]$OutputDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or $PSVersionTable.PSVersion.Major -lt 7) {
    throw 'This test requires native Windows and PowerShell 7.'
}
if (-not $env:RUNNER_TEMP -or -not (Test-Path -LiteralPath $env:RUNNER_TEMP -PathType Container)) {
    throw 'RUNNER_TEMP must identify an existing, disposable CI workspace.'
}

$applicationId = '4A36A2DD-2701-4EF4-A3B3-5C1EF1A8A1D0'
$uninstallSubkey = "Software\Microsoft\Windows\CurrentVersion\Uninstall\{$applicationId}_is1"
$applicationMutex = "Local\Spotiamp.Application.$applicationId"
$setupMutex = "Local\Spotiamp.Setup.$applicationId"
$baselineArchiveHash = '0798093954888d728a341617e108d5f6dfad58647e538fd7b33b685371e7f579'
$baselineBinaryHash = 'd7d97bb3aa421e3a330b64a826b0af728bdb72b80cb679c5129e52cc52ee58f6'
$testToken = [Guid]::NewGuid().ToString('N')
$testRoot = Join-Path $env:RUNNER_TEMP "Spotiamp installer test espaces é $testToken"
$installDirectory = Join-Path $testRoot 'Installation é avec espaces\Spotiamp'
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $testRoot 'report' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
$startMenuDirectory = Join-Path ([Environment]::GetFolderPath('Programs')) 'Spotiamp'
$startMenuLink = Join-Path $startMenuDirectory 'Spotiamp.lnk'
$desktopLink = Join-Path ([Environment]::GetFolderPath('DesktopDirectory')) 'Spotiamp.lnk'
$userDataDirectory = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'Spotiamp'
$userDataSentinel = Join-Path $userDataDirectory "installer-ci-$testToken.txt"
$qualityPath = Join-Path $userDataDirectory 'audio-quality'
$createdQuality = $false
$createdUserDataDirectory = $false
$ownedInstallation = $false
$uninstaller = $null
$stage = $null
$running = [Collections.Generic.List[Diagnostics.Process]]::new()
$checks = [Collections.Generic.List[string]]::new()
$report = [ordered]@{
    schema_version = 1
    success = $false
    expected_version = $ExpectedVersion
    platform = [Environment]::OSVersion.VersionString
    recorded_at_utc = [DateTime]::UtcNow.ToString('o')
    fixture_note = 'Current Inno installation overlaid with the pinned published alpha.8 portable files; actual old helper and actual new executable used. GitHub discovery/download and Spotify playback are not tested.'
    installation_directory = $installDirectory
    checks = $checks
    error = $null
    cleanup_errors = @()
}

function Assert-That([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
function Add-Check([string]$Message) {
    $checks.Add($Message)
    Write-Host "PASS: $Message"
}
function Get-Hash([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}
function Write-Utf8([string]$Path, [string]$Text) {
    [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false))
}
function Same-Path([string]$First, [string]$Second) {
    # The Rust helper restarts the reader through a canonical \\?\ path. Windows
    # can retain that spelling in MainModule.FileName for the new process.
    function Normalize-Path([string]$Value) {
        $value = [IO.Path]::GetFullPath($Value)
        if ($value.StartsWith('\\?\UNC\', [StringComparison]::OrdinalIgnoreCase)) {
            $value = '\\' + $value.Substring(8)
        } elseif ($value.StartsWith('\\?\', [StringComparison]::OrdinalIgnoreCase)) {
            $value = $value.Substring(4)
        }
        return $value.TrimEnd('\')
    }
    return [string]::Equals((Normalize-Path $First), (Normalize-Path $Second), [StringComparison]::OrdinalIgnoreCase)
}
function Wait-Until([string]$Description, [scriptblock]$Condition, [int]$Seconds = 15) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    do {
        $value = & $Condition
        if ($value) { return $value }
        Start-Sleep -Milliseconds 40
    } while ($watch.Elapsed.TotalSeconds -lt $Seconds)
    throw "Timed out waiting for $Description"
}
function Read-Registration {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($uninstallSubkey)
    if ($null -eq $key) { return $null }
    try {
        $values = @{}
        foreach ($name in $key.GetValueNames()) { $values[$name] = $key.GetValue($name) }
        return $values
    } finally { $key.Dispose() }
}
function Assert-NoMachineRegistration {
    foreach ($view in @([Microsoft.Win32.RegistryView]::Registry32, [Microsoft.Win32.RegistryView]::Registry64)) {
        $hive = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, $view)
        try {
            $key = $hive.OpenSubKey($uninstallSubkey)
            if ($null -ne $key) {
                $key.Dispose()
                throw "Unexpected machine-wide Spotiamp registration in $view"
            }
        } finally { $hive.Dispose() }
    }
}
function Start-Native([string]$File, [string[]]$Arguments = @(), [string]$WorkingDirectory) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $File
    $info.UseShellExecute = $false
    $info.WorkingDirectory = if ($WorkingDirectory) { $WorkingDirectory } else { [IO.Path]::GetDirectoryName($File) }
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($info)
    $running.Add($process)
    return $process
}
function Wait-Success([Diagnostics.Process]$Process, [int]$Seconds = 90) {
    Assert-That ($Process.WaitForExit($Seconds * 1000)) "Process $($Process.Id) did not exit within $Seconds seconds"
    Assert-That ($Process.ExitCode -eq 0) "Process $($Process.Id) failed with exit code $($Process.ExitCode)"
}
function Wait-Window([Diagnostics.Process]$Process) {
    return Wait-Until "visible UI for PID $($Process.Id)" {
        $Process.Refresh()
        if ($Process.HasExited) { throw "Reader exited before showing a native window: $($Process.ExitCode)" }
        $handle = $Process.MainWindowHandle
        if ($handle -ne [IntPtr]::Zero -and [SpotiampInstallerWindow]::IsWindowVisible($handle)) { return $handle }
        return $false
    }
}
function Close-Reader([Diagnostics.Process]$Process) {
    $Process.Refresh()
    if (-not $Process.HasExited) {
        $handle = Wait-Window $Process
        Assert-That ([SpotiampInstallerWindow]::PostMessage($handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) 'WM_CLOSE failed'
        Wait-Success $Process 10
    }
}
function Assert-ApplicationMutex {
    $mutex = $null
    try {
        $mutex = [Threading.Mutex]::OpenExisting($applicationMutex)
        Assert-That ($null -ne $mutex) 'Application mutex is missing while the reader is running'
    } finally { if ($null -ne $mutex) { $mutex.Dispose() } }
}
function Assert-Shortcuts {
    $shell = New-Object -ComObject WScript.Shell
    try {
        foreach ($link in @($startMenuLink, $desktopLink)) {
            Assert-That (Test-Path -LiteralPath $link -PathType Leaf) "Missing shortcut: $link"
            $shortcut = $shell.CreateShortcut($link)
            try {
                Assert-That (Same-Path $shortcut.TargetPath (Join-Path $installDirectory 'spotiamp.exe')) "Incorrect shortcut target: $link"
                Assert-That (Same-Path $shortcut.WorkingDirectory $installDirectory) "Incorrect shortcut working directory: $link"
                Assert-That ([string]::IsNullOrEmpty($shortcut.Arguments)) "Unexpected shortcut arguments: $link"
            } finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shortcut) }
        }
        Assert-That (@(Get-ChildItem -LiteralPath $startMenuDirectory -Filter '*.lnk').Count -eq 1) 'Reinstall duplicated the Start menu shortcut'
    } finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell) }
}
function Assert-Payload([string]$ExpectedRoot, [string]$InstalledRoot) {
    foreach ($file in Get-ChildItem -LiteralPath $ExpectedRoot -Recurse -File) {
        $relative = [IO.Path]::GetRelativePath($ExpectedRoot, $file.FullName)
        $installed = Join-Path $InstalledRoot $relative
        Assert-That (Test-Path -LiteralPath $installed -PathType Leaf) "Missing installed package file: $relative"
        Assert-That ((Get-Hash $installed) -eq (Get-Hash $file.FullName)) "Installed package bytes differ: $relative"
    }
}
function Assert-ExecutableResources([string]$Executable) {
    $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($Executable)
    Assert-That ($version.FileVersion -eq $ExpectedVersion) "Incorrect Windows FileVersion: $($version.FileVersion)"
    Assert-That ($version.ProductVersion -eq $ExpectedVersion) "Incorrect Windows ProductVersion: $($version.ProductVersion)"
    Assert-That ($version.ProductName -eq 'Spotiamp') 'Windows executable product name is missing'
    $iconPath = Join-Path $PSScriptRoot '..\assets\windows\spotiamp.ico'
    $actualIcon = [Drawing.Icon]::ExtractAssociatedIcon($Executable)
    $expectedIcon = [Drawing.Icon]::new($iconPath, $actualIcon.Size)
    $actual = $actualIcon.ToBitmap()
    $expected = $expectedIcon.ToBitmap()
    try {
        Assert-That ($actual.Size -eq $expected.Size) 'Executable icon dimensions differ from the application icon'
        $visible = 0
        for ($y = 0; $y -lt $actual.Height; $y++) {
            for ($x = 0; $x -lt $actual.Width; $x++) {
                $pixel = $actual.GetPixel($x, $y)
                $reference = $expected.GetPixel($x, $y)
                # RGB below full transparency is not part of the visible icon.
                Assert-That ($pixel.A -eq $reference.A -and ($pixel.A -eq 0 -or $pixel.ToArgb() -eq $reference.ToArgb())) 'Extracted EXE icon does not match the Spotiamp asset'
                if ($pixel.A -gt 0) { $visible++ }
            }
        }
        Assert-That ($visible -gt 0) 'Application icon is empty'
        $actual.Save((Join-Path $OutputDirectory 'executable-icon.png'), [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $actual.Dispose(); $expected.Dispose(); $actualIcon.Dispose(); $expectedIcon.Dispose()
    }
}
function Assert-Registration([string]$Version) {
    $registration = Read-Registration
    Assert-That ($null -ne $registration) 'Per-user uninstall registration is missing'
    Assert-That ($registration.DisplayName -eq 'Spotiamp') 'Incorrect registered product name'
    Assert-That ($registration.DisplayVersion -eq $Version) "Incorrect registered version: $($registration.DisplayVersion)"
    Assert-That (Same-Path $registration.InstallLocation $installDirectory) 'InstallLocation does not match the chosen directory'
    Assert-That ($registration.UninstallString.StartsWith('"' + $uninstaller + '"', [StringComparison]::OrdinalIgnoreCase)) 'UninstallString does not refer to this installation'
    Assert-NoMachineRegistration
}
function Invoke-Setup([string]$LogName) {
    $process = Start-Native $SetupExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-',
        '/LANG=english', '/TASKS=desktopicon', "/DIR=$installDirectory", "/LOG=$(Join-Path $OutputDirectory $LogName)")
    Wait-Success $process 120
}
function Invoke-Uninstall([string]$LogName) {
    $process = Start-Native $uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART',
        "/LOG=$(Join-Path $OutputDirectory $LogName)")
    Wait-Success $process 120
    [void](Wait-Until 'uninstaller and registration removed' {
        (-not (Test-Path -LiteralPath $uninstaller)) -and ($null -eq (Read-Registration))
    })
}
function Save-FrenchWizard {
    $bootstrap = Start-Native $SetupExe @('/LANG=french', '/SP-', "/DIR=$installDirectory", "/LOG=$(Join-Path $OutputDirectory 'wizard-french.log')")
    $family = [Collections.Generic.HashSet[int]]::new()
    [void]$family.Add($bootstrap.Id)
    try {
    $uiProcess = Wait-Until 'French Inno wizard' {
        # Inno's bootstrap can create an is-*.tmp child for the actual wizard.
        # Follow only descendants of our own process, never unrelated windows.
        foreach ($parentId in @($family)) {
            foreach ($child in Get-CimInstance Win32_Process -Filter "ParentProcessId = $parentId") {
                [void]$family.Add([int]$child.ProcessId)
            }
        }
        foreach ($processId in $family) {
            $candidate = $null
            $selected = $false
            try {
                $candidate = [Diagnostics.Process]::GetProcessById($processId)
                $handle = $candidate.MainWindowHandle
                if ($handle -ne [IntPtr]::Zero -and [SpotiampInstallerWindow]::IsWindowVisible($handle)) {
                    $selected = $true
                    return $candidate
                }
            } catch { continue }
            finally { if ($candidate -and -not $selected) { $candidate.Dispose() } }
        }
        return $false
    } 30
    $running.Add($uiProcess)
    Assert-That ($uiProcess.WaitForInputIdle(10000)) 'French wizard did not become responsive'
    $report.wizard_title = $uiProcess.MainWindowTitle
    $handle = Wait-Window $uiProcess
    $rectangle = [SpotiampInstallerWindow+Rect]::new()
    Assert-That ([SpotiampInstallerWindow]::GetWindowRect($handle, [ref]$rectangle)) 'Cannot obtain French wizard bounds'
    $width = $rectangle.Right - $rectangle.Left
    $height = $rectangle.Bottom - $rectangle.Top
    Assert-That ($width -gt 200 -and $height -gt 150 -and $width -lt 4096 -and $height -lt 4096) 'Unexpected French wizard dimensions'
    $bitmap = [Drawing.Bitmap]::new($width, $height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $dc = $graphics.GetHdc()
        try { Assert-That ([SpotiampInstallerWindow]::PrintWindow($handle, $dc, 2)) 'French wizard capture failed' }
        finally { $graphics.ReleaseHdc($dc) }
        $bitmap.Save((Join-Path $OutputDirectory 'installer-french.png'), [Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
    Assert-That ([SpotiampInstallerWindow]::PostMessage($handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) 'Cannot close the wizard preview'
    [void](Wait-Until 'French wizard cancellation' {
        $uiProcess.Refresh()
        if ($uiProcess.HasExited) { return $true }
        foreach ($window in [SpotiampInstallerWindow]::VisibleWindows($uiProcess.Id)) {
            # Inno's standard Yes/No exit confirmation uses IDYES (6), which
            # is independent of the translated button label.
            $yes = [SpotiampInstallerWindow]::GetDlgItem($window, 6)
            if ($yes -ne [IntPtr]::Zero -and [SpotiampInstallerWindow]::IsWindowVisible($yes)) {
                Assert-That ([SpotiampInstallerWindow]::PostMessage($window, 0x0111, [IntPtr]6, [IntPtr]::Zero)) 'Cannot confirm wizard cancellation'
            }
        }
        return $false
    } 20)
    Assert-That ($bootstrap.WaitForExit(10000)) 'Inno bootstrap did not exit after cancellation'
    Assert-That ($null -eq (Read-Registration)) 'Wizard preview unexpectedly installed the product'
    Assert-That (-not (Test-Path -LiteralPath $installDirectory)) 'Wizard preview unexpectedly created the installation directory'
    Add-Check 'French interactive wizard captured and cancelled normally before silent installation'
    } finally {
        # Capturing is diagnostic only. Even when a runner cannot paint or close
        # the wizard normally, no installer process may leak into the real test.
        foreach ($processId in $family) {
            $candidate = $null
            try {
                $candidate = [Diagnostics.Process]::GetProcessById($processId)
                if (-not $candidate.HasExited -and $candidate.StartTime -ge $bootstrap.StartTime) {
                    $candidate.Kill($true)
                    [void]$candidate.WaitForExit(5000)
                }
            } catch [ArgumentException] { }
            catch { $report.cleanup_errors += "Preview process cleanup: $_" }
            finally { if ($candidate) { $candidate.Dispose() } }
        }
    }
}

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class SpotiampInstallerWindow {
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left, Top, Right, Bottom; }
    private delegate bool EnumWindowCallback(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")]
    public static extern IntPtr GetDlgItem(IntPtr window, int id);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", EntryPoint="PostMessageW", SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);
    public static IntPtr[] VisibleWindows(int processId) {
        var result = new List<IntPtr>();
        EnumWindows((window, parameter) => {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            if (owner == processId && IsWindowVisible(window)) result.Add(window);
            return true;
        }, IntPtr.Zero);
        return result.ToArray();
    }
}
'@

New-Item -ItemType Directory -Path $testRoot, $OutputDirectory -Force | Out-Null
try {
    $SetupExe = (Resolve-Path -LiteralPath $SetupExe).Path
    $PortableZip = (Resolve-Path -LiteralPath $PortableZip).Path
    $BaselineZip = (Resolve-Path -LiteralPath $BaselineZip).Path
    Assert-That ($null -eq (Read-Registration)) 'Refusing to modify an existing per-user Spotiamp installation'
    Assert-NoMachineRegistration
    Assert-That (-not (Test-Path -LiteralPath $startMenuDirectory)) 'Refusing to replace an existing Spotiamp Start menu folder'
    Assert-That (-not (Test-Path -LiteralPath $desktopLink)) 'Refusing to replace an existing Spotiamp desktop shortcut'
    Assert-That ((Get-Hash $BaselineZip) -eq $baselineArchiveHash) 'Published alpha.8 archive does not match its pinned SHA256'
    $report.setup_sha256 = Get-Hash $SetupExe
    $report.portable_sha256 = Get-Hash $PortableZip
    $report.baseline_sha256 = $baselineArchiveHash
    Assert-That (Same-Path 'C:\Spotiamp é\spotiamp.exe' '\\?\C:\Spotiamp é\spotiamp.exe') 'Extended drive paths were not normalized'
    Assert-That (Same-Path '\\server\share\Spotiamp é\spotiamp.exe' '\\?\UNC\server\share\Spotiamp é\spotiamp.exe') 'Extended UNC paths were not normalized'

    $currentPackage = Join-Path $testRoot 'current-package'
    $baselinePackage = Join-Path $testRoot 'alpha8-package'
    [IO.Compression.ZipFile]::ExtractToDirectory($PortableZip, $currentPackage)
    [IO.Compression.ZipFile]::ExtractToDirectory($BaselineZip, $baselinePackage)
    $currentInfo = Get-Content -LiteralPath (Join-Path $currentPackage 'BUILD-INFO.json') -Raw | ConvertFrom-Json
    $baselineInfo = Get-Content -LiteralPath (Join-Path $baselinePackage 'BUILD-INFO.json') -Raw | ConvertFrom-Json
    Assert-That ($currentInfo.app_version -eq $ExpectedVersion) 'Current portable package has the wrong version'
    Assert-That ($currentInfo.target -eq 'x86_64-pc-windows-msvc') 'Expected the native Windows MSVC package'
    Assert-That ($currentInfo.binary_name -eq 'spotiamp.exe') 'Unexpected binary name'
    Assert-That ((Get-Hash (Join-Path $currentPackage 'spotiamp.exe')) -eq $currentInfo.binary_sha256) 'Current BUILD-INFO binary hash is incorrect'
    Assert-That ((Get-Item -LiteralPath (Join-Path $currentPackage 'spotiamp.exe')).Length -eq $currentInfo.binary_bytes) 'Current BUILD-INFO binary size is incorrect'
    Assert-That ($baselineInfo.app_version -eq '0.1.0-alpha.8') 'Expected published alpha.8 baseline'
    Assert-That ((Get-Hash (Join-Path $baselinePackage 'spotiamp.exe')) -eq $baselineBinaryHash) 'Published alpha.8 binary hash is incorrect'
    Assert-That ($currentInfo.binary_sha256 -ne $baselineBinaryHash) 'The upgrade must install different executable bytes'
    foreach ($name in @('LICENSE', 'README.md', 'THIRD-PARTY.json', 'THIRD-PARTY.txt', 'license-origins.json')) {
        Assert-That (Test-Path -LiteralPath (Join-Path $currentPackage $name) -PathType Leaf) "Missing packaged notice: $name"
    }
    $inventory = @(Get-Content -LiteralPath (Join-Path $currentPackage 'THIRD-PARTY.json') -Raw | ConvertFrom-Json)
    Assert-That ($inventory.Count -gt 0) 'Empty dependency inventory'
    $mplCount = 0
    foreach ($dependency in $inventory) {
        Assert-That (@($dependency.license_files).Count -gt 0) "Missing license path for $($dependency.name)"
        foreach ($license in $dependency.license_files) {
            Assert-That (Test-Path -LiteralPath (Join-Path $currentPackage $license) -PathType Leaf) "Missing dependency license: $license"
        }
        if ($dependency.bundled_source) {
            $mplCount++
            $source = Join-Path $currentPackage $dependency.bundled_source
            Assert-That (@(Get-ChildItem -LiteralPath $source -Recurse -File).Count -gt 0) "Missing bundled MPL sources: $source"
        }
    }
    Assert-That ($mplCount -gt 0) 'Expected the required MPL source bundles'
    Assert-ExecutableResources (Join-Path $currentPackage 'spotiamp.exe')
    Add-Check 'Portable BUILD-INFO, executable hash/size, license inventory and MPL source bundles validated'
    Add-Check 'Windows version resources and extracted application icon match the release and source asset'
    try { Save-FrenchWizard }
    catch {
        $report.wizard_capture_error = ($_ | Out-String)
        Write-Warning "Optional French wizard capture unavailable: $_"
    }
    [void](Wait-Until 'preview Setup mutex released before actual installation' {
        $mutex = $null
        try {
            $mutex = [Threading.Mutex]::OpenExisting($setupMutex)
            return $false
        } catch [Threading.WaitHandleCannotBeOpenedException] { return $true }
        finally { if ($mutex) { $mutex.Dispose() } }
    } 10)
    # Capture failure may be diagnostic; unintended installation never is.
    Assert-That ($null -eq (Read-Registration)) 'Wizard preview left a product registration'
    Assert-That (-not (Test-Path -LiteralPath $installDirectory)) 'Wizard preview left installation files'

    $createdUserDataDirectory = -not (Test-Path -LiteralPath $userDataDirectory)
    New-Item -ItemType Directory -Path $userDataDirectory -Force | Out-Null
    Write-Utf8 $userDataSentinel "user-data-$testToken"
    if (-not (Test-Path -LiteralPath $qualityPath)) {
        Write-Utf8 $qualityPath "160`n"
        $createdQuality = $true
    }
    $qualityHash = Get-Hash $qualityPath
    $ownedInstallation = $true
    Invoke-Setup 'install.log'
    $uninstallers = @(Get-ChildItem -LiteralPath $installDirectory -Filter 'unins*.exe' -File)
    Assert-That ($uninstallers.Count -eq 1) 'Expected exactly one Inno uninstaller'
    $uninstaller = $uninstallers[0].FullName
    Assert-Payload $currentPackage $installDirectory
    Assert-Registration $ExpectedVersion
    Assert-Shortcuts
    Assert-That (-not (Test-Path -LiteralPath (Join-Path $installDirectory 'spotiamp-source.zip'))) 'Unexpected independently versioned source companion in installation'
    Add-Check 'Silent per-user install in a Unicode/spaced directory matches every portable package byte and creates both shortcuts'

    $installedExe = Join-Path $installDirectory 'spotiamp.exe'
    $reader = Start-Native $installedExe @('--no-update-check', '--quit-after', '3')
    [void](Wait-Window $reader)
    Assert-ApplicationMutex
    Wait-Success $reader 15
    Add-Check 'Installed executable creates a visible native window, holds the application mutex and exits normally'

    $personalFile = Join-Path $installDirectory "my-file-$testToken.txt"
    Write-Utf8 $personalFile "personal-$testToken"
    # This is the intentionally constructed older-install fixture described above.
    foreach ($item in Get-ChildItem -LiteralPath $baselinePackage -Force) {
        Copy-Item -LiteralPath $item.FullName -Destination $installDirectory -Recurse -Force
    }
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($uninstallSubkey, $true)
    try { $key.SetValue('DisplayVersion', $baselineInfo.app_version, [Microsoft.Win32.RegistryValueKind]::String) }
    finally { $key.Dispose() }
    Assert-Payload $baselinePackage $installDirectory
    Assert-Registration $baselineInfo.app_version
    $uninstallerHash = Get-Hash $uninstaller
    $oldReader = Start-Native $installedExe @('--no-update-check')
    [void](Wait-Window $oldReader)
    $stage = Join-Path $installDirectory ".spotiamp-update-ci-$testToken"
    $package = Join-Path $stage 'package'
    [IO.Compression.ZipFile]::ExtractToDirectory($PortableZip, $package)
    $helperPath = Join-Path $stage 'helper.exe'
    Copy-Item -LiteralPath $installedExe -Destination $helperPath
    $entries = [Collections.Generic.List[object]]::new()
    $files = @(Get-ChildItem -LiteralPath $package -Recurse -File)
    foreach ($file in @($files | Where-Object { $_.FullName -ne (Join-Path $package 'spotiamp.exe') }) + @($files | Where-Object { $_.FullName -eq (Join-Path $package 'spotiamp.exe') })) {
        $relative = [IO.Path]::GetRelativePath($package, $file.FullName).Replace('\', '/')
        $entries.Add([ordered]@{ source = $relative; target = $relative; hash = Get-Hash $file.FullName })
    }
    $nonce = "ci-$testToken"
    $manifest = [ordered]@{
        schema = 1; nonce = $nonce; created = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
        parent_pid = $oldReader.Id; target_name = 'spotiamp.exe'; old_hash = $baselineBinaryHash
        new_hash = $currentInfo.binary_sha256; scale2 = $false; files = @($entries.ToArray())
    }
    Write-Utf8 (Join-Path $stage 'manifest.json') ($manifest | ConvertTo-Json -Depth 8)
    Copy-Item -LiteralPath (Join-Path $stage 'manifest.json') -Destination (Join-Path $OutputDirectory 'update-manifest.json')
    # Rust canonicalizes current_exe to the extended-length Windows spelling.
    # Its private handoff protocol requires the stage in the same spelling.
    $canonicalStage = if ($stage.StartsWith('\\')) { '\\?\UNC\' + $stage.Substring(2) } else { '\\?\' + $stage }
    # The real app starts its helper with the app's working directory. Starting
    # this copied helper in stage would make the restarted reader inherit stage
    # as its cwd, keeping that directory locked against removal on Windows.
    $helper = Start-Native $helperPath @('--apply-update', $canonicalStage, $nonce) -WorkingDirectory $installDirectory
    [void](Wait-Until 'real alpha.8 helper READY' {
        $helper.Refresh()
        Assert-That (-not $helper.HasExited) 'Native update helper exited before READY'
        $ready = Join-Path $stage 'ready'
        (Test-Path -LiteralPath $ready) -and ([IO.File]::ReadAllText($ready) -eq $nonce)
    })
    Write-Utf8 (Join-Path $stage 'go') $nonce
    # READY only attests preflight. The old reader must remain alive until we
    # request its normal WM_CLOSE; replacement cannot happen while Windows locks it.
    Assert-That (-not $oldReader.HasExited) 'Old reader exited before the helper handoff'
    Assert-That ((Get-Hash $installedExe) -eq $baselineBinaryHash) 'Executable changed before the running old reader closed'
    Close-Reader $oldReader
    Wait-Success $helper 40
    $newReader = Wait-Until 'new executable restarted by the real helper' {
        foreach ($candidate in [Diagnostics.Process]::GetProcessesByName('spotiamp')) {
            $selected = $false
            try {
                if ($candidate.Id -ne $oldReader.Id -and (Same-Path $candidate.MainModule.FileName $installedExe)) {
                    $selected = $true
                    return $candidate
                }
            } catch { continue }
            finally { if (-not $selected) { $candidate.Dispose() } }
        }
        return $false
    }
    $running.Add($newReader)
    [void](Wait-Window $newReader)
    Assert-ApplicationMutex
    Assert-That ((Get-Hash $installedExe) -eq $currentInfo.binary_sha256) 'The native updater did not install the new executable bytes'
    [void](Wait-Until 'committed update stage cleanup' { -not (Test-Path -LiteralPath $stage) } 40)
    Assert-Payload $currentPackage $installDirectory
    Assert-Registration $ExpectedVersion
    Assert-That ((Get-Hash $uninstaller) -eq $uninstallerHash) 'Native update changed the Inno uninstaller'
    Assert-Shortcuts
    Assert-That ([IO.File]::ReadAllText($personalFile) -eq "personal-$testToken") 'Native update changed personal install-directory data'
    Assert-That ([IO.File]::ReadAllText($userDataSentinel) -eq "user-data-$testToken") 'Native update changed user data outside the installation'
    Assert-That ((Get-Hash $qualityPath) -eq $qualityHash) 'Native update changed the audio preference'
    Close-Reader $newReader
    Add-Check 'Published alpha.8 helper replaced the running old installation with different current executable bytes, restarted its visible UI, cleaned staging and synchronized registered version'

    foreach ($directory in @('licenses', 'mpl-source', 'scripts')) {
        $owned = Join-Path $installDirectory $directory
        New-Item -ItemType Directory -Path $owned -Force | Out-Null
        Write-Utf8 (Join-Path $owned "update-added-$testToken.txt") 'File introduced after Inno installation'
    }
    Invoke-Setup 'reinstall.log'
    Assert-Payload $currentPackage $installDirectory
    Assert-Registration $ExpectedVersion
    Assert-Shortcuts
    Assert-That ([IO.File]::ReadAllText($personalFile) -eq "personal-$testToken") 'Reinstall changed a personal file'
    Assert-That ((Get-Hash $qualityPath) -eq $qualityHash) 'Reinstall changed the audio preference'
    Add-Check 'Reinstall after native update preserves personal files/preferences and keeps exactly one Start menu shortcut'

    Invoke-Uninstall 'uninstall.log'
    $ownedInstallation = $false
    Assert-That (-not (Test-Path -LiteralPath $startMenuLink)) 'Start menu shortcut survived uninstall'
    Assert-That (-not (Test-Path -LiteralPath $desktopLink)) 'Desktop shortcut survived uninstall'
    Assert-That (-not (Test-Path -LiteralPath $startMenuDirectory)) 'Product Start menu folder survived uninstall'
    Assert-NoMachineRegistration
    foreach ($directory in @('licenses', 'mpl-source', 'scripts')) {
        Assert-That (-not (Test-Path -LiteralPath (Join-Path $installDirectory $directory))) "Owned directory survived uninstall: $directory"
    }
    $leftover = @(Get-ChildItem -LiteralPath $installDirectory -Force)
    Assert-That ($leftover.Count -eq 1 -and -not $leftover[0].PSIsContainer -and (Same-Path $leftover[0].FullName $personalFile)) 'Product files, directories or updater lock/staging survived uninstall'
    Assert-That ([IO.File]::ReadAllText($personalFile) -eq "personal-$testToken") 'Uninstall changed a personal file in the installation directory'
    Assert-That ([IO.File]::ReadAllText($userDataSentinel) -eq "user-data-$testToken") 'Uninstall deleted external user data'
    Assert-That ((Get-Hash $qualityPath) -eq $qualityHash) 'Uninstall deleted or changed the audio preference'
    Add-Check 'Uninstall removes registration, shortcuts, package files, update-added owned files and updater lock; personal files and external preferences remain intact'
    $report.success = $true
} catch {
    $report.error = ($_ | Out-String)
    if ($stage -and (Test-Path -LiteralPath $stage)) {
        $report.remaining_stage_entries = @(Get-ChildItem -LiteralPath $stage -Force -Recurse | ForEach-Object {
            [IO.Path]::GetRelativePath($stage, $_.FullName)
        })
        foreach ($name in @('helper-error.txt', 'update-error.txt', 'ready', 'go', 'new-started', 'new-commit', 'old-started', 'old-commit')) {
            $diagnostic = Join-Path $stage $name
            if (Test-Path -LiteralPath $diagnostic -PathType Leaf) {
                try { Copy-Item -LiteralPath $diagnostic -Destination (Join-Path $OutputDirectory "update-$name") }
                catch { $report.cleanup_errors += "Update diagnostic copy: $_" }
            }
        }
    }
    throw
} finally {
    foreach ($process in $running) {
        try {
            $process.Refresh()
            if (-not $process.HasExited) {
                $process.Kill($true)
                [void]$process.WaitForExit(5000)
            }
        } catch { $report.cleanup_errors += "Process cleanup: $_" }
        finally { $process.Dispose() }
    }
    if ($ownedInstallation) {
        try {
            $registration = Read-Registration
            if ($null -ne $registration -and (Same-Path $registration.InstallLocation $installDirectory)) {
                if (-not $uninstaller) {
                    $candidates = @(Get-ChildItem -LiteralPath $installDirectory -Filter 'unins*.exe' -File)
                    if ($candidates.Count -eq 1) { $uninstaller = $candidates[0].FullName }
                }
                if ($uninstaller -and (Test-Path -LiteralPath $uninstaller)) { Invoke-Uninstall 'cleanup-uninstall.log' }
            }
        } catch { $report.cleanup_errors += "Installer cleanup: $_" }
    }
    try {
        if (Test-Path -LiteralPath $userDataSentinel) { Remove-Item -LiteralPath $userDataSentinel }
        if ($createdQuality -and (Test-Path -LiteralPath $qualityPath) -and ((Get-Hash $qualityPath) -eq $qualityHash)) {
            Remove-Item -LiteralPath $qualityPath
        }
        if ($createdUserDataDirectory -and (Test-Path -LiteralPath $userDataDirectory) -and @(Get-ChildItem -LiteralPath $userDataDirectory -Force).Count -eq 0) {
            Remove-Item -LiteralPath $userDataDirectory
        }
    } catch { $report.cleanup_errors += "Fixture data cleanup: $_" }
    Write-Utf8 (Join-Path $OutputDirectory 'installer-test-report.json') ($report | ConvertTo-Json -Depth 10)
    Write-Host "Installer test report: $(Join-Path $OutputDirectory 'installer-test-report.json')"
}
