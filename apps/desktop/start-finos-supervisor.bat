@echo off
rem start-finos-supervisor: watch restart.token and Start-Process the coding launch.
setlocal EnableExtensions
title finos supervisor
cd /d "%~dp0..\.."
if not exist "package.json" (
  echo Could not open the finos repo from %CD%
  pause
  exit /b 1
)

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
  echo restart.token present but finos-desktop.exe still running after 60s. Keeping token.
  goto sleep
)
del /F /Q "%TOKEN%" >nul 2>&1
echo Consumed restart.token. Starting a new finos ^(dev^) console.
call :start_dev
goto sleep

:start_dev
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

:sleep
ping -n 3 127.0.0.1 >nul
goto loop
