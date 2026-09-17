@echo off
rem start-finos-supervisor: watch restart.token and Start-Process the coding launch.
setlocal EnableExtensions
cd /d "%~dp0..\.."
if not exist "package.json" (
  echo Could not open the finos repo from %CD%
  pause
  exit /b 1
)

set "LOCK=%LOCALAPPDATA%\com.finos.desktop\supervisor.lock"
set "PIDFILE=%LOCK%\supervisor.pid"
set "DEVLOCK=%LOCALAPPDATA%\com.finos.desktop\dev-start.lock"
set "DEVPID=%DEVLOCK%\start.pid"
if exist "%PIDFILE%" (
  set /p OLDPID=<"%PIDFILE%"
)
if defined OLDPID (
  tasklist /FI "PID eq %OLDPID%" /NH 2>nul | find /I "cmd.exe" >nul
  if not errorlevel 1 (
    echo Supervisor already running.
    exit /b 0
  )
)
if exist "%LOCK%" rd /S /Q "%LOCK%" >nul 2>&1
mkdir "%LOCK%" >nul 2>&1
if errorlevel 1 (
  echo Could not take supervisor.lock
  pause
  exit /b 1
)
title finos supervisor
powershell -NoProfile -Command "[IO.File]::WriteAllText('%PIDFILE%', [string](Get-CimInstance Win32_Process -Filter ('ProcessId='+$PID)).ParentProcessId)"

set "TOKEN=%LOCALAPPDATA%\com.finos.desktop\restart.token"
set "DEVBAT=%~dp0start-finos-dev.bat"
set "FIRST=1"

echo finos supervisor watching %TOKEN%
echo Close this window or Ctrl+C to stop auto-relaunch.
echo.

:loop
if exist "%TOKEN%" goto consume
if "%FIRST%"=="1" goto first_pass
goto sleep

:first_pass
set "FIRST=0"
tasklist /FI "IMAGENAME eq finos-desktop.exe" 2>nul | find /I "finos-desktop.exe" >nul
if not errorlevel 1 goto sleep
echo No desktop running. Starting coding launch once.
call :start_dev
goto sleep

:consume
call :wait_exe_gone
if errorlevel 1 (
  echo stale restart.token: desktop is already running. Removing token.
  del /F /Q "%TOKEN%" >nul 2>&1
  goto sleep
)
call :wait_port_free 1420
del /F /Q "%TOKEN%" >nul 2>&1
echo Consumed restart.token. Starting a new finos ^(dev^) console.
call :start_dev
goto sleep

:start_dev
set "LIVEDEV="
if exist "%DEVPID%" (
  set /p LIVEDEV=<"%DEVPID%"
)
if defined LIVEDEV (
  tasklist /FI "PID eq %LIVEDEV%" /NH 2>nul | find /I "cmd.exe" >nul
  if not errorlevel 1 (
    echo A finos ^(dev^) start is already running. Not starting a second copy.
    exit /b 0
  )
)
powershell -NoProfile -Command "Start-Process -FilePath '%~dp0start-finos-dev.bat' -WindowStyle Normal"
exit /b 0

:wait_exe_gone
set /a _gone=0
:wait_exe_gone_loop
tasklist /FI "IMAGENAME eq finos-desktop.exe" 2>nul | find /I "finos-desktop.exe" >nul
if errorlevel 1 exit /b 0
set /a _gone+=1
if %_gone% GEQ 30 exit /b 1
ping -n 3 127.0.0.1 >nul
goto wait_exe_gone_loop

:wait_port_free
set /a _wait=0
:wait_port_free_loop
netstat -ano 2>nul | findstr /R /C:":%~1" | findstr LISTENING >nul
if errorlevel 1 exit /b 0
set /a _wait+=1
if %_wait% GEQ 20 exit /b 0
ping -n 2 127.0.0.1 >nul
goto wait_port_free_loop

:sleep
ping -n 3 127.0.0.1 >nul
goto loop
