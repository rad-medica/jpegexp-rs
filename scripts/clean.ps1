# Clean script for jpegexp-rs
# Usage: .\scripts\clean.ps1 [all|cargo|python]

param(
    [Parameter(Position=0)]
    [ValidateSet("all", "cargo", "python")]
    [string]$Mode = "all"
)

$ErrorActionPreference = "Stop"

function Clean-Cargo {
    Write-Host "Cleaning Cargo build artifacts..." -ForegroundColor Cyan
    cargo clean
    Write-Host "Cargo clean completed!" -ForegroundColor Green
}

function Clean-Python {
    Write-Host "Cleaning Python build artifacts..." -ForegroundColor Cyan
    Push-Location python
    try {
        if (Test-Path "dist") {
            Remove-Item -Recurse -Force dist
        }
        if (Test-Path "build") {
            Remove-Item -Recurse -Force build
        }
        Get-ChildItem -Filter "*.egg-info" -Recurse | Remove-Item -Recurse -Force
        Write-Host "Python clean completed!" -ForegroundColor Green
    }
    finally {
        Pop-Location
    }
}

switch ($Mode.ToLower()) {
    "all" {
        Clean-Cargo
        Clean-Python
    }
    "cargo" {
        Clean-Cargo
    }
    "python" {
        Clean-Python
    }
}

