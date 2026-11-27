# Hardware & Deployment Setup

Urbaniac dirancang agar ringan untuk dijalankan di laptop/mini-PC rumah (sebagai host bot & database PostgreSQL) sementara pengembangan kode dapat dilakukan di PC terpisah.

## Target Resource
- **Koneksi PostgreSQL**: dibatasi maksimal `5` koneksi (`DB_MAX_CONNECTIONS=5`).
- **Tokio runtime**: `worker_threads = 2` pada `src/main.rs`.
- **Kompilasi Cargo**: `.cargo/config.toml` membatasi build job agar penggunaan CPU & RAM tetap terkendali.
- **Scheduler**: satu instance terpusat (batch 25, konkurensi 2), tidur hingga job berikutnya jatuh tempo (tanpa polling rapat dan tanpa timer per pemain).

## Pemisahan Lingkungan (Dev PC vs Laptop VPS / Bot Host)
- **Dev PC (mesin coding):** hanya untuk menulis kode, verifikasi `cargo check`, `cargo test --lib`, dan `cargo test --no-run` (tanpa butuh PostgreSQL atau `.env`), lalu commit & push ke Git.
- **Laptop (VPS / Bot Host):** seluruh database PostgreSQL 15+, file `.env` (`DISCORD_TOKEN`, `DATABASE_URL`, `DISCORD_GUILD_ID`, `ACTIVITY_DURATION_PCT`), eksekusi `cargo test` (integration test), dan runtime bot (`cargo run --release`) tersedia dan dijalankan di laptop ini.

## Backup Database Sebelum Milestone Baru
```powershell
pg_dump -U postgres urbaniac > backup_milestone-N.sql
```
