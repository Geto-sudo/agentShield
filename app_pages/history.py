"""AgentShield activity history page."""

import pandas as pd
import streamlit as st

from components.header import render_header
from components.data_table import render_data_table
from data.data_source import get_initial_actions


def inject_history_styles() -> None:
    """Apply theme-aware styles to the history search field."""

    st.markdown(
        """
        <style>
        /* Search activity input */
        div[data-testid="stTextInput"] input {
            color: var(--text-primary) !important;
            background-color: var(--input-background) !important;
            caret-color: var(--accent) !important;
            border-color: var(--border) !important;
        }

        /* Search placeholder */
        div[data-testid="stTextInput"] input::placeholder {
            color: var(--text-secondary) !important;
            opacity: 1 !important;
        }

        /* Search label */
        div[data-testid="stTextInput"] label {
            color: var(--text-primary) !important;
        }

        /* Search focus */
        div[data-testid="stTextInput"] input:focus {
            border-color: var(--accent) !important;
            box-shadow: 0 0 0 1px var(--accent) !important;
        }
        </style>
        """,
        unsafe_allow_html=True,
    )


def render_history() -> None:
    """Render the complete agent activity history."""

    inject_history_styles()

    render_header(
        title="Activity history",
        eyebrow="AUDIT & MONITORING",
        subtitle=(
            "Review previously recorded agent actions "
            "and their decisions."
        ),
    )

    try:
        actions = get_initial_actions()

    except RuntimeError as error:
        st.error(
            f"Unable to load activity history: {error}"
        )
        return

    dataframe = pd.DataFrame(actions)

    if dataframe.empty:
        st.info("No activity has been recorded yet.")
        return

    # -----------------------------------------------------
    # HISTORY METRICS
    # -----------------------------------------------------

    total_actions = len(dataframe)

    approved = int(
        (dataframe["Decision"] == "Approved").sum()
    )

    rejected = int(
        (dataframe["Decision"] == "Rejected").sum()
    )

    escalated = int(
        (dataframe["Decision"] == "Escalated").sum()
    )

    first, second, third, fourth = st.columns(4)

    first.metric("Total actions", total_actions)
    second.metric("Approved", approved)
    third.metric("Rejected", rejected)
    fourth.metric("Escalated", escalated)

    st.divider()

    # -----------------------------------------------------
    # FILTERS
    # -----------------------------------------------------

    st.markdown("### Activity records")

    filter_column, search_column = st.columns([1, 2])

    with filter_column:

        decision_options = [
            "All decisions",
            "Approved",
            "Rejected",
            "Escalated",
        ]

        selected_decision = st.selectbox(
            "Filter by decision",
            options=decision_options,
        )

    with search_column:

        search_query = st.text_input(
            "Search activity",
            placeholder=(
                "Search by agent, merchant or description..."
            ),
        )

    filtered_dataframe = dataframe.copy()

    if selected_decision != "All decisions":

        filtered_dataframe = filtered_dataframe[
            filtered_dataframe["Decision"] == selected_decision
        ]

    if search_query.strip():

        query = search_query.strip().lower()

        searchable_columns = [
            column
            for column in ["Agent", "Action", "Reason"]
            if column in filtered_dataframe.columns
        ]

        if searchable_columns:

            search_mask = (
                filtered_dataframe[searchable_columns]
                .fillna("")
                .astype(str)
                .apply(
                    lambda column: column.str.lower().str.contains(
                        query,
                        regex=False,
                    )
                )
                .any(axis=1)
            )

            filtered_dataframe = filtered_dataframe[
                search_mask
            ]

    st.caption(
        f"{len(filtered_dataframe)} result(s) "
        f"out of {total_actions} recorded actions."
    )

    render_data_table(
        dataframe=filtered_dataframe,
        height=480,
        empty_message="No activity matches your filters.",
    )