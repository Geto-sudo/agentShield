# Décisions techniques

Courtes fiches de décision (ADR condensés) pour justifier les choix devant un
jury et lors d'une reprise du code.

---

## 1. Rust partout, pas de Python

**Décision** — le backend est écrit en Rust (axum + tokio + rusqlite).

**Pourquoi** — l'argument du produit est « la décision est déterministe et
auditable ». Un exécutable unique, typé, sans runtime interprété, rend cet
argument plus crédible et supprime toute une classe de bugs (sérialisation
implicite, arrondis, dépendances non épinglées).

**Conséquence** — pas de notebook pour explorer ; le coût d'entrée est plus élevé,
mais la reproductibilité est totale.

---

## 2. L'argent est un entier de centimes

**Décision** — toute somme est un `i64` de centimes. Aucun `f64` ne participe à
une décision ou à un calcul de solde.

**Pourquoi** — `0.1 + 0.2 != 0.3` ; sur des seuils (« 40,01 € appartient-il à la
tranche escalade ? ») un flottant rend la décision non reproductible.

**Conséquence** — conversion explicite vers la chaîne décimale PayPal
(`cents_to_paypal_amount`) et retour (`paypal_amount_to_cents`), testée.

---

## 3. Les règles font autorité, le LLM explique

**Décision** — l'issue (`APPROVE`/`ESCALATE`/`REFUSE`) est calculée par des règles
pures. Le LLM ne peut jamais changer un montant ni ouvrir une couverture.

**Pourquoi** — exigence directe du produit : un modèle qui approuve une dépense
n'est pas auditable, et un hallucination devient une perte d'argent.

**Conséquence** — `engine::expert::reconcile` ramène le verdict du modèle au
verdict déterministe quand le modèle veut indemniser au delà de ce que les règles
autorisent. Le montant du payout est calculé par le code, jamais par le modèle.

---

## 4. Une fonction de décision pure

**Décision** — `guardrail::evaluate(action, mandate, history, now)` ne fait aucune
I/O ; l'historique est un `HistoryWindow` fourni par l'appelant.

**Pourquoi** — une décision financière doit être rejouable : mêmes entrées,
même sortie, même explication. C'est testé par `decision_is_reproducible`.

**Conséquence** — le seeder et API passent par exactement le même chemin ; la
démo ne peut pas produire un état impossible.

---

## 5. Chaîne d'audit signée

**Décision** — chaque événement est chaîné puis signé :
`hash = sha256(prev | seq | ts | kind | subject | payload)`, puis
`signature = Ed25519(hash)` avec la clé d'audit du processus.

**Pourquoi** — « garder la preuve » n'a de valeur que si la preuve est
vérifiable. Une ligne modifiée casse la chaîne, et `GET /api/audit/verify` le
détecte et indique la position.

**Révision (v2)** — la chaîne seule ne suffisait pas : avec un accès en écriture
à SQLite, un attaquant recalculait `hash` et `prev_hash` de bout en bout et
`verify` répondait « intacte ». La signature Ed25519 ferme ce cas : sans la graine
privée, une chaîne re-hachée est refusée. La clé vient de
`AGENTSHIELD_AUDIT_SIGNING_KEY` (base64, 32 octets) ; sans elle, une clé
éphémère est générée au démarrage et seule sa clé **publique** est loggée —
même esprit que le mode `mock` de PayPal, au prix d'entrées non vérifiables
après un redémarrage.

**Ce qui reste hors périmètre** — l'ancrage externe : `verify` reste ancré dans
la base, donc la troncature de la fin de chaîne et la restauration d'une base
antérieure ne sont pas détectées. Il faudrait publier le `head_hash` hors de la
base (journal append-only, horodatage tiers). `TODO(ancrage externe)` dans
`backend/src/store/audit.rs`.

**Conséquence** — l'écriture métier et l'entrée d'audit partagent une
transaction ; une seule connexion sérialisée garantit l'ordre, et la signature
est calculée dans la même transaction que l'insertion.

---

## 6. Refuser, c'est enregistrer

**Décision** — une action refusée est persistée avec ses règles violées.

**Pourquoi** — la valeur pour l'utilisateur est de pouvoir répondre à « pourquoi
mon agent a-t-il été bloqué ? ». Un refus non tracé est invisible.

**Conséquence** — le tableau de bord distingue approuvé / escaladé / refusé, et
le budget consommé n'inclut que `APPROVE` + `ESCALATE`.

---

## 7. L'escalade expire et n'est jamais implicite

**Décision** — une escalade est `pending` pendant `AGENTSHIELD_APPROVAL_TTL_MINUTES`
(30 par défaut). Passé ce délai elle devient `expired` et aucun checkout n'est
possible. Il n'existe aucun chemin « approuvé par défaut ».

**Pourquoi** — une approbation tacite est un trou de sécurité.

**Conséquence** — `GET /api/approvals` purge à la lecture ; `confirm` renvoie
`409` sur une escalade expirée.

---

## 8. Deux passerelles PayPal derrière un trait

**Décision** — `PayPalGateway` a deux implémentations : `SandboxGateway`
(vrais appels OAuth + Orders + Payouts) et `MockGateway` (identifiants
`MOCK-…`).

**Pourquoi** — la démo doit fonctionner sans identifiants, sans réseau, et en CI.
Le mode réel s'active uniquement par présence des clés.

**Conséquence** — `mode` (`mock` / `live-sandbox`) est renvoyé dans chaque
réponse : l'interface ne peut pas mentir sur ce qui s'est passé.

---

## 9. SQLite, une connexion sérialisée

**Décision** — `rusqlite` avec une connexion derrière un `Mutex`.

**Pourquoi** — la charge est celle d'une démo ; la simplicité garantit la
cohérence de la chaîne d'audit et supprime tout risque de verrou SQLite.

**Conséquence** — le débit d'écriture est limité (quelques centaines d'actions/s
au mieux). À remplacer par un pool + `BEGIN IMMEDIATE` si besoin.

---

## 10. Schéma compilé dans le binaire

**Décision** — `include_str!("../../migrations/0001_init.sql")`.

**Pourquoi** — un binaire + un fichier de base suffisent ; pas de chemin de
migration à résoudre au runtime, pas de dérive entre le code et le SQL.

**Conséquence** — faire évoluer le schéma demandera un vrai runner de migrations.
Pour un prototype, `CREATE TABLE IF NOT EXISTS` couvre le besoin.

---

## 11. API-only, front séparé

**Décision** — aucune page n'est servie par le backend ; CORS permissif en dev.

**Pourquoi** — le front est développé par une autre personne, sur son propre
outillage. Coupler les deux obligerait à reconstruire le binaire Rust à chaque
changement d'interface.

**Conséquence** — `docs/API.md` et `docs/openapi.yaml` sont le contrat. Ils
doivent rester synchronisés avec le code.

---

## 12. Sans authentification

**Décision** — pas de clé d'API, pas de session.

**Pourquoi** — prototype local du hackathon ; l'ajout d'une authentification
n'apporte aucune démonstration produit et retarderait la sortie.

**Conséquence** — à ne pas exposer sur un réseau ouvert. C'est documenté.
