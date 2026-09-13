@echo off
setlocal
cd /d "%~dp0"
echo Building tbc-play (release)...
cargo build -p tbc-game --release
if errorlevel 1 (
  echo Build failed. Install Rust from https://rustup.rs/
  pause
  exit /b 1
)
echo.
echo Starting TBC Play...
target\release\tbc-play.exe
pause
