"""AgentShield expense proposal page."""

import html

import streamlit as st

from components.header import render_header
from data.backend_client import propose_action
from data.data_source import (
    get_categories,
    get_mandates_for_expense_form,
)


DECISION_LABELS = {
    "APPROVE": "Approved",
    "ESCALATE": "Needs review",
    "REFUSE": "Rejected",
}

DECISION_STYLES = {
    "APPROVE": {
        "color": "#16A36A",
        "background": "rgba(22, 163, 106, 0.10)",
        "border": "rgba(22, 163, 106, 0.35)",
        "message": "This expense has been approved.",
    },
    "ESCALATE": {
        "color": "#D99018",
        "background": "rgba(217, 144, 24, 0.10)",
        "border": "rgba(217, 144, 24, 0.35)",
        "message": "This expense requires human review.",
    },
    "REFUSE": {
        "color": "#DC4C4C",
        "background": "rgba(220, 76, 76, 0.10)",
        "border": "rgba(220, 76, 76, 0.35)",
        "message": "This expense has been rejected and cannot proceed.",
    },
}

# Temporary translations for known legacy backend messages.
# These should eventually be corrected at their Rust source.
KNOWN_TRANSLATIONS = {
    "Aucun checkout PayPal n'est créé.":
        "No PayPal checkout has been created.",
}


def translate_known_message(message: str) -> str:
    """Translate known legacy French messages without altering unknown text."""

    return KNOWN_TRANSLATIONS.get(message.strip(), message)


def inject_expense_styles() -> None:
    """Apply visual styles to the expense proposal page."""

    st.html("""
    <style>

        /* INPUT PLACEHOLDERS */

        div[data-testid="stTextInput"] input::placeholder,
        div[data-testid="stTextArea"] textarea::placeholder {
            color: var(--text-secondary) !important;
            opacity: 1 !important;
        }

        /* INPUT FOCUS */

        div[data-testid="stTextInput"] input:focus,
        div[data-testid="stTextArea"] textarea:focus {
            border-color: var(--accent) !important;
            box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.15) !important;
        }

        /* ANALYZE EXPENSE BUTTON */

        div[data-testid="stFormSubmitButton"] button {
            background: linear-gradient(
                110deg,
                var(--primary),
                var(--accent)
            ) !important;
            color: #FFFFFF !important;
            border: 1px solid var(--accent) !important;
            border-radius: 9px !important;
            min-height: 44px;
            font-weight: 650 !important;
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
        }

        div[data-testid="stFormSubmitButton"] button:focus-visible {
            outline: 3px solid var(--accent) !important;
            outline-offset: 3px;
        }

        /* RESULT CARDS */

        .decision-card {
            border-radius: 18px;
            padding: 25px 27px;
            border: 1px solid var(--decision-border);
            background: var(--decision-background);
            margin: 12px 0 22px 0;
        }

        .decision-label {
            color: var(--decision-color);
            font-size: 12px;
            font-weight: 750;
            letter-spacing: 0.11em;
            text-transform: uppercase;
            margin-bottom: 9px;
        }

        .decision-title {
            color: var(--decision-color);
            font-size: 30px;
            line-height: 1.2;
            font-weight: 750;
            margin: 0 0 9px 0;
        }

        .decision-message {
            color: var(--text-primary);
            font-size: 15px;
            line-height: 1.6;
            margin: 0;
        }

        .result-section-card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 16px;
            padding: 20px 22px;
            margin: 12px 0;
        }

        .result-section-title {
            color: var(--text-primary);
            font-size: 16px;
            font-weight: 700;
            margin-bottom: 12px;
        }

        .result-section-content {
            color: var(--text-secondary);
            font-size: 14px;
            line-height: 1.7;
            white-space: pre-wrap;
        }

        .risk-score-number {
            color: var(--text-primary);
            font-size: 27px;
            font-weight: 750;
            margin-bottom: 10px;
        }

        .risk-score-track {
            width: 100%;
            height: 9px;
            border-radius: 20px;
            background: var(--border);
            overflow: hidden;
        }

        .risk-score-fill {
            height: 100%;
            border-radius: 20px;
        }

        .risk-score-caption {
            color: var(--text-secondary);
            font-size: 12px;
            margin-top: 8px;
        }

        .result-list-item {
            color: var(--text-secondary);
            font-size: 14px;
            line-height: 1.6;
            padding: 8px 0;
            border-bottom: 1px solid var(--border);
        }

        .result-list-item:last-child {
            border-bottom: none;
        }

        @media (max-width: 640px) {
            .decision-card {
                padding: 20px;
            }

            .decision-title {
                font-size: 25px;
            }

            .result-section-card {
                padding: 17px;
            }
        }

        @media (prefers-reduced-motion: reduce) {
            div[data-testid="stFormSubmitButton"] button {
                transition: none;
            }
        }

    </style>
    """)


def render_decision_card(result: dict) -> None:
    """Display the main color-coded decision card."""

    decision = str(result.get("decision", "UNKNOWN")).upper()

    label = DECISION_LABELS.get(decision, decision)

    style = DECISION_STYLES.get(
        decision,
        {
            "color": "#64748B",
            "background": "rgba(100, 116, 139, 0.10)",
            "border": "rgba(100, 116, 139, 0.35)",
            "message": "The analysis result is unavailable.",
        },
    )

    safe_label = html.escape(label)
    safe_message = html.escape(style["message"])

    st.html(f"""
    <div class="decision-card"
         style="
            --decision-color: {style['color']};
            --decision-background: {style['background']};
            --decision-border: {style['border']};
         ">

        <div class="decision-label">
            ANALYSIS DECISION
        </div>

        <div class="decision-title">
            {safe_label}
        </div>

        <p class="decision-message">
            {safe_message}
        </p>

    </div>
    """)


def get_risk_score(result: dict):
    """Extract a risk score when the backend provides one."""

    score = result.get("risk_score")

    if score is None:
        risk_data = result.get("risk")

        if isinstance(risk_data, dict):
            score = risk_data.get("score")

    if score is None:
        return None

    try:
        score = float(score)
    except (TypeError, ValueError):
        return None

    return max(0, min(100, score))


def render_risk_score(result: dict) -> None:
    """Display the risk score as a visual progress indicator."""

    score = get_risk_score(result)

    if score is None:
        return

    if score < 30:
        color = "#16A36A"
    elif score < 70:
        color = "#D99018"
    else:
        color = "#DC4C4C"

    st.html(f"""
    <div class="result-section-card">

        <div class="result-section-title">
            Risk score
        </div>

        <div class="risk-score-number">
            {score:g} <span style="font-size:15px; color:var(--text-secondary);">
                / 100
            </span>
        </div>

        <div class="risk-score-track">
            <div class="risk-score-fill"
                 style="width:{score}%; background:{color};">
            </div>
        </div>

        <div class="risk-score-caption">
            Risk assessment provided by AgentShield.
        </div>

    </div>
    """)


def render_result_section(title: str, content: str) -> None:
    """Display a secondary result card."""

    safe_title = html.escape(title)
    safe_content = html.escape(content)

    st.html(f"""
    <div class="result-section-card">

        <div class="result-section-title">
            {safe_title}
        </div>

        <div class="result-section-content">
            {safe_content}
        </div>

    </div>
    """)


def render_result_list(title: str, items: list) -> None:
    """Display a list of backend messages inside a result card."""

    if not items:
        return

    rendered_items = ""

    for item in items:
        translated_item = translate_known_message(str(item))
        safe_item = html.escape(translated_item)

        rendered_items += f"""
        <div class="result-list-item">
            {safe_item}
        </div>
        """

    safe_title = html.escape(title)

    st.html(f"""
    <div class="result-section-card">

        <div class="result-section-title">
            {safe_title}
        </div>

        {rendered_items}

    </div>
    """)


def render_analysis_result() -> None:
    """Render the complete expense analysis result."""

    result = st.session_state.get("last_proposal_result")

    if not result:
        return

    st.divider()
    st.markdown("### Analysis result")

    render_decision_card(result)
    render_risk_score(result)

    explanation = result.get("explanation", "")

    if explanation:
        render_result_section(
            "Decision explanation",
            translate_known_message(str(explanation)),
        )

    reasons = result.get("reasons", [])

    if reasons:
        render_result_list("Risk signals", reasons)

    next_steps = result.get("next_steps", [])

    if next_steps:
        render_result_list(
            "Recommended next steps",
            next_steps,
        )

    st.caption(
        "This analysis is performed in the AgentShield "
        "Sandbox environment."
    )


def render_propose_action() -> None:
    """Render the expense proposal form."""

    inject_expense_styles()

    render_header(
        title="Propose an expense",
        eyebrow="SPENDING CONTROL",
        subtitle=(
            "Submit an AI agent purchase request for "
            "automated risk analysis."
        ),
    )

    st.info(
        "Every proposed expense is evaluated against its "
        "spending mandate before any payment can proceed."
    )

    # LOAD MANDATES

    try:
        mandates = get_mandates_for_expense_form()
        categories = get_categories()

    except RuntimeError as error:
        st.error(f"Unable to load spending mandates: {error}")
        return

    active_mandates = [
        mandate
        for mandate in mandates
        if mandate["status"].lower() == "active"
    ]

    if not active_mandates:
        st.warning(
            "No active spending mandate is available. "
            "Create or activate a mandate first."
        )
        return

    # MANDATE SELECTION

    st.markdown("### 1. Select a spending mandate")

    mandate_labels = {
        (
            f'{mandate["agent_name"]} — '
            f'{mandate["category_label"]} '
            f'({mandate["total_limit"]:.2f} '
            f'{mandate["currency"]})'
        ): mandate
        for mandate in active_mandates
    }

    with st.form("expense_proposal_form"):

        selected_label = st.selectbox(
            "Active mandate",
            options=list(mandate_labels.keys()),
        )

        selected_mandate = mandate_labels[selected_label]

        st.markdown("### 2. Purchase details")

        first_column, second_column = st.columns(2)

        with first_column:

            merchant = st.text_input(
                "Merchant",
                placeholder="Example: TechStore",
            )

            amount = st.number_input(
                "Amount (€)",
                min_value=0.01,
                value=10.00,
                step=1.00,
            )

            expense_type = st.text_input(
                "Expense type",
                value=selected_mandate["intent"],
                placeholder="Example: Computer equipment",
            )

        with second_column:

            merchant_country = st.text_input(
                "Merchant country",
                value="FR",
                help="Use a two-letter country code.",
            )

            delivery_days = st.number_input(
                "Expected delivery (days)",
                min_value=0,
                max_value=365,
                value=3,
                step=1,
            )

            currency = selected_mandate["currency"]

            st.text_input(
                "Currency",
                value=currency,
                disabled=True,
            )

        st.markdown("### 3. Additional information")

        description = st.text_area(
            "Purchase description",
            placeholder=(
                "Explain what the agent wants to purchase "
                "and why."
            ),
            height=120,
        )

        submitted = st.form_submit_button(
            "Analyze expense",
            type="primary",
            use_container_width=True,
        )

        if submitted:

            if not merchant.strip():
                st.error("Please enter a merchant name.")

            elif not expense_type.strip():
                st.error("Please enter an expense type.")

            elif not merchant_country.strip():
                st.error("Please enter the merchant country.")

            else:

                purchase_description = expense_type.strip()

                if description.strip():
                    purchase_description += (
                        "\n\n" + description.strip()
                    )

                payload = {
                    "mandate_id": selected_mandate["id"],
                    "agent_id": selected_mandate["agent_id"],
                    "kind": "purchase",
                    "merchant": merchant.strip(),
                    "merchant_country": merchant_country.strip().upper(),
                    "amount_cents": round(amount * 100),
                    "currency": currency,
                    "category": selected_mandate["category"],
                    "delivery_days": int(delivery_days),
                    "description": purchase_description,
                }

                try:

                    result = propose_action(payload)

                    st.session_state[
                        "last_proposal_result"
                    ] = result

                    st.rerun()

                except RuntimeError as error:

                    st.error(
                        f"Unable to analyze expense: {error}"
                    )

    # ANALYSIS RESULT

    render_analysis_result()