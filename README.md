# 🛡️ AgentShield

### Giving AI agents the freedom to act — without giving up human control.

**AgentShield is a Sandbox prototype for controlling and auditing AI agent spending.**

Built for the **PayPal AI Hackathon 2026**.

---

## The Problem

AI agents are becoming increasingly capable of performing tasks autonomously. As they interact with online services, they may also need to make purchases and initiate payments.

But giving an AI agent the ability to spend money raises an important question:

**How can users allow AI agents to make payments without losing control over their spending?**

Traditional payment systems are designed around human-initiated transactions. Autonomous agents introduce a different challenge: every proposed payment must be checked against the user's intentions, limits, and permissions before it proceeds.

---

## Our Solution

AgentShield introduces a spending-control layer between an AI agent and the PayPal payment flow.

Before a proposed payment can proceed through the AgentShield integration, the system checks it against a spending mandate defined by the human owner.

Based on the mandate and the configured thresholds, AgentShield produces one of three decisions:

| Decision | Meaning |
|---|---|
| 🟢 **APPROVE** | The proposed action satisfies the rules and can proceed. |
| 🟠 **ESCALATE** | The action requires explicit human approval before proceeding. |
| 🔴 **REFUSE** | The action violates a strict rule or exceeds the permitted limits. |

Every decision is recorded, creating an auditable history of the agent's proposed spending activity.

**The principle is simple: AI agents can act autonomously, but their spending remains subject to human-defined rules.**

---

## How It Works

The AgentShield workflow follows these steps:

1. **Define a mandate** — The human owner configures the agent's spending permissions and limits.
2. **Propose an expense** — An AI agent submits a payment proposal through the AgentShield backend.
3. **Evaluate the action** — The decision engine checks the proposal against the mandate and its strict rules.
4. **Make a decision** — AgentShield approves, escalates, or refuses the proposed action.
5. **Request human approval when necessary** — Escalated actions wait for an explicit human decision.
6. **Proceed through PayPal Sandbox** — Approved actions can continue through the integrated PayPal Sandbox payment flow.
7. **Record the evidence** — The action and its decision are stored in the audit history.

The decision engine is deterministic for its strict rules and spending thresholds. The AI expert can provide additional structured analysis, but it never determines payment amounts or overrides strict spending rules.

---

## Application Features

AgentShield includes six main frontend screens.

### 1. Dashboard

A central overview of the system, with key activity indicators and recent agent actions.

### 2. Spending Mandates

Create and manage spending mandates that define what an agent is allowed to do.

Mandates can include spending limits, categories, merchant restrictions, and other conditions.

### 3. Propose an Expense

Submit a proposed expense and receive an AgentShield decision based on the applicable mandate.

### 4. Human Approvals

Review actions that require explicit human approval before they can proceed.

### 5. Activity History

Consult the recorded actions and their decisions to understand what agents have proposed and what happened.

### 6. Simulated Wallet

View the internal AgentShield wallet information, including:

- Available balance
- Spending during the current period
- Premium
- Bonus-malus adjustment
- Shared reserve contribution
- Transaction history

The wallet and reserve are part of the prototype's simulated financial model. They are distinct from a user's real PayPal balance.

---

## Architecture

AgentShield uses a Python frontend connected to a Rust backend through a JSON REST API.

```text
                 HUMAN OWNER
                      |
                      v
             STREAMLIT FRONTEND
                      |
        +-------------+-------------+
        |             |             |
        v             v             v
     Dashboard     Mandates     Approvals
        |             |             |
        +-------------+-------------+
                      |
                      v
              PYTHON API CLIENT
                      |
                HTTP / JSON
                      |
                      v
                RUST BACKEND
                      |
        +-------------+-------------+
        |             |             |
        v             v             v
   Decision       SQLite       Audit Chain
    Engine        Storage       SHA-256
        |
        v
   PAYPAL SANDBOX
   Orders / Capture
```

### Technology Stack

| Layer | Technology |
|---|---|
| Frontend | Python, Streamlit |
| Data handling | Pandas |
| Backend | Rust |
| Backend framework | Axum |
| API communication | HTTP / JSON |
| Database | SQLite |
| Payment integration | PayPal Sandbox |
| AI expert | OpenAI API with deterministic fallback |
| Audit integrity | SHA-256 hash chain |

---

## Project Structure

```text
agentShield/
│
├── main.py                 # Streamlit entry point
│
├── styles/
│   └── theme.py
│
├── components/
│   ├── sidebar.py
│   ├── header.py
│   ├── metric_card.py
│   ├── status_badge.py
│   └── data_table.py
│
├── app_pages/
│   ├── dashboard.py
│   ├── mandates.py
│   ├── propose_expense.py
│   ├── approvals.py
│   ├── history.py
│   └── wallet.py
│
├── data/
│   ├── backend_client.py
│   ├── data_source.py
│   └── demo_data.py
│
├── backend/                # Rust (Axum) API, decision engine, SQLite, audit chain
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── .env.example
│   ├── migrations/
│   ├── src/
│   ├── tests/
│   └── README.md
│
├── docs/                   # API reference, OpenAPI spec, architecture, decisions, demo script
├── samples/                # Example JSON payloads for every API scenario
├── scripts/                # check.sh (CI), PowerShell run / demo / verify / reset helpers
└── .github/workflows/      # CI: Rust fmt, clippy, tests, release build, Python checks
```

---

## Getting Started

### Prerequisites

Install the following tools:

- Python
- Rust with the stable toolchain
- Git

### 1. Clone the Repository

```bash
git clone https://github.com/Geto-sudo/agentShield.git
cd agentShield
```

### 2. Set Up the Python Environment

```bash
python3 -m venv .venv
source .venv/bin/activate
```

Install the frontend dependencies:

```bash
pip install streamlit pandas
```

### 3. Start the Rust Backend

Open a terminal:

```bash
cd backend
```

If you do not already have a local `.env` file, create one from the example:

```bash
[ -f .env ] || cp .env.example .env
```

Start the backend:

```bash
cargo run
```

The backend listens on:

```text
http://127.0.0.1:8080
```

Check its health:

```bash
curl http://127.0.0.1:8080/api/health
```

### 4. Start the Streamlit Frontend

Open a second terminal from the project root:

```bash
source .venv/bin/activate
streamlit run main.py
```

The frontend will display the local Streamlit application address in the terminal.

**Important:** The backend must be running before the frontend can retrieve live application data.

### 5. Try the API (optional)

With the backend running, the seeded data already contains a mandate for *Atlas Shopper*: "New headphones, maximum 50 EUR, delivery within 5 days". These commands submit three purchases and show the three possible decisions:

```bash
MANDATE_ID=$(curl -s http://127.0.0.1:8080/api/mandates | python3 -c "import sys,json; print(next(m['id'] for m in json.load(sys.stdin)['mandates'] if m['max_total_cents'] == 20000))")

for AMOUNT in 3800 4500 6200; do
  curl -s -X POST http://127.0.0.1:8080/api/actions \
    -H "Content-Type: application/json" \
    -d "{\"mandate_id\":\"$MANDATE_ID\",\"merchant\":\"TechStore SAS\",\"merchant_country\":\"FR\",\"amount_cents\":$AMOUNT,\"currency\":\"EUR\",\"category\":\"electronics\",\"delivery_days\":4,\"description\":\"Wireless headphones\",\"kind\":\"purchase\"}" \
    | python3 -c "import sys,json; r=json.load(sys.stdin); print(r['decision'], '-', r['reasons'][0])"
done
```

Expected output:

```text
APPROVE - All strict mandate rules are satisfied, and 38,00 EUR is below the automatic approval threshold of 40,00 EUR.
ESCALATE - Amount between 40,00 EUR and 50,00 EUR: the action requires explicit owner confirmation.
REFUSE - Budget exceeded: 62,00 EUR requested against a mandate limit of 50,00 EUR.
```

Verify the audit chain:

```bash
curl -s http://127.0.0.1:8080/api/audit/verify
```

More payloads are available in [`samples/`](samples/), and a full walkthrough is in [`docs/DEMO.md`](docs/DEMO.md).

### Windows (PowerShell)

Helper scripts are provided in [`scripts/`](scripts/):

| Script | Purpose |
|---|---|
| `scripts/run.ps1` | Start the backend (`-Release` for an optimized build) |
| `scripts/demo.ps1` | Replay the full scenario against a running server: three decisions, a confirmed escalation, a Sandbox checkout and an audit verification |
| `scripts/verify.ps1` | Build, run the tests and the end-to-end demo in one go |
| `scripts/reset-db.ps1` | Delete the local SQLite database (`backend/data`); the demo data is re-seeded on the next start |

---

## Running the Checks

The same checks run in CI on every push and pull request:

```bash
bash scripts/check.sh
```

It verifies Rust formatting (`cargo fmt`), lints (`cargo clippy -D warnings`), runs the Rust test suite, builds the release binary, byte-compiles the Python frontend and checks that Streamlit and Pandas are installed.

---

## PayPal Sandbox

AgentShield integrates with the PayPal Sandbox payment flow.

The backend supports:

- PayPal Orders creation
- PayPal Orders capture
- PayPal Payouts
- A simulated mock mode when external credentials are not configured

The PayPal Sandbox integration is intended for demonstration and testing.

**No real money is involved in the hackathon prototype.**

For detailed backend configuration and PayPal integration instructions, see [`backend/README.md`](backend/README.md).

---

## Security and Auditability

AgentShield is designed around explicit spending permissions and traceable decisions.

Its backend includes:

- Human-defined spending mandates
- Strict deterministic spending rules
- Explicit human approval for escalated actions
- Idempotency support for action submissions
- Persistent action history
- An append-only SHA-256 audit chain
- An audit verification endpoint

The AI expert is not responsible for setting payment amounts or overriding the strict decision rules.

---

## Prototype Scope and Limitations

AgentShield is a **Sandbox prototype**, not a production financial service.

The following elements are simulated:

- AI agent identities and activity
- Internal wallet balances
- Claims and reimbursements
- Shared reserve mechanisms

PayPal operations use the Sandbox environment or the simulated mock gateway.

The prototype does not provide regulated insurance coverage, does not handle real customer funds, and does not control payments made outside the AgentShield + PayPal integration.

---

## Backend Documentation

For the complete technical documentation of the Rust backend, including API routes, decision rules, environment variables, and test instructions, see:

**[Backend README](backend/README.md)**

---

## Repository Branches

| Branch | Content |
|---|---|
| `main` | The complete, integrated application: Streamlit frontend, Rust backend, PayPal Sandbox integration, docs and CI. |
| `integration-backend` | The branch `main` was taken from. |
| `backend` | Backend-only line of work. It also contains **Ed25519 signing of every audit entry**, which is not merged into `main` yet: merging it requires resolving conflicts in the backend. |

---

## Hackathon Project

**Event:** PayPal AI Hackathon 2026
**Project:** AgentShield
**Category:** AI-powered payment control and spending governance

> AgentShield gives AI agents room to act while keeping spending decisions under human-defined control.

---

*Built as a Sandbox prototype for the PayPal AI Hackathon 2026.*