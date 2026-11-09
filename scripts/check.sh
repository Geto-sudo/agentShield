#!/usr/bin/env bash

set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_DIR="$ROOT_DIR/backend"

fail() {
    echo
    echo "ERREUR : $1" >&2
    exit 1
}

echo
echo "=========================================="
echo "       AGENTSHIELD - PROJECT CHECK"
echo "=========================================="

# 1. Vérifier les outils nécessaires
echo
echo "[1/7] Environnement"

command -v cargo >/dev/null 2>&1 || fail "Cargo est introuvable."
command -v python >/dev/null 2>&1 || fail "Python est introuvable."

rustc --version
cargo --version
python --version

# 2. Vérifier le formatage Rust
echo
echo "[2/7] Formatage Rust"

cd "$BACKEND_DIR"
cargo fmt --check || fail "Le formatage Rust doit être corrigé."

# 3. Analyser le code Rust
echo
echo "[3/7] Analyse Clippy"

cargo clippy --all-targets -- -D warnings \
    || fail "Clippy a détecté des erreurs ou avertissements."

# 4. Exécuter les tests Rust
echo
echo "[4/7] Tests Rust"

cargo test || fail "Les tests Rust ont échoué."

# 5. Compiler le backend en release
echo
echo "[5/7] Compilation release"

cargo build --release || fail "La compilation release a échoué."

# 6. Vérifier le frontend Python
echo
echo "[6/7] Vérifications Python"

cd "$ROOT_DIR"

python -m compileall -q main.py app_pages components data styles \
    || fail "Une erreur de syntaxe Python a été détectée."

python -c "import streamlit, pandas; print('Streamlit:', streamlit.__version__); print('Pandas:', pandas.__version__)" \
    || fail "Streamlit ou Pandas est absent."

# 7. Vérifier les différences Git
echo
echo "[7/7] Vérifications Git"

git diff --check || fail "Erreur d'espacement dans les modifications non indexées."
git diff --cached --check || fail "Erreur d'espacement dans les modifications indexées."

echo
echo "=========================================="
echo "  TOUTES LES VERIFICATIONS SONT PASSEES"
echo "=========================================="