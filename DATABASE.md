# Skema Database Urbaniac

Semua perubahan skema dikelola lewat file migrasi SQLx berurutan di `migrations/` (jangan pernah mengedit file migrasi yang sudah dijalankan).

## Daftar Tabel

### `0001_players.sql`
- **`players`**: profil pemain (`discord_user_id` PK, `username`, `avatar_url`, `level`, `xp`, timestamps).

### `0002_wallet.sql`
- **`wallets`**: saldo Bac pribadi pemain (`player_id` PK/FK, `balance NUMERIC(20,2) CHECK (balance >= 0)`).
- **`ledger_entries`**: buku besar perubahan wallet (`amount`, `balance_after`, `reason`, `idempotency_key UNIQUE`).

### `0003_inventory.sql`
- **`inventory_items`**: 1 baris = 1 jenis item per pemain (`PK (player_id, item_id)`, `quantity > 0`). Baris dihapus saat quantity habis.
- **`pending_claims`**: item overflow saat slot/stack penuh (`PK (player_id, item_id)`).

### `0004_activities.sql`
- **`player_activities`**: progres level, `xp`, dan `overflow_xp` per aktivitas.
- **`activity_runs`**: instance aktivitas (`status`: `running` | `ready` | `claimed`, `started_at`, `ends_at`).
- **`activity_run_rewards`**: hasil undian reward untuk run berstatus `ready` sebelum diklaim.

### `0005_scheduler.sql`
- **`scheduled_jobs`**: antrean job persisten (`kind`, `ref_id`, `run_at`, `status`: `pending` | `done` | `failed`, `attempts`, `last_error`). Unique index `(kind, ref_id)` mencegah job ganda.

### `0006_cities.sql`
- **`cities`**: kota per pemain (`player_id` PK/FK, `stage` default `'kampung'`, `treasury NUMERIC(20,2) CHECK (treasury >= 0)`).
- **`city_treasury_ledger`**: ledger terpisah untuk City Treasury (`idempotency_key UNIQUE`).
- **`city_facilities`**: fasilitas yang sudah dibangun (`PK (player_id, facility_id)`, `level`).
- **`city_projects`**: proyek pembangunan kota (`status`: `running` | `completed`, partial unique index `WHERE status = 'running'` membatasi 1 proyek berjalan per pemain).

### `0007_market.sql`
- **`npc_market_prices`**: harga dinamis NPC per item (`item_id` PK, `multiplier` 0.6000..1.6000, `recent_sales` tekanan suplai).
- **`broker_listings`**: listing Makelar (`player_id`, `item_id`, `quantity`, `target_unit_price`, `listing_fee`, `status`: `active` | `sold` | `expired` | `cancelled`, `sold_unit_price`, `commission`, `expires_at`).
