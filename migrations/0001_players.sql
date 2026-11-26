CREATE TABLE players (
    discord_user_id BIGINT PRIMARY KEY,
    username        TEXT        NOT NULL,
    avatar_url      TEXT,
    level           INTEGER     NOT NULL DEFAULT 1 CHECK (level >= 1),
    xp              BIGINT      NOT NULL DEFAULT 0 CHECK (xp >= 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
