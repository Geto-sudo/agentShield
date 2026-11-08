"""Reusable status badge component."""

import streamlit as st


STATUS_CLASSES = {
    "approved": "status-approved",
    "rejected": "status-rejected",
    "pending": "status-pending",
    "escalated": "status-escalated",
    "active": "status-approved",
    "inactive": "status-rejected",
    "expired": "status-rejected",
    "needs review": "status-pending",
}


def render_status_badge(status: str) -> None:
    """
    Display a colored badge representing a status.

    Args:
        status: Status label to display.
    """

    normalized_status = status.strip().lower()

    css_class = STATUS_CLASSES.get(
        normalized_status,
        "status-pending",
    )

    st.markdown(
        f"""
        <span class="status-badge {css_class}">
            {status}
        </span>
        """,
        unsafe_allow_html=True,
    )