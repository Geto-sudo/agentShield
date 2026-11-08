"""Reusable AgentShield sidebar navigation."""

import streamlit as st


NAVIGATION_OPTIONS = [
    "Dashboard",
    "Spending mandates",
    "Propose an expense",
    "Human approvals",
    "Activity history",
    "Simulated wallet",
]


def render_sidebar() -> str:
    """Render the application sidebar and return the selected page."""

    with st.sidebar:

        # =====================================
        # BRANDING
        # =====================================

        st.html("""
        <div style="padding: 0.8rem 0 1.2rem 0;">

            <div style="
                font-size: 1.55rem;
                font-weight: 750;
                letter-spacing: -0.04em;
                color: white;
            ">
                🛡️ AgentShield
            </div>

            <div style="
                font-size: 0.75rem;
                color: #CBD5E1;
                margin-top: 0.3rem;
                letter-spacing: 0.04em;
            ">
                AI AGENT SPENDING CONTROL
            </div>

        </div>
        """)

        st.divider()

        # =====================================
        # WORKSPACE NAVIGATION
        # =====================================

        st.html("""
        <div style="
            font-size: 0.72rem;
            font-weight: 700;
            color: #94A3B8;
            letter-spacing: 0.08em;
            margin-bottom: 0.6rem;
        ">
            WORKSPACE
        </div>
        """)

        selected_page = st.radio(
            "Navigation",
            NAVIGATION_OPTIONS,
            label_visibility="collapsed",
            key="agentshield_navigation",
        )

        st.divider()

        # =====================================
        # APPEARANCE SETTINGS
        # =====================================

        st.html("""
        <div style="
            font-size: 0.72rem;
            font-weight: 700;
            color: #94A3B8;
            letter-spacing: 0.08em;
            margin-bottom: 0.7rem;
        ">
            APPEARANCE
        </div>
        """)

        st.toggle(
            "Dark mode",
            value=False,
            key="agentshield_dark_mode",
            help="Switch between light and dark appearance.",
        )

        st.divider()

        # =====================================
        # ENVIRONMENT INFORMATION
        # =====================================

        st.html("""
        <div style="
            font-size: 0.72rem;
            color: #CBD5E1;
            line-height: 1.7;
        ">
            <strong style="color:white;">
                Sandbox environment
            </strong>
            <br>
            Prototype demonstration
            <br>
            No real money involved
        </div>
        """)

    return selected_page