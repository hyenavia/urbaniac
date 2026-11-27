-- Seed harga pasar NPC untuk item Fishing (Desa) dan Mining (Kota Kecil).
INSERT INTO npc_market_prices (item_id, multiplier)
VALUES
    ('river_fish', 1.0000),
    ('silver_carp', 1.0000),
    ('golden_eel', 1.0000),
    ('stone', 1.0000),
    ('iron_ore', 1.0000),
    ('amethyst', 1.0000),
    ('star_crystal', 1.0000)
ON CONFLICT (item_id) DO NOTHING;
