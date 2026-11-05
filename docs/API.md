# API AgentShield

Base : `http://127.0.0.1:8080`
Toutes les routes sont sous `/api`. Réponses et corps en JSON (`application/json`).
CORS permissif en développement — un front sur `localhost:5173`, `:3000`, etc. peut
appeler l'API directement.

**Toutes les sommes sont des entiers de centimes** (`6200` = 62,00 €).
Aucune somme n'est jamais un flottant : côté front, divisez par 100 pour l'affichage.

---

## Conventions

### Erreurs

```json
{ "error": { "code": "guardrail_denied", "message": "Refus : aucun checkout PayPal ne peut être créé pour une action refusée." } }
```

| Code | HTTP | Sens |
| --- | --- | --- |
| `bad_request` | 400 | payload invalide ou incohérent |
| `not_found` | 404 | ressource inconnue |
| `guardrail_denied` | 403 | le garde-fou interdit l'action |
| `conflict` | 409 | état incompatible (déjà réglé, escalade expirée…) |
| `paypal_error` | 502 | PayPal Sandbox a refusé ou est injoignable |
| `database_error` / `internal_error` | 500 | incident interne |

### Preuve d'audit

Chaque écriture renvoie

```json
{ "audit": { "seq": 12, "hash": "0acc6cf24e2701e3…", "chain_valid": true } }
```

`hash` = `sha256(prev_hash | seq | ts | kind | subject | payload)`, 64 caractères
hexadécimaux. `GET /api/audit/verify` recalcule toute la chaîne.

### Décisions

`"decision"` ∈ `"APPROVE"` | `"ESCALATE"` | `"REFUSE"` (majuscules).
`"risk.band"` ∈ `"low"` | `"moderate"` | `"high"` | `"critical"`.

---

## 1. Santé et tableau de bord

### `GET /api/health`

```json
{
  "status": "ok",
  "service": "agentshield-backend",
  "version": "0.1.0",
  "time": "2026-11-05T01:41:22Z",
  "modes": { "paypal": "mock", "expert": "deterministic-fallback" },
  "notice": "Prototype Sandbox — données simulées, aucun argent réel."
}
```

Utilisez `modes` pour afficher un bandeau « PayPal simulé » / « Sandbox live ».

### `GET /api/dashboard`

Agrégats pour l'écran d'accueil.

```json
{
  "totals": {
    "approved": 2, "escalated": 2, "refused": 2,
    "settled_cents": 4500, "reserve_cents": 247500,
    "pending_approvals": 0, "agents": 3
  },
  "agents": [ { "id": "…", "name": "Atlas Shopper", "owner": "…", "purpose": "…", "status": "active", "created_at": "…" } ],
  "recent_actions": [
    { "id": "…", "agent_id": "…", "merchant": "TechStore SAS", "amount_cents": 6200,
      "currency": "EUR", "decision": "REFUSE", "risk_score": 63,
      "created_at": "…", "explanation": "…" }
  ],
  "incidents": [
    { "id": "…", "agent_id": "…", "status": "open", "amount_cents": 7500,
      "currency": "EUR", "description": "…", "created_at": "…" }
  ],
  "audit": { "valid": true, "entries": 17, "head_hash": "…", "message": "17 entrées vérifiées, chaîne intacte." },
  "modes": { "paypal": "mock", "expert": "deterministic-fallback" },
  "sandbox_notice": "Prototype Sandbox : agents, soldes et remboursements sont simulés. Aucun argent réel."
}
```

---

## 2. Agents

### `GET /api/agents`

```json
{
  "agents": [
    { "agent": { "id": "…", "name": "Atlas Shopper", "owner": "Camille Renard",
                 "purpose": "…", "status": "active", "created_at": "…" },
      "wallet": { "agent_id": "…", "balance_cents": 12000, "premium_cents": 1200,
                  "bonus_malus_bps": 0, "reserve_contribution_cents": 120,
                  "spent_this_period_cents": 0, "updated_at": "…" } }
  ]
}
```

`wallet` peut être `null` pour un agent créé hors démonstration.

### `POST /api/agents`

```json
{ "name": "Atlas Shopper", "owner": "Camille", "purpose": "achats audio" }
```

`owner` et `purpose` sont optionnels. Un wallet est créé automatiquement (solde 50 €).
Réponse : `{ "agent": {…}, "audit": {…} }`.

---

## 3. Mandats

Objet `Mandate` :

```json
{
  "id": "36559b3c-…",
  "agent_id": "3d6aea9c-…",
  "owner_name": "Camille Renard",
  "intent": "Casque neuf, maximum 50 EUR, livraison sous 5 jours.",
  "category": "electronics",
  "currency": "EUR",
  "max_total_cents": 20000,
  "per_txn_max_cents": 5000,
  "auto_approve_max_cents": 4000,
  "escalate_until_cents": 5000,
  "allowed_merchants": ["TechStore SAS", "AudioLab"],
  "blocked_merchants": ["RogueShop"],
  "allowed_countries": ["FR"],
  "max_delivery_days": 5,
  "valid_from": "2026-11-05T01:41:10Z",
  "valid_until": "2026-12-05T01:41:10Z",
  "status": "active",
  "created_at": "2026-11-05T01:41:10Z"
}
```

### `POST /api/mandates`

Champs obligatoires : `agent_id`, `owner_name`, `intent`, `category`, `max_total_cents`.
Tout le reste a des valeurs par défaut dérivées du plafond total.

```json
{
  "agent_id": "…",
  "owner_name": "Camille Renard",
  "intent": "Casque neuf, maximum 50 EUR, livraison sous 5 jours.",
  "category": "electronics",
  "max_total_cents": 20000,
  "per_txn_max_cents": 5000,
  "auto_approve_max_cents": 4000,
  "escalate_until_cents": 5000,
  "allowed_merchants": ["TechStore SAS"],
  "blocked_merchants": ["RogueShop"],
  "allowed_countries": ["FR"],
  "max_delivery_days": 5,
  "valid_days": 30
}
```

Réponse :

```json
{
  "mandate": { … },
  "audit": { "seq": 4, "hash": "…", "chain_valid": true },
  "thresholds": { "auto_approve_max_cents": 4000, "escalate_until_cents": 5000 }
}
```

Règles de bornage appliquées côté serveur : `per_txn_max ≤ max_total`,
`auto_approve_max ≤ per_txn_max`, `escalate_until ∈ [auto_approve, per_txn_max]`.

- `allowed_merchants` vide ⇒ aucun contrôle de liste blanche
- `allowed_countries` vide ⇒ aucun contrôle de pays
- `max_delivery_days = 0` ⇒ aucun contrôle de délai
- `category = "any"` ⇒ toutes les catégories

### `GET /api/mandates?agent_id=<uuid>&limit=50`

```json
{ "mandates": [ … ] }
```

### `GET /api/mandates/{id}`

```json
{ "mandate": {…}, "committed_cents": 8300, "remaining_cents": 11700 }
```

`committed_cents` = somme des actions `APPROVE` + `ESCALATE` du mandat.

---

## 4. Actions — le cœur

### `POST /api/actions`

Envoie le « log JSON » de l'agent. Le serveur décide.

```json
{
  "mandate_id": "…",
  "agent_id": "…",                    // optionnel, déduit du mandat
  "kind": "purchase",                 // purchase | subscription | payout | refund
  "merchant": "TechStore SAS",
  "merchant_country": "FR",
  "amount_cents": 6200,
  "currency": "EUR",                  // optionnel, déduit du mandat
  "category": "electronics",
  "delivery_days": 4,
  "description": "Casque audio sans fil",
  "idempotency_key": "ui-1730812931",  // optionnel, protège du rejeu
  "occurred_at": "2026-11-05T01:41:22Z", // optionnel
  "metadata": { "source": "agent" }      // optionnel
}
```

Réponse `ProposalOutcome` :

```json
{
  "action": {
    "id": "d9c22928-…", "mandate_id": "…", "agent_id": "…", "kind": "purchase",
    "merchant": "TechStore SAS", "merchant_country": "FR",
    "amount_cents": 6200, "currency": "EUR", "category": "electronics",
    "delivery_days": 4, "description": "Casque audio sans fil",
    "idempotency_key": "…", "raw_log": { … },
    "decision": "REFUSE", "risk_score": 63, "risk_band": "high",
    "checks": [ … ], "reasons": [ … ], "explanation": "…",
    "approval_id": null, "order_id": null, "settled": false,
    "audit_hash": "d92a40b6…", "created_at": "2026-11-05T01:44:02Z"
  },
  "mandate": { … },
  "agent": { … },
  "decision": "REFUSE",
  "risk": {
    "score": 63,
    "band": "high",
    "factors": [
      { "code": "AMOUNT_INTENSITY", "label": "Part du plafond par transaction consommée", "points": 24 },
      { "code": "ABOVE_AUTO_APPROVE", "label": "Au dessus du seuil d'approbation automatique", "points": 15 }
    ]
  },
  "checks": [
    { "code": "MANDATE_ACTIVE", "label": "Mandat actif", "severity": "strict",
      "passed": true, "detail": "Le mandat est actif." },
    { "code": "AMOUNT_PER_TXN", "label": "Montant sous le plafond par transaction",
      "severity": "strict", "passed": false,
      "detail": "62,00 EUR demandés, plafond 50,00 EUR." }
  ],
  "reasons": ["Budget dépassé : 62,00 EUR demandés pour un plafond de 50,00 EUR sur ce mandat."],
  "explanation": "Achat de 62,00 EUR chez « TechStore SAS » proposé par l'agent, mandat « … ». … Décision : REFUSER. Score de risque : 63/100.",
  "thresholds": { "auto_approve_max_cents": 4000, "escalate_until_cents": 5000 },
  "approval": null,
  "audit": { "seq": 11, "hash": "…", "chain_valid": true },
  "next_steps": [
    "Aucun checkout PayPal n'est créé.",
    "L'agent reçoit l'explication ci-dessus et peut proposer une autre action."
  ]
}
```

Quand `decision = "ESCALATE"`, `approval` est un objet :

```json
{
  "id": "bd641d58-…", "action_id": "…", "status": "pending",
  "requested_at": "…", "expires_at": "…",
  "decided_at": null, "decided_by": null, "note": null, "created_at": "…"
}
```

**Codes de règles renvoyés dans `checks`**

| Code | Sévérité | Sens |
| --- | --- | --- |
| `MANDATE_ACTIVE` | strict | mandat non révoqué / non expiré |
| `MANDATE_PERIOD` | strict | date dans la fenêtre de validité |
| `ACTION_KIND` | strict | achat ou abonnement seulement |
| `CURRENCY_MATCH` | strict | même devise que le mandat |
| `AMOUNT_POSITIVE` | strict | montant > 0 |
| `AMOUNT_PER_TXN` | strict | ≤ plafond par transaction |
| `AMOUNT_TOTAL_BUDGET` | strict | cumul ≤ plafond du mandat |
| `CATEGORY_ALLOWED` | strict | catégorie autorisée |
| `MERCHANT_NOT_BLOCKED` | strict | marchand non bloqué |
| `MERCHANT_ALLOWED` | strict | marchand dans la liste blanche (si définie) |
| `COUNTRY_ALLOWED` | strict | pays autorisé (si défini) |
| `DELIVERY_WINDOW` | strict | délai annoncé ≤ délai exigé |
| `IDEMPOTENCY` | strict | pas de rejeu |
| `THRESHOLD_AUTO_APPROVE` | soft | sous le seuil automatique |
| `THRESHOLD_ESCALATION` | soft | sous la borne d'escalade |

### `GET /api/actions?agent_id=&mandate_id=&decision=&limit=50`

`decision` accepte `APPROVE`, `ESCALATE`, `REFUSE`.

```json
{ "actions": [ … ], "count": 3 }
```

### `GET /api/actions/{id}`

```json
{ "action": {…}, "mandate": {…}, "approval": {…}|null, "order": {…}|null }
```

---

## 5. Approbations (escalades)

### `GET /api/approvals?status=pending&limit=50`

Marque au passage comme `expired` les escalades hors fenêtre.

```json
{ "approvals": [ … ], "count": 1 }
```

### `POST /api/approvals/{id}/confirm`

```json
{ "decided_by": "Camille Renard", "note": "ok pour ce casque" }
```

Les deux champs sont optionnels (`{}` suffit).
Réponse : `{ "approval": {…}, "audit": {…}, "notice": "…" }`.

Effet : à partir de là, et seulement là, `POST /api/actions/{id}/paypal/order`
est autorisé pour cette action.

409 si l'escalade n'est plus `pending` ou si elle a expiré.

### `POST /api/approvals/{id}/deny`

Même contrat. L'action ne produira jamais de checkout.

---

## 6. Flux PayPal Sandbox

Objet `SandboxOrder` :

```json
{
  "id": "9f1c…",                       // identifiant interne
  "action_id": "…",                    // null pour un rechargement
  "purpose": "purchase",               // purchase | topup
  "paypal_order_id": "MOCK-ORDER-F021EB3A20CE47FD9",
  "status": "created",                 // created | approved | captured | failed
  "amount_cents": 4500,
  "currency": "EUR",
  "description": "TechStore SAS — Casque audio sans fil",
  "approve_url": "https://www.sandbox.paypal.com/checkoutnow?token=MOCK-ORDER-…",
  "capture_id": null,
  "mode": "mock",                      // mock | live-sandbox
  "failure_reason": null,
  "created_at": "…",
  "captured_at": null
}
```

### `POST /api/actions/{id}/paypal/order`

Aucun corps nécessaire (`{}`). Crée le checkout.

- 403 `guardrail_denied` si `decision = REFUSE`
- 403 `guardrail_denied` si `decision = ESCALATE` et approbation non confirmée/expirée
- 409 `conflict` si l'action est déjà réglée
- 200 sinon, et idempotent : une commande existante non échouée est renvoyée telle quelle

```json
{ "order": {…}, "action": {…}, "mode": "mock",
  "notice": "Commande PayPal Sandbox : aucun argent réel n'est transféré." }
```

### `POST /api/paypal/orders/{paypal_order_id}/capture`

⚠️ Le paramètre est le **`paypal_order_id`** (`MOCK-ORDER-…`), pas l'`id` interne.

```json
{
  "order": { … "status": "captured", "capture_id": "MOCK-CAPTURE-…" },
  "wallet": { "balance_cents": 7500, … },
  "action": { … },
  "audit": { "seq": 15, "hash": "…", "chain_valid": true },
  "notice": "Capture Sandbox : le wallet simulé a été mis à jour, aucun argent réel n'a bougé."
}
```

Effet : `purpose = purchase` débite le wallet et marque l'action réglée ;
`purpose = topup` crédite le wallet. Idempotent : recapturer une commande déjà
capturée renvoie 200 sans double débit.

### `GET /api/paypal/orders?limit=50`

```json
{ "orders": [ … ], "mode": "mock" }
```

---

## 7. Wallet et prime

### `GET /api/wallet/{agent_id}`

```json
{
  "wallet": {
    "agent_id": "…", "balance_cents": 7500, "premium_cents": 1200,
    "bonus_malus_bps": 0, "reserve_contribution_cents": 120,
    "spent_this_period_cents": 4500, "updated_at": "…"
  },
  "quote": {
    "currency": "EUR",
    "base_premium_cents": 1200,
    "risk_loading_cents": 192,
    "bonus_malus_cents": -160,
    "reserve_contribution_cents": 120,
    "total_cents": 1352,
    "lines": [
      { "code": "BASE", "label": "Prime de base", "amount_cents": 1200, "explanation": "…" },
      { "code": "RISK_LOADING", "label": "Chargement risque", "amount_cents": 192, "explanation": "…" },
      { "code": "BONUS_MALUS", "label": "Bonus historique", "amount_cents": -160, "explanation": "…" },
      { "code": "RESERVE", "label": "Contribution réserve commune", "amount_cents": 120, "explanation": "…" }
    ],
    "headline": "Prime simulée : 13,52 EUR (base 12,00 EUR, ajustements 0,32 EUR, réserve 1,20 EUR)."
  },
  "ledger": [
    { "id": 3, "agent_id": "…", "kind": "debit", "amount_cents": -4500,
      "balance_after_cents": 7500, "label": "Achat 45,00 EUR chez TechStore SAS — Casque audio sans fil",
      "reference": "MOCK-ORDER-…", "created_at": "…" }
  ]
}
```

Invariant : `base + risk_loading + bonus_malus + reserve == total`.

### `GET /api/wallet/{agent_id}/ledger?limit=100`

```json
{ "entries": [ … ] }
```

### `GET /api/wallet/{agent_id}/quote`

```json
{ "wallet": {…}, "quote": {…} }
```

### `POST /api/wallet/{agent_id}/topup`

```json
{ "amount_cents": 2000 }
```

Crée une commande `purpose = "topup"`. Capturez-la pour créditer le wallet.
Plafond de démonstration : 1 000,00 €.

```json
{ "order": {…}, "notice": "Rechargement Sandbox : capturez la commande pour créditer le wallet simulé." }
```

### `GET /api/wallet/reserve`

```json
{ "balance_cents": 247500,
  "ledger": [ { "id": 1, "kind": "payout", "amount_cents": -2500,
                "balance_after_cents": 247500, "label": "…", "reference": "MOCK-PAYOUT-…",
                "created_at": "…" } ] }
```

---

## 8. Audit

### `GET /api/audit?limit=200`

Entrées les plus récentes d'abord.

```json
{
  "entries": [
    { "id": 17, "seq": 17, "ts": "…", "kind": "action.refused", "subject": "d9c22928-…",
      "payload": { "amount_cents": 6200, "decision": "REFUSE", "violated_rules": ["AMOUNT_PER_TXN"] },
      "prev_hash": "…", "hash": "…" }
  ],
  "count": 17,
  "algorithm": "sha256(prev_hash | seq | ts | kind | subject | payload)"
}
```

Types d'entrées : `agent.created`, `mandate.created`, `action.approved`,
`action.escalated`, `action.refused`, `approval.confirmed`, `approval.denied`,
`paypal.order.created`, `paypal.order.captured`, `paypal.order.failed`,
`paypal.topup.created`, `incident.opened`, `recovery.payout`, `recovery.rejected`,
`recovery.review`.

### `GET /api/audit/verify` (ou `POST`)

```json
{
  "chain": {
    "valid": true,
    "entries": 17,
    "head_hash": "…",
    "checked_at": "…",
    "broken_at_seq": null,
    "message": "17 entrées vérifiées, chaîne intacte."
  }
}
```

Si une ligne a été modifiée en base, `valid` passe à `false` et `broken_at_seq`
indique la première entrée incohérente.

---

## 9. Récupération et sinistres

### `GET /api/incidents?status=open&limit=50`

`status` ∈ `open` | `under_review` | `resolved` | `rejected`.

```json
{
  "incidents": [
    { "id": "…", "agent_id": "…", "mandate_id": "…", "action_id": "…",
      "kind": "rogue_action", "status": "open",
      "description": "Dépense de 75,00 EUR chez RogueShop enregistrée hors du garde-fou ; le mandat plafonne à 50,00 EUR.",
      "amount_cents": 7500, "currency": "EUR", "overseen_by": "system",
      "verdict": null, "payout_id": null, "created_at": "…", "resolved_at": null }
  ],
  "count": 1
}
```

### `POST /api/recovery/simulate`

Corps optionnel :

```json
{}
```

```json
{ "incident_id": "…" }
```

```json
{ "agent_id": "…" }
```

Sans paramètre : instruit le plus ancien sinistre ouvert.
Avec `agent_id` : crée d'abord un nouveau scénario « action rogue » pour cet agent.

Réponse :

```json
{
  "outcome": {
    "incident": { … "status": "resolved", "verdict": { … }, "payout_id": "…" },
    "verdict": {
      "verdict": "FLAG",
      "confidence": 0.88,
      "summary_fr": "La dépense de 75,00 EUR dépasse le plafond de 50,00 EUR fixé par le mandat. L'écart de 25,00 EUR n'a jamais été autorisé.",
      "cited_rules": ["AMOUNT_PER_TXN", "AMOUNT_TOTAL_BUDGET"],
      "recommended_resolution": "PAYOUT_OVERAGE",
      "source": "deterministic-fallback",
      "latency_ms": 0
    },
    "payout": {
      "id": "…", "incident_id": "…", "paypal_batch_id": "MOCK-PAYOUT-…",
      "amount_cents": 2500, "currency": "EUR", "status": "pending",
      "mode": "mock", "recipient_email": "sandbox-owner@example.com",
      "note": "Écart de 25,00 EUR au delà du plafond du mandat.",
      "created_at": "…", "completed_at": "…"
    },
    "reserve_balance_cents": 247500,
    "steps": [
      "1. Sinistre « rogue_action » repris : 75,00 EUR chez RogueShop.",
      "2. Garde-fou rejoué sur le log : REFUSE (score 77/100, règles violées : AMOUNT_PER_TXN, AMOUNT_TOTAL_BUDGET).",
      "3. Expert « deterministic-fallback » : FLAG (confiance 88 %).",
      "4. Payout Sandbox (mock) émis : 25,00 EUR vers sandbox-owner@example.com."
    ]
  },
  "notice": "Simulation de récupération : ce n'est pas un produit d'assurance."
}
```

`verdict` ∈ `UPHOLD` | `FLAG` | `INCONCLUSIVE`.
`recommended_resolution` ∈ `PAYOUT_OVERAGE` | `REJECT_CLAIM` | `REQUEST_HUMAN_REVIEW`.

**Garde-fou sur le garde-fou** : le montant du payout est calculé par le code
(`montant − plafond du mandat`, plafonné par la réserve). Un LLM ne peut pas ouvrir
un remboursement que les règles refusent — le verdict est alors ramené au verdict
déterministe et `source` devient `openai:<modèle>+guardrail-clamp`.

### `GET /api/payouts?limit=50`

```json
{ "payouts": [ … ], "mode": "mock",
  "notice": "Payouts Sandbox de démonstration : aucune couverture réelle." }
```

---

## 10. Parcours type côté front

```js
const base = "http://127.0.0.1:8080/api";

// 1. Charger l'écran
const [agents, mandates, dash] = await Promise.all([
  fetch(`${base}/agents`).then(r => r.json()),
  fetch(`${base}/mandates`).then(r => r.json()),
  fetch(`${base}/dashboard`).then(r => r.json()),
]);

// 2. Proposer une action
const proposal = await fetch(`${base}/actions`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    mandate_id: mandates.mandates[0].id,
    merchant: "TechStore SAS",
    merchant_country: "FR",
    amount_cents: 6200,
    currency: "EUR",
    category: "electronics",
    delivery_days: 4,
    description: "Casque audio sans fil",
  }),
}).then(r => r.json());

// proposal.decision  -> "APPROVE" | "ESCALATE" | "REFUSE"
// proposal.approval  -> objet d'escalade si ESCALATE

// 3a. Si ESCALATE : le propriétaire tranche
await fetch(`${base}/approvals/${proposal.approval.id}/confirm`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ decided_by: "Camille" }),
});

// 3b. Si APPROVE (ou escalade confirmée) : checkout
const { order } = await fetch(`${base}/actions/${proposal.action.id}/paypal/order`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: "{}",
}).then(r => r.json());

// 4. Capture (après retour PayPal, ou directement en mode mock)
await fetch(`${base}/paypal/orders/${order.paypal_order_id}/capture`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: "{}",
});
```

### Suggestions d'écran

1. **Mandat** — afficher `intent`, `max_total_cents`, `per_txn_max_cents`, et la
   règle « ≤ 40 € approuvé · 40–50 € escaladé · > 50 € refusé ».
2. **Action** — formulaire pré-rempli avec le log JSON visible.
3. **Décision** — grand verdict coloré, score de risque, liste des `checks`,
   `explanation`, hash de preuve.
4. **Flux PayPal** — bouton « créer le checkout » (désactivé sauf `APPROVE`),
   lien `approve_url`, bouton « capturer ».
5. **Wallet** — solde, décomposition de la prime, journal.
6. **Audit** — statut de chaîne + dernières entrées.
