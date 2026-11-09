"""Data preparation layer for the AgentShield Streamlit application."""

from datetime import datetime
from typing import Any

from data.backend_client import (
    create_mandate,
    get_actions,
    get_agents,
    get_mandates,
    get_wallet,
    get_reserve,
)


# 1. AGENT DATA

def get_agents_count() -> int:
    return len(get_agents())


def get_agents_for_form() -> list[dict[str, str]]:
    agents = get_agents()

    return [
        {
            "id": agent["agent"]["id"],
            "name": agent["agent"]["name"],
        }
        for agent in agents
    ]


def _get_agent_names() -> dict[str, str]:
    agents = get_agents()

    return {
        agent["agent"]["id"]: agent["agent"]["name"]
        for agent in agents
    }


# 2. ACTION DATA

DECISION_LABELS = {
    "APPROVE": "Approved",
    "ESCALATE": "Escalated",
    "REFUSE": "Rejected",
}


def _format_action_date(raw_date: str | None) -> str:
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

        interface_actions.append({
            "Agent": agent_names.get(agent_id, "Unknown agent"),
            "Action": description,
            "Amount (€)": action["amount_cents"] / 100,
            "Decision": DECISION_LABELS.get(decision, decision),
            "Reason": action.get("explanation", ""),
            "Date": _format_action_date(
                action.get("created_at")
            ),
        })

    return interface_actions


# 3. SPENDING MANDATES

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
    return list(INTERFACE_CATEGORIES.keys())


def get_initial_mandates() -> list[dict[str, Any]]:
    backend_mandates = get_mandates()
    agent_names = _get_agent_names()

    interface_mandates = []

    for mandate in backend_mandates:
        backend_category = mandate.get("category", "other")
        backend_status = mandate.get("status", "unknown")

        interface_mandates.append({
            "Agent": agent_names.get(
                mandate["agent_id"],
                "Unknown agent",
            ),
            "Category": BACKEND_CATEGORIES.get(
                backend_category,
                backend_category,
            ),
            "Spending limit (€)": (
                mandate["max_total_cents"] / 100
            ),
            "Status": (
                "Active"
                if backend_status == "active"
                else backend_status
            ),
        })

    return interface_mandates


def get_mandates_for_expense_form() -> list[dict[str, Any]]:
    backend_mandates = get_mandates()
    agent_names = _get_agent_names()

    form_mandates = []

    for mandate in backend_mandates:
        backend_category = mandate.get("category", "other")

        form_mandates.append({
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
            "total_limit": (
                mandate["max_total_cents"] / 100
            ),
            "transaction_limit": (
                mandate["per_txn_max_cents"] / 100
            ),
            "currency": mandate.get("currency", "EUR"),
            "status": mandate.get("status", "unknown"),
        })

    return form_mandates


def create_mandate_in_backend(
    agent_id,
    owner_name,
    intent,
    category,
    limit_amount,
):
    if category not in INTERFACE_CATEGORIES:
        raise ValueError(
            f"Unsupported mandate category: {category}"
        )

    payload = {
        "agent_id": agent_id,
        "owner_name": owner_name.strip(),
        "intent": intent.strip(),
        "category": INTERFACE_CATEGORIES[category],
        "max_total_cents": round(limit_amount * 100),
    }

    return create_mandate(payload)


# 4. WALLET DATA

def _format_wallet_date(raw_date: str | None) -> str:
    """Format a backend timestamp for the wallet interface."""

    if not raw_date:
        return "Unknown date"

    try:
        parsed_date = datetime.fromisoformat(
            raw_date.replace("Z", "+00:00")
        )
        return parsed_date.strftime("%Y-%m-%d %H:%M")

    except ValueError:
        return "Unknown date"


def get_agent_wallet_data(agent_id: str) -> dict[str, Any]:
    """Retrieve and prepare one agent's wallet information."""

    response = get_wallet(agent_id)

    wallet = response["wallet"]
    quote = response["quote"]
    ledger = response.get("ledger", [])

    # Use the bonus-malus actually calculated by the pricing engine.
    bonus_malus_cents = quote["bonus_malus_cents"]
    base_premium_cents = quote["base_premium_cents"]

    # Convert the monetary adjustment into a percentage.
    # Example: -38 cents on a 1200-cent premium = -3.17%.
    if base_premium_cents > 0:
        bonus_malus_percent = (
            bonus_malus_cents / base_premium_cents
        ) * 100
        bonus_malus_bps = round(
            bonus_malus_cents * 10000 / base_premium_cents
        )
    else:
        bonus_malus_percent = 0.0
        bonus_malus_bps = 0

    transactions = []

    for entry in ledger:
        transactions.append({
            "Date": _format_wallet_date(
                entry.get("created_at")
            ),
            "Type": entry.get("kind", "unknown").title(),
            "Description": entry.get("label", ""),
            "Amount (€)": (
                entry.get("amount_cents", 0) / 100
            ),
            "Balance after (€)": (
                entry.get("balance_after_cents", 0) / 100
            ),
            "Reference": entry.get("reference") or "—",
        })

    return {
        "balance": wallet["balance_cents"] / 100,
        "premium": wallet["premium_cents"] / 100,
        "reserve_contribution": (
            wallet["reserve_contribution_cents"] / 100
        ),
        "spent_this_period": (
            wallet["spent_this_period_cents"] / 100
        ),
        "bonus_malus_bps": bonus_malus_bps,
        "bonus_malus_percent": bonus_malus_percent,
        "updated_at": _format_wallet_date(
            wallet.get("updated_at")
        ),
        "transactions": transactions,
    }

def get_shared_reserve_data() -> dict[str, Any]:
    """Retrieve and prepare the shared reserve information."""

    response = get_reserve()

    ledger = response.get("ledger", [])
    transactions = []

    for entry in ledger:
        transactions.append({
            "Date": _format_wallet_date(
                entry.get("created_at")
            ),
            "Type": entry.get("kind", "unknown").title(),
            "Description": entry.get("label", ""),
            "Amount (€)": (
                entry.get("amount_cents", 0) / 100
            ),
            "Balance after (€)": (
                entry.get("balance_after_cents", 0) / 100
            ),
            "Reference": entry.get("reference") or "—",
        })

    return {
        "balance": response["balance_cents"] / 100,
        "transactions": transactions,
    }