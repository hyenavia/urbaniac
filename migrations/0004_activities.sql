CREATE TABLE player_activities (
    player_id   BIGINT  NOT NULL REFERENCES players(discord_user_id),
    activity    TEXT    NOT NULL,
    level       INTEGER NOT NULL DEFAULT 1 CHECK (level >= 1),
    xp          BIGINT  NOT NULL DEFAULT 0 CHECK (xp >= 0),
    overflow_xp BIGINT  NOT NULL DEFAULT 0 CHECK (overflow_xp >= 0),
    PRIMARY KEY (player_id, activity)
);

-- running: sedang berjalan | ready: selesai, reward menunggu diambil | claimed: sudah diambil
-- Slot terpakai selama status running atau ready.
CREATE TABLE activity_runs (
    id           BIGSERIAL PRIMARY KEY,
    player_id    BIGINT      NOT NULL REFERENCES players(discord_user_id),
    activity     TEXT        NOT NULL,
    status       TEXT        NOT NULL CHECK (status IN ('running', 'ready', 'claimed')),
    started_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    ends_at      TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    claimed_at   TIMESTAMPTZ
);
CREATE INDEX idx_runs_player_active ON activity_runs(player_id) WHERE status IN ('running', 'ready');

CREATE TABLE activity_run_rewards (
    run_id   BIGINT NOT NULL REFERENCES activity_runs(id) ON DELETE CASCADE,
    item_id  TEXT   NOT NULL,
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (run_id, item_id)
);
