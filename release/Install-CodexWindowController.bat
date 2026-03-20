@echo off
setlocal

set "SOURCE_EXE=%~dp0CodexWindowController.exe"
set "INSTALL_DIR=%LOCALAPPDATA%\CodexWindowController"
set "TARGET_EXE=%INSTALL_DIR%\CodexWindowController.exe"
set "SHORTCUT_PATH=%USERPROFILE%\Desktop\Codex 窗口控制器.lnk"
set "STARTMENU_DIR=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Codex Window Controller"
set "STARTMENU_SHORTCUT=%STARTMENU_DIR%\Codex 窗口控制器.lnk"

if not exist "%SOURCE_EXE%" (
  echo CodexWindowController.exe not found next to this installer.
  pause
  exit /b 1
)

if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
if not exist "%STARTMENU_DIR%" mkdir "%STARTMENU_DIR%"
copy /Y "%SOURCE_EXE%" "%TARGET_EXE%" >nul

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$shell = New-Object -ComObject WScript.Shell; " ^
  "$shortcut = $shell.CreateShortcut('%SHORTCUT_PATH%'); " ^
  "$shortcut.TargetPath = '%TARGET_EXE%'; " ^
  "$shortcut.WorkingDirectory = '%INSTALL_DIR%'; " ^
  "$shortcut.Description = 'Codex 桌面版窗口动态控制器'; " ^
  "$shortcut.Save(); " ^
  "$start = $shell.CreateShortcut('%STARTMENU_SHORTCUT%'); " ^
  "$start.TargetPath = '%TARGET_EXE%'; " ^
  "$start.WorkingDirectory = '%INSTALL_DIR%'; " ^
  "$start.Description = 'Codex 桌面版窗口动态控制器'; " ^
  "$start.Save()"

echo Installed to:
echo %TARGET_EXE%
echo.
echo Desktop shortcut created:
echo %SHORTCUT_PATH%
echo.
echo Start menu shortcut created:
echo %STARTMENU_SHORTCUT%
echo.
pause
