# AgentShield — Technical Handoff

## 1. Project Overview

AgentShield is a prototype Sandbox designed to control and audit spending actions proposed by AI agents.

A human defines a spending mandate that establishes the permitted scope and limits. Before a proposed action proceeds through the payment flow, AgentShield evaluates it and returns one of three decisions:

- **APPROVE** — The action complies with the mandate.
- **ESCALATE** — Human confirmation is required.
- **REFUSE** — The action must not proceed.

This is a prototype. It does not provide real insurance coverage, does not control payments outside an AgentShield integration, and must not be presented as a production financial-control system.

## 2. Architecture

The project is organized into two main components:

- **Frontend:** Python, Streamlit and Pandas.
- **Backend:** Rust and Axum.
- **Persistence:** SQLite.
- **Communication:** HTTP API between frontend and backend.

The frontend must communicate with the backend through the API. It must not access the SQLite database directly.

The decision engine uses deterministic rules. An LLM may assist with dispute expertise, but it does not determine payout amounts.

The backend separates API handling, services, domain logic, decision engine, storage and PayPal gateway responsibilities.

## 3. Main Workflow

1. The user defines a spending mandate.
2. An AI agent proposes an action.
3. The backend evaluates the action against the mandate.
4. The decision is recorded.
5. An approved action may continue to the PayPal order flow.
6. An escalated action requires human confirmation before checkout.
7. A refused action must not proceed to checkout.
8. Relevant events are recorded in the audit trail.

Never use real funds for a demonstration. Do not trigger order creation, approval, capture or other payment operations during routine verification.

## 4. Important Technical References

Consult these project documents before changing behavior:

- `docs/API.md` — HTTP endpoints and payload contracts.
- `docs/ARCHITECTURE.md` — Component boundaries and data flow.
- `docs/DECISIONS.md` — Recorded architectural decisions.
- `docs/DEMO.md` — Demonstration procedure and safety notes.
- `backend/README.md` — Backend setup and implementation details.

## 5. Local Development Environment

The project has been developed on Kubuntu Linux.

Main technologies:

- Python 3.14
- Streamlit
- Pandas
- Rust
- Axum
- SQLite

The backend API is expected at:

`http://127.0.0.1:8080`

Health endpoint:

`GET /api/health`

Frontend startup:

```bash
cd "/home/dionysos/Desktop/Paypal 2026/projectnana"
source .venv/bin/activate
streamlit run main.py
```

Backend startup:

```bash
cd "/home/dionysos/Desktop/Paypal 2026/projectnana/backend"
cargo run
```

## 6. Verification

The project includes:

- `scripts/check.sh` — Main local verification script.
- `scripts/pre-commit` — Pre-commit verification wrapper.
- `.github/workflows/checks.yml` — GitHub Actions workflow.

The verification script is intended to check:

1. Rust formatting.
2. Rust Clippy warnings.
3. Rust tests.
4. Rust release build.
5. Python syntax compilation.
6. Frontend dependency imports.
7. Git whitespace and patch integrity.

The checks must not reset or modify the local database.

Do not run `scripts/demo.ps1` or `scripts/reset-db.ps1` as part of routine verification.

## 7. Git and Collaboration Safety

Before making changes:

```bash
git status --short
```

Review existing modifications before editing files.

Do not overwrite another contributor's work.

Do not commit or push without explicit agreement.

The current working branch at the time of this handoff is:

`integration-backend`

## 8. Current Project Constraints

- The SQLite database contains local project data and must be preserved.
- Environment files and credentials must never be exposed.
- No real PayPal transaction should be initiated during routine development.
- The frontend should remain independent of the database implementation.
- The backend API is the single communication boundary between frontend and backend.
- Demo data must be clearly distinguished from real data.

## 9. Current Verification Status

The backend health endpoint has previously returned a successful response.

Historical test results reported during development were:

- 28 unit tests passed.
- 9 HTTP integration tests passed.
- 37 tests in total.

These are historical results, not a claim that the current working tree has just passed a fresh verification run.

The new verification script, CI workflow and local pre-commit hook have been created, but their complete execution still needs to be confirmed.

## 10. Next Steps

1. Inspect the verification script and workflow.
2. Run the verification script and review any failures.
3. Confirm the CI workflow configuration.
4. Review the frontend/backend API integration.
5. Identify remaining integration gaps.
6. Prepare a safe, reproducible demonstration.

The priority is to preserve existing work, maintain a clear API boundary, and ensure that the prototype behaves consistently and safely.