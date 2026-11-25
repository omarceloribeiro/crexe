@echo off
setlocal
call "%~dp0build-executor.cmd"
if errorlevel 1 exit /b %errorlevel%
"%~dp0..\..\target\release\crexe.exe" exec "%~dp0..\..\examples\yaml\calculator_native.crexe" %*
exit /b %errorlevel%
