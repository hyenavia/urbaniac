-- Kru pemain untuk bonus Efficiency, Yield, Luck dan otomasi aktivitas.
CREATE TABLE player_crews (
    id                BIGSERIAL PRIMARY KEY,
    player_id         BIGINT      NOT NULL REFERENCES players(discord_user_id),
    name              TEXT        NOT NULL,
    rarity            TEXT        NOT NULL CHECK (rarity IN ('common', 'uncommon', 'rare', 'epic')),
    specialty         TEXT        NOT NULL CHECK (specialty IN ('farming', 'gathering', 'fishing', 'mining')),
    level             INTEGER     NOT NULL DEFAULT 1 CHECK (level >= 1 AND level <= 10),
    assigned_activity TEXT,
    auto_enabled      BOOLEAN     NOT NULL DEFAULT false,
    hired_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_crew_player ON player_crews(player_id);
CREATE UNIQUE INDEX uq_crew_assigned_activity
    ON player_crews(player_id, assigned_activity)
    WHERE assigned_activity IS NOT NULL;
