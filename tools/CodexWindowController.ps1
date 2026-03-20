Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class CodexWin32 {
    public const int GWL_EXSTYLE = -20;
    public const int WS_EX_LAYERED = 0x00080000;
    public const int WS_EX_TRANSPARENT = 0x00000020;
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

$script:SettingsPath = Join-Path $PSScriptRoot 'codex-window-controller.settings.json'

function Get-SavedSettings {
  if (-not (Test-Path $script:SettingsPath)) {
    return @{
      OpacityPercent = 45
      AlwaysOnTop = $true
      AutoWatch = $true
      LiveApply = $true
      ClickThrough = $false
    }
  }

  try {
    $raw = Get-Content $script:SettingsPath -Raw | ConvertFrom-Json
    return @{
      OpacityPercent = [Math]::Max(15, [Math]::Min(100, [int]$raw.OpacityPercent))
      AlwaysOnTop = [bool]$raw.AlwaysOnTop
      AutoWatch = [bool]$raw.AutoWatch
      LiveApply = [bool]$raw.LiveApply
      ClickThrough = [bool]$raw.ClickThrough
    }
  }
  catch {
    return @{
      OpacityPercent = 45
      AlwaysOnTop = $true
      AutoWatch = $true
      LiveApply = $true
      ClickThrough = $false
    }
  }
}

function Save-Settings {
  param(
    [int]$OpacityPercent,
    [bool]$AlwaysOnTop,
    [bool]$AutoWatch,
    [bool]$LiveApply,
    [bool]$ClickThrough
  )

  @{
    OpacityPercent = $OpacityPercent
    AlwaysOnTop = $AlwaysOnTop
    AutoWatch = $AutoWatch
    LiveApply = $LiveApply
    ClickThrough = $ClickThrough
  } | ConvertTo-Json | Set-Content $script:SettingsPath -Encoding UTF8
}

function Get-CodexWindow {
  Get-Process -Name 'Codex' -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Sort-Object Id -Descending |
    Select-Object -First 1
}

function Set-CodexWindowStyle {
  param(
    [System.Diagnostics.Process]$Process,
    [int]$OpacityPercent,
    [bool]$AlwaysOnTop,
    [bool]$ClickThrough
  )

  $handle = [IntPtr]$Process.MainWindowHandle
  if ($handle -eq [IntPtr]::Zero) {
    throw "Codex window handle not found."
  }

  $style = [CodexWin32]::GetWindowLong($handle, [CodexWin32]::GWL_EXSTYLE)
  $newStyle = ($style -bor [CodexWin32]::WS_EX_LAYERED)
  if ($ClickThrough) {
    $newStyle = ($newStyle -bor [CodexWin32]::WS_EX_TRANSPARENT)
  }
  else {
    $newStyle = ($newStyle -band (-bnot [CodexWin32]::WS_EX_TRANSPARENT))
  }
  [void][CodexWin32]::SetWindowLong($handle, [CodexWin32]::GWL_EXSTYLE, $newStyle)

  $alpha = [byte][Math]::Round(($OpacityPercent / 100.0) * 255)
  $topMode = if ($AlwaysOnTop) { [CodexWin32]::HWND_TOPMOST } else { [CodexWin32]::HWND_NOTOPMOST }

  if (-not [CodexWin32]::SetLayeredWindowAttributes($handle, 0, $alpha, [CodexWin32]::LWA_ALPHA)) {
    throw "Failed to set Codex window opacity."
  }

  if (-not [CodexWin32]::SetWindowPos(
    $handle,
    $topMode,
    0,
    0,
    0,
    0,
    ([CodexWin32]::SWP_NOMOVE -bor [CodexWin32]::SWP_NOSIZE -bor [CodexWin32]::SWP_NOACTIVATE -bor [CodexWin32]::SWP_SHOWWINDOW)
  )) {
    throw "Failed to update Codex topmost state."
  }
}

function Reset-CodexWindowStyle {
  param([System.Diagnostics.Process]$Process)

  $handle = [IntPtr]$Process.MainWindowHandle
  if ($handle -eq [IntPtr]::Zero) {
    throw "Codex window handle not found."
  }

  $style = [CodexWin32]::GetWindowLong($handle, [CodexWin32]::GWL_EXSTYLE)
  $newStyle = (($style -bor [CodexWin32]::WS_EX_LAYERED) -band (-bnot [CodexWin32]::WS_EX_TRANSPARENT))
  [void][CodexWin32]::SetWindowLong($handle, [CodexWin32]::GWL_EXSTYLE, $newStyle)

  [void][CodexWin32]::SetLayeredWindowAttributes($handle, 0, 255, [CodexWin32]::LWA_ALPHA)
  [void][CodexWin32]::SetWindowPos(
    $handle,
    [CodexWin32]::HWND_NOTOPMOST,
    0,
    0,
    0,
    0,
    ([CodexWin32]::SWP_NOMOVE -bor [CodexWin32]::SWP_NOSIZE -bor [CodexWin32]::SWP_NOACTIVATE -bor [CodexWin32]::SWP_SHOWWINDOW)
  )
}

$settings = Get-SavedSettings

$form = New-Object System.Windows.Forms.Form
$form.Text = 'Codex Window Controller'
$form.StartPosition = 'CenterScreen'
$form.Size = New-Object System.Drawing.Size(380, 345)
$form.MinimumSize = New-Object System.Drawing.Size(380, 345)
$form.MaximumSize = New-Object System.Drawing.Size(380, 345)
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.TopMost = $true
$form.BackColor = [System.Drawing.Color]::FromArgb(250, 248, 248, 248)

$title = New-Object System.Windows.Forms.Label
$title.Text = 'Codex Desktop Dynamic Control'
$title.Font = New-Object System.Drawing.Font('Segoe UI', 11, [System.Drawing.FontStyle]::Bold)
$title.Location = New-Object System.Drawing.Point(16, 14)
$title.Size = New-Object System.Drawing.Size(300, 24)
$form.Controls.Add($title)

$status = New-Object System.Windows.Forms.Label
$status.Text = 'Waiting for Codex desktop window...'
$status.ForeColor = [System.Drawing.Color]::FromArgb(70, 70, 70)
$status.Location = New-Object System.Drawing.Point(16, 42)
$status.Size = New-Object System.Drawing.Size(332, 20)
$form.Controls.Add($status)

$sliderLabel = New-Object System.Windows.Forms.Label
$sliderLabel.Text = 'Opacity'
$sliderLabel.Location = New-Object System.Drawing.Point(16, 78)
$sliderLabel.Size = New-Object System.Drawing.Size(80, 20)
$form.Controls.Add($sliderLabel)

$sliderValue = New-Object System.Windows.Forms.Label
$sliderValue.Text = "$($settings.OpacityPercent)%"
$sliderValue.TextAlign = 'MiddleRight'
$sliderValue.Location = New-Object System.Drawing.Point(296, 78)
$sliderValue.Size = New-Object System.Drawing.Size(52, 20)
$form.Controls.Add($sliderValue)

$slider = New-Object System.Windows.Forms.TrackBar
$slider.Minimum = 15
$slider.Maximum = 100
$slider.TickFrequency = 5
$slider.SmallChange = 1
$slider.LargeChange = 5
$slider.Value = $settings.OpacityPercent
$slider.Location = New-Object System.Drawing.Point(16, 98)
$slider.Size = New-Object System.Drawing.Size(332, 45)
$form.Controls.Add($slider)

$alwaysOnTop = New-Object System.Windows.Forms.CheckBox
$alwaysOnTop.Text = 'Keep Codex always on top'
$alwaysOnTop.Checked = $settings.AlwaysOnTop
$alwaysOnTop.Location = New-Object System.Drawing.Point(20, 138)
$alwaysOnTop.Size = New-Object System.Drawing.Size(220, 24)
$form.Controls.Add($alwaysOnTop)

$autoWatch = New-Object System.Windows.Forms.CheckBox
$autoWatch.Text = 'Auto-watch Codex window'
$autoWatch.Checked = $settings.AutoWatch
$autoWatch.Location = New-Object System.Drawing.Point(20, 164)
$autoWatch.Size = New-Object System.Drawing.Size(220, 24)
$form.Controls.Add($autoWatch)

$liveApply = New-Object System.Windows.Forms.CheckBox
$liveApply.Text = 'Apply immediately while dragging'
$liveApply.Checked = $settings.LiveApply
$liveApply.Location = New-Object System.Drawing.Point(20, 190)
$liveApply.Size = New-Object System.Drawing.Size(240, 24)
$form.Controls.Add($liveApply)

$clickThrough = New-Object System.Windows.Forms.CheckBox
$clickThrough.Text = 'Click-through Codex window'
$clickThrough.Checked = $settings.ClickThrough
$clickThrough.Location = New-Object System.Drawing.Point(20, 216)
$clickThrough.Size = New-Object System.Drawing.Size(220, 24)
$form.Controls.Add($clickThrough)

$applyButton = New-Object System.Windows.Forms.Button
$applyButton.Text = 'Apply'
$applyButton.Location = New-Object System.Drawing.Point(16, 266)
$applyButton.Size = New-Object System.Drawing.Size(78, 30)
$form.Controls.Add($applyButton)

$restoreInputButton = New-Object System.Windows.Forms.Button
$restoreInputButton.Text = 'Restore input'
$restoreInputButton.Location = New-Object System.Drawing.Point(102, 266)
$restoreInputButton.Size = New-Object System.Drawing.Size(92, 30)
$form.Controls.Add($restoreInputButton)

$resetButton = New-Object System.Windows.Forms.Button
$resetButton.Text = 'Reset'
$resetButton.Location = New-Object System.Drawing.Point(202, 266)
$resetButton.Size = New-Object System.Drawing.Size(78, 30)
$form.Controls.Add($resetButton)

$openCodexButton = New-Object System.Windows.Forms.Button
$openCodexButton.Text = 'Open Codex'
$openCodexButton.Location = New-Object System.Drawing.Point(16, 302)
$openCodexButton.Size = New-Object System.Drawing.Size(84, 30)
$form.Controls.Add($openCodexButton)

$closeButton = New-Object System.Windows.Forms.Button
$closeButton.Text = 'Close'
$closeButton.Location = New-Object System.Drawing.Point(278, 302)
$closeButton.Size = New-Object System.Drawing.Size(70, 30)
$form.Controls.Add($closeButton)

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 1200

function Invoke-ApplyCurrentSettings {
  $process = Get-CodexWindow
  if (-not $process) {
    $status.Text = 'Codex desktop window not found.'
    $status.ForeColor = [System.Drawing.Color]::FromArgb(180, 70, 70)
    return
  }

  try {
    Set-CodexWindowStyle -Process $process -OpacityPercent $slider.Value -AlwaysOnTop $alwaysOnTop.Checked -ClickThrough $clickThrough.Checked
    $status.Text = "Attached to Codex (PID=$($process.Id))"
    $status.ForeColor = [System.Drawing.Color]::FromArgb(60, 120, 70)
    Save-Settings -OpacityPercent $slider.Value -AlwaysOnTop $alwaysOnTop.Checked -AutoWatch $autoWatch.Checked -LiveApply $liveApply.Checked -ClickThrough $clickThrough.Checked
  }
  catch {
    $status.Text = $_.Exception.Message
    $status.ForeColor = [System.Drawing.Color]::FromArgb(180, 70, 70)
  }
}

$slider.add_ValueChanged({
  $sliderValue.Text = "$($slider.Value)%"
  if ($liveApply.Checked) {
    Invoke-ApplyCurrentSettings
  }
})

$alwaysOnTop.add_CheckedChanged({
  if ($liveApply.Checked) {
    Invoke-ApplyCurrentSettings
  }
})

$autoWatch.add_CheckedChanged({
  $timer.Enabled = $autoWatch.Checked
  Save-Settings -OpacityPercent $slider.Value -AlwaysOnTop $alwaysOnTop.Checked -AutoWatch $autoWatch.Checked -LiveApply $liveApply.Checked -ClickThrough $clickThrough.Checked
})

$liveApply.add_CheckedChanged({
  Save-Settings -OpacityPercent $slider.Value -AlwaysOnTop $alwaysOnTop.Checked -AutoWatch $autoWatch.Checked -LiveApply $liveApply.Checked -ClickThrough $clickThrough.Checked
})

$applyButton.add_Click({
  Invoke-ApplyCurrentSettings
})

$clickThrough.add_CheckedChanged({
  if ($liveApply.Checked) {
    Invoke-ApplyCurrentSettings
  }
  else {
    Save-Settings -OpacityPercent $slider.Value -AlwaysOnTop $alwaysOnTop.Checked -AutoWatch $autoWatch.Checked -LiveApply $liveApply.Checked -ClickThrough $clickThrough.Checked
  }
})

$restoreInputButton.add_Click({
  $clickThrough.Checked = $false
  Invoke-ApplyCurrentSettings
})

$resetButton.add_Click({
  $process = Get-CodexWindow
  if (-not $process) {
    $status.Text = 'Codex desktop window not found.'
    $status.ForeColor = [System.Drawing.Color]::FromArgb(180, 70, 70)
    return
  }

  try {
    Reset-CodexWindowStyle -Process $process
    $slider.Value = 100
    $alwaysOnTop.Checked = $false
    $clickThrough.Checked = $false
    $status.Text = 'Codex window restored to default.'
    $status.ForeColor = [System.Drawing.Color]::FromArgb(60, 120, 70)
    Save-Settings -OpacityPercent 100 -AlwaysOnTop $false -AutoWatch $autoWatch.Checked -LiveApply $liveApply.Checked -ClickThrough $false
  }
  catch {
    $status.Text = $_.Exception.Message
    $status.ForeColor = [System.Drawing.Color]::FromArgb(180, 70, 70)
  }
})

$openCodexButton.add_Click({
  try {
    Start-Process shell:AppsFolder\OpenAI.Codex_2p2nqsd0c76g0!Codex
    Start-Sleep -Milliseconds 900
    Invoke-ApplyCurrentSettings
  }
  catch {
    $status.Text = 'Failed to launch Codex desktop app.'
    $status.ForeColor = [System.Drawing.Color]::FromArgb(180, 70, 70)
  }
})

$closeButton.add_Click({
  $form.Close()
})

$timer.add_Tick({
  Invoke-ApplyCurrentSettings
})

$form.add_FormClosing({
  Save-Settings -OpacityPercent $slider.Value -AlwaysOnTop $alwaysOnTop.Checked -AutoWatch $autoWatch.Checked -LiveApply $liveApply.Checked -ClickThrough $clickThrough.Checked
})

$timer.Enabled = $autoWatch.Checked
Invoke-ApplyCurrentSettings
[void]$form.ShowDialog()
