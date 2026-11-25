@echo off
setlocal
set "CREXE_ROOT=%LOCALAPPDATA%\CREXE"
if defined CREXE_HOME set "CREXE_ROOT=%CREXE_HOME%"
if not exist "%CREXE_ROOT%\releases\v1\crexe.exe" (
  echo ERRO: instale a engine com scripts\windows\install.cmd.
  exit /b 1
)
"%CREXE_ROOT%\releases\v1\crexe.exe" associate
exit /b %errorlevel%
