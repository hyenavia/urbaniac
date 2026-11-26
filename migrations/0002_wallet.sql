CREATE TABLE wallets (
    player_id  BIGINT PRIMARY KEY REFERENCES players(discord_user_id),
    balance    NUMERIC(20,2) NOT NULL DEFAULT 0 CHECK (balance >= 0),
    updated_at TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE TABLE ledger_entries (
    id              BIGSERIAL PRIMARY KEY,
    player_id       BIGINT        NOT NULL REFERENCES players(discord_user_id),
    amount          NUMERIC(20,2) NOT NULL,
    balance_after   NUMERIC(20,2) NOT NULL,
    reason          TEXT          NOT NULL,
    idempotency_key TEXT          NOT NULL UNIQUE,
    created_at      TIMESTAMPTZ   NOT NULL DEFAULT now()
);
CREATE INDEX idx_ledger_player ON ledger_entries(player_id, id DESC);
