@echo off
setlocal
cd /d "%~dp0"

set "VENV=mechanics-forge\.venv"
set "PY=%VENV%\Scripts\python.exe"

if not exist "%PY%" (
  echo Creating Mechanics Forge virtual environment...
  python -m venv "%VENV%"
  if errorlevel 1 (
    echo Python 3 is required.
    pause
    exit /b 1
  )
  "%PY%" -m pip install --upgrade pip
  "%PY%" -m pip install -r mechanics-forge\requirements.txt
)

echo Starting Mechanics Forge at http://localhost:8503
echo Close this window or press Ctrl+C to stop.
"%PY%" -m streamlit run mechanics-forge\app.py --server.port 8503 --server.headless true

pause
