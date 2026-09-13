$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$venv = Join-Path $PSScriptRoot "mechanics-forge\.venv"
$py = Join-Path $venv "Scripts\python.exe"

if (-not (Test-Path $py)) {
    Write-Host "Creating Mechanics Forge virtual environment..."
    python -m venv $venv
    & $py -m pip install --upgrade pip
    & $py -m pip install -r (Join-Path $PSScriptRoot "mechanics-forge\requirements.txt")
}

Write-Host "Starting Mechanics Forge at http://localhost:8503"
& $py -m streamlit run (Join-Path $PSScriptRoot "mechanics-forge\app.py") --server.port 8503 --server.headless true
