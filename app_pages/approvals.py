"""AgentShield human approvals page."""

import streamlit as st

from components.header import render_header
from components.status_badge import render_status_badge

from data.backend_client import (
    confirm_approval,
    deny_approval,
    get_actions,
    get_agents,
    get_approvals,
)


def inject_approval_styles() -> None:
    """Apply theme-aware styles to the approval page."""

    st.markdown(
        """
        <style>
        /* Review note input */
        div[data-testid="stTextArea"] textarea {
            color: var(--text-primary) !important;
            background-color: var(--input-background) !important;
            caret-color: var(--accent) !important;
            border-color: var(--border) !important;
        }

        /* Placeholder */
        div[data-testid="stTextArea"] textarea::placeholder {
            color: var(--text-secondary) !important;
            opacity: 1 !important;
        }

        /* Focus state */
        div[data-testid="stTextArea"] textarea:focus {
            border-color: var(--accent) !important;
            box-shadow: 0 0 0 1px var(--accent) !important;
        }

        /* Review note label */
        div[data-testid="stTextArea"] label {
            color: var(--text-primary) !important;
        }
        </style>
        """,
        unsafe_allow_html=True,
    )


def _get_nested_value(data: dict, *keys, default=""):
    """Safely retrieve a nested dictionary value."""

    current = data

    for key in keys:
        if not isinstance(current, dict):
            return default

        current = current.get(key)

    return current if current is not None else default


def _get_agent_names() -> dict[str, str]:
    """Build an agent ID to name mapping."""

    agents = get_agents()

    return {
        agent["agent"]["id"]: agent["agent"]["name"]
        for agent in agents
        if isinstance(agent, dict) and isinstance(
            agent.get("agent"), dict
        )
    }


def render_approvals() -> None:
    """Render the human approval management page."""

    inject_approval_styles()

    render_header(
        title="Human approvals",
        eyebrow="HUMAN OVERSIGHT",
        subtitle=(
            "Review spending requests that require "
            "human authorization."
        ),
    )

    st.info(
        "Confirming an approval authorizes the corresponding "
        "Sandbox workflow. It does not automatically mean "
        "that a real payment has been made."
    )

    # -----------------------------------------------------
    # LOAD APPROVAL DATA
    # -----------------------------------------------------

    try:
        approvals = get_approvals(
            status="pending",
            limit=50,
        )

        actions = get_actions()
        agent_names = _get_agent_names()

    except RuntimeError as error:
        st.error(f"Unable to load approval data: {error}")
        return

    if not approvals:
        st.success("There are no pending approvals.")
        return

    actions_by_id = {
        action.get("id"): action
        for action in actions
        if action.get("id")
    }

    st.markdown("### Requests awaiting your decision")

    st.caption(
        f"{len(approvals)} request(s) currently require "
        "human review."
    )

    # -----------------------------------------------------
    # APPROVAL CARDS
    # -----------------------------------------------------

    for index, approval in enumerate(approvals):

        approval_id = approval.get("id", "")

        action_id = approval.get("action_id", "")

        action = actions_by_id.get(action_id, {})

        agent_id = (
            action.get("agent_id")
            or approval.get("agent_id")
            or ""
        )

        agent_name = agent_names.get(
            agent_id,
            "Unknown agent",
        )

        merchant = action.get(
            "merchant",
            approval.get("merchant", "Unknown merchant"),
        )

        amount_cents = action.get(
            "amount_cents",
            approval.get("amount_cents", 0),
        )

        currency = action.get(
            "currency",
            approval.get("currency", "EUR"),
        )

        description = action.get(
            "description",
            approval.get("description", ""),
        )

        explanation = action.get(
            "explanation",
            approval.get("explanation", ""),
        )

        decision = action.get("decision", "ESCALATE")

        with st.container(border=True):

            first_column, second_column = st.columns(
                [3, 1]
            )

            with first_column:
                st.markdown(f"### {merchant}")

                st.caption(
                    f"Approval ID: {approval_id}"
                )

            with second_column:
                st.markdown(
                    f"""
                    <div style="
                        text-align:right;
                        font-size:1.45rem;
                        font-weight:700;
                        color:#142B4A;
                    ">
                        {amount_cents / 100:.2f} {currency}
                    </div>
                    """,
                    unsafe_allow_html=True,
                )

            st.markdown("---")

            detail_left, detail_right = st.columns(2)

            with detail_left:
                st.markdown("**AI agent**")
                st.write(agent_name)

                st.markdown("**Engine decision**")
                render_status_badge(
                    "Escalated"
                    if decision == "ESCALATE"
                    else decision
                )

            with detail_right:
                st.markdown("**Description**")
                st.write(description or "No description provided.")

                expiration = approval.get(
                    "expires_at",
                    "Not specified",
                )

                st.markdown("**Expiration**")
                st.write(expiration)

            if explanation:
                st.markdown("**Risk explanation**")
                st.write(explanation)

            note = st.text_area(
                "Review note",
                key=f"approval_note_{approval_id}",
                placeholder="Add an optional decision note...",
            )

            approve_column, reject_column = st.columns(2)

            with approve_column:
                confirm_clicked = st.button(
                    "Confirm approval",
                    key=f"confirm_{approval_id}",
                    type="primary",
                    use_container_width=True,
                )

            with reject_column:
                deny_clicked = st.button(
                    "Reject request",
                    key=f"deny_{approval_id}",
                    use_container_width=True,
                )

            if confirm_clicked:

                try:
                    result = confirm_approval(
                        approval_id,
                        decided_by="Owner",
                        note=note,
                    )

                    st.session_state["approval_message"] = (
                        "Approval confirmed successfully."
                    )

                    st.rerun()

                except RuntimeError as error:
                    st.error(
                        f"Unable to confirm approval: {error}"
                    )

            if deny_clicked:

                try:
                    result = deny_approval(
                        approval_id,
                        decided_by="Owner",
                        note=note,
                    )

                    st.session_state["approval_message"] = (
                        "Approval rejected successfully."
                    )

                    st.rerun()

                except RuntimeError as error:
                    st.error(
                        f"Unable to reject approval: {error}"
                    )

    message = st.session_state.pop(
        "approval_message",
        None,
    )

    if message:
        st.success(message)