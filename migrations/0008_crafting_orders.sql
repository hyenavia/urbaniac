-- Seed harga pasar NPC untuk item hasil crafting (Processed & Finished).
INSERT INTO npc_market_prices (item_id, multiplier)
VALUES
    ('flour', 1.0000),
    ('plank', 1.0000),
    ('herb_extract', 1.0000),
    ('bread', 1.0000),
    ('wooden_crate', 1.0000),
    ('healing_tonic', 1.0000)
ON CONFLICT (item_id) DO NOTHING;

-- Antrean crafting yang diselesaikan oleh scheduler.
CREATE TABLE crafting_jobs (
    id           BIGSERIAL PRIMARY KEY,
    player_id    BIGINT      NOT NULL REFERENCES players(discord_user_id),
    recipe_id    TEXT        NOT NULL,
    quantity     BIGINT      NOT NULL CHECK (quantity > 0),
    status       TEXT        NOT NULL CHECK (status IN ('running', 'completed')),
    auto_sell    BOOLEAN     NOT NULL DEFAULT false,
    started_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    ends_at      TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ
);
CREATE INDEX idx_crafting_player_running ON crafting_jobs(player_id) WHERE status = 'running';

-- Reputasi toko pemain + preferensi auto_craft / auto_sell.
CREATE TABLE shop_reputation (
    player_id        BIGINT      PRIMARY KEY REFERENCES players(discord_user_id),
    level            INTEGER     NOT NULL DEFAULT 1 CHECK (level >= 1),
    xp               BIGINT      NOT NULL DEFAULT 0 CHECK (xp >= 0),
    completed_orders BIGINT      NOT NULL DEFAULT 0 CHECK (completed_orders >= 0),
    auto_craft       BOOLEAN     NOT NULL DEFAULT false,
    auto_sell        BOOLEAN     NOT NULL DEFAULT false,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Pesanan NPC untuk item hasil crafting.
CREATE TABLE npc_orders (
    id           BIGSERIAL PRIMARY KEY,
    player_id    BIGINT        NOT NULL REFERENCES players(discord_user_id),
    item_id      TEXT          NOT NULL,
    quantity     BIGINT        NOT NULL CHECK (quantity > 0),
    reward_bac   NUMERIC(20,2) NOT NULL CHECK (reward_bac > 0),
    rep_xp       BIGINT        NOT NULL CHECK (rep_xp > 0),
    status       TEXT          NOT NULL CHECK (status IN ('active', 'fulfilled', 'expired')),
    created_at   TIMESTAMPTZ   NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ   NOT NULL,
    fulfilled_at TIMESTAMPTZ
);
CREATE INDEX idx_orders_player_active ON npc_orders(player_id) WHERE status = 'active';
