# Urbaniac

Persistent economy dan city-building game lewat Discord Bot.
Rust + Serenity + Tokio + PostgreSQL + SQLx. Modular monolith.

## Status: Tahap 2 (Aktivitas, Scheduler, Jual NPC)

Sudah berfungsi (setelah `cargo check` lolos di mesinmu):
- `/start` membuat profil + wallet (100 Bac awal) dan membuka menu; `/help`.
- Tombol Profil, Inventory, Klaim Pending; hanya pemilik menu yang bisa menekannya.
- Wallet dengan ledger idempoten (`rust_decimal`, saldo tidak pernah negatif).
- Inventory: satu jenis item = satu slot, stack limit, overflow ke Pending Claim.

Tahap 2:
- Farming dan Gathering dengan 2 Activity Slot; tetap berjalan saat pemain offline.
- Scheduler persisten (tabel `scheduled_jobs`): satu instance, batch 25, 2 worker, tidur sampai job
  berikutnya jatuh tempo. Aktivitas yang selesai saat bot mati diproses saat bot menyala lagi.
- Ambil reward (item ke inventory, sisa ke Pending Claim) + XP aktivitas dan pemain.
- Progression dasar: level cap mengikuti City Stage, XP berlebih disimpan sebagai overflow.
- Jual ke NPC lewat menu pilih item + tombol jumlah; atomik dan idempoten.

Belum ada (tahap 3): kota Kampung, proyek kota (Sumur, Lumbung, Balai Kampung), City Treasury,
dan dokumen ARCHITECTURE.md, DATABASE.md, HARDWARE_SETUP.md.

Lihat SETUP_WINDOWS.md untuk menjalankan, GAME_RULES.md untuk angka game.
