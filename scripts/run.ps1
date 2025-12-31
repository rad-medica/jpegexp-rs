# Run script for jpegexp-rs CLI
# Usage: .\scripts\run.ps1 [args...]

param(
    [Parameter(ValueFromRemainingArguments=$true)]
    [string[]]$Args
)

$ErrorActionPreference = "Stop"

# Build if needed
if (-not (Test-Path "target\debug\jpegexp.exe")) {
    Write-Host "Building jpegexp (debug)..." -ForegroundColor Cyan
    cargo build --bin jpegexp
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed!" -ForegroundColor Red
        exit 1
    }
}

# Run the CLI
$binary = "target\debug\jpegexp.exe"
if ($Args.Count -eq 0) {
    Write-Host "Running: $binary --help" -ForegroundColor Cyan
    & $binary --help
} else {
    Write-Host "Running: $binary $($Args -join ' ')" -ForegroundColor Cyan
    & $binary @Args
}

