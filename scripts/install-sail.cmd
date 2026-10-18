@echo off
setlocal
net session >nul 2>&1
if errorlevel 1 (
    if not "%~1"=="" (
        echo This installer needs administrator rights for driver/service restore.
        echo Right-click install-sail.cmd and choose "Run as administrator",
        echo then run it with your options from that elevated window.
        exit /b 1
    )
    powershell.exe -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-sail.ps1" %*
if "%~1"=="" pause
