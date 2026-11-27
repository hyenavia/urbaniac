# Game Rules (Initial Playable Scope)

Semua angka di bawah adalah nilai awal sementara (spesifikasi tidak menentukannya).

| Aturan                               | Nilai                                                                                          | Lokasi di kode                                                                 |
| ------------------------------------ | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Bac awal pemain                      | 100.00                                                                                         | `domain/economy::starter_bac`                                                  |
| Slot inventory dasar                 | 12 (+4 per level Lumbung, tanpa batas jumlah per item)                                         | `domain/resources::BASE_SLOT_CAPACITY`, `domain/city::GRANARY_SLOTS_PER_LEVEL` |
| XP naik level (pemain dan aktivitas) | 100 x level                                                                                    | `domain/player::xp_to_next`                                                    |
| Activity Slot per City Stage         | Kampung 2, Desa 3, Kota Kecil+ 4                                                               | `domain/activities::activity_slots_for_stage`                                  |
| Durasi Farming / Gathering           | 10 menit / 5 menit                                                                             | `domain/activities::FARMING`, `GATHERING`                                      |
| Durasi Fishing / Mining              | 8 menit (Desa) / 12 menit (Kota Kecil)                                                         | `domain/activities::FISHING`, `MINING`                                         |
| XP per aktivitas selesai             | 35 (Farm), 25 (Gather), 40 (Fishing), 55 (Mining)                                              | `domain/activities`                                                            |
| Undian reward                        | 3 undian per aktivitas, tabel drop di `domain/activities`                                      |                                                                                |
| Bonus yield level aktivitas          | +4% per level di atas 1                                                                        | `domain/activities::yield_bonus_pct`                                           |
| Level cap aktivitas                  | Kampung 10, Desa 20, Kota Kecil 30, Kota Dagang 40, Metropolis 50                              | `domain/progression::activity_level_cap`                                       |
| Harga NPC (Bac/unit)                 | Raw 3–12, Processed 9.5–40, Finished 28–68                                                     | `domain/resources::ITEMS`                                                      |
| Proyek Sumur                         | 50 Bac + Kayu x10, 5 menit (+10% Farming & Gathering yield)                                    | `domain/city::WELL`                                                            |
| Proyek Lumbung (Lv 1–5)              | 80×Lv Bac + Kayu 15×Lv + Gandum 10×Lv, 8 menit (+4 slot tas & +3% yield/Lv)                    | `domain/city::GRANARY`                                                         |
| Proyek Balai Kampung                 | 150 Bac + Kayu x25 + Kentang x10, 10 menit (naik ke Desa; butuh Sumur & Lumbung)               | `domain/city::TOWN_HALL`                                                       |
| Proyek Pasar Desa                    | 200 Bac + Papan x10 + Roti x5, 10 menit (+5% harga jual ke NPC; Desa)                          | `domain/city::MARKET_HALL`                                                     |
| Proyek Balai Kota Kecil              | 350 Bac + Peti x5 + Ikan Sungai x10, 15 menit (naik ke Kota Kecil)                             | `domain/city::CITY_HALL`                                                       |
| Proyek Pelabuhan Dagang              | 600 Bac + Papan x20 + Bijih Besi x10, 20 menit (naik ke Kota Dagang; 3 Slot Craft, +15% Order) | `domain/city::TRADE_PORT`                                                      |
| Proyek Menara Metropolis             | 1200 Bac + Kecubung x5 + Tonik Herba x5, 30 menit (naik ke Metropolis; +15% Speed)             | `domain/city::METROPOLIS_TOWER`                                                |

Aturan inventory: satu jenis item memakai satu slot dengan tumpukan tak terbatas.
Item baru yang tidak kebagian slot kosong masuk Pending Claim dan
bisa diklaim lewat tombol di menu Inventory saat ada slot kosong.

Wallet & City Treasury: semua perubahan Bac pribadi tercatat di `ledger_entries`, sedangkan City Treasury tercatat terpisah di `city_treasury_ledger` dengan `idempotency_key` unik. Pemain dapat setor dan tarik dari Treasury, dan proyek kota memotong Treasury terlebih dahulu sebelum Wallet.

## Aktivitas

Satu aktivitas memakai satu slot dari mulai sampai rewardnya diambil. Setelah durasi habis,
scheduler menandainya siap dan mengundi reward (memakai level aktivitas saat itu). Reward dan XP
diberikan saat pemain menekan Ambil Reward.

## XP overflow

Saat level aktivitas sudah di cap, XP berlebih disimpan sebagai overflow dan otomatis diterapkan
ketika cap naik (misalnya saat Balai Kampung menaikkan kota dari Kampung ke Desa).

## Crafting, Pesanan NPC, & Kru

- **Crafting:** `Raw -> Processed -> Finished` (`domain/crafting`) dengan dukungan batch crafting (1, 5, 10, Max, Custom). Maks 2 crafting berjalan bersamaan (3 setelah Pelabuhan Dagang); memberi XP pemain (+10 Processed / +20 Finished per batch), dan `Auto-Sell` dapat diaktifkan agar hasil craft langsung terjual ke harga pasar NPC.
- **Pesanan NPC & Reputasi:** hingga 3 pesanan aktif dengan bonus +35% dari harga dasar NPC, ditambah +3% per level reputasi toko (+15% lagi setelah Pelabuhan Dagang), serta tombol Ganti Pesanan (cooldown 5 menit) dan +20 XP pemain per unit pesanan.
- **Kru & Automasi:** maksimal 4 kru (`100.00 Bac` rekrut; spesialisasi hanya dari aktivitas yang sudah terbuka). Memberi bonus Efficiency, Yield, Luck, serta otomasi ulang aktivitas (langsung memulai aktivitas jika slot kosong saat ditugaskan `AUTO`).

## Jual ke NPC & Pasar Makelar

- **NPC Market (dinamis) & Kalender Global:** harga beli NPC bergerak dari multiplier pasar (`0.70x`–`1.60x`) dikalikan modifier **Kalender Global** (`Musim`, `Event Dunia`, dan `Tren Item`; 1 hari = 2 jam = 8 tick market).
- **Makelar / Broker:** tanpa trading langsung antar pemain. Pemain dapat menitipkan maksimal 5 listing aktif dengan pilihan target harga (+10% / +20% / +35%) dan durasi (2 / 6 / 24 jam).
  - **Fee listing:** 1% dari total nilai target (minimal `0.50 Bac`), dibayar di muka dan tidak dikembalikan jika batal/kedaluwarsa.
  - **Komisi penjualan:** 3% dari harga jual kotor saat permintaan NPC menyentuh harga target; hasil bersih masuk Wallet dan tercatat di `ledger_entries`.

## Ekonomi Kota (Pajak, Maintenance, Welfare) & Reward Progresi

- **Siklus Ekonomi Kota:** scheduler (`city_economy_tick` tiap 30 menit, diskalakan `ACTIVITY_DURATION_PCT`) memungut **Citizen Tax** ke City Treasury lalu membayar maintenance fasilitas (`5 Bac/fasilitas`) & gaji kru yang bertugas (`8 Bac/kru aktif`).
- **Tradeoff Pajak & Welfare:** pajak `5%` / `10%` menjaga Welfare menuju `100%` (bonus yield maksimal), sedangkan `15%` memberi pemasukan pajak lebih besar tetapi menurunkan Welfare ke `80%`. Jika Treasury kosong, Treasury tetap `0.00 Bac` dan Welfare turun bertahap.
- **Reward Progresi:** pemain mendapat bonus Bac otomatis & idempoten saat mencapai Level 5 (`100 Bac`), 10 (`250 Bac`), 15 (`400 Bac`), dan 25 (`800 Bac`), dan XP overflow aktivitas langsung diterapkan saat stage kota naik.
