@echo off
setlocal
title finos (dev)
cd /d "%~dp0..\.."
if not exist "package.json" (
  echo Could not open the finos repo from %CD%
  pause
  exit /b 1
)

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

where node >nul 2>&1
if errorlevel 1 (
  echo Node.js is required. Install it, then run this file again.
  pause
  exit /b 1
)
where cargo >nul 2>&1
if errorlevel 1 (
  echo Rust/cargo is required. Install it, then run this file again.
  pause
  exit /b 1
)

if not exist "node_modules\" (
  echo Installing npm workspaces...
  call npm install
  if errorlevel 1 (
    echo npm install failed.
    pause
    exit /b 1
  )
)

echo Stopping any previous finos desktop so the live SQLite file and port 1420 are free...
taskkill /IM finos-desktop.exe /F >nul 2>&1
for /f "tokens=5" %%P in ('netstat -ano ^| findstr /R /C:":1420 .*LISTENING"') do (
  taskkill /PID %%P /F >nul 2>&1
)

echo Starting finos desktop ^(Vite UI + Tauri host + local SQLite^)...
echo This console is expected for daily coding. Close the window or Ctrl+C to stop.
echo.
call npm run desktop
if errorlevel 1 (
  echo.
  echo finos did not start. See the error above.
  pause
  exit /b 1
)
endlocal
