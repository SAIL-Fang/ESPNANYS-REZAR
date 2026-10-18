@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0razer-update-core.ps1" -Disable
echo.
pause
