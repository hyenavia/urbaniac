# Game Rules (Initial Playable Scope)

Semua angka di bawah adalah nilai awal sementara (spesifikasi tidak menentukannya).

| Aturan | Nilai | Lokasi di kode |
|---|---|---|
| Bac awal pemain | 100.00 | `domain/economy::starter_bac` |
| Slot inventory dasar | 12 (+4 per level Lumbung) | `domain/resources::BASE_SLOT_CAPACITY`, `domain/city::GRANARY_SLOTS_PER_LEVEL` |
| Stack limit | 100 (Herba Liar 50) | `domain/resources::ITEMS` |
| XP naik level (pemain dan aktivitas) | 100 x level | `domain/player::xp_to_next` |
| Activity Slot dasar | 2 | `domain/activities::BASE_ACTIVITY_SLOTS` |
| Durasi Farming / Gathering | 10 menit / 5 menit | `domain/activities::FARMING`, `GATHERING` |
| XP per aktivitas selesai | 10 (aktivitas dan pemain) | `domain/activities` |
| Undian reward | 3 undian per aktivitas, tabel drop di `domain/activities` | |
| Bonus yield level aktivitas | +2% per level di atas 1 | `domain/activities::yield_bonus_pct` |
| Level cap aktivitas | Kampung 10, Desa 20, Kota Kecil 30, Kota Dagang 40, Metropolis 50 | `domain/progression::activity_level_cap` |
| Harga NPC (Bac/unit) | Gandum 3, Kentang 4, Kayu 3, Buah Hutan 3.5, Herba Liar 12 | `domain/resources::ITEMS` |
| Proyek Sumur | 50 Bac + Kayu x10, 5 menit | `domain/city::WELL` |
| Proyek Lumbung | 80 Bac + Kayu x15 + Gandum x10, 8 menit (+4 slot inventory) | `domain/city::GRANARY` |
| Proyek Balai Kampung | 150 Bac + Kayu x25 + Kentang x10, 10 menit (naik ke Desa; butuh Sumur & Lumbung) | `domain/city::TOWN_HALL` |

Aturan inventory: satu jenis item memakai satu slot dan menumpuk sampai stack limit.
Item yang tidak muat (stack penuh atau tidak ada slot baru) masuk Pending Claim dan
bisa diklaim lewat tombol saat ada ruang.

Wallet & City Treasury: semua perubahan Bac pribadi tercatat di `ledger_entries`, sedangkan City Treasury tercatat terpisah di `city_treasury_ledger` dengan `idempotency_key` unik.

## Aktivitas
Satu aktivitas memakai satu slot dari mulai sampai rewardnya diambil. Setelah durasi habis,
scheduler menandainya siap dan mengundi reward (memakai level aktivitas saat itu). Reward dan XP
diberikan saat pemain menekan Ambil Reward. Efficiency, Luck, Activity Tool, dan Crew belum ada.

## XP overflow
Saat level aktivitas sudah di cap, XP berlebih disimpan sebagai overflow dan otomatis diterapkan
ketika cap naik (misalnya saat Balai Kampung menaikkan kota dari Kampung ke Desa).

## Kota & Proyek
Setiap pemain memulai dengan kota berstatus **Kampung** dan City Treasury `0.00 Bac`.
Hanya satu proyek kota yang dapat berjalan pada satu waktu, dan diselesaikan otomatis oleh scheduler persisten.

## Jual ke NPC & Pasar Makelar
- **NPC Market (dinamis):** harga beli NPC bergerak dalam rentang `0.70x` sampai `1.50x` dari harga dasar berdasarkan pembaruan berkala (`market_tick` tiap 15 menit, diskalakan `ACTIVITY_DURATION_PCT`) dan volume penjualan pemain (`domain/market`).
- **Makelar / Broker:** tanpa trading langsung antar pemain. Pemain dapat menitipkan maksimal 5 listing aktif dengan pilihan target harga (+10% / +25% / +40%) dan durasi (1 / 6 / 24 jam).
  - **Fee listing:** 2% dari total nilai target (minimal `1.00 Bac`), dibayar di muka dan tidak dikembalikan jika batal/kedaluwarsa.
  - **Komisi penjualan:** 5% dari harga jual kotor saat permintaan NPC menyentuh harga target; hasil bersih masuk Wallet dan tercatat di `ledger_entries`.
