-- Satu baris = satu jenis item = satu slot. Baris dihapus saat quantity menjadi 0.
CREATE TABLE inventory_items (
    player_id BIGINT NOT NULL REFERENCES players(discord_user_id),
    item_id   TEXT   NOT NULL,
    quantity  BIGINT NOT NULL CHECK (quantity > 0),
    PRIMARY KEY (player_id, item_id)
);

-- Item yang tidak muat di inventory menunggu di sini sampai diklaim.
CREATE TABLE pending_claims (
    player_id  BIGINT      NOT NULL REFERENCES players(discord_user_id),
    item_id    TEXT        NOT NULL,
    quantity   BIGINT      NOT NULL CHECK (quantity > 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, item_id)
);
