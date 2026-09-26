@echo off
setlocal
set "CREXE_CONFIG_ENGINE=%LOCALAPPDATA%\CREXE\releases\v1\crexe.exe"
if defined CREXE_HOME set "CREXE_CONFIG_ENGINE=%CREXE_HOME%\releases\v1\crexe.exe"
if not exist "%CREXE_CONFIG_ENGINE%" set "CREXE_CONFIG_ENGINE=%~dp0..\..\target\release\crexe.exe"
"%CREXE_CONFIG_ENGINE%" configure %*
exit /b %errorlevel%
