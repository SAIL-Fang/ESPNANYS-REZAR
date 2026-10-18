@echo off
setlocal
set "ROOT=%~dp0.."
call "%ROOT%\scripts\build.cmd" -Stage
if errorlevel 1 exit /b 1
call "%ROOT%\scripts\ui.cmd" build --debug --no-bundle
if errorlevel 1 exit /b 1
copy /Y "%ROOT%\ui\src-tauri\target\debug\sail_ui.exe" "%LOCALAPPDATA%\SAIL\bin\sail_ui.exe" >nul
powershell.exe -NoProfile -Command "Start-Process -FilePath '%LOCALAPPDATA%\SAIL\bin\sail_ui.exe'"
