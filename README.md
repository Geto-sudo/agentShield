# AgentShield — backend Rust

Garde-fou de dépenses PayPal pour agents IA.
**Prototype Sandbox** du *PayPal AI Hackathon 2026*.

> Avant qu'un agent IA crée un paiement PayPal, AgentShield vérifie un mandat
> défini par l'humain, garde la preuve de la décision et approuve, escalade ou
> refuse l'action.

Ce dépôt contient **uniquement le backend** : une API JSON en Rust. Le front est
développé séparément et consomme `/api/*` (CORS ouvert en développement).

---

## Ce qui est implémenté

| Bloc | État | Détail |
| --- | --- | --- |
| Mandat | ✅ | plafond total, plafond par transaction, seuils auto/escalade, catégorie, marchands autorisés/bloqués, pays, délai de livraison, fenêtre de validité |
| Log JSON d'action | ✅ | schéma typé (`POST /api/actions`), clé d'idempotence |
| Décision | ✅ | `APPROVE` / `ESCALATE` / `REFUSE`, 14 règles strictes, score de risque 0-100, explication en français |
| Escalade | ✅ | approbation humaine explicite, expiration, refus impossible à contourner |
| Preuve | ✅ | chaîne d'audit SHA-256 append-only, signature Ed25519 par entrée, clé publique exposée |
| PayPal Sandbox | ✅ | `Orders` create + capture, `Payouts` ; mode `mock` automatique sans clés |
| Wallet | ✅ | solde débité à la capture, journal (ledger), prime + bonus-malus + réserve commune |
| Récupération | ✅ | scénario « action rogue », verdict expert structuré, payout Sandbox plafonné |
| Expert IA | ✅ | OpenAI (`gpt-4o-mini` par défaut) + repli déterministe ; le LLM ne fixe jamais un montant |
| SQLite | ✅ | schéma SQL embarqué, une seule source de vérité |
| Tests | ✅ | 34 tests unitaires + 9 tests d'intégration HTTP |

Limite assumée : les agents, soldes, sinistres et remboursements sont **simulés**.
Aucun argent réel, aucune couverture réelle.

---

## Démarrage

Prérequis : Rust (toolchain `stable`), aucune dépendance système.

```bash
cd backend
cp .env.example .env      # optionnel : tout fonctionne sans clés
cargo run
```

```text
AgentShield — http://127.0.0.1:8080
SQLite      : data/agentshield.db
PayPal      : mock
Expert      : deterministic-fallback
```

Au premier lancement, une base de démonstration est créée (3 agents, 3 mandats,
1 sinistre ouvert, un historique). `GET /` renvoie la liste des routes,
`GET /api/health` l'état du service.

Vérification rapide :

```bash
curl http://127.0.0.1:8080/api/health
curl http://127.0.0.1:8080/api/mandates
```

> Windows : `scripts\run.ps1` fait la même chose avec un environnement propre.

---

## Le parcours en une requête

```bash
curl -X POST http://127.0.0.1:8080/api/actions \
  -H "Content-Type: application/json" \
  -d '{
    "mandate_id": "<id du mandat>",
    "merchant": "TechStore SAS",
    "merchant_country": "FR",
    "amount_cents": 6200,
    "currency": "EUR",
    "category": "electronics",
    "delivery_days": 4,
    "description": "Casque audio sans fil"
  }'
```

Réponse (extrait) :

```json
{
  "decision": "REFUSE",
  "risk": { "score": 63, "band": "high" },
  "reasons": ["Budget dépassé : 62,00 EUR demandés pour un plafond de 50,00 EUR sur ce mandat."],
  "explanation": "Achat de 62,00 EUR chez « TechStore SAS » proposé par l'agent, mandat « … ». …",
  "audit": { "seq": 11, "hash": "d92a40b6…" },
  "next_steps": ["Aucun checkout PayPal n'est créé.", "…"]
}
```

Avec `3800` → `APPROVE`, avec `4500` → `ESCALATE` (une approbation est créée).

---

## Politique de décision

Ordre d'évaluation, **entièrement déterministe** :

1. **Règles strictes** — si une seule échoue : `REFUSE`. Aucune exception, aucun LLM.
   `MANDATE_ACTIVE`, `MANDATE_PERIOD`, `ACTION_KIND`, `CURRENCY_MATCH`,
   `AMOUNT_POSITIVE`, `AMOUNT_PER_TXN`, `AMOUNT_TOTAL_BUDGET`, `CATEGORY_ALLOWED`,
   `MERCHANT_NOT_BLOCKED`, `MERCHANT_ALLOWED`, `COUNTRY_ALLOWED`,
   `DELIVERY_WINDOW`, `IDEMPOTENCY`.
2. **Seuils** — toutes les règles passent :
   - `montant ≤ auto_approve_max_cents` → `APPROVE`
   - `auto_approve_max_cents < montant ≤ escalate_until_cents` → `ESCALATE`
   - au delà → `REFUSE` (filet de sécurité)

Le score de risque ne change jamais l'issue : il informe, trie et alimente la prime.

---

## Configuration

Tout est optionnel ; sans clés, le service tourne en mode simulé complet.

| Variable | Défaut | Rôle |
| --- | --- | --- |
| `AGENTSHIELD_ADDR` | `127.0.0.1:8080` | adresse d'écoute |
| `AGENTSHIELD_DB` | `data/agentshield.db` | chemin SQLite (`:memory:` accepté) |
| `AGENTSHIELD_SEED` | `true` | jeu de démonstration au premier lancement |
| `AGENTSHIELD_AUTO_APPROVE_MAX_CENTS` | `4000` | seuil auto par défaut (40 €) |
| `AGENTSHIELD_APPROVAL_TTL_MINUTES` | `30` | durée de vie d'une escalade |
| `AGENTSHIELD_PUBLIC_URL` | `http://127.0.0.1:8080` | URLs de retour checkout |
| `AGENTSHIELD_PAYOUT_EMAIL` | `sandbox-owner@example.com` | destinataire des payouts de démo |
| `AGENTSHIELD_AUDIT_SIGNING_KEY` | — | graine Ed25519 (base64) de signature de l'audit ; vide = clé éphémère |
| `PAYPAL_CLIENT_ID` / `PAYPAL_CLIENT_SECRET` | — | active le vrai Sandbox PayPal |
| `PAYPAL_BASE_URL` | `https://api-m.sandbox.paypal.com` | API PayPal |
| `PAYPAL_FORCE_MOCK` | `false` | force le mode simulé |
| `OPENAI_API_KEY` | — | active l'expert IA réel |
| `OPENAI_MODEL` | `gpt-4o-mini` | modèle utilisé |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | compatible Azure/proxy |

`GET /api/health` expose le mode effectif (`mock` / `live-sandbox`,
`deterministic-fallback` / `openai`) et la clé publique d'audit
(`audit.public_key`, `audit.key_source`).

---

## Documentation

- [`docs/API.md`](docs/API.md) — référence complète des routes, avec exemples.
- [`docs/openapi.yaml`](docs/openapi.yaml) — spécification OpenAPI 3.1.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — composants, flux, garanties.
- [`docs/DECISIONS.md`](docs/DECISIONS.md) — pourquoi ces choix techniques.
- [`docs/DEMO.md`](docs/DEMO.md) — script de démo en 2 minutes.
- [`samples/`](samples) — payloads JSON prêts à l'emploi.

---

## Tests

```bash
cd backend
cargo test          # 34 unitaires + 9 intégrations HTTP
cargo clippy --all-targets
```

Les tests d'intégration utilisent une base `:memory:` et la passerelle PayPal
simulée : aucun réseau, aucun compte.

---

## Formulation à garder

> « AgentShield est un prototype Sandbox de contrôle et de traçabilité des
> dépenses d'agents IA. »

À ne pas dire : qu'il s'agit d'une assurance réglementée, qu'il protège des
utilisateurs réels, ou qu'il bloque des paiements qui ne passent pas par
l'intégration AgentShield + PayPal.
