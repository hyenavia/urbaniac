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
| Durasi Fishing / Mining | 8 menit (Desa) / 12 menit (Kota Kecil) | `domain/activities::FISHING`, `MINING` |
| XP per aktivitas selesai | 10 (Farm/Gather), 15 (Fishing), 20 (Mining) | `domain/activities` |
| Undian reward | 3 undian per aktivitas, tabel drop di `domain/activities` | |
| Bonus yield level aktivitas | +2% per level di atas 1 | `domain/activities::yield_bonus_pct` |
| Level cap aktivitas | Kampung 10, Desa 20, Kota Kecil 30, Kota Dagang 40, Metropolis 50 | `domain/progression::activity_level_cap` |
| Harga NPC (Bac/unit) | Gandum 3, Kentang 4, Kayu 3, Buah Hutan 3.5, Herba Liar 12 | `domain/resources::ITEMS` |
| Proyek Sumur | 50 Bac + Kayu x10, 5 menit | `domain/city::WELL` |
| Proyek Lumbung (Lv 1–5) | 80×Lv Bac + Kayu 15×Lv + Gandum 10×Lv, 8 menit (+4 slot/Lv) | `domain/city::GRANARY` |
| Proyek Balai Kampung | 150 Bac + Kayu x25 + Kentang x10, 10 menit (naik ke Desa; butuh Sumur & Lumbung) | `domain/city::TOWN_HALL` |
| Proyek Pasar Desa | 200 Bac + Papan x10 + Roti x5, 10 menit (Desa) | `domain/city::MARKET_HALL` |
| Proyek Balai Kota Kecil | 350 Bac + Peti x5 + Ikan Sungai x10, 15 menit (naik ke Kota Kecil) | `domain/city::CITY_HALL` |
| Proyek Pelabuhan Dagang | 600 Bac + Papan x20 + Bijih Besi x10, 20 menit (naik ke Kota Dagang) | `domain/city::TRADE_PORT` |
| Proyek Menara Metropolis | 1200 Bac + Kecubung x5 + Tonik Herba x5, 30 menit (naik ke Metropolis) | `domain/city::METROPOLIS_TOWER` |

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

## Crafting, Pesanan NPC, & Kru
- **Crafting:** `Raw -> Processed -> Finished` (`domain/crafting`). Maks 2 crafting berjalan bersamaan; `Auto-Sell` dapat diaktifkan agar hasil craft langsung terjual ke harga pasar NPC.
- **Pesanan NPC & Reputasi:** hingga 3 pesanan aktif dengan bonus +25% dari harga dasar NPC, ditambah +3% per level reputasi toko.
- **Kru & Automasi:** maksimal 4 kru (`100.00 Bac` rekrut). Memberi bonus Efficiency (durasi), Yield (hasil), dan Luck (peluang roll ekstra), serta otomasi ulang aktivitas tanpa menambah Activity Slot.
Setiap pemain memulai dengan kota berstatus **Kampung** dan City Treasury `0.00 Bac`.
Hanya satu proyek kota yang dapat berjalan pada satu waktu, dan diselesaikan otomatis oleh scheduler persisten.

## Jual ke NPC & Pasar Makelar
- **NPC Market (dinamis) & Kalender Global:** harga beli NPC bergerak dari multiplier pasar (`0.70x`–`1.50x`) dikalikan modifier **Kalender Global** (`Musim`, `Event Dunia`, dan `Tren Item` di `domain/world`).
- **Makelar / Broker:** tanpa trading langsung antar pemain. Pemain dapat menitipkan maksimal 5 listing aktif dengan pilihan target harga (+10% / +25% / +40%) dan durasi (1 / 6 / 24 jam).
  - **Fee listing:** 2% dari total nilai target (minimal `1.00 Bac`), dibayar di muka dan tidak dikembalikan jika batal/kedaluwarsa.
  - **Komisi penjualan:** 5% dari harga jual kotor saat permintaan NPC menyentuh harga target; hasil bersih masuk Wallet dan tercatat di `ledger_entries`.

## Ekonomi Kota (Pajak, Maintenance, Welfare) & Reward Progresi
- **Siklus Ekonomi Kota:** scheduler (`city_economy_tick` tiap 30 menit, diskalakan `ACTIVITY_DURATION_PCT`) memungut **Citizen Tax** ke City Treasury lalu membayar maintenance fasilitas (`5 Bac/fasilitas`) & gaji kru (`8 Bac/kru`).
- **Tanpa saldo negatif & tanpa denda:** jika Treasury kosong, Treasury tetap `0.00 Bac`, namun **Welfare** dan **Efektivitas** turun bertahap (mengurangi bonus yield aktivitas).
- **Reward Progresi:** pemain mendapat bonus Bac otomatis & idempoten saat mencapai Level 5 (`100 Bac`), 10 (`250 Bac`), 15 (`400 Bac`), dan 25 (`800 Bac`), dan XP overflow aktivitas langsung diterapkan saat stage kota naik.
