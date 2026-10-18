REM ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
REM Author: SAIL
REM Copyright (c) 2026 SAIL
REM SPDX-License-Identifier: MIT
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0build.ps1" %*
