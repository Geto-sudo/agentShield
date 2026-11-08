"""Central design system for the AgentShield application."""

import streamlit as st


THEMES = {
    "Light": {
        "primary": "#142B4A",
        "accent": "#2563EB",
        "background": "#F5F7FA",
        "surface": "#FFFFFF",
        "surface_secondary": "#EEF2F7",
        "text_primary": "#172337",
        "text_secondary": "#64748B",
        "border": "#E5EAF0",
        "success": "#16A36A",
        "warning": "#D99018",
        "danger": "#DC4C4C",
        "sidebar": "#142B4A",
        "input_background": "#FFFFFF",
        "button_text": "#FFFFFF",
        "shadow": "0 2px 8px rgba(20, 43, 74, 0.04)",
    },
    "Dark": {
        "primary": "#080F1C",
        "accent": "#3B82F6",
        "background": "#0B1220",
        "surface": "#172337",
        "surface_secondary": "#202F45",
        "text_primary": "#F1F5F9",
        "text_secondary": "#B4C0D0",
        "border": "#33445C",
        "success": "#34D399",
        "warning": "#FBBF24",
        "danger": "#FB7185",
        "sidebar": "#080F1C",
        "input_background": "#172337",
        "button_text": "#FFFFFF",
        "shadow": "0 2px 10px rgba(0, 0, 0, 0.18)",
    },
}

# Compatibility with components that still import COLORS.
COLORS = THEMES["Light"]

RADIUS = {
    "small": "8px",
    "medium": "12px",
    "large": "16px",
}


def apply_theme(theme_name: str = "Light") -> None:
    """Apply the selected AgentShield visual theme."""

    if theme_name not in THEMES:
        theme_name = "Light"

    theme = THEMES[theme_name]

    css = f"""
    <style>

    :root {{
        color-scheme: {"dark" if theme_name == "Dark" else "light"};

        --primary: {theme["primary"]};
        --accent: {theme["accent"]};
        --background: {theme["background"]};
        --surface: {theme["surface"]};
        --surface-secondary: {theme["surface_secondary"]};
        --text-primary: {theme["text_primary"]};
        --text-secondary: {theme["text_secondary"]};
        --border: {theme["border"]};
        --success: {theme["success"]};
        --warning: {theme["warning"]};
        --danger: {theme["danger"]};
        --sidebar: {theme["sidebar"]};
        --input-background: {theme["input_background"]};
        --button-text: {theme["button_text"]};
    }}

    /* GLOBAL BACKGROUND */

    html, body, .stApp,
    [data-testid="stAppViewContainer"],
    [data-testid="stMain"] {{
        background-color: var(--background) !important;
        color: var(--text-primary) !important;
    }}

    .block-container {{
        max-width: 1440px;
        padding: 3.5rem 3rem 3rem;
    }}

    [data-testid="stHeader"] {{
        background-color: var(--background) !important;
    }}

    .stDeployButton {{
        display: none !important;
    }}

    /* SIDEBAR */

    [data-testid="stSidebar"] {{
        background-color: var(--sidebar) !important;
        border-right: 1px solid rgba(255,255,255,0.08);
    }}

    [data-testid="stSidebar"] [data-testid="stMarkdownContainer"],
    [data-testid="stSidebar"] [data-testid="stWidgetLabel"],
    [data-testid="stSidebar"] label,
    [data-testid="stSidebar"] p,
    [data-testid="stSidebar"] span {{
        color: #FFFFFF !important;
    }}

    [data-testid="stSidebar"] hr {{
        border-color: rgba(255,255,255,0.15);
    }}

    /* HEADINGS AND TEXT */

    h1, h2, h3, h4, h5, h6 {{
        color: var(--text-primary) !important;
    }}

    h1 {{
        font-size: 2.2rem;
        font-weight: 700;
        letter-spacing: -0.04em;
    }}

    h2 {{
        font-size: 1.5rem;
        font-weight: 650;
        letter-spacing: -0.025em;
    }}

    h3 {{
        font-size: 1.15rem;
        font-weight: 600;
    }}

    p, li {{
        color: var(--text-secondary);
    }}

    [data-testid="stMarkdownContainer"] p,
    [data-testid="stCaptionContainer"],
    [data-testid="stCaptionContainer"] p {{
        color: var(--text-secondary);
    }}

    [data-testid="stMarkdownContainer"] strong {{
        color: var(--text-primary);
    }}

        /* SIDEBAR TOGGLE LABEL */

    [data-testid="stSidebar"] [data-testid="stToggle"] label,
    [data-testid="stSidebar"] [data-testid="stToggle"] label p,
    [data-testid="stSidebar"] [data-testid="stToggle"] label span {{
        color: #FFFFFF !important;
    }}

    /* METRIC CARDS */

    .metric-card {{
        background-color: var(--surface);
        border: 1px solid var(--border);
        border-radius: {RADIUS["large"]};
        padding: 1.25rem;
        min-height: 125px;
        box-shadow: {theme["shadow"]};
        transition: border-color 0.2s ease, transform 0.2s ease;
    }}

    .metric-card:hover {{
        border-color: var(--accent);
        transform: translateY(-2px);
    }}

    .metric-card-label {{
        color: var(--text-secondary) !important;
        font-size: 0.78rem;
        font-weight: 600;
        letter-spacing: 0.045em;
        text-transform: uppercase;
    }}

    .metric-card-value {{
        color: var(--text-primary) !important;
        font-size: 1.9rem;
        font-weight: 700;
        margin-top: 0.55rem;
        letter-spacing: -0.04em;
    }}

    .metric-card-description {{
        color: var(--text-secondary) !important;
        font-size: 0.8rem;
        margin-top: 0.35rem;
    }}

    /* NATIVE STREAMLIT METRICS */

    [data-testid="stMetric"] {{
        background-color: var(--surface);
        border: 1px solid var(--border);
        border-radius: {RADIUS["medium"]};
        padding: 1rem;
    }}

    [data-testid="stMetricLabel"],
    [data-testid="stMetricLabel"] p {{
        color: var(--text-secondary) !important;
    }}

    [data-testid="stMetricValue"],
    [data-testid="stMetricValue"] div {{
        color: var(--text-primary) !important;
    }}

    /* BUTTONS */

    .stButton > button,
    [data-testid="stFormSubmitButton"] > button {{
        min-height: 2.6rem;
        padding: 0.45rem 1rem;
        border-radius: {RADIUS["small"]};
        font-weight: 600;
        border: 1px solid var(--border);
        background-color: var(--surface);
        color: var(--text-primary) !important;
        transition: all 0.2s ease;
    }}

    .stButton > button p,
    [data-testid="stFormSubmitButton"] > button p {{
        color: inherit !important;
    }}

    .stButton > button:hover,
    [data-testid="stFormSubmitButton"] > button:hover {{
        border-color: var(--accent);
    }}

    .stButton > button[kind="primary"],
    [data-testid="stFormSubmitButton"] button[kind="primary"] {{
        background-color: var(--accent) !important;
        border: 1px solid var(--accent) !important;
        color: #FFFFFF !important;
    }}

    .stButton > button[kind="primary"] *,
    [data-testid="stFormSubmitButton"] button[kind="primary"] * {{
        color: #FFFFFF !important;
    }}

    /* INPUTS */

    .stTextInput input,
    .stNumberInput input,
    .stTextArea textarea,
    .stDateInput input,
    .stTimeInput input {{
        background-color: var(--input-background) !important;
        color: var(--text-primary) !important;
        border-color: var(--border) !important;
        border-radius: {RADIUS["small"]};
    }}

    .stTextInput label,
    .stNumberInput label,
    .stTextArea label,
    .stSelectbox label,
    .stDateInput label,
    .stTimeInput label,
    .stRadio label,
    .stCheckbox label {{
        color: var(--text-primary) !important;
    }}


            /* SELECTBOXES AND DROPDOWN MENUS */

    [data-testid="stSelectbox"] [data-baseweb="select"] > div {{
        background-color: var(--input-background) !important;
        border: 1px solid var(--border) !important;
        color: var(--text-primary) !important;
    }}

    [data-testid="stSelectbox"] [data-baseweb="select"] > div > div {{
        background-color: transparent !important;
        color: var(--text-primary) !important;
    }}

    [data-testid="stSelectbox"] [data-baseweb="select"] span {{
        color: var(--text-primary) !important;
    }}

    [data-testid="stSelectbox"] [data-baseweb="select"] input {{
        background-color: transparent !important;
        color: var(--text-primary) !important;
    }}

    [data-testid="stSelectbox"] [data-baseweb="select"] svg {{
        fill: var(--text-secondary) !important;
    }}

    [data-baseweb="popover"],
    [data-baseweb="menu"],
    [role="listbox"] {{
        background-color: var(--surface) !important;
        border-color: var(--border) !important;
    }}

    [role="option"] {{
        background-color: var(--surface) !important;
        color: var(--text-primary) !important;
    }}

    [role="option"]:hover {{
        background-color: var(--surface-secondary) !important;
    }}


    /* NUMBER INPUT + AND - BUTTONS */

    [data-testid="stNumberInput"] button {{
        background-color: var(--surface-secondary) !important;
        color: var(--text-primary) !important;
        border-color: var(--border) !important;
    }}

    [data-testid="stNumberInput"] button:hover {{
        background-color: var(--accent) !important;
        color: #FFFFFF !important;
    }}

    /* FORMS */

    [data-testid="stForm"] {{
        background-color: var(--surface) !important;
        border: 1px solid var(--border);
        border-radius: {RADIUS["medium"]};
        padding: 1rem;
    }}

    /* DATA TABLES */

    [data-testid="stDataFrame"],
    [data-testid="stTable"] {{
        background-color: var(--surface) !important;
        border: 1px solid var(--border);
        border-radius: {RADIUS["medium"]};
        overflow: hidden;
    }}

    /* ALERTS */

    [data-testid="stAlert"] {{
        border-radius: {RADIUS["medium"]};
    }}

    /* EXPANDERS */

    [data-testid="stExpander"] {{
        background-color: var(--surface) !important;
        border: 1px solid var(--border);
        border-radius: {RADIUS["medium"]};
    }}

    [data-testid="stExpander"] summary,
    [data-testid="stExpander"] summary span {{
        color: var(--text-primary) !important;
    }}

    /* DIVIDERS */

    hr {{
        border-color: var(--border);
        margin-top: 1.5rem;
        margin-bottom: 1.5rem;
    }}

    /* RESPONSIVE */

    @media (max-width: 900px) {{
        .block-container {{
            padding-left: 1.25rem;
            padding-right: 1.25rem;
        }}

        h1 {{
            font-size: 1.8rem;
        }}
    }}

    @media (max-width: 600px) {{
        .block-container {{
            padding-left: 0.8rem;
            padding-right: 0.8rem;
        }}

        .metric-card {{
            padding: 1rem;
        }}
    }}

    </style>
    """

    st.markdown(css, unsafe_allow_html=True)