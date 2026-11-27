-- Kota per pemain, dibuat saat /start (plus backfill pemain yang sudah ada).
CREATE TABLE cities (
    player_id  BIGINT PRIMARY KEY REFERENCES players(discord_user_id),
    stage      TEXT          NOT NULL DEFAULT 'kampung',
    treasury   NUMERIC(20,2) NOT NULL DEFAULT 0 CHECK (treasury >= 0),
    created_at TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ   NOT NULL DEFAULT now()
);

INSERT INTO cities (player_id)
SELECT discord_user_id FROM players
ON CONFLICT DO NOTHING;

-- Ledger terpisah untuk City Treasury (terpisah dari Player Wallet).
CREATE TABLE city_treasury_ledger (
    id              BIGSERIAL PRIMARY KEY,
    player_id       BIGINT        NOT NULL REFERENCES players(discord_user_id),
    amount          NUMERIC(20,2) NOT NULL,
    balance_after   NUMERIC(20,2) NOT NULL,
    reason          TEXT          NOT NULL,
    idempotency_key TEXT          NOT NULL UNIQUE,
    created_at      TIMESTAMPTZ   NOT NULL DEFAULT now()
);
CREATE INDEX idx_city_treasury_player ON city_treasury_ledger(player_id, id DESC);

-- Fasilitas kota yang sudah dibangun (hasil penyelesaian proyek).
CREATE TABLE city_facilities (
    player_id   BIGINT      NOT NULL REFERENCES players(discord_user_id),
    facility_id TEXT        NOT NULL,
    level       INTEGER     NOT NULL DEFAULT 1 CHECK (level >= 1),
    built_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, facility_id)
);

-- Proyek pembangunan kota yang sedang berjalan atau sudah selesai.
CREATE TABLE city_projects (
    id           BIGSERIAL PRIMARY KEY,
    player_id    BIGINT      NOT NULL REFERENCES players(discord_user_id),
    project_id   TEXT        NOT NULL,
    status       TEXT        NOT NULL CHECK (status IN ('running', 'completed')),
    started_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    ends_at      TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX uq_city_projects_running ON city_projects(player_id) WHERE status = 'running';
