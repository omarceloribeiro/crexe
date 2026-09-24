@echo off
setlocal
set "CREXE_ROOT=%LOCALAPPDATA%\CREXE"
if defined CREXE_HOME set "CREXE_ROOT=%CREXE_HOME%"
if not exist "%CREXE_ROOT%\releases\v1\crexe.exe" (
  echo ERRO: engine instalada nao encontrada.
  exit /b 1
)
"%CREXE_ROOT%\releases\v1\crexe.exe" unassociate
exit /b %errorlevel%
