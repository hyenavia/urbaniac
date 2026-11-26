# Urbaniac

Persistent economy dan city-building game lewat Discord Bot.
Rust + Serenity + Tokio + PostgreSQL + SQLx. Modular monolith.

## Status: Tahap 1 (Fondasi)

Sudah berfungsi (setelah `cargo check` lolos di mesinmu):
- `/start` membuat profil + wallet (100 Bac awal) dan membuka menu; `/help`.
- Tombol Profil, Inventory, Klaim Pending; hanya pemilik menu yang bisa menekannya.
- Wallet dengan ledger idempoten (`rust_decimal`, saldo tidak pernah negatif).
- Inventory: satu jenis item = satu slot, stack limit, overflow ke Pending Claim.

Belum ada (tahap berikutnya): Farming/Gathering, Activity Slots, progression XP,
scheduler persisten, jual ke NPC, kota Kampung dan proyek kota.

Lihat SETUP_WINDOWS.md untuk menjalankan, GAME_RULES.md untuk angka game.
