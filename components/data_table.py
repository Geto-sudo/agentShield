"""Reusable theme-aware HTML data table component."""

import pandas as pd
import streamlit as st


def render_data_table(
    dataframe: pd.DataFrame,
    height: int = 400,
    hide_index: bool = True,
    use_container_width: bool = True,
    empty_message: str = "No data available.",
) -> None:
    """Display a responsive HTML table using the active application theme."""

    if dataframe.empty:
        st.info(empty_message)
        return

    display_dataframe = dataframe.copy()

    if hide_index:
        display_dataframe = display_dataframe.reset_index(drop=True)

    table_html = display_dataframe.to_html(
        index=not hide_index,
        border=0,
        escape=True,
        classes="agentshield-data-table",
        na_rep="—",
    )

    width_style = "width: 100%;" if use_container_width else "width: auto;"

    st.html(
        f"""
        <style>
            .agentshield-table-container {{
                width: 100%;
                max-height: {height}px;
                overflow: auto;
                border: 1px solid var(--border);
                border-radius: 12px;
                background: var(--surface);
            }}

            .agentshield-data-table {{
                {width_style}
                border-collapse: separate;
                border-spacing: 0;
                color: var(--text-primary);
                background: var(--surface);
                font-size: 0.88rem;
            }}

            .agentshield-data-table thead th {{
                position: sticky;
                top: 0;
                z-index: 2;
                padding: 14px 16px;
                background: var(--surface-secondary);
                color: var(--text-primary);
                font-weight: 600;
                text-align: left;
                white-space: nowrap;
                border-bottom: 1px solid var(--border);
            }}

            .agentshield-data-table tbody td {{
                padding: 13px 16px;
                color: var(--text-primary);
                background: var(--surface);
                border-bottom: 1px solid var(--border);
                vertical-align: middle;
            }}

            .agentshield-data-table tbody tr:nth-child(even) td {{
                background: var(--surface-secondary);
            }}

            .agentshield-data-table tbody tr:hover td {{
                background: var(--input-background);
            }}

            .agentshield-data-table tbody tr:last-child td {{
                border-bottom: none;
            }}

            .agentshield-data-table th,
            .agentshield-data-table td {{
                border-right: 1px solid var(--border);
            }}

            .agentshield-data-table th:last-child,
            .agentshield-data-table td:last-child {{
                border-right: none;
            }}
        </style>

        <div class="agentshield-table-container">
            {table_html}
        </div>
        """
    )