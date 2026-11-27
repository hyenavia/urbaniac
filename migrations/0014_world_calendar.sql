-- Fase 8: Kalender Global, Musim, Event, Tren, dan Modifier Lokal Kota.
CREATE TABLE world_calendar (
    id            INTEGER     PRIMARY KEY CHECK (id = 1),
    day           INTEGER     NOT NULL DEFAULT 1 CHECK (day >= 1),
    season        TEXT        NOT NULL DEFAULT 'spring' CHECK (season IN ('spring', 'summer', 'autumn', 'winter')),
    active_event  TEXT,
    trend_item_id TEXT,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO world_calendar (id, day, season, active_event, trend_item_id)
VALUES (1, 1, 'spring', NULL, NULL)
ON CONFLICT (id) DO NOTHING;

ALTER TABLE cities
    ADD COLUMN IF NOT EXISTS local_modifier NUMERIC(6,4) NOT NULL DEFAULT 1.0000 CHECK (local_modifier >= 0.8000 AND local_modifier <= 1.3000);
