@echo off
rem capture-finos-dev-log: run the coding launch with everything written to a
rem file, so the first real error survives the console scrollback.
rem
rem npm only reports the LAST child's exit code. On Windows a terminated child
rem shows as 4294967295 (-1), which says nothing. The cause is always earlier.
setlocal
title finos (dev, logging)
cd /d "%~dp0..\.."
if not exist "package.json" (
  echo Could not open the finos repo from %CD%
  pause
  exit /b 1
)

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "LOG=%TEMP%\finos-dev.log"

echo Writing the whole session to %LOG%
echo Press Ctrl+C to stop. Nothing appears here until it ends.
echo.
call npm run dev > "%LOG%" 2>&1
set "RC=%ERRORLEVEL%"

echo.
echo finos exited with %RC%. Full session: %LOG%
echo If npm itself is the problem, rerun with: npm run dev --loglevel verbose
echo npm's own debug logs: %LOCALAPPDATA%\npm-cache\_logs
echo.
echo First error lines (the top one is usually the cause):
setlocal EnableDelayedExpansion
set /a _n=0
for /f "usebackq tokens=* delims=" %%L in (`findstr /N /I /C:"error" /C:"panicked" /C:"is not recognized" /C:"already in use" "%LOG%" 2^>nul`) do (
  if !_n! LSS 12 (
    echo   %%L
    set /a _n+=1
  )
)
if !_n! EQU 0 echo   none matched - open %LOG% and read from the top.
endlocal

echo.
echo Send %LOG% if you need someone else to read it.
pause
endlocal & exit /b %RC%
