# Architecture

## Vue d'ensemble

```text
   Agent IA (simulé)                    Propriétaire humain
        │                                      │
        │ (1) log JSON                         │ (0) mandat
        ▼                                      ▼
  ┌─────────────────────────────────────────────────────────┐
  │                 API HTTP  (axum)                        │
  │        /api/actions   /api/mandates   …                 │
  └───────────────────────────┬─────────────────────────────┘
                              │
                              ▼
  ┌─────────────────────────────────────────────────────────┐
  │                    service  (logique)                   │
  │  propose · decide_approval · create_order · run_recovery│
  └───────┬───────────────┬──────────────────┬──────────────┘
          │               │                  │
          ▼               ▼                  ▼
  ┌──────────────┐  ┌─────────────┐  ┌──────────────────────┐
  │ engine       │  │ store       │  │ paypal               │
  │ guardrail    │  │ SQLite      │  │ SandboxGateway ──────┼──► api-m.sandbox.paypal.com
  │ risk         │  │ audit chain │  │ MockGateway          │
  │ pricing      │  └─────────────┘  └──────────────────────┘
  │ expert ──────┼──────────────────────────────────────────────► api.openai.com (optionnel)
  └──────────────┘
```

Le backend est **API-only** : aucune page n'est servie. Le front consomme
`/api/*` depuis son propre serveur (CORS ouvert).

## Composants

| Module | Responsabilité | Dépendances |
| --- | --- | --- |
| `api` | routes, extraction, codes HTTP | `service` |
| `service` | orchestration métier, transactions | `engine`, `store`, `paypal` |
| `domain` | types, argent en centimes, sérialisation | — |
| `engine::guardrail` | décision pure `APPROVE/ESCALATE/REFUSE` | `domain` |
| `engine::risk` | score 0-100, facteurs explicables | `domain` |
| `engine::pricing` | prime = base + risque + bonus-malus + réserve | `domain` |
| `engine::expert` | verdict IA structuré, repli déterministe | `domain`, réseau |
| `store` | SQLite, requêtes, chaîne d'audit | `domain` |
| `paypal` | trait `PayPalGateway` + 2 implémentations | `domain` |

`engine` ne connaît ni SQLite ni HTTP : ce sont des fonctions pures, donc
testables et reproductibles. `service` est le seul endroit qui décide *quand*
écrire, et le fait toujours dans une transaction.

## Flux « action proposée »

1. `POST /api/actions` décode un `ProposedAction`.
2. `service::propose` charge le mandat et l'agent.
3. `build_history` calcule la fenêtre : budget déjà engagé, actions des 15
   dernières minutes, marchands connus, doublon d'idempotence, sinistres ouverts.
4. `guardrail::evaluate` rend un `GuardrailVerdict` (pur, sans I/O).
5. Une transaction :
   - insère l'approbation si `ESCALATE` ;
   - ajoute l'entrée d'audit et récupère son hash ;
   - insère l'action avec `audit_hash`.
6. La réponse contient décision, risque, contrôles, explication, preuve et
   `next_steps`.

Une action `REFUSE` est **enregistrée** : c'est une preuve, pas un non-événement.

## Flux PayPal

```text
POST /api/actions/{id}/paypal/order
      │  refus si decision = REFUSE
      │  refus si ESCALATE et approbation non confirmée / expirée
      │  refus si déjà réglée
      ▼
  gateway.create_order()  ──►  POST /v2/checkout/orders   (mode live)
      │                        id + approve_url déterministes (mode mock)
      ▼
  orders INSERT + actions.order_id + audit

POST /api/paypal/orders/{paypal_order_id}/capture
      ▼
  gateway.capture_order() ──►  POST /v2/checkout/orders/{id}/capture
      ▼
  transaction : order captured
                wallet debit|credit + ledger
                actions.settled = 1 (si purchase)
                audit
```

Idempotence : recapturer une commande déjà capturée ne re-débite pas.

## Flux récupération

1. `POST /api/recovery/simulate` prend un sinistre ouvert (ou en crée un avec
   `agent_id`).
2. Le garde-fou est **rejoué** sur le log enregistré : c'est la vérité
   déterministe.
3. `ExpertService::judge` interroge OpenAI si une clé est configurée, sinon le
   repli déterministe répond.
4. `reconcile` ramène le verdict au verdict déterministe si le modèle tente
   d'ouvrir une couverture que les règles refusent.
5. Le montant du payout est **calculé** : `montant − plafond du mandat`, borné par
   la réserve disponible.
6. Payout Sandbox, mouvement de réserve, mise à jour du sinistre, audit.

## Données

`migrations/0001_init.sql` est compilé dans le binaire (`include_str!`) : une
seule source de vérité, aucun fichier à déployer.

Tables : `agents`, `mandates`, `actions`, `approvals`, `orders`, `wallets`,
`ledger`, `reserve`, `reserve_ledger`, `incidents`, `payouts`, `audit_log`, `meta`.

Conventions :
- sommes en `INTEGER` de centimes ;
- dates en `TEXT` RFC 3339 UTC ;
- listes et objets imbriqués en `TEXT` JSON ;
- identifiants en `TEXT` (UUID v4).

## Chaîne d'audit

Chaque événement est chaîné **et signé** :

```text
hash(n)      = sha256( hash(n-1) | seq | ts | kind | subject | payload )
signature(n) = Ed25519( hash(n) )        avec la clé d'audit du processus
```

- `seq` strictement croissant et unique ;
- `hash(0)` = `GENESIS` ;
- `verify` recalcule toute la chaîne depuis le début, puis vérifie la signature
  de **chaque** entrée contre la clé publique attendue ; il s'arrête sur la
  première incohérence — hash, chaînage, signature absente ou invalide — et
  renvoie `broken_at_seq` ;
- toute écriture métier et son entrée d'audit partagent la même transaction.

### Signature

| Élément | Détail |
| --- | --- |
| Algorithme | Ed25519 (`ed25519-dalek`, version épinglée `=2.1.1` pour rester sous MSRV 1.75) |
| Message signé | le `hash` de l'entrée, en hexadécimal |
| Clé privée | graine de 32 octets lue dans `AGENTSHIELD_AUDIT_SIGNING_KEY` (base64) |
| Clé publique | base64, colonne `pubkey` de chaque entrée, exposée par `GET /api/audit/pubkey` et `GET /api/health` |
| Colonnes | `signature` et `pubkey` (`TEXT`) ajoutées au schéma, et par `ALTER TABLE` idempotent pour les bases existantes |
| Journalisation | seule la clé **publique** est loggée, jamais la graine |

Sans `AGENTSHIELD_AUDIT_SIGNING_KEY`, une clé éphémère est générée au démarrage
(comme le mode `mock` de PayPal : tout reste utilisable sans configuration). Sa
clé publique est loggée et `key_source` vaut `ephemeral`. Conséquence assumée :
**les entrées écrites par un processus précédent ne sont plus vérifiables** —
`verify` les signale comme signées par une autre clé. Pour une démo qui redémarre,
ou pour un audit qui doit survivre au processus, définir une clé stable :

```bash
openssl rand -base64 32     # → AGENTSHIELD_AUDIT_SIGNING_KEY
```

Ce que la signature ferme : un accès en écriture à SQLite ne suffit plus à
réécrire la chaîne, même en la re-hachant intégralement (test
`rehashed_chain_still_fails_without_the_private_key`).

Ce qu'elle ne ferme pas : `verify` reste **ancré dans la base**. Supprimer les
dernières entrées (troncature) ou restaurer une base antérieure complète reste
indétectable par le processus ; il faudrait publier le `head_hash` hors de la
base (journal append-only, horodatage tiers, transparency log). C'est hors
périmètre du prototype et marqué `TODO(ancrage externe)` dans
`backend/src/store/audit.rs`. Les entrées antérieures à l'introduction de la
signature (`signature` à `NULL`) sont refusées par `verify` : une base de démo
créée avant cette version doit être recréée.

## Concurrence

Une connexion SQLite derrière un `Mutex` : les écritures sont sérialisées, donc
la chaîne d'audit ne peut pas entrelacer deux entrées. C'est volontairement
simple ; un pool + `BEGIN IMMEDIATE` serait l'étape suivante.

## Modes dégradés

| Situation | Comportement |
| --- | --- |
| Pas de clés PayPal | `MockGateway`, identifiants `MOCK-…`, tout le flux reste démontrable |
| Pas de clé OpenAI | verdict `deterministic-fallback`, même structure JSON |
| OpenAI en erreur / timeout | repli déterministe + `tracing::warn` |
| Pas de clé d'audit | clé Ed25519 éphémère, clé **publique** loggée ; les entrées d'un processus précédent ne sont plus vérifiables |
| Réserve insuffisante | sinistre en `under_review`, aucun payout |
| Escalade expirée | `409`, aucun checkout possible |

## Ce que le prototype ne fait pas

- Aucun argent réel, aucune couverture réelle.
- Aucune authentification : l'API suppose un environnement local de démo.
- Aucun multi-tenant.
- Aucun ancrage externe de l'audit (`TODO(ancrage externe)`) : la troncature et
  la restauration d'une base antérieure ne sont pas détectées.
- Aucun blocage d'un paiement qui ne passe pas par l'intégration AgentShield.
