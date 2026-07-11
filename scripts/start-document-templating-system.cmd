@echo off
setlocal
set "PROJECT_ROOT=%~dp0.."
cd /d "%PROJECT_ROOT%"

if not defined CARGO_TARGET_DIR set "CARGO_TARGET_DIR=%PROJECT_ROOT%\target-current"

where cargo >nul 2>nul
if not errorlevel 1 (
  if exist "%VCINSTALLDIR%\Tools\MSVC" (
    cargo run --release -- %*
    exit /b %ERRORLEVEL%
  )

  if exist "C:\msys64\mingw64\bin\gcc.exe" (
    if exist "%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin\cargo.exe" (
      set "RUSTC=%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin\rustc.exe"
      set "RUSTDOC=%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin\rustdoc.exe"
      "%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin\cargo.exe" run --release -- %*
      exit /b %ERRORLEVEL%
    )

    cargo +stable-x86_64-pc-windows-gnu run --release -- %*
    exit /b %ERRORLEVEL%
  )

  cargo run --release -- %*
  exit /b %ERRORLEVEL%
)

if exist "%CARGO_TARGET_DIR%\release\document-templating-system.exe" (
  "%CARGO_TARGET_DIR%\release\document-templating-system.exe" %*
  exit /b %ERRORLEVEL%
)

if exist "target\release\document-templating-system.exe" (
  "target\release\document-templating-system.exe" %*
  exit /b %ERRORLEVEL%
)

echo error: document-templating-system binary is missing and Cargo was not found. 1>&2
exit /b 1
