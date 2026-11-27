-- Harga pasar dinamis NPC per item.
CREATE TABLE npc_market_prices (
    item_id      TEXT          PRIMARY KEY,
    multiplier   NUMERIC(6,4)  NOT NULL DEFAULT 1.0000 CHECK (multiplier >= 0.6000 AND multiplier <= 1.6000),
    recent_sales BIGINT        NOT NULL DEFAULT 0 CHECK (recent_sales >= 0),
    updated_at   TIMESTAMPTZ   NOT NULL DEFAULT now()
);

INSERT INTO npc_market_prices (item_id, multiplier)
VALUES
    ('wheat', 1.0000),
    ('potato', 1.0000),
    ('wood', 1.0000),
    ('berries', 1.0000),
    ('wild_herb', 1.0000)
ON CONFLICT (item_id) DO NOTHING;

-- Listing Makelar / Broker (tanpa trading langsung antar pemain; terjual lewat permintaan NPC).
CREATE TABLE broker_listings (
    id                BIGSERIAL PRIMARY KEY,
    player_id         BIGINT        NOT NULL REFERENCES players(discord_user_id),
    item_id           TEXT          NOT NULL,
    quantity          BIGINT        NOT NULL CHECK (quantity > 0),
    target_unit_price NUMERIC(20,2) NOT NULL CHECK (target_unit_price > 0),
    listing_fee       NUMERIC(20,2) NOT NULL CHECK (listing_fee >= 0),
    status            TEXT          NOT NULL CHECK (status IN ('active', 'sold', 'expired', 'cancelled')),
    sold_unit_price   NUMERIC(20,2),
    commission        NUMERIC(20,2),
    created_at        TIMESTAMPTZ   NOT NULL DEFAULT now(),
    expires_at        TIMESTAMPTZ   NOT NULL,
    closed_at         TIMESTAMPTZ
);

CREATE INDEX idx_broker_active_item ON broker_listings(item_id, target_unit_price) WHERE status = 'active';
CREATE INDEX idx_broker_player_active ON broker_listings(player_id) WHERE status = 'active';
