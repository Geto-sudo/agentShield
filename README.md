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
projectnana/
│
├── main.py
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
└── backend/
    ├── Cargo.toml
    ├── Cargo.lock
    ├── .env.example
    ├── migrations/
    ├── src/
    ├── tests/
    └── README.md
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
git clone https://github.com/Geto-sudo/projectnana.git
cd projectnana
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

## Hackathon Project

**Event:** PayPal AI Hackathon 2026
**Project:** AgentShield
**Category:** AI-powered payment control and spending governance

> AgentShield gives AI agents room to act while keeping spending decisions under human-defined control.

---

*Built as a Sandbox prototype for the PayPal AI Hackathon 2026.*