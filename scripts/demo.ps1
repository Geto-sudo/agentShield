# AgentShield — rejoue le parcours complet contre un serveur déjà lancé.
#
# Usage :
#   .\scripts\demo.ps1
#   .\scripts\demo.ps1 -Base http://127.0.0.1:8080
#
# Le script ne crée rien de destructif : il crée un agent + un mandat dédiés,
# déroule les trois décisions, confirme une escalade, crée et capture un
# checkout Sandbox, puis vérifie la chaîne d'audit.

param([string]$Base = "http://127.0.0.1:8080")

$ErrorActionPreference = "Stop"
$api = "$Base/api"

function Step($text) { Write-Host "`n== $text" -ForegroundColor Cyan }
function Ok($text) { Write-Host "   OK  $text" -ForegroundColor Green }
function Info($text) { Write-Host "       $text" -ForegroundColor Gray }

$health = Invoke-RestMethod "$api/health"
Info "modes : paypal=$($health.modes.paypal) expert=$($health.modes.expert)"

# --- 1. Agent + mandat -----------------------------------------------------
Step "1. Mandat"
$agent = (Invoke-RestMethod "$api/agents" -Method Post -ContentType "application/json" -Body (@{
    name = "Demo Agent"; owner = "Camille Renard"; purpose = "démonstration"
} | ConvertTo-Json)).agent

$mandate = (Invoke-RestMethod "$api/mandates" -Method Post -ContentType "application/json" -Body (@{
    agent_id = $agent.id
    owner_name = "Camille Renard"
    intent = "Casque neuf, maximum 50 EUR, livraison sous 5 jours."
    category = "electronics"
    max_total_cents = 20000
    per_txn_max_cents = 5000
    auto_approve_max_cents = 4000
    escalate_until_cents = 5000
    allowed_countries = @("FR")
    blocked_merchants = @("RogueShop")
    max_delivery_days = 5
    valid_days = 30
} | ConvertTo-Json)).mandate

Ok "mandat $($mandate.id)"
Info "plafond $($mandate.max_total_cents) / par achat $($mandate.per_txn_max_cents) / auto $($mandate.auto_approve_max_cents)"

function Propose($amount) {
    Invoke-RestMethod "$api/actions" -Method Post -ContentType "application/json" -Body (@{
        mandate_id = $mandate.id
        merchant = "TechStore SAS"
        merchant_country = "FR"
        amount_cents = $amount
        currency = "EUR"
        category = "electronics"
        delivery_days = 4
        description = "Casque audio sans fil"
        idempotency_key = "demo-$amount-$([guid]::NewGuid())"
    } | ConvertTo-Json)
}

# --- 2. Trois décisions ----------------------------------------------------
Step "2. Décisions"

$refused = Propose 6200
Ok "62,00 EUR -> $($refused.decision) (risque $($refused.risk.score)/100)"
Info $refused.reasons[0]

$escalated = Propose 4500
Ok "45,00 EUR -> $($escalated.decision) (approbation $($escalated.approval.status))"

$approved = Propose 3800
Ok "38,00 EUR -> $($approved.decision) (risque $($approved.risk.score)/100)"

# --- 3. Le refus interdit le checkout --------------------------------------
Step "3. Un refus ne produit jamais de checkout"
try {
    Invoke-RestMethod "$api/actions/$($refused.action.id)/paypal/order" -Method Post -ContentType "application/json" -Body "{}" | Out-Null
    Write-Host "   ECHEC : un checkout a été créé pour une action refusée" -ForegroundColor Red
    exit 1
} catch {
    Ok "403 guardrail_denied"
}

# --- 4. Escalade : confirmation humaine ------------------------------------
Step "4. Escalade sans confirmation"
try {
    Invoke-RestMethod "$api/actions/$($escalated.action.id)/paypal/order" -Method Post -ContentType "application/json" -Body "{}" | Out-Null
    Write-Host "   ECHEC : checkout créé sans confirmation" -ForegroundColor Red
    exit 1
} catch {
    Ok "403 guardrail_denied"
}

Invoke-RestMethod "$api/approvals/$($escalated.approval.id)/confirm" -Method Post -ContentType "application/json" -Body (@{
    decided_by = "Camille Renard"; note = "confirmé en démo"
} | ConvertTo-Json) | Out-Null
Ok "escalade confirmée par le propriétaire"

# --- 5. Checkout + capture -------------------------------------------------
Step "5. Flux PayPal Sandbox"
$order = (Invoke-RestMethod "$api/actions/$($escalated.action.id)/paypal/order" -Method Post -ContentType "application/json" -Body "{}").order
Ok "commande $($order.paypal_order_id) ($($order.mode))"

$capture = Invoke-RestMethod "$api/paypal/orders/$($order.paypal_order_id)/capture" -Method Post -ContentType "application/json" -Body "{}"
Ok "capturée : solde $($capture.wallet.balance_cents) centimes"

# --- 6. Wallet -------------------------------------------------------------
Step "6. Wallet et prime"
$wallet = Invoke-RestMethod "$api/wallet/$($agent.id)"
Ok "solde $($wallet.wallet.balance_cents) centimes"
foreach ($line in $wallet.quote.lines) { Info "$($line.label) : $($line.amount_cents)" }
Info $wallet.quote.headline

# --- 7. Récupération -------------------------------------------------------
Step "7. Scénario « action rogue »"
$recovery = Invoke-RestMethod "$api/recovery/simulate" -Method Post -ContentType "application/json" -Body (@{ agent_id = $agent.id } | ConvertTo-Json)
Ok "verdict $($recovery.outcome.verdict.verdict) ($($recovery.outcome.verdict.source))"
Info $recovery.outcome.verdict.summary_fr
if ($recovery.outcome.payout) {
    Ok "payout $($recovery.outcome.payout.paypal_batch_id) : $($recovery.outcome.payout.amount_cents) centimes"
}
foreach ($s in $recovery.outcome.steps) { Info $s }

# --- 8. Preuve -------------------------------------------------------------
Step "8. Chaîne d'audit"
$chain = (Invoke-RestMethod "$api/audit/verify").chain
if ($chain.valid) { Ok $chain.message } else { Write-Host "   CHAINE ROMPUE : $($chain.message)" -ForegroundColor Red; exit 1 }

Write-Host "`nParcours complet terminé." -ForegroundColor Green
