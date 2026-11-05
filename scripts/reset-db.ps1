# AgentShield — remise à zéro de la base de démonstration
#
# Supprime backend\data (SQLite + WAL). Le prochain démarrage recrée le jeu de
# démonstration si AGENTSHIELD_SEED n'est pas désactivé.

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$data = Join-Path $root "backend\data"

if (Test-Path $data) {
    $resolved = (Resolve-Path $data).Path
    Write-Host "Suppression de $resolved" -ForegroundColor Yellow
    Remove-Item -Recurse -Force $resolved
    Write-Host "Base réinitialisée." -ForegroundColor Green
} else {
    Write-Host "Aucune base à supprimer ($data)." -ForegroundColor DarkGray
}
