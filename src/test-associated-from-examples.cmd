@echo off
setlocal
call "%~dp0build-executor.cmd"
if errorlevel 1 exit /b %errorlevel%
pushd "%~dp0examples"
"%~dp0target\release\crexe.exe" calculator_native.crexe
set "ERR=%errorlevel%"
popd
exit /b %ERR%
