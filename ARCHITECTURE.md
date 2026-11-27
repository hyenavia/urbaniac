# Arsitektur Urbaniac

Urbaniac adalah **modular monolith** berbasis Rust (`serenity` + `tokio` + `sqlx` + PostgreSQL).

## Struktur Lapisan (`src/`)

1. **`domain/`** — Aturan bisnis murni tanpa ketergantungan I/O atau database:
   - `player`: rumus XP per level (`100 * level`).
   - `economy`: format mata uang `Bac` (`Decimal`, 2 desimal), starter grant, harga NPC.
   - `resources`: katalog item (`ITEMS`), rarity, stack limit, dan logika penempatan slot + overflow (`plan_add`).
   - `activities`: definisi aktivitas (`Farming`, `Gathering`), durasi, drop table, bonus yield per level.
   - `progression`: `apply_xp` dengan level cap per `CityStage` dan penyimpanan `overflow_xp`.
   - `city`: `CityStage`, definisi proyek awal (`Sumur`, `Lumbung`, `Balai Kampung`), dan kalkulasi kapasitas Lumbung.

2. **`infrastructure/`** — Akses database PostgreSQL dan antrean job persisten:
   - `database`: koneksi `PgPool` (maks 5 koneksi) dan migrasi otomatis `sqlx::migrate!("./migrations")`.
   - `repositories`: query terstruktur (`player_repo`, `wallet_repo`, `inventory_repo`, `activity_repo`, `city_repo`) dengan row locking (`FOR UPDATE`).
   - `scheduler`: manajemen tabel `scheduled_jobs` (`enqueue`, `due_jobs`, `lock_pending`, `mark_done`, `record_failure`).

3. **`application/`** — Orkestrasi use-case dan transaksi atomik:
   - `player`: registrasi idempoten (`/start`) yang membuat profil, wallet, dan kota Kampung.
   - `economy`: mutasi wallet dan penjualan NPC (`sell_to_npc`) dalam satu transaksi.
   - `inventory`: tampilan inventory, klaim Pending Claim, dan kapasitas slot dinamis (dasar + Lumbung).
   - `activities`: mulai aktivitas, overview, penyelesaian oleh worker (`complete_run`), dan klaim reward (`collect_ready`).
   - `city`: overview kota, mulai proyek (`start_project`), penyelesaian proyek (`complete_project`), dan setoran ke City Treasury (`deposit_to_treasury`).

4. **`workers/`** — Satu instance background scheduler:
   - Memproses batch maks 25 job jatuh tempo dengan konkurensi 2 worker.
   - Tidur hingga `next_run_at` (maks 60 detik) atau dibangunkan langsung lewat `tokio::sync::Notify` saat ada job baru.
   - Menangani `activity_complete` dan `city_project_complete`.

5. **`bot/`** — Antarmuka Discord (`serenity`):
   - `commands`: `/start`, `/help`.
   - `components`: builder tombol/select menu dan parser `custom_id` (`ub:<aksi>:<owner_id>[:<data>]`).
   - `interactions`: validasi pemilik komponen dan routing ke layer `application`.
   - `embeds`: formatter embed Discord.
