@echo off
setlocal
call "%~dp0build-executor.cmd"
if errorlevel 1 exit /b %errorlevel%
"%~dp0target\release\crexe.exe" exec "%~dp0examples\calculator_native_chat.crexe" --rebuild
endlocal
