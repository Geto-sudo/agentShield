"""Reusable page header component."""

import streamlit as st


def render_header(
    title: str,
    subtitle: str = "",
    eyebrow: str = "",
) -> None:
    """Display a consistent page header."""

    if eyebrow:
        st.html(f"""
        <div style="
            color: var(--accent);
            font-size: 0.75rem;
            font-weight: 700;
            letter-spacing: 0.08em;
            margin-top: 0.5rem;
            margin-bottom: 0.55rem;
            text-transform: uppercase;
        ">
            {eyebrow}
        </div>
        """)

    st.title(title)

    if subtitle:
        st.html(f"""
        <p style="
            color: var(--text-secondary);
            font-size: 0.95rem;
            margin-top: -0.5rem;
            margin-bottom: 1.5rem;
        ">
            {subtitle}
        </p>
        """)