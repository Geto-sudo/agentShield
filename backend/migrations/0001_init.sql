-- AgentShield - schéma SQLite (prototype Sandbox, données simulées)
-- Toutes les sommes sont stockées en centimes (INTEGER) pour éviter les flottants.
-- Toutes les dates sont stockées en RFC 3339 UTC (TEXT).

CREATE TABLE IF NOT EXISTS agents (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    owner       TEXT NOT NULL,
    purpose     TEXT NOT NULL,
    status      TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mandates (
    id                      TEXT PRIMARY KEY,
    agent_id                TEXT NOT NULL REFERENCES agents(id),
    owner_name              TEXT NOT NULL,
    intent                  TEXT NOT NULL,
    category                TEXT NOT NULL,
    currency                TEXT NOT NULL,
    max_total_cents         INTEGER NOT NULL,
    per_txn_max_cents       INTEGER NOT NULL,
    auto_approve_max_cents  INTEGER NOT NULL,
    escalate_until_cents    INTEGER NOT NULL,
    allowed_merchants       TEXT NOT NULL DEFAULT '[]',
    blocked_merchants       TEXT NOT NULL DEFAULT '[]',
    allowed_countries       TEXT NOT NULL DEFAULT '[]',
    max_delivery_days       INTEGER NOT NULL,
    valid_from              TEXT NOT NULL,
    valid_until             TEXT NOT NULL,
    status                  TEXT NOT NULL,
    created_at              TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mandates_agent ON mandates(agent_id);

CREATE TABLE IF NOT EXISTS actions (
    id              TEXT PRIMARY KEY,
    mandate_id      TEXT NOT NULL,
    agent_id        TEXT NOT NULL,
    kind            TEXT NOT NULL,
    merchant        TEXT NOT NULL,
    merchant_country TEXT,
    amount_cents    INTEGER NOT NULL,
    currency        TEXT NOT NULL,
    category        TEXT NOT NULL,
    delivery_days   INTEGER,
    description     TEXT NOT NULL,
    idempotency_key TEXT,
    occurred_at     TEXT,
    metadata        TEXT,
    raw_log         TEXT NOT NULL,
    decision        TEXT NOT NULL,
    risk_score      INTEGER NOT NULL,
    risk_band       TEXT NOT NULL,
    checks          TEXT NOT NULL,
    reasons         TEXT NOT NULL,
    explanation     TEXT NOT NULL,
    approval_id     TEXT,
    order_id        TEXT,
    settled         INTEGER NOT NULL DEFAULT 0,
    audit_hash      TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_actions_agent   ON actions(agent_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_actions_mandate ON actions(mandate_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_actions_idem    ON actions(mandate_id, idempotency_key);
CREATE INDEX IF NOT EXISTS idx_actions_decision ON actions(decision);

CREATE TABLE IF NOT EXISTS approvals (
    id           TEXT PRIMARY KEY,
    action_id    TEXT NOT NULL,
    status       TEXT NOT NULL,
    requested_at TEXT NOT NULL,
    expires_at   TEXT NOT NULL,
    decided_at   TEXT,
    decided_by   TEXT,
    note         TEXT,
    created_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_approvals_status ON approvals(status);

CREATE TABLE IF NOT EXISTS orders (
    id              TEXT PRIMARY KEY,
    action_id       TEXT,
    purpose         TEXT NOT NULL,
    paypal_order_id TEXT NOT NULL UNIQUE,
    status          TEXT NOT NULL,
    amount_cents    INTEGER NOT NULL,
    currency        TEXT NOT NULL,
    description     TEXT NOT NULL,
    approve_url     TEXT,
    capture_id      TEXT,
    mode            TEXT NOT NULL,
    failure_reason  TEXT,
    created_at      TEXT NOT NULL,
    captured_at     TEXT
);

CREATE TABLE IF NOT EXISTS wallets (
    agent_id                     TEXT PRIMARY KEY,
    balance_cents                INTEGER NOT NULL,
    premium_cents                INTEGER NOT NULL,
    bonus_malus_bps              INTEGER NOT NULL,
    reserve_contribution_cents   INTEGER NOT NULL,
    spent_this_period_cents      INTEGER NOT NULL,
    updated_at                   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ledger (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id            TEXT NOT NULL,
    kind                TEXT NOT NULL,
    amount_cents        INTEGER NOT NULL,
    balance_after_cents INTEGER NOT NULL,
    label               TEXT NOT NULL,
    reference           TEXT,
    created_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ledger_agent ON ledger(agent_id, id DESC);

CREATE TABLE IF NOT EXISTS reserve (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    balance_cents INTEGER NOT NULL,
    currency      TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS reserve_ledger (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    kind                TEXT NOT NULL,
    amount_cents        INTEGER NOT NULL,
    balance_after_cents INTEGER NOT NULL,
    label               TEXT NOT NULL,
    reference           TEXT,
    created_at          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS incidents (
    id           TEXT PRIMARY KEY,
    agent_id     TEXT NOT NULL,
    mandate_id   TEXT NOT NULL,
    action_id    TEXT,
    kind         TEXT NOT NULL,
    status       TEXT NOT NULL,
    description  TEXT NOT NULL,
    amount_cents INTEGER NOT NULL,
    currency     TEXT NOT NULL,
    overseen_by  TEXT NOT NULL,
    verdict      TEXT,
    payout_id    TEXT,
    created_at   TEXT NOT NULL,
    resolved_at  TEXT
);

CREATE TABLE IF NOT EXISTS payouts (
    id              TEXT PRIMARY KEY,
    incident_id     TEXT,
    paypal_batch_id TEXT NOT NULL,
    amount_cents    INTEGER NOT NULL,
    currency        TEXT NOT NULL,
    status          TEXT NOT NULL,
    mode            TEXT NOT NULL,
    recipient_email TEXT NOT NULL,
    note            TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    completed_at    TEXT
);

CREATE TABLE IF NOT EXISTS audit_log (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    seq     INTEGER NOT NULL UNIQUE,
    ts      TEXT NOT NULL,
    kind    TEXT NOT NULL,
    subject TEXT NOT NULL,
    payload TEXT NOT NULL,
    prev_hash TEXT NOT NULL,
    hash      TEXT NOT NULL,
    -- Signature Ed25519 du hash (base64) et clé publique qui l'a produite.
    -- NULL uniquement pour les entrées écrites avant l'introduction de la signature.
    signature TEXT,
    pubkey    TEXT
);

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
