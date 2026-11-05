# AgentShield — démarrage du backend (Windows / PowerShell)
#
# Usage :
#   .\scripts\run.ps1
#   .\scripts\run.ps1 -Release
#
# La toolchain Rust de cette machine est `stable-x86_64-pc-windows-gnu` et
# s'appuie sur le linker MinGW-w64 de w64devkit (portable). Le script l'ajoute
# au PATH si le dossier existe, puis lance le serveur.

param(
    [switch]$Release,
    [string]$Addr = "127.0.0.1:8080"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$backend = Join-Path $root "backend"

$w64 = "C:\Users\HIUG\.dsh-tools\w64devkit\bin"
if (Test-Path $w64) {
    $env:PATH = "$w64;$env:USERPROFILE\.cargo\bin;$env:PATH"
} else {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo introuvable. Installez Rust (https://rustup.rs) puis relancez."
}

$env:AGENTSHIELD_ADDR = $Addr
if (-not $env:RUST_LOG) { $env:RUST_LOG = "info,agentshield=debug" }

Push-Location $backend
try {
    Write-Host "AgentShield — API sur http://$Addr (Ctrl+C pour arrêter)" -ForegroundColor Cyan
    if ($Release) {
        cargo run --release
    } else {
        cargo run
    }
} finally {
    Pop-Location
}
