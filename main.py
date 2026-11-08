"""Main Streamlit application for AgentShield."""

import streamlit as st

from styles.theme import apply_theme
from components.sidebar import render_sidebar

from app_pages.dashboard import render_dashboard
from app_pages.mandates import render_mandates
from app_pages.propose_expense import render_propose_action
from app_pages.approvals import render_approvals
from app_pages.history import render_history
from app_pages.wallet import render_wallet


# =========================================================
# 1. APPLICATION CONFIGURATION
# =========================================================

st.set_page_config(
    page_title="AgentShield",
    page_icon="🛡️",
    layout="wide",
    initial_sidebar_state="expanded",
)


# =========================================================
# 2. APPLICATION NAVIGATION
# =========================================================

selected_page = render_sidebar()


# =========================================================
# 3. APPLY SELECTED THEME
# =========================================================

dark_mode_enabled = st.session_state.get(
    "agentshield_dark_mode",
    False,
)

selected_theme = "Dark" if dark_mode_enabled else "Light"

apply_theme(selected_theme)


# =========================================================
# 4. PAGE ROUTING
# =========================================================

PAGE_RENDERERS = {
    "Dashboard": render_dashboard,
    "Spending mandates": render_mandates,
    "Propose an expense": render_propose_action,
    "Human approvals": render_approvals,
    "Activity history": render_history,
    "Simulated wallet": render_wallet,
}


# =========================================================
# 5. RENDER SELECTED PAGE
# =========================================================

page_renderer = PAGE_RENDERERS.get(selected_page)

if page_renderer is not None:

    page_renderer()

else:

    st.error(
        "The selected page could not be found. "
        "Please select another page from the sidebar."
    )


# =========================================================
# 6. APPLICATION FOOTER
# =========================================================

st.html("""
<div style="
    margin-top: 3rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
    text-align: center;
    color: var(--text-secondary);
    font-size: 0.75rem;
">
    AgentShield · AI Agent Spending Control
    <br>
    Sandbox Prototype · No real money involved
</div>
""")