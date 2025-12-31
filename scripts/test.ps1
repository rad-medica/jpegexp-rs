# Test script for jpegexp-rs
# Usage: .\scripts\test.ps1 [all|unit|verbose]

param(
    [Parameter(Position=0)]
    [ValidateSet("all", "unit", "verbose")]
    [string]$Mode = "all"
)

$ErrorActionPreference = "Stop"

function Run-Tests {
    param([switch]$Verbose)
    
    if ($Verbose) {
        Write-Host "Running tests (verbose)..." -ForegroundColor Cyan
        cargo test -- --nocapture
    } else {
        Write-Host "Running tests..." -ForegroundColor Cyan
        cargo test
    }
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Tests failed!" -ForegroundColor Red
        exit 1
    }
    Write-Host "All tests passed!" -ForegroundColor Green
}

function Run-PythonTests {
    Write-Host "Running Python tests..." -ForegroundColor Cyan
    if (Test-Path "tests\comprehensive_test.py") {
        python tests\comprehensive_test.py
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Python tests failed!" -ForegroundColor Red
            exit 1
        }
        Write-Host "Python tests passed!" -ForegroundColor Green
    } else {
        Write-Host "Python test file not found, skipping..." -ForegroundColor Yellow
    }
}

switch ($Mode.ToLower()) {
    "all" {
        Run-Tests
        Run-PythonTests
    }
    "unit" {
        Run-Tests
    }
    "verbose" {
        Run-Tests -Verbose
    }
}

