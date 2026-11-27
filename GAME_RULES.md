# Game Rules (tahap 1)

Semua angka di bawah adalah nilai awal sementara (spesifikasi tidak menentukannya).

| Aturan | Nilai | Lokasi di kode |
|---|---|---|
| Bac awal pemain | 100.00 | `domain/economy::starter_bac` |
| Slot inventory dasar | 12 | `domain/resources::BASE_SLOT_CAPACITY` |
| Stack limit | 100 (Herba Liar 50) | `domain/resources::ITEMS` |
| XP naik level (pemain dan aktivitas) | 100 x level | `domain/player::xp_to_next` |
| Activity Slot dasar | 2 | `domain/activities::BASE_ACTIVITY_SLOTS` |
| Durasi Farming / Gathering | 10 menit / 5 menit | `domain/activities::FARMING`, `GATHERING` |
| XP per aktivitas selesai | 10 (aktivitas dan pemain) | `domain/activities` |
| Undian reward | 3 undian per aktivitas, tabel drop di `domain/activities` | |
| Bonus yield level aktivitas | +2% per level di atas 1 | `domain/activities::yield_bonus_pct` |
| Level cap aktivitas | Kampung 10, Desa 20, Kota Kecil 30, Kota Dagang 40, Metropolis 50 | `domain/progression::activity_level_cap` |
| Harga NPC (Bac/unit) | Gandum 3, Kentang 4, Kayu 3, Buah Hutan 3.5, Herba Liar 12 | `domain/resources::ITEMS` |

Aturan inventory: satu jenis item memakai satu slot dan menumpuk sampai stack limit.
Item yang tidak muat (stack penuh atau tidak ada slot baru) masuk Pending Claim dan
bisa diklaim lewat tombol saat ada ruang.

Wallet: semua perubahan Bac tercatat di `ledger_entries` dengan `idempotency_key` unik.
City Treasury terpisah akan dibuat di tahap City.

## Aktivitas
Satu aktivitas memakai satu slot dari mulai sampai rewardnya diambil. Setelah durasi habis,
scheduler menandainya siap dan mengundi reward (memakai level aktivitas saat itu). Reward dan XP
diberikan saat pemain menekan Ambil Reward. Efficiency, Luck, Activity Tool, dan Crew belum ada.

## XP overflow
Saat level aktivitas sudah di cap, XP berlebih disimpan sebagai overflow dan otomatis diterapkan
ketika cap naik. Cap saat ini selalu Kampung karena kota per pemain dibuat di tahap City.

## Jual ke NPC
Harga tetap (belum dinamis). Menjual mengurangi item dan menambah Bac dalam satu transaksi dengan
idempotency key dari ID interaksi Discord.
