-- Mendukung upgrade level fasilitas (mis. Lumbung Lv 1..5) dan proyek stage Desa..Metropolis.
ALTER TABLE city_projects
    ADD COLUMN IF NOT EXISTS target_level INTEGER NOT NULL DEFAULT 1;
