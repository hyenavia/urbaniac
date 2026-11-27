# Catatan Lingkungan Proyek (Urbaniac)

- **PC Development (mesin ini):** hanya digunakan untuk menulis kode dan verifikasi lokal (`cargo check`, `cargo test --lib`, `cargo test --no-run`). Tidak ada PostgreSQL lokal maupun file `.env` di PC ini.
- **Laptop (VPS / Bot Host):** database PostgreSQL, file `.env` (`DISCORD_TOKEN`, `DATABASE_URL`, dll.), eksekusi bot (`cargo run`), dan integration test (`cargo test`) dijalankan di laptop yang dijadikan VPS.
- **Aturan Git:** jangan menjalankan `git commit` atau `git tag` secara langsung/otomatis; biarkan pengguna yang me-review dan meng-commit sendiri (kecuali diminta secara eksplisit).
- **Git (`commit` & `push`):** jangan lakukan `git commit` maupun `git push` secara otomatis; tunggu instruksi eksplisit dari pengguna.
