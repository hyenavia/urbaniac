CREATE TABLE guild_welcome_configs (
    guild_id     BIGINT      PRIMARY KEY,
    channel_id   BIGINT      NOT NULL,
    auto_role_id BIGINT,
    title        TEXT        NOT NULL,
    message      TEXT        NOT NULL,
    enabled      BOOLEAN     NOT NULL DEFAULT TRUE,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE giveaways (
    id           BIGSERIAL     PRIMARY KEY,
    guild_id     BIGINT        NOT NULL,
    channel_id   BIGINT        NOT NULL,
    message_id   BIGINT        NOT NULL DEFAULT 0,
    host_id      BIGINT        NOT NULL,
    kind         TEXT          NOT NULL CHECK (kind IN ('standard', 'cash')),
    prize        TEXT          NOT NULL,
    cash_amount  NUMERIC(20,2) NOT NULL DEFAULT 0 CHECK (cash_amount >= 0),
    winner_count INTEGER       NOT NULL DEFAULT 1 CHECK (winner_count >= 1 AND winner_count <= 20),
    ends_at      TIMESTAMPTZ   NOT NULL,
    status       TEXT          NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'ended')),
    winner_ids   BIGINT[]      NOT NULL DEFAULT '{}',
    created_at   TIMESTAMPTZ   NOT NULL DEFAULT now(),
    ended_at     TIMESTAMPTZ
);
CREATE INDEX idx_giveaways_status_ends ON giveaways(status, ends_at);

CREATE TABLE giveaway_entries (
    giveaway_id BIGINT      NOT NULL REFERENCES giveaways(id) ON DELETE CASCADE,
    user_id     BIGINT      NOT NULL,
    entered_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (giveaway_id, user_id)
);
