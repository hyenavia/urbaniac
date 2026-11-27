-- Fase 9: Reward Progresi Pemain (idempoten per milestone).
CREATE TABLE player_progression_rewards (
    player_id  BIGINT      NOT NULL REFERENCES players(discord_user_id),
    reward_key TEXT        NOT NULL,
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, reward_key)
);
