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
**Status:** sebagian terverifikasi (pemilik proyek melaporkan `/start` berjalan).

Isi:
- [x] Struktur proyek, config environment variable, pool PostgreSQL maks 5, migrasi otomatis
- [x] `/start` membuat profil dan wallet (100 Bac), `/help`
- [~] Wallet dengan ledger idempoten, saldo tidak pernah negatif
- [~] Inventory: satu jenis item satu slot, stack limit, Pending Claim
- [~] Tombol Profil, Inventory, Klaim Pending dengan validasi pemilik

Syarat keluar:
- [ ] `cargo check` tanpa error
- [ ] `cargo test --lib` lolos
- [ ] `cargo test` lolos (`tests/foundation.rs`)
- [x] `/start` berjalan di server Discord nyata
- [ ] Menekan tombol menu milik pemain lain ditolak

Uji manual: `/start` dua kali (saldo tetap 100 Bac, profil tidak dobel) -> tombol Profil -> Inventory.

### Milestone 2 — Aktivitas, scheduler, jual NPC
**Status:** kode ditulis, belum terverifikasi sama sekali.

Isi:
- [~] Farming dan Gathering, 2 Activity Slot
- [~] Scheduler persisten (`scheduled_jobs`), satu instance, batch 25, 2 worker
- [~] Ambil reward (inventory + Pending Claim) dan XP aktivitas/pemain
- [~] Progression dasar, level cap per City Stage, XP overflow
- [~] Jual ke NPC atomik dan idempoten

Syarat keluar:
- [ ] `cargo check`, `cargo test --lib`, `cargo test` (`tests/stage2.rs`) lolos
- [ ] Dengan `ACTIVITY_DURATION_PCT=5`: mulai Farming -> tunggu ~30 detik -> status "siap diambil"
- [ ] Ambil Reward: item masuk inventory, XP naik, slot bebas
- [ ] Mulai aktivitas lalu matikan bot sebelum selesai, nyalakan lagi: aktivitas tetap selesai
- [ ] Slot ketiga ditolak saat 2 slot terpakai
- [ ] Jual item: inventory berkurang, Bac bertambah sesuai harga; saldo benar di Profil
- [ ] Inventory penuh saat ambil reward: sisa masuk Pending Claim, bisa diklaim setelah ada ruang

### Milestone 3 — Kota dan dokumentasi (belum dimulai)
Isi:
- [ ] Tabel kota per pemain, City Stage Kampung dibuat saat `/start`
- [ ] City Treasury terpisah dari Player Wallet
- [ ] Project vs Facility; proyek awal Sumur, Lumbung, Balai Kampung dengan durasi dan biaya
- [ ] Penyelesaian proyek lewat scheduler yang sama (jenis job baru)
- [ ] Lumbung menambah slot dan kapasitas inventory (menggantikan `slot_capacity` konstan)
- [ ] `activity_level_cap` dan unlock aktivitas memakai stage kota nyata
- [ ] ARCHITECTURE.md, DATABASE.md, HARDWARE_SETUP.md (GAME_RULES, README, SETUP_WINDOWS diperbarui)

Syarat keluar:
- [ ] Semua perintah cek/test lolos, termasuk tes baru untuk kota dan proyek
- [ ] Mulai proyek, restart bot, proyek tetap selesai
- [ ] Bac tidak cukup atau resource kurang: proyek ditolak tanpa mengubah saldo
- [ ] Seluruh daftar "Pemain harus dapat" di bagian 6 spesifikasi bisa dilakukan end-to-end
- [ ] Enam dokumen wajib ada dan sesuai kode

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
