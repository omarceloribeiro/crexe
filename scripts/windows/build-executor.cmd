@echo off
setlocal
set "CARGO_EXE=cargo"
where cargo >nul 2>nul
if errorlevel 1 (
  set "CARGO_HOME=%LOCALAPPDATA%\CREXE\devtools\cargo"
  set "RUSTUP_HOME=%LOCALAPPDATA%\CREXE\devtools\rustup"
  set "CARGO_EXE=%LOCALAPPDATA%\CREXE\devtools\cargo\bin\cargo.exe"
  if not exist "%LOCALAPPDATA%\CREXE\devtools\cargo\bin\cargo.exe" (
    echo ERRO: instale Rust/Cargo para compilar a engine.
    exit /b 1
  )
)
pushd "%~dp0..\.."
if errorlevel 1 exit /b 1
"%CARGO_EXE%" build --release --locked
set "CREXE_BUILD_EXIT=%errorlevel%"
popd
exit /b %CREXE_BUILD_EXIT%
