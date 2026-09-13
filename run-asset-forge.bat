@echo off
setlocal
cd /d "%~dp0"

set "VENV=asset-forge\.venv"
set "PY=%VENV%\Scripts\python.exe"

if not exist "%PY%" (
  echo Creating Asset Forge virtual environment...
  python -m venv "%VENV%"
  if errorlevel 1 (
    echo Python 3 is required.
    pause
    exit /b 1
  )
  "%PY%" -m pip install --upgrade pip
  "%PY%" -m pip install -r asset-forge\requirements.txt
  if errorlevel 1 (
    echo Failed to install dependencies.
    pause
    exit /b 1
  )
)

echo Starting Asset Forge at http://localhost:8502
echo Close this window or press Ctrl+C to stop.
"%PY%" -m streamlit run asset-forge\app.py --server.port 8502 --server.headless true

pause
