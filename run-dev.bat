@echo off
setlocal

cd /d "%~dp0"

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-dev.ps1" %*
set "EXIT_CODE=%ERRORLEVEL%"

if not "%EXIT_CODE%"=="0" (
    echo.
    echo O script terminou com erro ^(codigo %EXIT_CODE%^).
    pause
)

exit /b %EXIT_CODE%
