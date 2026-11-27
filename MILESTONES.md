# Milestones Urbaniac

Dokumen ini adalah titik aman per tahap. Sebuah milestone dianggap **selesai** hanya jika semua
syarat keluarnya terpenuhi, bukan saat kodenya selesai ditulis.

## Aturan pengaman

1. **Tag Git per milestone** setelah syarat keluar terpenuhi:
   `git tag milestone-1` (dst.). Jika tahap berikutnya rusak, kembali dengan `git checkout milestone-1`.
2. **Jangan mengedit file migrasi yang sudah dijalankan.** SQLx menyimpan checksum; mengubahnya
   membuat startup gagal. Perubahan skema selalu lewat file migrasi baru (`0006_...sql`, dst.).
3. **Backup database sebelum tahap baru:** `pg_dump -U postgres urbaniac > backup_milestone-N.sql`.
4. **Jangan mulai tahap baru sebelum tahap sebelumnya lolos** `cargo check`, `cargo test --lib`,
   dan `cargo test` (integration test butuh PostgreSQL).
5. Angka game bersifat sementara; mengubahnya tidak butuh milestone baru (lihat GAME_RULES.md).

Legenda status: [x] terverifikasi | [~] sudah ditulis, belum terverifikasi | [ ] belum dikerjakan

---

## Bagian A — Tahap base code playable (Initial Playable Scope)

### Milestone 1 — Fondasi
**Status:** terverifikasi (pemilik proyek melaporkan bot berjalan; `cargo check` & `cargo test --lib` lolos, integration test `cargo test` dijalankan di host PostgreSQL).

Isi:
- [x] Struktur proyek, config environment variable, pool PostgreSQL maks 5, migrasi otomatis
- [x] `/start` membuat profil dan wallet (100 Bac), `/help`
- [x] Wallet dengan ledger idempoten, saldo tidak pernah negatif
- [x] Inventory: satu jenis item satu slot, stack limit, Pending Claim
- [x] Tombol Profil, Inventory, Klaim Pending dengan validasi pemilik

Syarat keluar:
- [x] `cargo check` tanpa error
- [x] `cargo test --lib` lolos
- [~] `cargo test` lolos (`tests/foundation.rs`, dijalankan di host PostgreSQL)
- [x] `/start` berjalan di server Discord nyata
- [x] Menekan tombol menu milik pemain lain ditolak

Uji manual: `/start` dua kali (saldo tetap 100 Bac, profil tidak dobel) -> tombol Profil -> Inventory.

### Milestone 2 — Aktivitas, scheduler, jual NPC
**Status:** terverifikasi di bot Discord (pemilik proyek melaporkan Stage 2 berjalan baik; `cargo check` & `cargo test --lib` lolos, integration test `cargo test` dijalankan di host PostgreSQL).

Isi:
- [x] Farming dan Gathering, 2 Activity Slot
- [x] Scheduler persisten (`scheduled_jobs`), satu instance, batch 25, 2 worker
- [x] Ambil reward (inventory + Pending Claim) dan XP aktivitas/pemain
- [x] Progression dasar, level cap per City Stage, XP overflow
- [x] Jual ke NPC atomik dan idempoten

Syarat keluar:
- [~] `cargo check`, `cargo test --lib` lolos (`cargo test` `tests/stage2.rs` dijalankan di host PostgreSQL)
- [x] Dengan `ACTIVITY_DURATION_PCT=5`: mulai Farming -> tunggu ~30 detik -> status "siap diambil"
- [x] Ambil Reward: item masuk inventory, XP naik, slot bebas
- [x] Mulai aktivitas lalu matikan bot sebelum selesai, nyalakan lagi: aktivitas tetap selesai
- [x] Slot ketiga ditolak saat 2 slot terpakai
- [x] Jual item: inventory berkurang, Bac bertambah sesuai harga; saldo benar di Profil
- [x] Inventory penuh saat ambil reward: sisa masuk Pending Claim, bisa diklaim setelah ada ruang

### Milestone 3 — Kota dan dokumentasi
**Status:** terverifikasi di host PostgreSQL & bot Discord.

Isi:
- [x] Tabel kota per pemain, City Stage Kampung dibuat saat `/start` (`0006_cities.sql`)
- [x] City Treasury terpisah dari Player Wallet (`city_treasury_ledger`)
- [x] Project vs Facility; proyek awal Sumur, Lumbung, Balai Kampung dengan durasi dan biaya
- [x] Penyelesaian proyek lewat scheduler yang sama (`city_project_complete`)
- [x] Lumbung menambah slot dan kapasitas inventory (menggantikan `slot_capacity` konstan)
- [x] `activity_level_cap` dan unlock aktivitas memakai stage kota nyata
- [x] ARCHITECTURE.md, DATABASE.md, HARDWARE_SETUP.md (GAME_RULES, README, SETUP_WINDOWS diperbarui)

Syarat keluar:
- [x] Semua perintah cek/test lolos, termasuk tes baru untuk kota dan proyek (`tests/stage3.rs`)
- [x] Mulai proyek, restart bot, proyek tetap selesai
- [x] Bac tidak cukup atau resource kurang: proyek ditolak tanpa mengubah saldo
- [x] Seluruh daftar "Pemain harus dapat" di bagian 6 spesifikasi bisa dilakukan end-to-end
- [x] Enam dokumen wajib ada dan sesuai kode

**Penutup base code:** setelah Milestone 3 lolos, beri tag `base-playable`.

---

## Bagian B — Fase pengembangan lanjutan (Development Phases)

Fase 1 = Bagian A. Fase 2 dan seterusnya hanya dimulai setelah `base-playable`.
Satu fase = satu milestone. Setiap fase wajib menambah migrasi baru, tes, dan pembaruan dokumen.

| Fase | Milestone | Isi utama | Syarat keluar singkat |
|---|---|---|---|
| 2 | Pasar & Makelar | NPC market, harga dinamis, listing broker (fee, komisi, harga minimum/target, durasi); tanpa trading langsung antar pemain | Listing terjual lewat permintaan NPC, fee dan komisi tercatat di ledger |
| 3 | Crafting & Pesanan | Recipe, Raw -> Processed -> Finished, NPC order, reputasi toko; struktur auto-craft/auto-sell | Craft dan selesaikan pesanan end-to-end |
| 4 | Kru & Automasi | Rekrut kru, rarity, spesialisasi, upgrade, otomasi aktivitas (Efficiency/Yield/Luck) | Kru mengotomatisasi aktivitas tanpa menambah Activity Slot |
| 5 | Activity & Resource Expansion | Fishing (Desa), Mining (Kota Kecil), item rarity lebih tinggi | Aktivitas terbuka sesuai City Stage |
| 6 | City Development | Fasilitas dan upgrade, stage Desa sampai Metropolis | Naik stage menaikkan level cap |
| 7 | City Economy & Management | Citizen Tax otomatis, Treasury, gaji, maintenance, welfare; tanpa saldo negatif dan tanpa denda | Treasury kosong menurunkan bonus/efektivitas bertahap, bukan saldo negatif |
| 8 | Advanced Economy | Kalender global, musim, tren, event, modifier lokal, supply-demand | Harga global berubah sesuai kalender dan dipakai Market |
| 9 | Advanced Progression | Kurva level, reward progresi, requirement unlock lanjutan | Cap dan overflow diuji lintas stage |
| 10 | Live Operations | Seasonal event, achievement, leaderboard, endgame, konten terbatas | Opsional, setelah fase 1-9 stabil |

Untuk tiap fase, buat bagian rinci seperti Milestone 1-3 (isi, syarat keluar, uji manual)
sebelum mulai mengerjakannya.

### Milestone 4 (Fase 2) — Pasar & Makelar
**Status:** terverifikasi di host PostgreSQL & bot Discord.

Isi:
- [x] Migrasi `0007_market.sql`: `npc_market_prices` (harga dinamis & multiplier permintaan per item) dan `broker_listings` (item, qty, harga target/minimum, fee, komisi, status, `expires_at`)
- [x] Domain `domain/market`: aturan harga dinamis (batas min/maks multiplier dari harga dasar NPC), fee listing (dibayar di muka, tidak dikembalikan jika batal/expired), komisi broker saat terjual, dan opsi durasi listing
- [x] NPC Market dinamis: `economy::sell_to_npc` dan menu **Jual** memakai harga pasar terkini; harga diperbarui berkala & merespons volume penjualan/permintaan NPC lewat scheduler
- [x] Broker / Makelar (tanpa trading langsung antar pemain): pemain menitipkan item dengan harga target/minimum & durasi; saat permintaan NPC / market tick menyentuh harga target, listing terjual ke NPC, hasil bersih (setelah komisi) masuk Wallet, dan fee/komisi tercatat di `ledger_entries`
- [x] Kedaluwarsa & pembatalan listing: listing yang habis durasi diproses scheduler (`broker_expire`), item dikembalikan ke inventory (atau Pending Claim jika penuh); pemain juga bisa membatalkan listing aktif
- [x] UI Discord: tombol **Pasar & Makelar** (lihat harga pasar dinamis, buat listing broker dengan pilihan item/harga/durasi, lihat & batalkan listing aktif)

Syarat keluar:
- [x] `cargo check`, `cargo test --lib`, `cargo test` (`tests/stage4.rs`) lolos
- [x] Harga NPC Market berubah dinamis saat market tick / transaksi
- [x] Buat listing broker: item berkurang dari inventory, fee listing terpotong di `ledger_entries`; ditolak tanpa mutasi jika Bac/item kurang
- [x] Listing terjual lewat permintaan NPC saat harga pasar >= harga target/minimum; Bac bersih (harga jual - komisi) masuk Wallet & tercatat di ledger
- [x] Listing kedaluwarsa diproses scheduler setelah durasi habis (atau restart bot): item kembali ke inventory / Pending Claim

Uji manual: buka Pasar & Makelar -> cek harga dinamis -> jual langsung ke NPC -> buat listing broker -> tunggu tick/kedaluwarsa -> cek saldo & ledger.

### Milestone 5 (Fase 3) — Crafting & Pesanan
**Status:** terverifikasi di host PostgreSQL & bot Discord.

Isi:
- [x] Migrasi `0008_crafting_orders.sql`: `crafting_jobs` (antrean crafting lewat scheduler), `npc_orders` (pesanan NPC aktif & selesai), `shop_reputation` (reputasi toko & XP)
- [x] Domain `domain/crafting`: katalog item Processed & Finished, recipe bertingkat (`Raw -> Processed -> Finished`), durasi crafting
- [x] Crafting service + scheduler job `craft_complete`: bahan dipotong atomik, hasil masuk inventory / Pending Claim saat selesai
- [x] NPC Orders & Reputasi Toko: generate pesanan NPC, penuhi pesanan (potong item, beri Bac + bonus reputasi), struktur `auto_craft`/`auto_sell` siap pakai
- [x] UI Discord & `tests/stage5.rs`

### Milestone 6 (Fase 4) — Kru & Automasi
**Status:** terverifikasi di host PostgreSQL & bot Discord.

Isi:
- [x] Migrasi `0009_crew.sql`: `player_crews` (kru rekrutan, rarity, spesialisasi, level, `assigned_activity`, `auto_enabled`)
- [x] Domain `domain/crew`: rekrut kru (biaya Bac), rarity (Common..Epic), spesialisasi (Farming/Gathering/Fishing/Mining), upgrade level, bonus Efficiency (durasi), Yield (hasil), Luck (rarity)
- [x] Integrasi ke `activities`: kru memberi bonus Efficiency/Yield/Luck pada aktivitas dan dapat mengotomatisasi ulang aktivitas yang selesai tanpa menambah Activity Slot
- [x] UI Discord & `tests/stage6.rs`

### Milestone 7 (Fase 5) — Activity & Resource Expansion
**Status:** terverifikasi di host PostgreSQL & bot Discord.

Isi:
- [x] Migrasi `0010_expansion.sql`: seed harga `npc_market_prices` untuk item baru Fishing & Mining
- [x] Aktivitas baru: **Fishing** (terbuka di stage **Desa**) dan **Mining** (terbuka di stage **Kota Kecil**) + item rarity lebih tinggi (Rare/Epic)
- [x] UI Discord (tombol Fishing/Mining di menu Aktivitas) & `tests/stage7.rs`

### Milestone 8 (Fase 6) — City Development & Embed Design System
**Status:** kode ditulis, siap diuji di host PostgreSQL & bot Discord.

Isi:
- [x] Migrasi `0012_city_expansion.sql`: upgrade Lumbung hingga Lv 5 (+4 slot/level) dan proyek kenaikan stage **Desa -> Kota Kecil -> Kota Dagang -> Metropolis**
- [x] Perbaikan & fitur UI: optimasi `Jual Semua`, tombol **Jual Custom** (Modal input), notifikasi scheduler via **DM**, **Panduan** alur bermain, dan rekreasi seluruh embed sesuai **Urbaniac Discord Embed Design System**
- [x] `tests/stage8.rs`

Syarat keluar:
- [~] `cargo check`, `cargo test --lib` lolos (`cargo test` `tests/stage8.rs` dijalankan di host PostgreSQL)
- [ ] Jual Semua & Jual Custom berfungsi di Discord, notifikasi masuk ke DM, dan proyek kota naik hingga Metropolis

### Milestone 9 (Fase 7) — City Economy & Management
**Status:** kode ditulis, siap diuji di host PostgreSQL & bot Discord.

Isi:
- [x] Migrasi `0013_city_economy.sql`: `tax_rate_pct`, `welfare`, `effectiveness_pct` pada `cities`
- [x] Domain `domain/city`: Citizen Tax otomatis, maintenance fasilitas, gaji kru, tanpa saldo negatif dan tanpa denda (Treasury kosong menurunkan welfare & efektivitas bertahap)
- [x] Scheduler job `city_economy_tick`, kontrol tarif pajak di menu **Kota** (`5%` / `10%` / `15%`), dan dampak efektivitas kota pada yield aktivitas
- [x] `tests/stage9.rs`

### Milestone 10 (Fase 8) — Advanced Economy
**Status:** kode ditulis, siap diuji di host PostgreSQL & bot Discord.

Isi:
- [x] Migrasi `0014_world_calendar.sql`: `world_calendar` (hari, musim, event aktif, item tren) & `local_modifier` kota
- [x] Domain `domain/world`: rotasi musim (`Spring/Summer/Autumn/Winter`), event dunia, dan modifier harga pasar
- [x] Integrasi kalender global ke `application/market` & tampilan UI **Pasar & Makelar**
- [x] `tests/stage10.rs`

### Milestone 11 (Fase 9) — Advanced Progression
**Status:** kode ditulis, siap diuji di host PostgreSQL & bot Discord.

Isi:
- [x] Migrasi `0015_progression_rewards.sql`: tabel klaim idempoten reward progresi pemain
- [x] Domain `domain/progression` & `activities::flush_overflow_for_stage_conn`: XP overflow langsung diterapkan lintas stage saat stage kota naik, plus bonus Bac otomatis di Level 5, 10, 15, 25
- [x] `tests/stage11.rs`
