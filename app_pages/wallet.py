"""AgentShield simulated wallet page."""

import pandas as pd
import streamlit as st

from components.header import render_header
from components.metric_card import render_metric_card
from components.data_table import render_data_table

from data.data_source import (
    get_agents_for_form,
    get_agent_wallet_data,
    get_shared_reserve_data,
)


def render_wallet() -> None:
    """Render the AgentShield backend wallet."""

    render_header(
        title="Simulated wallet",
        eyebrow="SANDBOX FINANCIAL OVERVIEW",
        subtitle=(
            "Explore agent wallet balances, financial activity "
            "and the shared reserve."
        ),
    )

    st.warning(
        "Simulation only. These figures belong to the internal "
        "AgentShield ledger and do not represent real PayPal funds."
    )

    # 1. LOAD AVAILABLE AGENTS

    try:
        agents = get_agents_for_form()

    except RuntimeError as error:
        st.error(f"Unable to load agents: {error}")
        return

    if not agents:
        st.info(
            "No agents are available. Create an agent "
            "before consulting a wallet."
        )
        return

    # 2. AGENT SELECTION

    agent_names = {
        agent["id"]: agent["name"]
        for agent in agents
    }

    agent_ids = list(agent_names.keys())

    selected_agent_id = st.selectbox(
        "Select an agent",
        options=agent_ids,
        format_func=lambda agent_id: agent_names[agent_id],
        key="wallet_selected_agent",
    )

    # 3. LOAD SELECTED AGENT WALLET

    try:
        wallet_data = get_agent_wallet_data(
            selected_agent_id
        )

    except RuntimeError as error:
        st.error(f"Unable to load this wallet: {error}")
        return

    except (KeyError, TypeError, ValueError) as error:
        st.error(
            "The backend returned an unexpected wallet response."
        )
        st.caption(str(error))
        return

    # 4. WALLET OVERVIEW

    st.markdown("### Wallet overview")

    first_column, second_column = st.columns(2)

    with first_column:
        render_metric_card(
            label="AVAILABLE BALANCE",
            value=f"€{wallet_data['balance']:,.2f}",
            description="Current internal wallet balance",
            accent_color="#2563EB",
        )

    with second_column:
        render_metric_card(
            label="SPENT THIS PERIOD",
            value=f"€{wallet_data['spent_this_period']:,.2f}",
            description="Approved and settled expenses",
        )

    third_column, fourth_column = st.columns(2)

    with third_column:
        render_metric_card(
            label="ESTIMATED MONTHLY PREMIUM",
            value=f"€{wallet_data['premium']:,.2f}",
            description="Estimated monthly coverage cost",
        )

    with fourth_column:
        render_metric_card(
            label="ESTIMATED RESERVE CONTRIBUTION",
            value=(
                f"€{wallet_data['reserve_contribution']:,.2f}"
            ),
            description="Estimated contribution to the shared reserve",
        )

    # 5. BONUS-MALUS INDICATOR

    st.markdown("### Bonus-Malus adjustment")

    bonus_malus = wallet_data["bonus_malus_percent"]

    if bonus_malus < 0:
        bonus_malus_color = "#16A34A"
        bonus_malus_description = (
            "Bonus applied: the adjustment is negative."
        )

    elif bonus_malus > 0:
        bonus_malus_color = "#DC2626"
        bonus_malus_description = (
            "Malus applied: the adjustment is positive."
        )

    else:
        bonus_malus_color = "#2563EB"
        bonus_malus_description = (
            "No bonus or malus adjustment is currently applied."
        )

    bonus_column, _ = st.columns(2)

    with bonus_column:
        render_metric_card(
            label="BONUS-MALUS",
            value=f"{bonus_malus:+.2f}%",
            description=bonus_malus_description,
            accent_color=bonus_malus_color,
        )

    st.caption(
        "This percentage is calculated by the AgentShield "
        "backend and displayed for information only."
    )

    st.caption(
        f"Last wallet update: {wallet_data['updated_at']}"
    )

    st.divider()

    # 6. AGENT TRANSACTION HISTORY

    st.markdown("### Agent transaction history")

    transactions_df = pd.DataFrame(
        wallet_data["transactions"]
    )

    render_data_table(
        dataframe=transactions_df,
        height=380,
        empty_message="No transactions recorded for this agent.",
    )

    st.divider()

    # 7. SHARED RESERVE

    st.markdown("### Shared reserve")

    try:
        reserve_data = get_shared_reserve_data()

    except RuntimeError as error:
        st.error(
            f"Unable to load the shared reserve: {error}"
        )
        return

    reserve_column, reserve_info_column = st.columns(2)

    with reserve_column:
        render_metric_card(
            label="SHARED RESERVE BALANCE",
            value=f"€{reserve_data['balance']:,.2f}",
            description="Current balance of the common reserve",
            accent_color="#2563EB",
        )

    with reserve_info_column:
        render_metric_card(
            label="RECORDED RESERVE OPERATIONS",
            value=len(reserve_data["transactions"]),
            description="Operations in the reserve ledger",
        )

    st.markdown("#### Reserve transaction history")

    reserve_df = pd.DataFrame(
        reserve_data["transactions"]
    )

    render_data_table(
        dataframe=reserve_df,
        height=300,
        empty_message="No reserve transactions recorded.",
    )

    st.divider()

    # 8. EXPLANATORY NOTICE

    st.markdown("### About this wallet")

    st.html("""
        <div style="
            padding: 18px 20px;
            border: 1px solid var(--border);
            border-radius: 12px;
            background: var(--surface);
            color: var(--text-secondary);
            line-height: 1.7;
        ">
            <strong style="color: var(--text-primary);">
                Internal AgentShield financial ledger
            </strong>
            <p style="margin-bottom: 0;">
                This wallet displays simulated balances and
                transactions recorded by AgentShield. Each agent
                has an individual wallet, while the shared reserve
                is managed separately. These figures do not represent
                the balance of a real PayPal account, and no real
                money is transferred through this screen.
            </p>
        </div>
    """)
