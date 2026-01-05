@echo off
echo [Server] Starting mock server...
echo [Server] Done loading.

:loop
set /p input=
echo [Server] Received: %input%
if "%input%"=="stop" (
    echo [Server] Stopping...
    timeout /t 2
    exit
)
if "%input%"=="/stop" (
    echo [Server] Stopping...
    timeout /t 2
    exit
)
goto loop
