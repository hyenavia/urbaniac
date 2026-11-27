# Urbaniac

Persistent economy dan city-building game lewat Discord Bot.
Rust + Serenity + Tokio + PostgreSQL + SQLx. Modular monolith.

## Status: Tahap 3 (Initial Playable Scope — Kota & Dokumentasi)

Sudah diimplementasikan:
- `/start` membuat profil, wallet (100 Bac awal), dan kota stage **Kampung**; `/help`.
- Tombol Profil, Inventory, Aktivitas, **Kota**, Jual, dan Klaim Pending; hanya pemilik menu yang bisa menekannya.
- Wallet pribadi (`wallets` + `ledger_entries`) dan City Treasury (`cities.treasury` + `city_treasury_ledger`) terpisah serta idempoten (`rust_decimal`, saldo tidak pernah negatif).
- Inventory: satu jenis item = satu slot, stack limit, overflow ke Pending Claim, kapasitas dinamis (12 dasar + 4 per level **Lumbung**).
- Farming dan Gathering dengan 2 Activity Slot; tetap berjalan saat pemain offline.
- Scheduler persisten (`scheduled_jobs`): satu instance, batch 25, 2 worker; menangani aktivitas dan proyek kota meski bot sempat mati.
- Proyek kota awal (**Sumur**, **Lumbung**, **Balai Kampung**) dengan biaya Bac + resource dan durasi; Balai Kampung menaikkan kota ke **Desa** (level cap aktivitas naik 10 -> 20).
- Jual ke NPC lewat menu pilih item + tombol jumlah; atomik dan idempoten.

Dokumentasi: [ARCHITECTURE.md](ARCHITECTURE.md), [DATABASE.md](DATABASE.md), [HARDWARE_SETUP.md](HARDWARE_SETUP.md), [GAME_RULES.md](GAME_RULES.md), [SETUP_WINDOWS.md](SETUP_WINDOWS.md), [MILESTONES.md](MILESTONES.md).
