REM ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
REM Author: SAIL
REM Copyright (c) 2026 SAIL
REM SPDX-License-Identifier: MIT
@echo off
setlocal
set "VCVARS=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS%" goto :have_vcvars
echo vcvars64.bat not found: %VCVARS%
exit /b 1

:have_vcvars
call "%VCVARS%" >nul
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%~dp0..\ui"
npm run tauri -- %*
