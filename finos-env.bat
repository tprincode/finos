@echo off
rem Shared PATH for repo-root finos.bat and apps\desktop\start-finos-dev.bat.
rem Explorer and shortcut launches often omit Node; keep this in sync with both entry bats.
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
if exist "%ProgramFiles%\nodejs\" set "PATH=%ProgramFiles%\nodejs;%PATH%"
if exist "%ProgramFiles(x86)%\nodejs\" set "PATH=%ProgramFiles(x86)%\nodejs;%PATH%"
if exist "%LocalAppData%\Programs\node\" set "PATH=%LocalAppData%\Programs\node;%PATH%"
