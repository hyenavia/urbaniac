ALTER TABLE players
    ADD COLUMN IF NOT EXISTS lang TEXT NOT NULL DEFAULT 'en' CHECK (lang IN ('en', 'id'));
