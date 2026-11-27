# Rancangan Discord Activity — Kota Visual (Urbaniac)

Tujuan: menyediakan antarmuka visual interaktif di dalam Discord (Discord Activity) yang berjalan berdampingan dengan bot Discord yang sudah ada. Kedua antarmuka memakai `application/`, `domain/`, `infrastructure/`, `workers/`, dan PostgreSQL yang sama sehingga aturan game tetap tunggal dan progres pemain selalu sinkron.

## 1. Arsitektur

```
Discord client
  └─ iframe Activity (React + TypeScript + Vite + PixiJS + Embedded App SDK)
        │  /api/*  (relatif, lewat proxy Discord + URL Mapping)
        ▼
  Cloudflare Tunnel HTTPS  ──►  proses Rust tunggal (urbaniac.exe)
                                  ├─ web/   (Axum: /api/* + static files dari ACTIVITY_STATIC_DIR)
                                  ├─ bot/   (Serenity: slash commands, buttons, DM notifications)
                                  ├─ application/ domain/ infrastructure/ (dipakai bersama)
                                  └─ workers/ (scheduler persisten)
                                        └─ PostgreSQL (pool maks 5)
```

- **Opt-in di binary yang sama:** server Axum (`web/`) hanya aktif jika `ACTIVITY_ENABLED=true` (default `false`), sehingga bot production tidak terganggu selama pengembangan atau sebelum migrasi siap.
- **Static assets eksternal:** hasil build frontend (`activity/dist/`) **tidak** di-embed ke dalam `urbaniac.exe`, melainkan disajikan dari direktori eksternal (`ACTIVITY_STATIC_DIR`) memakai `tower_http::services::ServeDir` dengan fallback SPA ke `index.html`.
- **Single source of truth:** `web/` hanya adapter tipis (validasi token -> panggil layer `application/` -> kirim DTO JSON). Tidak ada duplikasi aturan game di frontend.

## 2. Frontend & Tampilan Kota

- **Stack:** React + TypeScript + Vite + `@discord/embedded-app-sdk` untuk seluruh HUD, panel aksi, modal konfirmasi, dan navigasi; **PixiJS** di dalam komponen `<CityCanvas />` untuk peta kota.
- **Sudut pandang:** **Sudut 3/4 (Three-Quarter View)** dengan tata letak tetap (bukan penempatan bebas) sesuai stage kota (`Kampung` -> `Desa` -> `Kota Kecil` -> `Kota Dagang` -> `Metropolis`).
- **Aset visual:** tahap awal memakai placeholder terstruktur agar alur dan interaksi dapat diuji lebih dulu; sprite/ilustrasi final disiapkan menyusul melalui mapping `facility/stage -> asset`.
- **Semua request relatif:** frontend hanya memanggil path relatif (`/api/...`) tanpa hardcode host/port agar kompatibel dengan proxy Discord (`discordsays.com`).

## 3. Autentikasi & Keamanan

1. Frontend memanggil `discordSdk.ready()` lalu `commands.authorize(...)` untuk memperoleh OAuth2 `code`.
2. Backend `POST /api/token` menukar `code` dengan `access_token` ke Discord API memakai `DISCORD_CLIENT_ID` dan `DISCORD_CLIENT_SECRET` (secret tidak pernah dikirim ke frontend).
3. Frontend memanggil `commands.authenticate({ access_token })`, lalu mengirim `Authorization: Bearer <access_token>` pada setiap request API.
4. Backend memverifikasi token ke `GET https://discord.com/api/v10/users/@me` dengan cache TTL dan kapasitas terbatas. Identitas `discord_user_id` **selalu** berasal dari token terverifikasi (tidak pernah dari body atau query klien) dan diserialisasi sebagai `String` JSON agar aman di JavaScript (snowflake 64-bit).
5. Perlindungan tambahan: rate limit per pemain, batas ukuran body request (64 KiB), header keamanan (`CSP`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`), dan tidak memakai CDN pihak ketiga.

## 4. API Kontrak (Tahap 1, Tahap 3, & Tahap 4)

Semua endpoint di bawah `/api`, format JSON, wajib Bearer token kecuali `/health` dan `/token`:

| Method + Path                       | Memanggil                                      | Catatan                                                         |
| ----------------------------------- | ---------------------------------------------- | --------------------------------------------------------------- |
| `GET /api/health`                   | —                                              | Health check tanpa auth                                         |
| `GET /api/events`                   | `EventHub` (SSE)                               | Notifikasi real-time ber-batas (maks 64 total, 3 per pemain)    |
| `POST /api/token`                   | Discord OAuth2                                 | Tukar `code` -> `access_token`, tanpa Bearer                    |
| `POST /api/player/register`         | `player::register`                             | Registrasi kota Kampung awal (idempoten)                        |
| `POST /api/player/lang`             | `player::set_lang` / `toggle_lang`             | Ubah bahasa `en`/`id`                                           |
| `GET /api/me`                       | `player::get_player_profile`                   | Status registrasi (`registered`), profil, saldo, stage          |
| `GET /api/city`                     | `city::overview`                               | Stage, treasury, pajak, welfare, fasilitas, proyek berjalan     |
| `GET /api/city/projects/quote`      | `city::quote` / `city::quote_all`              | Opsional `?facility=<key>`; biaya, item, prasyarat, `can_start` |
| `POST /api/city/projects/start`     | `city::start_project`                          | Body `{ facility, request_id }`; idempoten + `notify_one()`     |
| `POST /api/city/treasury/deposit`   | `city::deposit_to_treasury`                    | Body `{ amount, request_id }`; idempoten                        |
| `POST /api/city/treasury/withdraw`  | `city::withdraw_from_treasury`                 | Body `{ amount, request_id }`; idempoten                        |
| `POST /api/city/tax`                | `city::set_tax_rate`                           | Body `{ tax_rate_pct }`                                         |
| `GET /api/inventory`                | `inventory::view` + `economy::sell_unit_price` | Slot, item, harga jual NPC saat ini, dan `pending`              |
| `POST /api/inventory/claim`         | `inventory::claim_pending`                     | Klaim item dari Pending Claim                                   |
| `GET /api/market`                   | `market::overview`                             | Kalender dunia, harga dinamis NPC, listing broker aktif         |
| `POST /api/market/sell`             | `economy::sell_to_npc`                         | Body `{ item_id, quantity?, sell_all?, request_id }`; idempoten |
| `POST /api/market/listings`         | `market::create_listing`                       | Buat listing broker (`2h`/`6h`/`24h`) + `notify_one()`          |
| `POST /api/market/listings/cancel`  | `market::cancel_listing`                       | Batalkan listing broker aktif                                   |
| `GET /api/crafting`                 | `crafting::overview`                           | Reputasi toko, antrean craft, pesanan NPC, 6 resep              |
| `POST /api/crafting/start`          | `crafting::start_craft_batch`                  | Mulai crafting batch + `notify_one()`                           |
| `POST /api/crafting/auto-sell`      | `crafting::toggle_auto_sell`                   | Toggle auto-sell hasil craft                                    |
| `POST /api/crafting/orders/fulfill` | `crafting::fulfill_order`                      | Penuhi pesanan NPC                                              |
| `POST /api/crafting/orders/refresh` | `crafting::refresh_orders`                     | Ganti pesanan NPC                                               |
| `GET /api/crew`                     | `crew::list`                                   | Daftar kru, bonus, dan biaya rekrut/upgrade                     |
| `POST /api/crew/recruit`            | `crew::recruit`                                | Rekrut kru (idempoten via `request_id`)                         |
| `POST /api/crew/upgrade`            | `crew::upgrade`                                | Upgrade kru (idempoten via `request_id`)                        |
| `POST /api/crew/assign`             | `crew::assign`                                 | Tugaskan kru ke aktivitas + toggle auto                         |
| `GET /api/activities`               | `activities::overview`                         | Slot, aktivitas berjalan/siap, progres level, katalog aktivitas |
| `POST /api/activities/start`        | `activities::start`                            | Body `{ kind }` + `notify_one()`                                |
| `POST /api/activities/collect`      | `activities::collect_ready`                    | Ambil seluruh reward aktivitas yang siap                        |

## 5. Lingkungan, Development, & Deployment

- **Production (Laptop 24 jam):** menjalankan PostgreSQL, `urbaniac.exe` (Bot + Axum API + static `activity-dist`), dan Cloudflare Tunnel HTTPS produksi.
- **Development (PC):** menjalankan Vite dev server lokal dengan Mock API (`VITE_USE_MOCK_API=true`) dan Cloudflare Tunnel terpisah untuk pengujian Activity di Discord tanpa menyentuh server/database production. Pengujian integrasi backend penuh dijalankan pada database dev di laptop.
- **CI/CD & Supervisor:** `.github/workflows/build-release.yml` tetap khusus membangun `urbaniac.exe` + `SHA256SUMS.txt`. Build frontend (`activity/dist/`) akan dipublikasikan oleh workflow terpisah dan diunduh otomatis oleh deployment supervisor di laptop setelah implementasi siap.
