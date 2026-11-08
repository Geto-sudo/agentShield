"""Data preparation layer for the AgentShield Streamlit application."""

from datetime import datetime
from typing import Any

from data.backend_client import (
    create_mandate,
    get_actions,
    get_agents,
    get_mandates,
)

from data.demo_data import (
    DEMO_INITIAL_BALANCE,
    DEMO_WALLET_TRANSACTIONS,
)


# =========================================================
# 1. AGENT DATA
# =========================================================

def get_agents_count() -> int:
    """Return the number of agents registered in the backend."""

    return len(get_agents())


def get_agents_for_form() -> list[dict[str, str]]:
    """Return agents formatted for selection forms."""

    agents = get_agents()

    return [
        {
            "id": agent["agent"]["id"],
            "name": agent["agent"]["name"],
        }
        for agent in agents
    ]


def _get_agent_names() -> dict[str, str]:
    """Build a mapping between agent IDs and agent names."""

    agents = get_agents()

    return {
        agent["agent"]["id"]: agent["agent"]["name"]
        for agent in agents
    }


# =========================================================
# 2. ACTION DATA
# =========================================================

DECISION_LABELS = {
    "APPROVE": "Approved",
    "ESCALATE": "Escalated",
    "REFUSE": "Rejected",
}


def _format_action_date(raw_date: str | None) -> str:
    """Convert a backend timestamp into a readable date."""

    if not raw_date:
        return "Unknown date"

    try:
        parsed_date = datetime.fromisoformat(
            raw_date.replace("Z", "+00:00")
        )

        return parsed_date.strftime("%Y-%m-%d %H:%M")

    except ValueError:
        return "Unknown date"


def get_initial_actions() -> list[dict[str, Any]]:
    """Retrieve backend actions and format them for the interface."""

    backend_actions = get_actions()
    agent_names = _get_agent_names()

    interface_actions = []

    for action in backend_actions:
        agent_id = action["agent_id"]
        decision = action["decision"]

        description = action.get("description") or (
            f"{action.get('kind', 'Action')} "
            f"at {action.get('merchant', 'Unknown merchant')}"
        )

        interface_actions.append(
            {
                "Agent": agent_names.get(agent_id, "Unknown agent"),
                "Action": description,
                "Amount (€)": action["amount_cents"] / 100,
                "Decision": DECISION_LABELS.get(decision, decision),
                "Reason": action.get("explanation", ""),
                "Date": _format_action_date(
                    action.get("created_at")
                ),
            }
        )

    return interface_actions


# =========================================================
# 3. SPENDING MANDATES
# =========================================================

INTERFACE_CATEGORIES = {
    "Office supplies": "office",
    "Computer equipment": "electronics",
    "Software subscriptions": "subscription",
    "Transportation": "transport",
    "Accommodation": "accommodation",
    "Other": "other",
}

BACKEND_CATEGORIES = {
    backend_value: interface_label
    for interface_label, backend_value
    in INTERFACE_CATEGORIES.items()
}


def get_categories() -> list[str]:
    """Return the categories available in the mandate form."""

    return list(INTERFACE_CATEGORIES.keys())


def get_initial_mandates() -> list[dict[str, Any]]:
    """Retrieve backend mandates and format them for the interface."""

    backend_mandates = get_mandates()
    agent_names = _get_agent_names()

    interface_mandates = []

    for mandate in backend_mandates:
        backend_category = mandate.get("category", "other")
        backend_status = mandate.get("status", "unknown")

        interface_mandates.append(
            {
                "Agent": agent_names.get(
                    mandate["agent_id"],
                    "Unknown agent",
                ),
                "Category": BACKEND_CATEGORIES.get(
                    backend_category,
                    backend_category,
                ),
                "Spending limit (€)": mandate["max_total_cents"] / 100,
                "Status": (
                    "Active"
                    if backend_status == "active"
                    else backend_status
                ),
            }
        )

    return interface_mandates


def get_mandates_for_expense_form() -> list[dict[str, Any]]:
    """Retrieve complete mandate data for the expense form."""

    backend_mandates = get_mandates()
    agent_names = _get_agent_names()

    form_mandates = []

    for mandate in backend_mandates:
        backend_category = mandate.get("category", "other")

        form_mandates.append(
            {
                "id": mandate["id"],
                "agent_id": mandate["agent_id"],
                "agent_name": agent_names.get(
                    mandate["agent_id"],
                    "Unknown agent",
                ),
                "intent": mandate.get("intent", ""),
                "category": backend_category,
                "category_label": BACKEND_CATEGORIES.get(
                    backend_category,
                    backend_category,
                ),
                "total_limit": mandate["max_total_cents"] / 100,
                "transaction_limit": (
                    mandate["per_txn_max_cents"] / 100
                ),
                "currency": mandate.get("currency", "EUR"),
                "status": mandate.get("status", "unknown"),
            }
        )

    return form_mandates


def create_mandate_in_backend(
    agent_id: str,
    owner_name: str,
    intent: str,
    category: str,
    limit_amount: float,
) -> dict[str, Any]:
    """Create a spending mandate through the backend API."""

    if category not in INTERFACE_CATEGORIES:
        raise ValueError(f"Unsupported mandate category: {category}")

    payload = {
        "agent_id": agent_id,
        "owner_name": owner_name.strip(),
        "intent": intent.strip(),
        "category": INTERFACE_CATEGORIES[category],
        "max_total_cents": round(limit_amount * 100),
    }

    return create_mandate(payload)


# =========================================================
# 4. SIMULATED WALLET
# =========================================================

def get_wallet_transactions() -> list[dict[str, Any]]:
    """Return the fictional transactions of the simulated wallet."""

    return [
        transaction.copy()
        for transaction in DEMO_WALLET_TRANSACTIONS
    ]


def get_initial_balance() -> float:
    """Return the initial fictional balance of the simulated wallet."""

    return DEMO_INITIAL_BALANCE