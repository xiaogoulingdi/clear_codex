param(
  [ValidateRange(15, 100)]
  [int]$OpacityPercent = 45,

  [switch]$AlwaysOnTop,

  [switch]$Watch,

  [ValidateRange(200, 10000)]
  [int]$IntervalMs = 1200,

  [switch]$Reset
)

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class Win32Window {
    public const int GWL_EXSTYLE = -20;
    public const int WS_EX_LAYERED = 0x00080000;
    public const uint LWA_ALPHA = 0x2;
    public static readonly IntPtr HWND_TOPMOST = new IntPtr(-1);
    public static readonly IntPtr HWND_NOTOPMOST = new IntPtr(-2);
    public const uint SWP_NOSIZE = 0x0001;
    public const uint SWP_NOMOVE = 0x0002;
    public const uint SWP_NOACTIVATE = 0x0010;
    public const uint SWP_SHOWWINDOW = 0x0040;

    [DllImport("user32.dll", SetLastError = true)]
    public static extern int GetWindowLong(IntPtr hWnd, int nIndex);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern int SetWindowLong(IntPtr hWnd, int nIndex, int dwNewLong);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetLayeredWindowAttributes(IntPtr hwnd, uint crKey, byte bAlpha, uint dwFlags);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetWindowPos(
        IntPtr hWnd,
        IntPtr hWndInsertAfter,
        int X,
        int Y,
        int cx,
        int cy,
        uint uFlags
    );
}
"@

function Get-CodexWindow {
  Get-Process -Name 'Codex' -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Sort-Object Id -Descending |
    Select-Object -First 1
}

function Set-CodexWindowStyle {
  param(
    [System.Diagnostics.Process]$Process
  )

  $handle = [IntPtr]$Process.MainWindowHandle
  if ($handle -eq [IntPtr]::Zero) {
    throw "Codex window handle not found."
  }

  $style = [Win32Window]::GetWindowLong($handle, [Win32Window]::GWL_EXSTYLE)
  if (($style -band [Win32Window]::WS_EX_LAYERED) -eq 0) {
    [void][Win32Window]::SetWindowLong(
      $handle,
      [Win32Window]::GWL_EXSTYLE,
      ($style -bor [Win32Window]::WS_EX_LAYERED)
    )
  }

  if ($Reset) {
    $alpha = [byte]255
    $topMode = [Win32Window]::HWND_NOTOPMOST
  }
  else {
    $alpha = [byte][Math]::Round(($OpacityPercent / 100.0) * 255)
    $topMode = if ($AlwaysOnTop) { [Win32Window]::HWND_TOPMOST } else { [Win32Window]::HWND_NOTOPMOST }
  }

  if (-not [Win32Window]::SetLayeredWindowAttributes($handle, 0, $alpha, [Win32Window]::LWA_ALPHA)) {
    throw "Failed to set Codex window opacity."
  }

  if (-not [Win32Window]::SetWindowPos(
    $handle,
    $topMode,
    0,
    0,
    0,
    0,
    ([Win32Window]::SWP_NOMOVE -bor [Win32Window]::SWP_NOSIZE -bor [Win32Window]::SWP_NOACTIVATE -bor [Win32Window]::SWP_SHOWWINDOW)
  )) {
    throw "Failed to update Codex topmost state."
  }

  $mode = if ($Reset) { "reset" } else { "opacity=$OpacityPercent% topmost=$($AlwaysOnTop.IsPresent)" }
  Write-Host "Applied to Codex window (PID=$($Process.Id), title='$($Process.MainWindowTitle)'): $mode"
}

function Invoke-Once {
  $process = Get-CodexWindow
  if (-not $process) {
    throw "No visible Codex desktop window found. Start the Codex desktop app first."
  }

  Set-CodexWindowStyle -Process $process
}

if ($Watch) {
  Write-Host "Watching Codex desktop window..."
  while ($true) {
    try {
      Invoke-Once
    }
    catch {
      Write-Host $_.Exception.Message
    }
    Start-Sleep -Milliseconds $IntervalMs
  }
}
else {
  Invoke-Once
}
