# Build script for jpegexp-rs
# Usage: .\scripts\build.ps1 [debug|release|all]

param(
    [Parameter(Position=0)]
    [ValidateSet("debug", "release", "all", "cli", "python")]
    [string]$Mode = "debug"
)

$ErrorActionPreference = "Stop"

function Build-Debug {
    Write-Host "Building debug..." -ForegroundColor Cyan
    cargo build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Debug build failed!" -ForegroundColor Red
        exit 1
    }
    Write-Host "Debug build completed!" -ForegroundColor Green
}

function Build-Release {
    Write-Host "Building release..." -ForegroundColor Cyan
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Release build failed!" -ForegroundColor Red
        exit 1
    }
    Write-Host "Release build completed!" -ForegroundColor Green
}

function Build-CLI {
    Write-Host "Building CLI (release)..." -ForegroundColor Cyan
    cargo build --release --bin jpegexp
    if ($LASTEXITCODE -ne 0) {
        Write-Host "CLI build failed!" -ForegroundColor Red
        exit 1
    }
    Write-Host "CLI build completed!" -ForegroundColor Green
    Write-Host "Binary location: target\release\jpegexp.exe" -ForegroundColor Yellow
}

function Build-Python {
    Write-Host "Building Python bindings..." -ForegroundColor Cyan
    Push-Location python
    try {
        maturin develop
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Python bindings build failed!" -ForegroundColor Red
            exit 1
        }
        Write-Host "Python bindings build completed!" -ForegroundColor Green
    }
    finally {
        Pop-Location
    }
}

switch ($Mode.ToLower()) {
    "debug" {
        Build-Debug
    }
    "release" {
        Build-Release
    }
    "all" {
        Build-Debug
        Build-Release
        Build-Python
    }
    "cli" {
        Build-CLI
    }
    "python" {
        Build-Python
    }
}

