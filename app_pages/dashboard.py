"""AgentShield main dashboard."""

import pandas as pd
import streamlit as st

from components.header import render_header
from components.metric_card import render_metric_card
from data.data_source import get_initial_actions, get_agents_count
from data.backend_client import get_approvals


def render_recent_activity_table(dataframe):
    if dataframe.empty:
        st.info("No recent activity has been recorded yet.")
        return

    table_html = dataframe.to_html(
        index=False,
        border=0,
        classes="agentshield-activity-table",
        escape=True,
    )

    st.html(
        f"""
        <style>
            .activity-table-container {{
                width: 100%;
                overflow-x: auto;
                border: 1px solid var(--border);
                border-radius: 12px;
                background: var(--surface);
            }}

            .agentshield-activity-table {{
                width: 100%;
                border-collapse: collapse;
                font-size: 13px;
                color: var(--text-primary);
                background: var(--surface);
            }}

            .agentshield-activity-table thead th {{
                background: var(--surface-secondary);
                color: var(--text-secondary);
                font-size: 11px;
                font-weight: 700;
                text-transform: uppercase;
                letter-spacing: 0.04em;
                padding: 14px 12px;
                text-align: left;
                border-bottom: 1px solid var(--border);
                white-space: nowrap;
            }}

            .agentshield-activity-table tbody td {{
                padding: 14px 12px;
                border-bottom: 1px solid var(--border);
                color: var(--text-primary);
                vertical-align: top;
                background: var(--surface);
            }}

            .agentshield-activity-table tbody tr:nth-child(even) td {{
                background: var(--surface-secondary);
            }}

            .agentshield-activity-table tbody tr:hover td {{
                background: var(--surface-secondary);
            }}

            .agentshield-activity-table th,
            .agentshield-activity-table td {{
                border-right: 1px solid var(--border);
            }}

            .agentshield-activity-table th:last-child,
            .agentshield-activity-table td:last-child {{
                border-right: none;
            }}

            .agentshield-activity-table td:nth-child(1),
            .agentshield-activity-table td:nth-child(2),
            .agentshield-activity-table td:nth-child(3) {{
                white-space: nowrap;
            }}

            .agentshield-activity-table td:nth-child(2) {{
                font-weight: 600;
            }}

            .agentshield-activity-table tbody tr:last-child td {{
                border-bottom: none;
            }}

            .agentshield-activity-table td:last-child {{
                white-space: nowrap;
                color: var(--text-secondary);
                font-size: 12px;
            }}

            .agentshield-activity-table td:nth-child(4) {{
                min-width: 280px;
                max-width: 480px;
                line-height: 1.6;
                overflow-wrap: anywhere;
            }}

            @media (max-width: 768px) {{
                .agentshield-activity-table {{
                    min-width: 850px;
                }}
            }}
        </style>

        <div class="activity-table-container">
            {table_html}
        </div>
        """
    )


def render_dashboard() -> None:
    """Render the main AgentShield dashboard."""

    render_header(
        title="Dashboard",
        eyebrow="OVERVIEW",
        subtitle=(
            "Monitor your AI agents, spending decisions "
            "and approval activity."
        ),
    )

    try:
        actions = get_initial_actions()
        agents_count = get_agents_count()
        pending_approvals = get_approvals(
            status="pending",
            limit=50,
        )

    except RuntimeError as error:
        st.error(f"Unable to load dashboard data: {error}")
        return

    actions_df = pd.DataFrame(actions)

    total_actions = len(actions_df)

    approved_count = 0
    rejected_count = 0
    escalated_count = 0

    if not actions_df.empty and "Decision" in actions_df.columns:
        approved_count = int(
            (actions_df["Decision"] == "Approved").sum()
        )

        rejected_count = int(
            (actions_df["Decision"] == "Rejected").sum()
        )

        escalated_count = int(
            (actions_df["Decision"] == "Escalated").sum()
        )

    pending_count = len(pending_approvals)

    # -----------------------------------------------------
    # KEY PERFORMANCE INDICATORS
    # -----------------------------------------------------

    st.markdown("### Key performance indicators")

    columns = st.columns(4)

    metrics = [
        ("REGISTERED AGENTS", agents_count, "Agents connected"),
        ("RECORDED ACTIONS", total_actions, "Total recorded decisions"),
        ("APPROVED ACTIONS", approved_count, "Authorized by the engine"),
        ("PENDING APPROVALS", pending_count, "Waiting for human review"),
    ]

    for column, metric in zip(columns, metrics):
        with column:
            render_metric_card(
                label=metric[0],
                value=metric[1],
                description=metric[2],
            )

    st.markdown("")

    # -----------------------------------------------------
    # DECISION DISTRIBUTION
    # -----------------------------------------------------

    left_column, right_column = st.columns([1.15, 1])

    with left_column:

        st.markdown("### Decision distribution")

        st.caption(
            "Breakdown of recorded agent decisions."
        )


        if total_actions:
            approved_pct = (approved_count / total_actions) * 100
            rejected_pct = (rejected_count / total_actions) * 100
            escalated_pct = (escalated_count / total_actions) * 100

            approved_end = approved_pct
            rejected_end = approved_end + rejected_pct

            chart_html = f"""
            <div style="
                display: flex;
                align-items: center;
                justify-content: center;
                gap: 42px;
                padding: 24px 12px;
                background: var(--surface);
                border: 1px solid var(--border);
                border-radius: 16px;
                flex-wrap: wrap;
            ">

                <div style="
                    width: 190px;
                    height: 190px;
                    border-radius: 50%;
                    background: conic-gradient(
                        var(--success) 0% {approved_end}%,
                        var(--danger) {approved_end}% {rejected_end}%,
                        var(--warning) {rejected_end}% 100%
                    );
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    flex-shrink: 0;
                ">
                    <div style="
                        width: 126px;
                        height: 126px;
                        border-radius: 50%;
                        background: var(--surface);
                        display: flex;
                        flex-direction: column;
                        align-items: center;
                        justify-content: center;
                    ">
                        <span style="
                            font-size: 30px;
                            font-weight: 700;
                            color: var(--text-primary);
                            line-height: 1.2;
                        ">{total_actions}</span>

                        <span style="
                            font-size: 12px;
                            color: var(--text-secondary);
                            margin-top: 4px;
                        ">Total actions</span>
                    </div>
                </div>

                <div style="
                    display: flex;
                    flex-direction: column;
                    gap: 18px;
                    min-width: 165px;
                ">

                    <div style="display: flex; align-items: center; gap: 10px;">
                        <span style="
                            width: 11px;
                            height: 11px;
                            border-radius: 50%;
                            background: var(--success);
                            flex-shrink: 0;
                        "></span>

                        <div style="flex: 1;">
                            <div style="font-size: 13px; color: var(--text-secondary);">
                                Approved
                            </div>
                            <div style="
                                font-size: 19px;
                                font-weight: 700;
                                color: var(--text-primary);
                            ">{approved_pct:.1f}%</div>
                            <div style="font-size: 12px; color: var(--text-secondary);">
                                {approved_count} actions
                            </div>
                        </div>
                    </div>

                    <div style="display: flex; align-items: center; gap: 10px;">
                        <span style="
                            width: 11px;
                            height: 11px;
                            border-radius: 50%;
                            background: var(--danger);
                            flex-shrink: 0;
                        "></span>

                        <div style="flex: 1;">
                            <div style="font-size: 13px; color: var(--text-secondary);">
                                Rejected
                            </div>
                            <div style="
                                font-size: 19px;
                                font-weight: 700;
                                color: var(--text-primary);
                            ">{rejected_pct:.1f}%</div>
                            <div style="font-size: 12px; color: var(--text-secondary);">
                                {rejected_count} actions
                            </div>
                        </div>
                    </div>

                    <div style="display: flex; align-items: center; gap: 10px;">
                        <span style="
                            width: 11px;
                            height: 11px;
                            border-radius: 50%;
                            background: var(--warning);
                            flex-shrink: 0;
                        "></span>

                        <div style="flex: 1;">
                            <div style="font-size: 13px; color: var(--text-secondary);">
                                Escalated
                            </div>
                            <div style="
                                font-size: 19px;
                                font-weight: 700;
                                color: var(--text-primary);
                            ">{escalated_pct:.1f}%</div>
                            <div style="font-size: 12px; color: var(--text-secondary);">
                                {escalated_count} actions
                            </div>
                        </div>
                    </div>

                </div>
            </div>
            """

            st.html(chart_html)

        else:
            st.info("No decisions have been recorded yet.")



    # -----------------------------------------------------
    # DECISION SUMMARY
    # -----------------------------------------------------

    with right_column:

        st.markdown("### Decision summary")

        st.caption(
            "Current distribution of recorded actions."
        )

        summary_items = [
            ("Approved", approved_count, "var(--success)"),
            ("Rejected", rejected_count, "var(--danger)"),
            ("Escalated", escalated_count, "var(--accent)"),
        ]

        for label, value, color in summary_items:

            st.html(f"""
            <div style="
                display: flex;
                justify-content: space-between;
                align-items: center;
                padding: 1rem;
                margin-bottom: 0.65rem;

                background-color: var(--surface);
                border: 1px solid var(--border);
                border-radius: 12px;
            ">

                <span style="
                    color: var(--text-primary);
                    font-weight: 600;
                ">
                    {label}
                </span>

                <span style="
                    color: {color};
                    font-size: 1.25rem;
                    font-weight: 700;
                ">
                    {value}
                </span>

            </div>
            """)

    st.markdown("")

    # -----------------------------------------------------
    # RECENT ACTIVITY
    # -----------------------------------------------------

    st.markdown("### Recent activity")

    st.caption(
        "The latest recorded agent actions."
    )

    if actions_df.empty:

        st.info(
            "No recent activity is available."
        )

        return

    recent_actions = actions_df.head(5).copy()

    render_recent_activity_table(recent_actions)

    st.caption(
        f"Displaying the latest {len(recent_actions)} "
        f"actions out of {total_actions} recorded actions."
    )