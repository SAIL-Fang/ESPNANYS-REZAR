@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0block-razer-network.ps1" %*
exit /b %errorlevel%
