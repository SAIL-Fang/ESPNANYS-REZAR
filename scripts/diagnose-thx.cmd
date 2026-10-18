REM ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
REM Author: SAIL
REM Copyright (c) 2026 SAIL
REM SPDX-License-Identifier: MIT
@echo off
rem SAIL THX diagnostics launcher.
rem Bypasses the PowerShell execution policy for this one script only.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0diagnose-thx.ps1" %*
exit /b %ERRORLEVEL%
