$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$venv = Join-Path $PSScriptRoot "dashboard\.venv"
$py = Join-Path $venv "Scripts\python.exe"

if (-not (Test-Path $py)) {
    Write-Host "Creating dashboard virtual environment..."
    python -m venv $venv
    & $py -m pip install --upgrade pip
    & $py -m pip install -r (Join-Path $PSScriptRoot "dashboard\requirements.txt")
}

Write-Host "Starting dashboard at http://localhost:8501"
& $py -m streamlit run (Join-Path $PSScriptRoot "dashboard\app.py") --server.headless true
