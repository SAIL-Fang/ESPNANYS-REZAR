@echo off
rem SAIL THX diagnostics launcher.
rem Bypasses the PowerShell execution policy for this one script only.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0diagnose-thx.ps1" %*
exit /b %ERRORLEVEL%
