@echo off
setlocal
echo Removendo associacao .crexe do usuario atual...
reg delete "HKCU\Software\Classes\.crexe" /f >nul 2>nul
reg delete "HKCU\Software\Classes\CREXE.File" /f >nul 2>nul
echo OK.
endlocal
