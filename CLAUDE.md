# Catatan Lingkungan Proyek (Urbaniac)

- **PC Development (mesin ini):** hanya digunakan untuk menulis kode dan verifikasi lokal (`cargo check`, `cargo test --lib`, `cargo test --no-run`). Tidak ada PostgreSQL lokal maupun file `.env` di PC ini.
- **Laptop (VPS / Bot Host):** database PostgreSQL, file `.env` (`DISCORD_TOKEN`, `DATABASE_URL`, dll.), eksekusi bot (`cargo run`), dan integration test (`cargo test`) dijalankan di laptop yang dijadikan VPS.
- **Aturan Git:** jangan menjalankan `git tag` secara otomatis. Untuk workflow deployment otomatis: setelah perubahan selesai dan diverifikasi (`cargo check` / `cargo test --lib`), commit dan push ke `main` sesuai instruksi pengguna, lalu pantau GitHub Actions hingga workflow selesai.
- **Deployment Otomatis (GitHub Actions):**
  1. Lakukan perubahan kode di repository `hyenavia/urbaniac`, branch `main`.
  2. Jangan melakukan `cargo build --release` secara lokal di PC.
  3. GitHub Actions (`.github/workflows/build-release.yml`) akan otomatis membangun `urbaniac.exe` dan menerbitkannya ke GitHub Releases (`urbaniac-<sha>`) setiap kali push ke `main`.
  4. Jangan mengubah konfigurasi PostgreSQL, `.env`, webhook, atau supervisor laptop tanpa instruksi.
  5. Laporkan commit SHA dan status workflow setelah push, dan pastikan workflow berhasil sebelum menyatakan deployment siap.
