import type { StreamStatus } from "../api/sse";
import type { CitySnapshot } from "../api/types";
import { tr, type UiLang } from "../i18n";

interface TopHudProps {
  snapshot: CitySnapshot;
  refreshing: boolean;
  streamStatus: StreamStatus;
  lang: UiLang;
  busy: boolean;
  onRefresh: () => void;
  onToggleLang: () => void;
}

export function TopHud({
  snapshot,
  refreshing,
  streamStatus,
  lang,
  busy,
  onRefresh,
  onToggleLang,
}: TopHudProps) {
  const { me, city } = snapshot;
  const profile = me.profile;
  const s = tr(lang);
  const xpPct = profile
    ? Math.min(100, Math.round((profile.xp / Math.max(1, profile.xp_needed)) * 100))
    : 0;

  return (
    <header className="top-hud">
      <div className="hud-identity">
        <div className="hud-avatar">
          {me.avatar_url ? (
            <img src={me.avatar_url} alt={me.username} />
          ) : (
            me.username.slice(0, 1).toUpperCase()
          )}
        </div>
        <div>
          <div className="hud-name-row">
            <span className="hud-name">{me.username}</span>
            {profile && <span className="pill pill-gold">Lv {profile.level}</span>}
            {city && (
              <span className="pill pill-green">
                {city.stage_name} • {city.era}
              </span>
            )}
            <span
              className={`pill ${
                streamStatus === "connected" ? "pill-sky" : "pill-orange"
              }`}
              title="Status koneksi real-time"
            >
              <span className={`live-dot ${streamStatus}`} />
              {streamStatus === "connected" ? s.liveOn : s.liveReconnecting}
            </span>
          </div>
          {profile && (
            <div className="xp-bar" title={`XP ${profile.xp} / ${profile.xp_needed}`}>
              <span style={{ width: `${xpPct}%` }} />
            </div>
          )}
        </div>
      </div>

      <div className="hud-metrics">
        <div className="metric-chip gold">
          <span>Wallet</span>
          <strong>{profile ? `${profile.balance} Bac` : "—"}</strong>
        </div>
        <div className="metric-chip">
          <span>Treasury</span>
          <strong>{city ? `${city.treasury} Bac` : "—"}</strong>
        </div>
        <div className="metric-chip">
          <span>Welfare</span>
          <strong>
            {city ? `${city.welfare}% (${city.effectiveness_pct}%)` : "—"}
          </strong>
        </div>
        <button
          type="button"
          className="btn-ghost"
          disabled={busy}
          onClick={onToggleLang}
          title="Ganti Bahasa (ID / EN)"
        >
          {lang.toUpperCase()}
        </button>
        <button
          type="button"
          className="btn-ghost"
          onClick={onRefresh}
          disabled={refreshing}
        >
          {refreshing ? s.refreshing : s.refresh}
        </button>
      </div>
    </header>
  );
}
