@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%rmu-mcp-server-fresh.ps1" %*
exit /b %ERRORLEVEL%
