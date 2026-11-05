# Script de démonstration (2 minutes)

Objectif : montrer **un mandat, une action, une décision claire**, puis la preuve
et le parcours de récupération.

## Préparation

```powershell
cd backend
$env:PAYPAL_FORCE_MOCK = "true"   # mode simulé : aucun compte requis
cargo run
```

La base est créée au premier lancement, avec 3 agents (dont un sinistre ouvert).

---

## 0:00 — Le mandat

```powershell
$m = (Invoke-RestMethod http://127.0.0.1:8080/api/mandates).mandates |
     Where-Object { $_.max_total_cents -eq 20000 } | Select-Object -First 1
$m | Select-Object intent, max_total_cents, per_txn_max_cents, auto_approve_max_cents, escalate_until_cents
```

> « Camille a fixé : casque neuf, 50 € maximum par achat, livraison sous 5 jours.
> En dessous de 40 €, l'agent peut payer seul. »

---

## 0:25 — L'agent propose 62 €

```powershell
$body = @{
  mandate_id = $m.id; merchant = "TechStore SAS"; merchant_country = "FR"
  amount_cents = 6200; currency = "EUR"; category = "electronics"
  delivery_days = 4; description = "Casque audio sans fil"
} | ConvertTo-Json

$r = Invoke-RestMethod http://127.0.0.1:8080/api/actions -Method Post -Body $body -ContentType "application/json"
$r.decision; $r.reasons; "preuve #$($r.audit.seq) $($r.audit.hash)"
```

> « REFUSE. Budget dépassé. L'explication est générée par les règles, le hash
> est la preuve. Aucun checkout PayPal ne peut être créé. »

Vérification :

```powershell
Invoke-RestMethod "http://127.0.0.1:8080/api/actions/$($r.action.id)/paypal/order" -Method Post -Body "{}" -ContentType "application/json"
# -> 403 guardrail_denied
```

---

## 0:50 — 45 € déclenche une escalade

```powershell
$body2 = $body | ConvertFrom-Json
$body2.amount_cents = 4500
$body2 = $body2 | ConvertTo-Json

$e = Invoke-RestMethod http://127.0.0.1:8080/api/actions -Method Post -Body $body2 -ContentType "application/json"
$e.decision                     # ESCALATE
$e.approval.status              # pending
```

> « Rien ne se passe tout seul : quelqu'un doit confirmer. Et l'approbation
> expire. »

```powershell
# Sans confirmation -> refus
Invoke-RestMethod "http://127.0.0.1:8080/api/actions/$($e.action.id)/paypal/order" -Method Post -Body "{}" -ContentType "application/json"
# -> 403

# Confirmation humaine
Invoke-RestMethod "http://127.0.0.1:8080/api/approvals/$($e.approval.id)/confirm" -Method Post -Body '{"decided_by":"Camille"}' -ContentType "application/json"
```

---

## 1:15 — Le flux PayPal

```powershell
$o = Invoke-RestMethod "http://127.0.0.1:8080/api/actions/$($e.action.id)/paypal/order" -Method Post -Body "{}" -ContentType "application/json"
$o.order.paypal_order_id
$o.order.approve_url

$c = Invoke-RestMethod "http://127.0.0.1:8080/api/paypal/orders/$($o.order.paypal_order_id)/capture" -Method Post -Body "{}" -ContentType "application/json"
$c.wallet.balance_cents
```

> « Commande créée en Sandbox, capturée, wallet simulé débité. En mode live, le
> même code appelle `api-m.sandbox.paypal.com`. »

---

## 1:30 — Le wallet expliqué

```powershell
$w = Invoke-RestMethod "http://127.0.0.1:8080/api/wallet/$($m.agent_id)"
$w.wallet.balance_cents
$w.quote.lines | Select-Object label, amount_cents, explanation
$w.quote.headline
```

> « La prime n'est pas un chiffre magique : base, chargement risque, bonus-malus,
> contribution à la réserve commune. »

---

## 1:45 — La récupération

```powershell
$rec = Invoke-RestMethod http://127.0.0.1:8080/api/recovery/simulate -Method Post -Body "{}" -ContentType "application/json"
$rec.outcome.verdict.verdict
$rec.outcome.verdict.summary_fr
$rec.outcome.payout.amount_cents
$rec.outcome.steps
```

> « Une dépense a contourné le garde-fou. L'expert compare mandat et logs, rend un
> verdict structuré, et la réserve commune émet un payout Sandbox de l'écart
> exact — calculé par le code, pas par le modèle. »

---

## 1:55 — La preuve

```powershell
(Invoke-RestMethod http://127.0.0.1:8080/api/audit/verify).chain
(Invoke-RestMethod "http://127.0.0.1:8080/api/audit?limit=5").entries |
  Select-Object seq, kind, subject
```

> « Chaque décision, chaque commande, chaque payout est chaîné par hash.
> Modifiez une ligne en base : la vérification casse et indique où. »

---

## Réponses aux questions probables

| Question | Réponse |
| --- | --- |
| Où est le LLM ? | `engine::expert`, utilisé pour qualifier un litige. Il ne décide jamais d'un montant. |
| Et si OpenAI tombe ? | Repli déterministe, même schéma de réponse, `source` change. |
| Comment empêchez-vous un agent de contourner ? | Le checkout n'existe que si `decision != REFUSE` **et** l'approbation est confirmée. Un agent ne peut pas forger une approbation : c'est un autre endpoint, avec expiration. |
| C'est de l'assurance ? | Non. Prototype Sandbox, remboursements simulés, aucune couverture réelle. |
| Et la concurrence ? | Une connexion sérialisée ; la chaîne d'audit ne peut pas s'entrelacer. |
| Pourquoi pas de front ici ? | Il est développé séparément ; le contrat est `docs/API.md` + `docs/openapi.yaml`. |
