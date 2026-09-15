@echo off
setlocal
title finos
set "EXE=%LOCALAPPDATA%\finos\finos-desktop.exe"
if not exist "%EXE%" (
  echo Installed finos.exe is not on this PC yet.
  echo.
  echo That is the no-console household launch. It is a frozen cut, not the daily coding start.
  echo When you want the Desktop icon to catch up:
  echo   1. Fully quit any running finos window and this console.
  echo   2. From the repo root: npm run desktop:build
  echo   3. Run apps\desktop\src-tauri\target\release\bundle\nsis\finos_0.1.0_x64-setup.exe
  echo   4. Pin Start Menu / Desktop to %%LOCALAPPDATA%%\finos\finos-desktop.exe
  echo.
  echo Keep start-finos-dev.bat / Desktop finos.bat for the next host change.
  echo Do not run the installed exe and the dev app at the same time.
  pause
  exit /b 1
)

tasklist /FI "IMAGENAME eq finos-desktop.exe" | find /I "finos-desktop.exe" >nul
if not errorlevel 1 (
  echo finos is already running. Close that window before starting another copy.
  echo The live SQLite file cannot be opened twice.
  pause
  exit /b 1
)

start "" "%EXE%"
endlocal
