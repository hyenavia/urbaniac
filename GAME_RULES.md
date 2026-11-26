# Game Rules (tahap 1)

Semua angka di bawah adalah nilai awal sementara (spesifikasi tidak menentukannya).

| Aturan | Nilai | Lokasi di kode |
|---|---|---|
| Bac awal pemain | 100.00 | `domain/economy::starter_bac` |
| Slot inventory dasar | 12 | `domain/resources::BASE_SLOT_CAPACITY` |
| Stack limit | 100 (Herba Liar 50) | `domain/resources::ITEMS` |
| XP naik level | 100 x level | `domain/player::xp_to_next` |

Aturan inventory: satu jenis item memakai satu slot dan menumpuk sampai stack limit.
Item yang tidak muat (stack penuh atau tidak ada slot baru) masuk Pending Claim dan
bisa diklaim lewat tombol saat ada ruang.

Wallet: semua perubahan Bac tercatat di `ledger_entries` dengan `idempotency_key` unik.
City Treasury terpisah akan dibuat di tahap City.
