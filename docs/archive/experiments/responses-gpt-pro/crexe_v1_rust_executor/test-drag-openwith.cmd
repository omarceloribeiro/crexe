@echo off
setlocal
call "%~dp0build-executor.cmd"
if errorlevel 1 exit /b %errorlevel%
echo Testando modo drag/open-with:
"%~dp0target\release\crexe.exe" "%~dp0examples\calculator_native.crexe" --rebuild
endlocal
