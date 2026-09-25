@echo off
rem finos.bat: one coding job. This window is the Restart parent.
rem It Start-Process-es apps\desktop\start-finos-dev.bat. It never runs npm itself.
setlocal EnableExtensions
set "REPO=%~dp0"
if "%REPO:~-1%"=="\" set "REPO=%REPO:~0,-1%"
if exist "%REPO%\package.json" goto repo_ok
if exist "%REPO%\finos\package.json" (
  set "REPO=%REPO%\finos"
  goto repo_ok
)
if exist "%USERPROFILE%\repo\finos\package.json" (
  set "REPO=%USERPROFILE%\repo\finos"
  goto repo_ok
)
if exist "%USERPROFILE%\finos\package.json" (
  set "REPO=%USERPROFILE%\finos"
  goto repo_ok
)
echo Could not open the finos repo from %CD%
echo Tried %~dp0 , %~dp0finos , %USERPROFILE%\repo\finos
pause
exit /b 1
:repo_ok
cd /d "%REPO%"
if not exist "%REPO%\package.json" (
  echo Could not open the finos repo from %REPO%
  pause
  exit /b 1
)
if not exist "%REPO%\apps\desktop\start-finos-dev.bat" (
  echo Missing %REPO%\apps\desktop\start-finos-dev.bat
  pause
  exit /b 1
)

if not defined LOCALAPPDATA (
  echo LOCALAPPDATA is not set.
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

set "APPDATA_DIR=%LOCALAPPDATA%\com.finos.desktop"
set "LOCK=%APPDATA_DIR%\supervisor.lock"
set "PIDFILE=%LOCK%\supervisor.pid"
set "DEVLOCK=%APPDATA_DIR%\dev-start.lock"
set "DEVPID=%DEVLOCK%\start.pid"
set "TOKEN=%APPDATA_DIR%\restart.token"
set "DEVBAT=%REPO%\apps\desktop\start-finos-dev.bat"

mkdir "%APPDATA_DIR%" >nul 2>&1
if not exist "%APPDATA_DIR%\" (
  echo Could not write %APPDATA_DIR%
  pause
  exit /b 1
)

set "OLDPID="
if exist "%PIDFILE%" (
  set /p OLDPID=<"%PIDFILE%"
)
if defined OLDPID (
  call :pid_is_live_cmd "%OLDPID%"
  if not errorlevel 1 (
    tasklist /FI "IMAGENAME eq finos-desktop.exe" 2>nul | find /I "finos-desktop.exe" >nul
    if not errorlevel 1 (
      echo %DATE% parent %TIME% Already running. Use the open finos window. Not starting a second copy.
      exit 0
    )
    echo %DATE% parent %TIME% Live parent, desktop gone. Signaling restart.token so the parent starts one child.
    if not exist "%TOKEN%" (
      powershell -NoProfile -Command "[IO.File]::WriteAllText('%TOKEN%', 'reason=owner-start-desktop-gone' + [char]10)"
    )
    call :wait_desktop_or_child
    if not errorlevel 1 (
      echo Parent started the desktop. This window is extra.
      exit 0
    )
    echo Parent loop is stuck. Close the other finos window, then run finos.bat again.
    pause
    exit /b 1
  )
)

if exist "%LOCK%" rd /S /Q "%LOCK%" >nul 2>&1
mkdir "%LOCK%" >nul 2>&1
if errorlevel 1 (
  echo Could not take supervisor.lock
  pause
  exit /b 1
)

title finos
powershell -NoProfile -Command "[IO.File]::WriteAllText('%PIDFILE%', [string](Get-CimInstance Win32_Process -Filter ('ProcessId='+$PID)).ParentProcessId)"

set "FIRST=1"
echo finos supervisor watching %TOKEN%
echo File -- Restart writes that token and this window Start-Process-es a new finos ^(dev^) console.
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
echo %DATE% parent %TIME% No desktop running. Starting coding launch once.
call :start_dev
goto sleep

:consume
call :wait_exe_gone
if errorlevel 1 (
  echo %DATE% parent %TIME% stale restart.token: desktop is already running. Removing token.
  del /F /Q "%TOKEN%" >nul 2>&1
  goto sleep
)
call :wait_port_free 1420
del /F /Q "%TOKEN%" >nul 2>&1
echo %DATE% parent %TIME% Consumed restart.token. Starting a new finos ^(dev^) console.
call :start_dev
goto sleep

:start_dev
set "LIVEDEV="
if exist "%DEVPID%" (
  set /p LIVEDEV=<"%DEVPID%"
)
if defined LIVEDEV (
  call :pid_is_live_cmd "%LIVEDEV%"
  if not errorlevel 1 (
    echo %DATE% parent %TIME% A finos ^(dev^) start is already running. Not starting a second copy.
    exit /b 0
  )
)
powershell -NoProfile -Command "Start-Process -FilePath '%DEVBAT%' -WindowStyle Normal"
exit /b 0

:pid_is_live_cmd
set "CHECKPID=%~1"
if "%CHECKPID%"=="" exit /b 1
tasklist /FI "PID eq %CHECKPID%" /NH 2>nul | find /I "cmd.exe" >nul
if errorlevel 1 (
  tasklist /FI "PID eq %CHECKPID%" /NH 2>nul | find /I "powershell" >nul
  if errorlevel 1 exit /b 1
)
rem Untitled leftover cmd is not the supervisor. Title must be finos or the command is finos.bat.
powershell -NoProfile -Command "$p = Get-CimInstance Win32_Process -Filter ('ProcessId=%CHECKPID%'); if (-not $p) { exit 1 }; $c = [string]$p.CommandLine; $t = [string](Get-Process -Id %CHECKPID% -EA SilentlyContinue).MainWindowTitle; if ($t -eq 'finos' -or $c -match '(?i)[/\\]finos\.bat') { exit 0 } else { exit 1 }"
exit /b %ERRORLEVEL%

:wait_desktop_or_child
set /a _up=0
:wait_desktop_or_child_loop
tasklist /FI "IMAGENAME eq finos-desktop.exe" 2>nul | find /I "finos-desktop.exe" >nul
if not errorlevel 1 exit /b 0
if exist "%DEVPID%" (
  set "CHILD="
  set /p CHILD=<"%DEVPID%"
  if defined CHILD (
    call :pid_is_live_cmd "%CHILD%"
    if not errorlevel 1 exit /b 0
  )
)
set /a _up+=1
if %_up% GEQ 15 exit /b 1
ping -n 3 127.0.0.1 >nul
goto wait_desktop_or_child_loop

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
