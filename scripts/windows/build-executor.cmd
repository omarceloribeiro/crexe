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
"%CARGO_EXE%" build --release --locked --manifest-path "%~dp0..\..\Cargo.toml"
exit /b %errorlevel%
