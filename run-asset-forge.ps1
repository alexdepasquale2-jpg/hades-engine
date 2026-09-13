$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$venv = Join-Path $PSScriptRoot "asset-forge\.venv"
$py = Join-Path $venv "Scripts\python.exe"

if (-not (Test-Path $py)) {
    Write-Host "Creating Asset Forge virtual environment..."
    python -m venv $venv
    & $py -m pip install --upgrade pip
    & $py -m pip install -r (Join-Path $PSScriptRoot "asset-forge\requirements.txt")
}

Write-Host "Starting Asset Forge at http://localhost:8502"
& $py -m streamlit run (Join-Path $PSScriptRoot "asset-forge\app.py") --server.port 8502 --server.headless true
