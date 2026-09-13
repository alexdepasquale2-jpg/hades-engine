@echo off
setlocal
cd /d "%~dp0"

set "VENV=hades-studio\.venv"
set "PY=%VENV%\Scripts\python.exe"

if not exist "%PY%" (
  echo Creating Hades Studio virtual environment...
  python -m venv "%VENV%"
  if errorlevel 1 (
    echo Python 3 is required. Install from https://www.python.org/downloads/
    pause
    exit /b 1
  )
  "%PY%" -m pip install --upgrade pip
  "%PY%" -m pip install -r hades-studio\requirements.txt
  if errorlevel 1 (
    echo Failed to install Studio dependencies.
    pause
    exit /b 1
  )
)

echo Starting Hades Studio at http://localhost:8500
echo Close this window or press Ctrl+C to stop.
"%PY%" -m streamlit run hades-studio\app.py --server.port 8500 --server.headless true

pause
