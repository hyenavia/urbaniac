# Urbaniac Website

Responsive static landing page and legal documentation for GitHub Pages.

## Included
- `index.html` — Urbaniac landing page (`Urbaniac — A Discord-Based Urban Life RPG`) with progressive 5-stage SVG city skyline showcase, gameplay system cards, and interactive Discord embed preview
- `terms/index.html` — Terms of Service (`/terms/`)
- `privacy/index.html` — Privacy Policy (`/privacy/`)
- `style.css` — Midnight Sky-Slate × Urbaniac key-art palette (`#0F1726`, `#162133`, `#1E2D45`, `#FFD166`, `#7CC260`, `#56C2F0`), `Space Grotesk` + `Plus Jakarta Sans` + `JetBrains Mono`, WCAG AA/AAA contrast, GPU-composited transitions, and `prefers-reduced-motion` support
- `script.js` — accessible mobile drawer (`<= 900px`), interruptible 5-stage city progression tabs with progressive SVG skyline and GPU `scaleX` progress rail, interactive Discord embed switcher (`Market` / `Crafting` / `Crew`), scroll-spy navigation indicator, and dynamic footer year
- `assets/banner-1440.webp` & `assets/banner-720.webp` — responsive WebP key art (`1440×540` & `720×270`)
- `assets/banner.jpg` — JPEG fallback & OpenGraph / Twitter social preview (`1024×384`)
- `assets/icon-96.webp`, `assets/favicon.png`, `assets/icon.png` — optimized navbar mark (`96×96` WebP), browser favicon (`64×64` PNG), and Apple Touch icon (`180×180` PNG)

## Details already configured
- Support email: `urbaniac.support@gmail.com`
- Discord invite: `https://discord.gg/sRUcs3NZpC`
- Gameplay scope: synced with Milestone 11 / Fase 9 (5 city stages from Hamlet to Metropolis, 4 activities, dynamic NPC market & broker, tiered crafting & orders, crew automation, and zero-debt city governance)

## Publish with GitHub Pages
1. Open repository **Settings → Pages**.
2. Under **Build and deployment**, select **Deploy from a branch**.
3. Select branch `main` and folder `/docs` (or `/ (root)` if hosted in a dedicated website repo), then save.
4. Wait for the published URL shown in the Pages settings.

This is a static informational website. It does not include a player login, live game dashboard, database, or ticketing system.
