@echo off
setlocal

set "EXE=%~dp0target\release\crexe.exe"

if not exist "%EXE%" (
  echo ERRO: "%EXE%" nao encontrado.
  echo Rode primeiro: build-executor.cmd
  exit /b 1
)

echo Associando .crexe ao CREXE runtime...
reg add "HKCU\Software\Classes\.crexe" /ve /d "CREXE.File" /f >nul
reg add "HKCU\Software\Classes\CREXE.File" /ve /d "CREXE Creative Executable" /f >nul
reg add "HKCU\Software\Classes\CREXE.File\shell\open\command" /ve /d "\"%EXE%\" \"%%1\"" /f >nul

echo.
echo OK. Agora voce pode dar dois cliques em arquivos .crexe
echo ou arrastar um .crexe em cima de:
echo "%EXE%"
echo.
endlocal
