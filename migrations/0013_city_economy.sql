-- Fase 7: Ekonomi Kota (Citizen Tax, Welfare, Effectiveness, tanpa saldo negatif/denda).
ALTER TABLE cities
    ADD COLUMN IF NOT EXISTS tax_rate_pct      INTEGER NOT NULL DEFAULT 10 CHECK (tax_rate_pct >= 0 AND tax_rate_pct <= 25),
    ADD COLUMN IF NOT EXISTS welfare           INTEGER NOT NULL DEFAULT 100 CHECK (welfare >= 0 AND welfare <= 100),
    ADD COLUMN IF NOT EXISTS effectiveness_pct INTEGER NOT NULL DEFAULT 100 CHECK (effectiveness_pct >= 50 AND effectiveness_pct <= 125),
    ADD COLUMN IF NOT EXISTS last_economy_tick TIMESTAMPTZ;
