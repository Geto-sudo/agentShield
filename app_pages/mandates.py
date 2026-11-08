"""AgentShield spending mandates page."""

import pandas as pd
import streamlit as st

from components.header import render_header
from components.metric_card import render_metric_card

from data.data_source import (
    create_mandate_in_backend,
    get_agents_for_form,
    get_categories,
    get_initial_mandates,
)


def inject_mandates_styles() -> None:
    """Apply theme-aware styles to the mandates page."""

    st.html(
        """
        <style>
            /* =========================================
               FORM LABELS
            ========================================= */

            div[data-testid="stWidgetLabel"] p,
            div[data-testid="stWidgetLabel"] label,
            .stTextInput label,
            .stNumberInput label,
            .stSelectbox label {
                color: var(--text-primary) !important;
                font-weight: 500 !important;
            }


            /* =========================================
               TEXT INPUTS
            ========================================= */

            div[data-testid="stTextInput"] input,
            div[data-testid="stNumberInput"] input {
                background-color: var(--input-background) !important;
                color: var(--text-primary) !important;
                border: 1px solid var(--border) !important;
                border-radius: 8px !important;
            }

            div[data-testid="stTextInput"] input::placeholder {
                color: var(--text-secondary) !important;
                opacity: 1 !important;
            }

            div[data-testid="stTextInput"] input:focus,
            div[data-testid="stNumberInput"] input:focus {
                border-color: var(--accent) !important;
                box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.15) !important;
            }


            /* =========================================
                SELECTBOX — SELECTED VALUE
            ========================================= */

            div[data-testid="stSelectbox"] div[data-baseweb="select"] {
                background-color: var(--input-background) !important;
            }

            div[data-testid="stSelectbox"] div[data-baseweb="select"] > div {
                background-color: var(--input-background) !important;
                border: 1px solid var(--border) !important;
                color: var(--text-primary) !important;
                border-radius: 8px !important;
            }

            /* Selected value */
            div[data-testid="stSelectbox"] div[data-baseweb="select"] span {
                color: var(--text-primary) !important;
            }

            /* Dropdown arrow */
            div[data-testid="stSelectbox"] div[data-baseweb="select"] svg {
                fill: var(--text-secondary) !important;
            }

            /* Focus */
            div[data-testid="stSelectbox"] div[data-baseweb="select"] > div:focus-within {
                border-color: var(--accent) !important;
                box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.15) !important;
            }

            /* =========================================
               CREATE MANDATE BUTTON
            ========================================= */

            div[data-testid="stFormSubmitButton"] button {
                background: linear-gradient(
                    110deg,
                    var(--primary),
                    var(--accent)
                ) !important;

                color: #FFFFFF !important;
                border: 1px solid var(--accent) !important;
                border-radius: 9px !important;

                font-weight: 650 !important;
                letter-spacing: 0.01em;

                min-height: 44px;

                box-shadow: 0 3px 8px rgba(37, 99, 235, 0.16);

                transition:
                    transform 160ms ease,
                    box-shadow 160ms ease,
                    filter 160ms ease;
            }

            div[data-testid="stFormSubmitButton"] button p {
                color: #FFFFFF !important;
                font-weight: 650 !important;
            }

            div[data-testid="stFormSubmitButton"] button:hover {
                filter: brightness(1.10);
                transform: translateY(-2px);

                box-shadow: 0 6px 15px rgba(37, 99, 235, 0.25);

                border-color: var(--accent) !important;
            }

            div[data-testid="stFormSubmitButton"] button:active {
                transform: translateY(0);
                filter: brightness(0.95);
                box-shadow: 0 2px 5px rgba(37, 99, 235, 0.18);
            }

            div[data-testid="stFormSubmitButton"] button:focus-visible {
                outline: 3px solid var(--accent) !important;
                outline-offset: 3px;
            }


            /* =========================================
               RESPONSIVE TABLE
            ========================================= */

            @media (max-width: 768px) {
                .agentshield-mandates-table {
                    min-width: 650px;
                }
            }

            @media (prefers-reduced-motion: reduce) {
                div[data-testid="stFormSubmitButton"] button {
                    transition: none;
                }
            }

            /* =========================================
                SIDEBAR — DARK MODE LABEL
            ========================================= */

            section[data-testid="stSidebar"] label,
            section[data-testid="stSidebar"] label p,
            section[data-testid="stSidebar"] [data-testid="stWidgetLabel"] p {
            color: #FFFFFF !important;
                }

            /* =========================================
                REGISTERED MANDATES TABLE
            ========================================= */

            .mandates-table-container {
                width: 100%;
                overflow-x: auto;
                border: 1px solid var(--border);
                border-radius: 12px;
                background: var(--surface);
            }

            .agentshield-mandates-table {
                width: 100%;
                border-collapse: collapse;
                font-size: 13px;
                color: var(--text-primary);
                background: var(--surface);
            }

            .agentshield-mandates-table thead th {
                padding: 14px 16px;
                background: var(--surface-secondary);
                color: var(--text-secondary);
                border-bottom: 1px solid var(--border);
                text-align: left;
                white-space: nowrap;
                font-size: 11px;
                font-weight: 700;
                letter-spacing: 0.04em;
                text-transform: uppercase;
            }

            .agentshield-mandates-table tbody td {
                padding: 15px 16px;
                background: var(--surface);
                color: var(--text-primary);
                border-bottom: 1px solid var(--border);
                vertical-align: middle;
            }

            .agentshield-mandates-table tbody tr:nth-child(even) td {
                background: var(--surface-secondary);
            }

            .agentshield-mandates-table tbody tr:hover td {
                background: var(--surface-secondary);
            }

            .agentshield-mandates-table tbody tr:last-child td {
                border-bottom: none;
            }

            .agentshield-mandates-table th,
            .agentshield-mandates-table td {
                border-right: 1px solid var(--border);
            }

            .agentshield-mandates-table th:last-child,
            .agentshield-mandates-table td:last-child {
                border-right: none;
            }

            .agentshield-mandates-table td:last-child {
                white-space: nowrap;
                font-weight: 600;
            }

            @media (max-width: 768px) {
                .agentshield-mandates-table {
                    min-width: 650px;
                }
            }
        </style>
        """
    )


def render_mandates_table(dataframe: pd.DataFrame) -> None:
    """Render registered mandates using the active AgentShield theme."""

    if dataframe.empty:
        st.info("No spending mandates have been created yet.")
        return

    table_html = dataframe.to_html(
        index=False,
        border=0,
        classes="agentshield-mandates-table",
        escape=True,
    )

    st.html(
        f"""
        <div class="mandates-table-container">
            {table_html}
        </div>
        """
    )


def render_mandates() -> None:
    """Render spending mandate management."""

    # Inject styles before rendering the page widgets.
    # This also works when the mandates table is empty.
    inject_mandates_styles()

    render_header(
        title="Spending mandates",
        eyebrow="POLICY MANAGEMENT",
        subtitle=(
            "Define and manage the spending permissions "
            "granted to your AI agents."
        ),
    )

    st.info(
        "A spending mandate defines what an AI agent is "
        "allowed to purchase and how much it can spend."
    )

    # -----------------------------------------------------
    # LOAD EXISTING DATA
    # -----------------------------------------------------

    try:
        mandates = get_initial_mandates()
        agents = get_agents_for_form()
        categories = get_categories()

    except RuntimeError as error:
        st.error(f"Unable to load mandate data: {error}")
        return

    mandates_df = pd.DataFrame(mandates)

    active_count = 0

    if not mandates_df.empty and "Status" in mandates_df.columns:
        active_count = int(
            (mandates_df["Status"] == "Active").sum()
        )

    # -----------------------------------------------------
    # MANDATE METRICS
    # -----------------------------------------------------

    first_column, second_column = st.columns(2)

    with first_column:
        render_metric_card(
            label="TOTAL MANDATES",
            value=len(mandates),
            description="Registered spending policies",
        )

    with second_column:
        render_metric_card(
            label="ACTIVE MANDATES",
            value=active_count,
            description="Currently enabled policies",
            accent_color="#16A36A",
        )

    # -----------------------------------------------------
    # REGISTERED MANDATES
    # -----------------------------------------------------

    st.markdown("### Registered mandates")

    if mandates_df.empty:
        st.info("No spending mandates have been created yet.")
    else:
        render_mandates_table(mandates_df)

    st.divider()

    # -----------------------------------------------------
    # CREATE A NEW MANDATE
    # -----------------------------------------------------

    st.markdown("### Create a spending mandate")

    st.caption(
        "Configure the permissions and spending limit "
        "for an AI agent."
    )

    if not agents:
        st.warning(
            "No registered agents are available. "
            "Register an agent before creating a mandate."
        )
        return

    with st.form("create_mandate_form"):

        agent_options = {
            agent["name"]: agent["id"]
            for agent in agents
        }

        selected_agent_name = st.selectbox(
            "AI agent",
            options=list(agent_options.keys()),
        )

        owner_name = st.text_input(
            "Owner name",
            placeholder="Enter the mandate owner",
        )

        intent = st.text_input(
            "Mandate purpose",
            placeholder="Example: Purchase office equipment",
        )

        category = st.selectbox(
            "Spending category",
            options=categories,
        )

        limit_amount = st.number_input(
            "Maximum total spending limit (€)",
            min_value=0.01,
            value=100.00,
            step=10.00,
        )

        submitted = st.form_submit_button(
            "Create mandate",
            type="primary",
            use_container_width=True,
        )

        if submitted:

            if not owner_name.strip():
                st.error("Please enter the owner name.")

            elif not intent.strip():
                st.error("Please describe the mandate purpose.")

            else:
                try:
                    result = create_mandate_in_backend(
                        agent_id=agent_options[selected_agent_name],
                        owner_name=owner_name,
                        intent=intent,
                        category=category,
                        limit_amount=limit_amount,
                    )

                    st.success(
                        "Spending mandate created successfully."
                    )

                    with st.expander("Creation response"):
                        st.json(result)

                    st.rerun()

                except (RuntimeError, ValueError) as error:
                    st.error(
                        f"Unable to create mandate: {error}"
                    )