"""Reusable metric card component."""

import streamlit as st


def render_metric_card(
    label: str,
    value: str | int | float,
    description: str = "",
    accent_color: str | None = None,
) -> None:
    """Display a reusable metric card."""

    description_html = ""

    if description:
        description_html = f"""
        <div class="metric-card-description">
            {description}
        </div>
        """

    value_style = ""

    if accent_color:
        value_style = f"color: {accent_color};"

    card_html = f"""
    <div class="metric-card">

        <div class="metric-card-label">
            {label}
        </div>

        <div class="metric-card-value" style="{value_style}">
            {value}
        </div>

        {description_html}

    </div>
    """

    st.html(card_html)