@echo off
setlocal
chcp 65001 >nul

cd /d "%~dp0"

where pwsh.exe >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    set "PS_EXE=pwsh.exe"
) else (
    set "PS_EXE=powershell.exe"
)

"%PS_EXE%" -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-dev.ps1" %*
set "EXIT_CODE=%ERRORLEVEL%"

if not "%EXIT_CODE%"=="0" (
    echo.
    echo O script terminou com erro ^(codigo %EXIT_CODE%^).
    pause
)

exit /b %EXIT_CODE%
