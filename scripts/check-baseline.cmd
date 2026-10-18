@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0check-baseline.ps1" %*
if "%~1"=="" pause
