@echo off
setlocal EnableExtensions
title finos (dev)
cd /d "%~dp0..\.."
if not exist "package.json" (
  echo Could not open the finos repo from %CD%
  pause
  exit /b 1
)

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "DEVLOCK=%LOCALAPPDATA%\com.finos.desktop\dev-start.lock"
set "DEVPID=%DEVLOCK%\start.pid"

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

if exist "%DEVPID%" (
  set /p OTHER= <"%DEVPID%"
)
if defined OTHER (
  tasklist /FI "PID eq %OTHER%" /NH 2>nul | find /I "cmd.exe" >nul
  if not errorlevel 1 (
    echo Another finos ^(dev^) start is already running. Use that window.
    echo This window must not kill port 1420 or Vite dies and tauri reports beforeDevCommand failed.
    pause
    exit /b 0
  )
)
if exist "%DEVLOCK%" rd /S /Q "%DEVLOCK%" >nul 2>&1
mkdir "%DEVLOCK%" >nul 2>&1
if errorlevel 1 (
  echo Another finos ^(dev^) start is already running. Use that window.
  pause
  exit /b 0
)
powershell -NoProfile -Command "[IO.File]::WriteAllText('%DEVPID%', [string](Get-CimInstance Win32_Process -Filter ('ProcessId='+$PID)).ParentProcessId)"

echo Stopping any previous finos desktop so the live SQLite file and port 1420 are free...
taskkill /IM finos-desktop.exe /F >nul 2>&1
call :kill_port 1420
call :kill_port 1421
call :wait_port_free 1420
if errorlevel 1 (
  echo Port 1420 is still in use. Close the other finos console, then run this file again.
  echo If you clicked Restart, the old window may show "Lifecycle script `dev` failed" — that is the old Vite exiting. Use this window.
  rd /S /Q "%DEVLOCK%" >nul 2>&1
  pause
  exit /b 1
)

echo Starting finos desktop ^(Vite UI + Tauri host + local SQLite^)...
echo This console is expected for daily coding. Close the window or Ctrl+C to stop.
echo If Restart opened this window, close the previous finos console. Its "dev failed" line is the old session dying.
echo.
call npm run desktop
set DEVEXIT=%ERRORLEVEL%
rd /S /Q "%DEVLOCK%" >nul 2>&1
if %DEVEXIT% EQU 0 goto started_ok
rem Killed Vite / Restart: npm reports 4294967295 (-1). That is the old session ending.
if %DEVEXIT% EQU 4294967295 goto old_session_ended
if %DEVEXIT% EQU -1 goto old_session_ended
echo.
echo finos did not start. See the error above.
pause
exit /b 1

:old_session_ended
echo The previous Vite session ended. If you clicked Restart, use the new finos ^(dev^) window.
exit /b 0

:started_ok
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
