@echo off
setlocal
call "%~dp0build-executor.cmd"
if errorlevel 1 exit /b %errorlevel%
"%~dp0..\..\target\release\crexe.exe" install
if errorlevel 1 exit /b %errorlevel%
call "%~dp0configure.cmd"
exit /b %errorlevel%
