# AgentShield — Rust Backend

**PayPal spending guardrail for AI agents.**
**Sandbox prototype** for the *PayPal AI Hackathon 2026*.

> Before an AI agent creates a PayPal payment, AgentShield checks a human-defined spending mandate, records evidence of the decision, and approves, escalates, or refuses the action.

This directory contains **the backend only**: a JSON API written in Rust. The frontend is developed separately and consumes the `/api/*` endpoints. CORS is open during development.

---

## Implemented Features

| Component | Status | Description |
|---|---|---|
| Spending mandates | ✅ | Total spending limit, per-transaction limit, automatic approval and escalation thresholds, categories, allowed and blocked merchants, countries, delivery time limits, and validity period |
| Action JSON log | ✅ | Typed schema (`POST /api/actions`) and idempotency key |
| Decision engine | ✅ | `APPROVE` / `ESCALATE` / `REFUSE`, 13 strict rules, risk score from 0 to 100, and explanations in French |
| Human escalation | ✅ | Explicit human approval, expiration, and refusal that cannot be bypassed |
| Audit evidence | ✅ | Append-only SHA-256 audit chain and verification endpoint |
| PayPal Sandbox | ✅ | PayPal Orders creation and capture, Payouts, and automatic mock mode when credentials are unavailable |
| Wallet | ✅ | Balance deducted upon capture, transaction ledger, premium, bonus-malus adjustment, and shared reserve |
| Recovery | ✅ | Rogue action scenario, structured expert verdict, and capped Sandbox payout |
| AI expert | ✅ | OpenAI (`gpt-4o-mini` by default) with deterministic fallback; the LLM never determines payment amounts |
| SQLite | ✅ | Embedded SQL schema and a single source of truth |
| Tests | ✅ | 28 unit tests and 9 HTTP integration tests |

**Known limitation:** Agents, balances, claims, and reimbursements are **simulated**. No real money is involved, and no real insurance coverage is provided.

---

## Getting Started

**Prerequisite:** Rust with the `stable` toolchain. No additional system dependencies are required.

```bash
cd backend
cp .env.example .env      # Optional: the backend works without external API keys
cargo run
```

Expected startup information in mock mode:

```text
AgentShield — http://127.0.0.1:8080
SQLite      : data/agentshield.db
PayPal      : mock
Expert      : deterministic-fallback
```

On the first launch, a demonstration database is created with 3 agents, 3 mandates, 1 open claim, and sample history.

- `GET /` returns the list of available routes.
- `GET /api/health` returns the service health and active integration modes.

### Quick health check

```bash
curl http://127.0.0.1:8080/api/health
curl http://127.0.0.1:8080/api/mandates
```

> **Windows:** `../scripts/run.ps1` performs the equivalent startup process with a clean environment.

---

## The Workflow in a Single Request

Submit a proposed action to the backend:

```bash
curl -X POST http://127.0.0.1:8080/api/actions \
  -H "Content-Type: application/json" \
  -d '{
    "mandate_id": "<mandate-id>",
    "merchant": "TechStore SAS",
    "merchant_country": "FR",
    "amount_cents": 6200,
    "currency": "EUR",
    "category": "electronics",
    "delivery_days": 4,
    "description": "Wireless headphones"
  }'
```

Example response (excerpt):

```json
{
  "decision": "REFUSE",
  "risk": { "score": 63, "band": "high" },
  "reasons": [
    "Budget exceeded: EUR 62.00 requested for a EUR 50.00 mandate limit."
  ],
  "explanation": "A purchase of EUR 62.00 at TechStore SAS was proposed by the agent under mandate ...",
  "audit": { "seq": 11, "hash": "d92a40b6..." },
  "next_steps": [
    "No PayPal checkout is created.",
    "..."
  ]
}
```

With an amount of `3800` cents, the example produces `APPROVE`. With `4500` cents, it produces `ESCALATE` and creates a human approval request.

---

## Decision Policy

The decision process is **fully deterministic**.

### 1. Strict rules

If even one strict rule fails, the action is refused (`REFUSE`). There are no exceptions and no LLM override.

The rules are:

- `MANDATE_ACTIVE`
- `MANDATE_PERIOD`
- `ACTION_KIND`
- `CURRENCY_MATCH`
- `AMOUNT_POSITIVE`
- `AMOUNT_PER_TXN`
- `AMOUNT_TOTAL_BUDGET`
- `CATEGORY_ALLOWED`
- `MERCHANT_NOT_BLOCKED`
- `MERCHANT_ALLOWED`
- `COUNTRY_ALLOWED`
- `DELIVERY_WINDOW`
- `IDEMPOTENCY`

### 2. Spending thresholds

If all strict rules pass, the amount determines the next decision:

- `amount ≤ auto_approve_max_cents` → `APPROVE`
- `auto_approve_max_cents < amount ≤ escalate_until_cents` → `ESCALATE`
- `amount > escalate_until_cents` → `REFUSE` as a safety fallback

The risk score never changes the decision outcome. It provides additional information, helps prioritize actions, and contributes to premium calculations.

---

## Configuration

All configuration variables are optional. Without external API keys, the backend can run in a fully simulated mode.

| Variable | Default | Purpose |
|---|---|---|
| `AGENTSHIELD_ADDR` | `127.0.0.1:8080` | Listening address |
| `AGENTSHIELD_DB` | `data/agentshield.db` | SQLite database path (`:memory:` is also supported) |
| `AGENTSHIELD_SEED` | `true` | Load demonstration data on first launch |
| `AGENTSHIELD_AUTO_APPROVE_MAX_CENTS` | `4000` | Default automatic approval threshold (EUR 40) |
| `AGENTSHIELD_APPROVAL_TTL_MINUTES` | `30` | Human approval request expiration time |
| `AGENTSHIELD_PUBLIC_URL` | `http://127.0.0.1:8080` | Checkout return URLs |
| `AGENTSHIELD_PAYOUT_EMAIL` | `sandbox-owner@example.com` | Recipient for demonstration payouts |
| `PAYPAL_CLIENT_ID` / `PAYPAL_CLIENT_SECRET` | — | Enables the PayPal Sandbox integration |
| `PAYPAL_BASE_URL` | `https://api-m.sandbox.paypal.com` | PayPal API base URL |
| `PAYPAL_FORCE_MOCK` | `false` | Forces simulated PayPal mode |
| `OPENAI_API_KEY` | — | Enables the real AI expert |
| `OPENAI_MODEL` | `gpt-4o-mini` | AI model |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | API base URL; compatible with Azure or a proxy |

`GET /api/health` exposes the effective integration modes:

- PayPal: `mock` or `live-sandbox`
- Expert: `deterministic-fallback` or `openai`

---

## Documentation

The backend documentation is available in the following files:

- [`docs/API.md`](../docs/API.md) — Complete API route reference with examples.
- [`docs/openapi.yaml`](../docs/openapi.yaml) — OpenAPI 3.1 specification.
- [`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) — Components, workflows, and guarantees.
- [`docs/DECISIONS.md`](../docs/DECISIONS.md) — Technical decisions and their rationale.
- [`docs/DEMO.md`](../docs/DEMO.md) — Two-minute demonstration script.
- [`samples/`](../samples) — Ready-to-use JSON payloads.

---

## Tests

Run the backend tests from the `backend` directory:

```bash
cd backend
cargo test
cargo clippy --all-targets
```

The integration tests use an in-memory SQLite database and the simulated PayPal gateway. They do not require a real PayPal account or external network access.

---

## Recommended Project Description

> "AgentShield is a Sandbox prototype for controlling and auditing AI agent spending."

Do not describe AgentShield as regulated insurance, claim that it protects real users, or suggest that it blocks payments outside the AgentShield + PayPal integration.