# Job de vérification : build + tests + démo de bout en bout.
# Usage : .\scripts\verify.ps1

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$backend = Join-Path $root "backend"

$w64 = "C:\Users\HIUG\.dsh-tools\w64devkit\bin"
if (Test-Path $w64) { $env:PATH = "$w64;$env:USERPROFILE\.cargo\bin;$env:PATH" }
$env:CARGO_TERM_COLOR = "never"

Push-Location $backend
try {
    Write-Host "== cargo fmt --check" -ForegroundColor Cyan
    cargo fmt --check

    Write-Host "== cargo clippy" -ForegroundColor Cyan
    cargo clippy --all-targets -- -D warnings

    Write-Host "== cargo test" -ForegroundColor Cyan
    cargo test

    Write-Host "== cargo build --release" -ForegroundColor Cyan
    cargo build --release
} finally {
    Pop-Location
}

Write-Host "`nVérification terminée." -ForegroundColor Green
