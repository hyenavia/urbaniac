# Setup Windows 10 (tanpa Docker)

1. Install Rust stable MSVC dari https://rustup.rs (butuh Visual Studio Build Tools: "Desktop development with C++").
2. Install PostgreSQL 15+ untuk Windows. Buat database:
   `psql -U postgres -c "CREATE DATABASE urbaniac;"`
3. Buat bot di https://discord.com/developers/applications, salin token. Undang bot dengan scope
   `bot` dan `applications.commands`. Tidak perlu privileged intent.
4. Salin `.env.example` menjadi `.env`, isi `DISCORD_TOKEN` dan `DATABASE_URL`.
   Isi `DISCORD_GUILD_ID` agar slash command langsung muncul di servermu.
5. Jalankan:
   ```
   cargo check
   cargo test --lib          # unit test, tanpa database
   cargo test                # termasuk integration test (butuh DATABASE_URL)
   cargo run
   ```
   Migrasi dijalankan otomatis saat bot start. `.cargo/config.toml` sudah membatasi build ke 2 job.

## Testing cepat
Isi `ACTIVITY_DURATION_PCT=5` di `.env` agar aktivitas 20x lebih cepat (Farming 30 detik,
Gathering 15 detik). Kembalikan ke 100 untuk bermain normal.
