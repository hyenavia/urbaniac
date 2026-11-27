-- Menyimpan ID private thread per pemain untuk gameplay & notifikasi background.
ALTER TABLE players
    ADD COLUMN IF NOT EXISTS thread_id BIGINT;
