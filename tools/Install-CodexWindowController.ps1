$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$sourceExe = Join-Path $root 'release\CodexWindowController.exe'
$installDir = Join-Path $env:LOCALAPPDATA 'CodexWindowController'
$targetExe = Join-Path $installDir 'CodexWindowController.exe'
$shortcutPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Codex Window Controller.lnk'

if (-not (Test-Path $sourceExe)) {
  throw "Built exe not found: $sourceExe. Run Build-CodexWindowControllerExe.ps1 first."
}

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item $sourceExe $targetExe -Force

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $targetExe
$shortcut.WorkingDirectory = $installDir
$shortcut.Description = 'Dynamic controller for Codex desktop window'
$shortcut.Save()

Write-Host "Installed to: $targetExe"
Write-Host "Desktop shortcut: $shortcutPath"
