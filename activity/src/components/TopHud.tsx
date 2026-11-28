import type { StreamStatus } from "../api/sse";
import type { CitySnapshot } from "../api/types";
import { tr } from "../i18n";

interface TopHudProps {
  snapshot: CitySnapshot;
  refreshing: boolean;
  streamStatus: StreamStatus;
  onRefresh: () => void;
}

function PixelCoinIcon() {
  return (
    <svg
      className="pixel-icon"
      width="12"
      height="12"
      viewBox="0 0 12 12"
      shapeRendering="crispEdges"
      aria-hidden="true"
    >
      <rect x="3" y="1" width="6" height="1" fill="#2d1a0e" />
      <rect x="2" y="2" width="1" height="8" fill="#2d1a0e" />
      <rect x="9" y="2" width="1" height="8" fill="#2d1a0e" />
      <rect x="3" y="10" width="6" height="1" fill="#2d1a0e" />
      <rect x="3" y="2" width="6" height="8" fill="#f5c242" />
      <rect x="3" y="2" width="4" height="2" fill="#ffe680" />
      <rect x="5" y="4" width="2" height="4" fill="#bd811a" />
    </svg>
  );
}

function PixelChestIcon() {
  return (
    <svg
      className="pixel-icon"
      width="12"
      height="12"
      viewBox="0 0 12 12"
      shapeRendering="crispEdges"
      aria-hidden="true"
    >
      <rect x="1" y="2" width="10" height="8" fill="#2d1a0e" />
      <rect x="2" y="3" width="8" height="2" fill="#b87442" />
      <rect x="2" y="5" width="8" height="4" fill="#8c4f29" />
      <rect x="2" y="5" width="8" height="1" fill="#2d1a0e" />
      <rect x="5" y="4" width="2" height="3" fill="#ffe680" />
    </svg>
  );
}

function PixelHeartIcon() {
  return (
    <svg
      className="pixel-icon"
      width="12"
      height="12"
      viewBox="0 0 12 12"
      shapeRendering="crispEdges"
      aria-hidden="true"
    >
      <rect x="2" y="2" width="3" height="1" fill="#2d1a0e" />
      <rect x="7" y="2" width="3" height="1" fill="#2d1a0e" />
      <rect x="1" y="3" width="10" height="4" fill="#d9443b" />
      <rect x="2" y="7" width="8" height="2" fill="#d9443b" />
      <rect x="4" y="9" width="4" height="1" fill="#d9443b" />
      <rect x="5" y="10" width="2" height="1" fill="#2d1a0e" />
      <rect x="2" y="3" width="2" height="2" fill="#ff948a" />
    </svg>
  );
}

export function TopHud({
  snapshot,
  refreshing,
  streamStatus,
  onRefresh,
}: TopHudProps) {
  const { me, city } = snapshot;
  const profile = me.profile;
  const s = tr();
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
              title="Real-time connection status"
            >
              <span className={`live-dot ${streamStatus}`} />
              {streamStatus === "connected" ? s.liveOn : s.liveReconnecting}
            </span>
            {me.voice_boost_active && (
              <span
                className="pill pill-boost"
                title="Voice Boost active while connected to Voice Boost"
              >
                🔊 Boost +{me.voice_boost_speed_pct ?? 20}% Spd • +
                {me.voice_boost_xp_pct ?? 15}% XP
              </span>
            )}
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
          <span className="metric-chip-label">
            <PixelCoinIcon />
            Wallet
          </span>
          <strong>{profile ? `${profile.balance} Bac` : "—"}</strong>
        </div>
        <div className="metric-chip">
          <span className="metric-chip-label">
            <PixelChestIcon />
            Treasury
          </span>
          <strong>{city ? `${city.treasury} Bac` : "—"}</strong>
        </div>
        <div className="metric-chip">
          <span className="metric-chip-label">
            <PixelHeartIcon />
            Welfare
          </span>
          <strong>
            {city ? `${city.welfare}% (${city.effectiveness_pct}%)` : "—"}
          </strong>
        </div>
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
