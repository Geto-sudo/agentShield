"""Fictional demonstration data for the AgentShield application.

This module contains sample data only.
It does not contain any Streamlit interface logic.
All records in this module are fictional.
"""


# =========================================================
# 1. DEMONSTRATION AGENT ACTIONS
# =========================================================

DEMO_ACTIONS = [
    {
        "Agent": "Shopping-Agent-01",
        "Action": "Office supplies purchase",
        "Amount (€)": 24.50,
        "Decision": "Approved",
        "Reason": "Compliant with spending mandate",
        "Date": "2026-11-04 14:32",
    },
    {
        "Agent": "Travel-Agent-02",
        "Action": "Hotel reservation",
        "Amount (€)": 48.00,
        "Decision": "Pending",
        "Reason": "Human approval required",
        "Date": "2026-11-04 13:15",
    },
    {
        "Agent": "Tech-Agent-03",
        "Action": "Headphones purchase",
        "Amount (€)": 62.00,
        "Decision": "Rejected",
        "Reason": "Fictional spending limit exceeded",
        "Date": "2026-11-04 11:05",
    },
    {
        "Agent": "Shopping-Agent-01",
        "Action": "Keyboard purchase",
        "Amount (€)": 35.00,
        "Decision": "Approved",
        "Reason": "Compliant with spending mandate",
        "Date": "2026-11-03 09:40",
    },
    {
        "Agent": "Travel-Agent-02",
        "Action": "Transportation booking",
        "Amount (€)": 180.00,
        "Decision": "Rejected",
        "Reason": "Unauthorized category",
        "Date": "2026-11-03 08:25",
    },
    {
        "Agent": "Office-Agent-04",
        "Action": "Paper purchase",
        "Amount (€)": 18.00,
        "Decision": "Approved",
        "Reason": "Compliant with spending mandate",
        "Date": "2026-11-02 16:10",
    },
    {
        "Agent": "Tech-Agent-03",
        "Action": "Monitor purchase",
        "Amount (€)": 240.00,
        "Decision": "Pending",
        "Reason": "Human approval required in the simulation",
        "Date": "2026-11-02 10:20",
    },
    {
        "Agent": "Office-Agent-04",
        "Action": "Pen purchase",
        "Amount (€)": 12.00,
        "Decision": "Approved",
        "Reason": "Compliant with spending mandate",
        "Date": "2026-11-01 15:45",
    },
]


# =========================================================
# 2. DEMONSTRATION SPENDING MANDATES
# =========================================================

DEMO_MANDATES = [
    {
        "Agent": "Shopping-Agent-01",
        "Category": "Office supplies",
        "Spending limit (€)": 100.00,
        "Status": "Active",
    },
    {
        "Agent": "Travel-Agent-02",
        "Category": "Transportation",
        "Spending limit (€)": 200.00,
        "Status": "Active",
    },
]


# =========================================================
# 3. SIMULATED WALLET TRANSACTIONS
# =========================================================

DEMO_WALLET_TRANSACTIONS = [
    {
        "Date": "2026-11-04",
        "Description": "Initial fictional deposit",
        "Amount (€)": 2500.00,
        "Type": "Simulated credit",
    },
    {
        "Date": "2026-11-03",
        "Description": "Office supplies purchase",
        "Amount (€)": -35.00,
        "Type": "Simulated transaction",
    },
    {
        "Date": "2026-11-02",
        "Description": "Paper purchase",
        "Amount (€)": -18.00,
        "Type": "Simulated transaction",
    },
]


# =========================================================
# 4. DEMONSTRATION SETTINGS
# =========================================================

DEMO_INITIAL_BALANCE = 2500.00


# =========================================================
# 5. DEMONSTRATION EXPENSE CATEGORIES
# =========================================================

DEMO_CATEGORIES = [
    "Office supplies",
    "Clothing",
    "Computer equipment",
    "Transportation",
    "Accommodation",
    "Other",
]