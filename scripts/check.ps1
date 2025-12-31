# Code quality check script for jpegexp-rs
# Usage: .\scripts\check.ps1 [all|fmt|clippy|test]

param(
    [Parameter(Position=0)]
    [ValidateSet("all", "fmt", "clippy", "test")]
    [string]$Mode = "all"
)

$ErrorActionPreference = "Stop"

function Check-Fmt {
    Write-Host "Checking code formatting..." -ForegroundColor Cyan
    cargo fmt -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Format check failed! Run 'cargo fmt' to fix." -ForegroundColor Red
        exit 1
    }
    Write-Host "Format check passed!" -ForegroundColor Green
}

function Check-Clippy {
    Write-Host "Running clippy..." -ForegroundColor Cyan
    cargo clippy -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Clippy check failed!" -ForegroundColor Red
        exit 1
    }
    Write-Host "Clippy check passed!" -ForegroundColor Green
}

function Check-Test {
    Write-Host "Running tests..." -ForegroundColor Cyan
    cargo test
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Tests failed!" -ForegroundColor Red
        exit 1
    }
    Write-Host "Tests passed!" -ForegroundColor Green
}

switch ($Mode.ToLower()) {
    "all" {
        Check-Fmt
        Check-Clippy
        Check-Test
    }
    "fmt" {
        Check-Fmt
    }
    "clippy" {
        Check-Clippy
    }
    "test" {
        Check-Test
    }
}

