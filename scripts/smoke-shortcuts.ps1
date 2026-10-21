# Smoke test for the common shortcuts on Windows.
#
# Sends every Windows preset in src/presets.json the way a mapped button does
# (`pairdock-cli send`) and checks the effect on the system, then undoes it.
# Run it on an unlocked desktop and leave the mouse and keyboard alone for
# about half a minute:
#
#   cargo build -p pairdock-cli
#   powershell -ExecutionPolicy Bypass -File scripts\smoke-shortcuts.ps1
#
# It briefly creates and removes a virtual desktop, opens and closes its own
# test window, changes the volume and mutes it, and puts both back. Presets
# whose effect cannot be read back are reported as MANUAL.

param([string]$Cli = (Join-Path $PSScriptRoot '..\target\debug\pairdock-cli.exe'))

$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Cli)) { throw "$Cli not found; run: cargo build -p pairdock-cli" }

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class Win {
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);

    public static uint ForegroundPid() { uint p; GetWindowThreadProcessId(GetForegroundWindow(), out p); return p; }
    public static string ForegroundClass() { var s = new StringBuilder(256); GetClassName(GetForegroundWindow(), s, 256); return s.ToString(); }
    public static string ForegroundTitle() { var s = new StringBuilder(256); GetWindowText(GetForegroundWindow(), s, 256); return s.ToString(); }
}

// Core Audio: the default render endpoint's volume and mute state.
[Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IAudioEndpointVolume {
    int RegisterControlChangeNotify(IntPtr n); int UnregisterControlChangeNotify(IntPtr n);
    int GetChannelCount(out uint c); int SetMasterVolumeLevel(float l, Guid g);
    int SetMasterVolumeLevelScalar(float l, Guid g); int GetMasterVolumeLevel(out float l);
    int GetMasterVolumeLevelScalar(out float l); int SetChannelVolumeLevel(uint c, float l, Guid g);
    int SetChannelVolumeLevelScalar(uint c, float l, Guid g); int GetChannelVolumeLevel(uint c, out float l);
    int GetChannelVolumeLevelScalar(uint c, out float l); int SetMute(bool m, Guid g); int GetMute(out bool m);
}
[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDevice { int Activate(ref Guid id, int ctx, IntPtr p, [MarshalAs(UnmanagedType.IUnknown)] out object o); }
[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator { int EnumAudioEndpoints(int f, int s, out IntPtr d); int GetDefaultAudioEndpoint(int flow, int role, out IMMDevice d); }
[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] class MMDeviceEnumerator { }

public static class Audio {
    static IAudioEndpointVolume Endpoint() {
        var e = (IMMDeviceEnumerator)new MMDeviceEnumerator();
        IMMDevice d; e.GetDefaultAudioEndpoint(0, 1, out d);
        var id = typeof(IAudioEndpointVolume).GUID; object o;
        d.Activate(ref id, 23, IntPtr.Zero, out o);
        return (IAudioEndpointVolume)o;
    }
    public static float Volume() { float v; Endpoint().GetMasterVolumeLevelScalar(out v); return v; }
    public static bool Muted() { bool m; Endpoint().GetMute(out m); return m; }
    public static void Restore(float volume, bool muted) {
        var e = Endpoint(); e.SetMasterVolumeLevelScalar(volume, Guid.Empty); e.SetMute(muted, Guid.Empty);
    }
}
"@

function Send([string]$keys) {
    & $Cli send $keys
    if ($LASTEXITCODE -ne 0) { throw "pairdock-cli send $keys failed" }
    Start-Sleep -Milliseconds 1200
}

function Desktop {
    $p = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops'
    [guid]::new([byte[]]$p.CurrentVirtualDesktop)
}

# The virtual desktops from left to right.
function Desktops {
    $ids = [byte[]](Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops').VirtualDesktopIDs
    if (-not $ids) { return @(Desktop) }
    for ($i = 0; $i -lt $ids.Length; $i += 16) { [guid]::new([byte[]]$ids[$i..($i + 15)]) }
}

function Go-ToDesktop([guid]$id) {
    $list = @(Desktops)
    $steps = [array]::IndexOf($list, $id) - [array]::IndexOf($list, (Desktop))
    $key = if ($steps -lt 0) { 'Win+Ctrl+Left' } else { 'Win+Ctrl+Right' }
    for ($i = 0; $i -lt [math]::Abs($steps); $i++) { Send $key }
}

# A window of our own with a focused text box that writes its text to a file.
$textFile = Join-Path $env:TEMP 'pairdock-smoke.txt'
function Open-TestWindow {
    Remove-Item $textFile -ErrorAction SilentlyContinue
    $script = @"
Add-Type -AssemblyName System.Windows.Forms
`$f = New-Object Windows.Forms.Form; `$f.Text = 'PairDock smoke test'; `$f.TopMost = `$true
`$t = New-Object Windows.Forms.TextBox; `$t.Dock = 'Top'; `$f.Controls.Add(`$t)
`$t.Add_TextChanged({ [IO.File]::WriteAllText('$textFile', `$t.Text) })
`$f.Add_Shown({ `$f.Activate(); `$t.Focus() })
[Windows.Forms.Application]::Run(`$f)
"@
    $p = Start-Process powershell -ArgumentList '-NoProfile', '-WindowStyle', 'Hidden', '-Command', $script -PassThru
    for ($i = 0; $i -lt 20; $i++) {
        Start-Sleep -Milliseconds 500
        if ([Win]::ForegroundPid() -eq $p.Id) { return $p }
    }
    Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
    throw 'the test window did not come to the front'
}

$results = [ordered]@{}
function Check([string]$keys, [scriptblock]$test) {
    try { $results[$keys] = & $test } catch { $results[$keys] = "FAIL ($($_.Exception.Message))" }
}

$presets = Get-Content (Join-Path $PSScriptRoot '..\src\presets.json') -Raw | ConvertFrom-Json
$keys = @($presets.windows) + @($presets.all) | ForEach-Object { $_.keys }

# Volume keys also unmute, so the original state is put back at the end.
$volume = [Audio]::Volume(); $muted = [Audio]::Muted()

foreach ($k in $keys) {
    switch ($k) {
        'Win+D' {
            Check $k {
                Send $k; $ok = [Win]::ForegroundClass() -eq 'Progman'; Send $k
                if ($ok) { 'PASS' } else { 'FAIL (the desktop did not come to the front)' }
            }
        }
        'Win+Tab' {
            Check $k {
                Send $k; $ok = [Win]::ForegroundClass() -eq 'XamlExplorerHostIslandWindow'; Send 'Esc'
                if ($ok) { 'PASS' } else { 'FAIL (Task view did not open)' }
            }
        }
        'Win+Shift+S' {
            Check $k {
                Send $k; $ok = (Get-Process -Id ([Win]::ForegroundPid())).ProcessName -eq 'SnippingTool'; Send 'Esc'
                if ($ok) { 'PASS' } else { 'FAIL (the screenshot overlay did not open)' }
            }
        }
        { $_ -in 'Win+Ctrl+Left', 'Win+Ctrl+Right' } {
            Check $k {
                # Windows adds the new desktop at the right end and switches to it.
                $origin = Desktop; Send 'Win+Ctrl+D'; $temp = Desktop
                try {
                    if ($temp -eq $origin) { throw 'could not create a test desktop' }
                    $list = @(Desktops); $left = $list[[array]::IndexOf($list, $temp) - 1]
                    if ($k -eq 'Win+Ctrl+Left') { Send $k; $ok = (Desktop) -eq $left }
                    else { Send 'Win+Ctrl+Left'; Send $k; $ok = (Desktop) -eq $temp }
                } finally {
                    Go-ToDesktop $temp; Send 'Win+Ctrl+F4'; Go-ToDesktop $origin
                }
                if ($ok) { 'PASS' } else { 'FAIL (the desktop did not change)' }
            }
        }
        'Alt+Tab' {
            Check $k {
                $p = Open-TestWindow
                try { Send $k; $ok = [Win]::ForegroundPid() -ne $p.Id }
                finally { Stop-Process -Id $p.Id -ErrorAction SilentlyContinue }
                if ($ok) { 'PASS' } else { 'FAIL (the window in front did not change)' }
            }
        }
        'Alt+F4' {
            Check $k {
                $p = Open-TestWindow
                Send $k; $closed = $p.HasExited
                if (-not $closed) { Stop-Process -Id $p.Id -ErrorAction SilentlyContinue }
                if ($closed) { 'PASS' } else { 'FAIL (the test window stayed open)' }
            }
        }
        'Win+Period' {
            Check $k {
                $p = Open-TestWindow
                try {
                    # Enter inserts the emoji the panel has selected; in this
                    # single-line text box it adds nothing on its own.
                    Send $k; Send 'Return'; Send 'Esc'
                    $text = Get-Content $textFile -Raw -ErrorAction SilentlyContinue
                } finally { Stop-Process -Id $p.Id -ErrorAction SilentlyContinue }
                if ($text) { 'PASS' } else { 'FAIL (no emoji was inserted)' }
            }
        }
        'VolumeUp' {
            Check $k {
                if ([Audio]::Volume() -ge 0.99) { Send 'VolumeDown' }
                $before = [Audio]::Volume(); Send $k; $after = [Audio]::Volume()
                if ($after -gt $before) { 'PASS' } else { "FAIL (volume $before -> $after)" }
            }
        }
        'VolumeDown' {
            Check $k {
                if ([Audio]::Volume() -le 0.01) { Send 'VolumeUp' }
                $before = [Audio]::Volume(); Send $k; $after = [Audio]::Volume()
                if ($after -lt $before) { 'PASS' } else { "FAIL (volume $before -> $after)" }
            }
        }
        'Mute' {
            Check $k {
                $before = [Audio]::Muted(); Send $k; $after = [Audio]::Muted()
                if ($after -ne $before) { 'PASS' } else { 'FAIL (mute did not change)' }
            }
        }
        default { $results[$k] = 'MANUAL' }
    }
}

[Audio]::Restore($volume, $muted)
Remove-Item $textFile -ErrorAction SilentlyContinue
$results.GetEnumerator() | ForEach-Object { '{0,-16} {1}' -f $_.Key, $_.Value }
if ($results.Values -like 'FAIL*') { exit 1 }
