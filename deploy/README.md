# Panduan Deployment Urbaniac (Backend + Activity Frontend)

Dokumen ini menjelaskan format release asset, cara kerja perluasan `deploy/supervisor.ps1`, langkah pengujian, dan prosedur migrasi/rollback aman di laptop production.

---

## 1. Format Release Asset (GitHub Releases)

Setiap push ke `main` yang menyentuh Rust atau `activity/` memicu `.github/workflows/build-release.yml` dan menghasilkan satu release bertag `urbaniac-<40-char-sha>` berisi 5 file:

1. `urbaniac.exe` — binary Windows x86_64 MSVC (bot + optional Axum web API/static server).
2. `SHA256SUMS.txt` — checksum SHA-256 untuk `urbaniac.exe` dan `frontend-dist.zip`.
3. `frontend-dist.zip` — arsip isi `activity/dist/*` (dibangun dengan `VITE_USE_MOCK_API=false`).
4. `frontend-dist.zip.sha256` — checksum SHA-256 khusus `frontend-dist.zip`.
5. `release-manifest.json` — metadata commit (`commit`, `backend_tree_hash`, `frontend_tree_hash`, `exe_sha256`, `frontend_sha256`) agar supervisor tahu apakah backend berubah atau hanya frontend.

> Opsional di GitHub Repository Settings -> Variables: Anda dapat mengisi `VITE_DISCORD_CLIENT_ID`, tetapi jika tidak diisi, frontend otomatis membacanya dari `GET /api/config` milik server (`DISCORD_CLIENT_ID` di `.env` laptop).

---

## 2. Struktur Direktori di Laptop Production

Supervisor tidak menjalankan `cargo` maupun `npm`/`node` di laptop. Semua aset diunduh ke `C:\UrbaniacDeploy\`:

- `D:\Projects\urbaniac\target\deploy-<short>\release\urbaniac.exe` — executable per commit.
- `C:\UrbaniacDeploy\frontend\current\` — folder aktif yang dilayani Axum (`ACTIVITY_STATIC_DIR`).
- `C:\UrbaniacDeploy\frontend\previous\` — versi frontend sebelumnya untuk rollback cepat.
- `C:\UrbaniacDeploy\frontend\staging\` — folder sementara saat ekstrak & verifikasi `index.html`.
- File state supervisor:
  - `deployed-commit.txt`, `deployed-exe.txt`, `deployed-backend-hash.txt`, `deployed-frontend-commit.txt`, `bot.pid`.

---

## 3. Perilaku Perluasan Supervisor (`deploy/supervisor.ps1`)

1. **Verifikasi SHA-256 & Kelengkapan Asset:** jika release belum ada atau salah satu asset belum selesai diunggah, supervisor mencatat log dan **tidak menyentuh bot maupun frontend yang sedang berjalan**.
2. **Frontend-Only Update (Zero Bot Downtime):** jika `backend_tree_hash` (atau SHA-256 `urbaniac.exe`) sama dengan yang sedang berjalan, supervisor hanya memperbarui `C:\UrbaniacDeploy\frontend\current\` secara atomik **tanpa mematikan bot Rust**.
3. **Update Backend + Frontend Terkoordinasi:** frontend diswap ke `current` (menyimpan versi lama di `previous`), lalu bot baru dijalankan dan diverifikasi log `"bot terhubung"`. Jika startup gagal, supervisor otomatis mengembalikan `previous -> current` dan menyalakan executable lama agar backend dan frontend tetap di commit yang sama.
4. **Tahan Error Startup:** error jaringan/GitHub saat laptop boot tidak lagi menghentikan supervisor secara permanen; supervisor tetap masuk ke loop pemeriksaan 5 menit.

---

## 4. Prosedur Migrasi & Pengujian Aman (Jangan Langsung Aktifkan di Production Sebelum Siap)

1. **Merge & Tunggu CI Selesai (Bot Production Masih Tetap Jalan):**
   - Setelah Anda menyetujui merge `feat/discord-activity` -> `main`, pantau tab **Actions** di GitHub hingga release `urbaniac-<sha>` berisi kelima asset di atas.
   - Selama `ACTIVITY_ENABLED=false` (atau belum disetel di `.env` laptop), `urbaniac.exe` yang baru tetap 100% hanya menjalankan bot seperti biasa.
2. **Perbarui Script Supervisor di Laptop (Saat Maintenance Singkat):**
   - Backup script supervisor lama di laptop.
   - Salin [deploy/supervisor.ps1](supervisor.ps1) ke lokasi supervisor laptop dan restart supervisor.
   - Pastikan log `C:\UrbaniacDeploy\logs\supervisor.log` menunjukkan `C:\UrbaniacDeploy\frontend\current\index.html` berhasil terpasang dan bot tetap terhubung.
3. **Aktivasi Activity (Saat Sudah Siap Uji / Rilis di Laptop):**
   - Pastikan backup DB (`pg_dump -U postgres urbaniac > backup.sql`).
   - Tambahkan di `.env` laptop:
     ```dotenv
     ACTIVITY_ENABLED=true
     ACTIVITY_BIND_ADDR=127.0.0.1:8080
     ACTIVITY_STATIC_DIR=C:\UrbaniacDeploy\frontend\current
     DISCORD_CLIENT_ID=<APP_ID>
     DISCORD_CLIENT_SECRET=<CLIENT_SECRET>
     ```
   - Restart bot (atau biarkan supervisor menyalakan ulang), lalu arahkan Cloudflare Tunnel production ke `http://127.0.0.1:8080` dan uji `/api/health` serta Activity di Discord.

## 5. Prosedur Rollback Darurat

- **Jika Activity bermasalah sementara bot harus tetap jalan:** set `ACTIVITY_ENABLED=false` di `.env` laptop dan restart bot — web server langsung nonaktif tanpa mengganggu bot atau PostgreSQL.
- **Jika perlu kembali ke commit sebelumnya:** reset/revert `main` di GitHub (atau arahkan `deployed-exe.txt` ke executable lama & pindahkan `frontend\previous` ke `frontend\current`).
