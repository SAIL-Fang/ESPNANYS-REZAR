@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0backup-thx.ps1" %*
