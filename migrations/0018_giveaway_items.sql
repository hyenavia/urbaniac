ALTER TABLE giveaways DROP CONSTRAINT IF EXISTS giveaways_kind_check;
ALTER TABLE giveaways
    ADD CONSTRAINT giveaways_kind_check CHECK (kind IN ('standard', 'cash', 'item'));

ALTER TABLE giveaways
    ADD COLUMN IF NOT EXISTS item_id  TEXT,
    ADD COLUMN IF NOT EXISTS item_qty BIGINT NOT NULL DEFAULT 0 CHECK (item_qty >= 0);
