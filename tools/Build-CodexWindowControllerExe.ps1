$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$source = Join-Path $root 'desktop-controller\Program.cs'
$installerSource = Join-Path $root 'desktop-controller\Installer.cs'
$outputDir = Join-Path $root 'release'
$exePath = Join-Path $outputDir 'CodexWindowController.exe'
$installerExePath = Join-Path $outputDir 'CodexWindowControllerInstaller.exe'
$csc = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\Roslyn\csc.exe'

if (-not (Test-Path $csc)) {
  throw "csc.exe not found at $csc"
}

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

& $csc `
  /nologo `
  /target:winexe `
  /optimize+ `
  /out:$exePath `
  /reference:System.dll `
  /reference:System.Drawing.dll `
  /reference:System.Windows.Forms.dll `
  $source

if ($LASTEXITCODE -ne 0) {
  throw "Build failed."
}

& $csc `
  /nologo `
  /target:winexe `
  /optimize+ `
  /out:$installerExePath `
  /reference:System.dll `
  /reference:System.Drawing.dll `
  /reference:System.Windows.Forms.dll `
  /reference:Microsoft.CSharp.dll `
  $installerSource

if ($LASTEXITCODE -ne 0) {
  throw "Installer build failed."
}

Write-Host "Built: $exePath"
Write-Host "Built installer: $installerExePath"
