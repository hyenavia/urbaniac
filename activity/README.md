# Urbaniac Activity Frontend (`activity/`)

Frontend visual untuk Discord Activity Urbaniac berbasis **React + TypeScript + Vite + PixiJS (Sudut 3/4 View) + `@discord/embedded-app-sdk`**.

> Catatan PowerShell di Windows: jika `npm` diblokir oleh Execution Policy (`npm.ps1`), gunakan **`npm.cmd`**.

---

## 1. Pengujian Cepat di Browser PC (Mock API)

Cocok untuk menguji tampilan peta 3/4 view, navigasi, panel inspeksi, dan preset data tanpa perlu PostgreSQL atau backend Rust:

```powershell
cd e:\myLittleProject\urbaniac\activity
npm.cmd install
npm.cmd run dev
```

Buka `http://localhost:5173` di browser. Di pojok kiri atas kanvas terdapat toolbar **MOCK DEV** untuk mengganti preset:
- `Kampung (Awal)`
- `Desa (Fishing)`
- `Metropolis (Lengkap)`
- `Belum /start`

---

## 2. Pengujian di Dalam Discord (Desktop / Mobile) via Cloudflare Tunnel Dev

Gunakan aplikasi Discord **khusus development** (terpisah dari bot production) agar konfigurasi server production tidak tersentuh.

1. Salin `.env.example` menjadi `.env.local` di folder `activity/`:
   ```dotenv
   VITE_USE_MOCK_API=true
   VITE_DISCORD_CLIENT_ID=<CLIENT_ID_APP_DEV>
   ```
   *(Jangan pernah menaruh `DISCORD_CLIENT_SECRET` di file frontend.)*
2. Jalankan Vite dev server di PC:
   ```powershell
   npm.cmd run dev
   ```
3. Di terminal lain di PC, jalankan Cloudflare Tunnel ke port `5173`:
   ```powershell
   cloudflared tunnel --url http://localhost:5173
   ```
   Salin hostname tunnel yang muncul (misalnya `xxxx-yyyy.trycloudflare.com`).
4. Di **Discord Developer Portal** (aplikasi dev):
   - Buka **Activities -> Settings** dan aktifkan Activity.
   - Buka **Activities -> URL Mappings**, isi Prefix `/` -> Target `xxxx-yyyy.trycloudflare.com` (tanpa `https://`).
5. Buka channel voice/text di server test Discord, klik tombol **Launch Activity** pada aplikasi dev tersebut.

---

## 3. Pengujian Integrasi Penuh dengan Backend Rust (`src/web/`)

Saat menguji dengan backend Axum + PostgreSQL dev (misalnya di laptop):

1. Pastikan backend berjalan dengan `ACTIVITY_ENABLED=true`, `DISCORD_CLIENT_ID`, dan `DISCORD_CLIENT_SECRET` terisi.
2. Di `activity/.env.local`:
   ```dotenv
   VITE_USE_MOCK_API=false
   VITE_DISCORD_CLIENT_ID=<CLIENT_ID_APP_DEV>
   VITE_API_PROXY_TARGET=http://127.0.0.1:8080
   ```
3. Jalankan `npm.cmd run dev` + `cloudflared tunnel --url http://localhost:5173`, atau build produksi lokal dengan `npm.cmd run build` (hasilnya ada di `activity/dist/`).
