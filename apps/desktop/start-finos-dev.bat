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
call :kill_port 1420
call :kill_port 1421
call :wait_port_free 1420
if errorlevel 1 (
  echo Port 1420 is still in use. Close the other finos console, then run this file again.
  echo If you clicked Restart, the old window may show "Lifecycle script `dev` failed" — that is the old Vite exiting. Use this window.
  pause
  exit /b 1
)

echo Starting finos desktop ^(Vite UI + Tauri host + local SQLite^)...
echo This console is expected for daily coding. Close the window or Ctrl+C to stop.
echo If Restart opened this window, close the previous finos console. Its "dev failed" line is the old session dying.
echo.
call npm run desktop
if errorlevel 1 (
  echo.
  echo finos did not start. See the error above.
  pause
  exit /b 1
)
endlocal
exit /b 0

:kill_port
for /f "tokens=5" %%P in ('netstat -ano 2^>nul ^| findstr /R /C:":%~1" ^| findstr LISTENING') do (
  taskkill /PID %%P /T /F >nul 2>&1
)
exit /b 0

:wait_port_free
set /a _wait=0
:wait_port_free_loop
netstat -ano 2>nul | findstr /R /C:":%~1" | findstr LISTENING >nul
if errorlevel 1 exit /b 0
call :kill_port %~1
set /a _wait+=1
if %_wait% GEQ 20 exit /b 1
timeout /t 1 /nobreak >nul
goto wait_port_free_loop
