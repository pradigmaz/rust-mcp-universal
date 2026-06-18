@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
set "REPO_ROOT=%SCRIPT_DIR%.."
set "SUPERVISOR=%REPO_ROOT%\target\release\rmu-mcp-supervisor.exe"
if not exist "%SUPERVISOR%" (
  echo rmu-mcp-supervisor.exe missing. Run scripts\reload-rmu-mcp-child.cmd first. 1>&2
  exit /b 1
)
set "RMU_REPO_ROOT=%REPO_ROOT%"
"%SUPERVISOR%" %*
exit /b %ERRORLEVEL%
