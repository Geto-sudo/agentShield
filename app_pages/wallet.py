"""AgentShield simulated wallet page."""

import pandas as pd
import streamlit as st

from components.header import render_header
from components.metric_card import render_metric_card
from components.data_table import render_data_table

from data.data_source import (
    get_initial_balance,
    get_wallet_transactions,
)


def render_wallet() -> None:
    """Render the fictional AgentShield wallet."""

    render_header(
        title="Simulated wallet",
        eyebrow="SANDBOX FINANCIAL OVERVIEW",
        subtitle=(
            "Explore the fictional wallet balance and "
            "transactions used in the prototype."
        ),
    )

    st.warning(
        "Simulation only. This wallet contains fictional "
        "data and does not represent real PayPal funds."
    )

    # -----------------------------------------------------
    # LOAD SIMULATED DATA
    # -----------------------------------------------------

    balance = get_initial_balance()
    transactions = get_wallet_transactions()

    transactions_df = pd.DataFrame(transactions)

    # -----------------------------------------------------
    # WALLET OVERVIEW
    # -----------------------------------------------------

    st.markdown("### Wallet overview")

    first_column, second_column = st.columns(2)

    with first_column:

        render_metric_card(
            label="SIMULATED BALANCE",
            value=f"€{balance:,.2f}",
            description="Fictional available balance",
            accent_color="#2563EB",
        )

    with second_column:

        render_metric_card(
            label="RECORDED TRANSACTIONS",
            value=len(transactions),
            description=(
                "Transactions in the demonstration dataset"
            ),
        )

    st.divider()

    # -----------------------------------------------------
    # TRANSACTION HISTORY
    # -----------------------------------------------------

    st.markdown("### Transaction history")

    if transactions_df.empty:

        st.info(
            "No simulated transactions are available."
        )

    else:

        render_data_table(
            dataframe=transactions_df,
            height=380,
        )

    st.divider()

    # -----------------------------------------------------
    # SIMULATION NOTICE
    # -----------------------------------------------------

    st.markdown("### About this wallet")

    st.html("""
    <div style="
        background-color: var(--surface);
        border: 1px solid var(--border);
        border-radius: 12px;
        padding: 1.25rem;
    ">

        <div style="
            font-weight: 700;
            color: var(--text-primary);
            margin-bottom: 0.5rem;
        ">
            Prototype Sandbox environment
        </div>

        <div style="
            color: var(--text-secondary);
            line-height: 1.7;
            font-size: 0.9rem;
        ">
            This wallet is used to demonstrate the
            financial monitoring concepts behind AgentShield.
            Its balance and transaction history are fictional.
            No real money is stored or transferred through
            this simulated wallet.
        </div>

    </div>
    """)