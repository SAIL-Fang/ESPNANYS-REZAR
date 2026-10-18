@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0package-sail.ps1" %*
if "%~1"=="" pause
