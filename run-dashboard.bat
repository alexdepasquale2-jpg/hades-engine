@echo off
setlocal
cd /d "%~dp0"

set "VENV=dashboard\.venv"
set "PY=%VENV%\Scripts\python.exe"

if not exist "%PY%" (
  echo Creating dashboard virtual environment...
  python -m venv "%VENV%"
  if errorlevel 1 (
    echo Python 3 is required. Install from https://www.python.org/downloads/
    pause
    exit /b 1
  )
  "%PY%" -m pip install --upgrade pip
  "%PY%" -m pip install -r dashboard\requirements.txt
  if errorlevel 1 (
    echo Failed to install dashboard dependencies.
    pause
    exit /b 1
  )
)

echo Starting dashboard at http://localhost:8501
echo Close this window or press Ctrl+C to stop.
"%PY%" -m streamlit run dashboard\app.py --server.headless true

pause
