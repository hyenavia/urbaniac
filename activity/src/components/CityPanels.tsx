import { useState } from "react";
import { newRequestId } from "../api/client";
import type { ActivityKindKey, CitySnapshot } from "../api/types";
import type { MapSlotId } from "../scene/assetRegistry";
import { formatCountdown } from "./InspectorPanel";

export type NavTab =
  | "map"
  | "city"
  | "inventory"
  | "activities"
  | "market"
  | "crafting"
  | "crew";

interface CityPanelsProps {
  tab: "city" | "inventory" | "activities";
  snapshot: CitySnapshot;
  nowMs: number;
  busy: boolean;
  onClose: () => void;
  onDepositTreasury: (amount: string, requestId: string) => void;
  onWithdrawTreasury: (amount: string, requestId: string) => void;
  onSetTaxRate: (pct: number) => void;
  onClaimPending: () => void;
  onSellToNpc: (params: {
    itemId: string;
    quantity?: number;
    sellAll?: boolean;
    requestId: string;
  }) => void;
  onStartActivity: (kind: ActivityKindKey) => void;
  onCollectActivities: () => void;
  onInspectSlot: (slotId: MapSlotId) => void;
}

const TAX_OPTIONS = [5, 10, 15, 20];

export function CityPanels({
  tab,
  snapshot,
  nowMs,
  busy,
  onClose,
  onDepositTreasury,
  onWithdrawTreasury,
  onSetTaxRate,
  onClaimPending,
  onSellToNpc,
  onStartActivity,
  onCollectActivities,
  onInspectSlot,
}: CityPanelsProps) {
  const { me, city, inventory, activities, quotes, clockOffsetMs } = snapshot;
  const [treasuryAmount, setTreasuryAmount] = useState("25");

  const title =
    tab === "city"
      ? "Profil & Ekonomi Kota"
      : tab === "inventory"
        ? "Inventory & Pending Claim"
        : "Aktivitas & Proyek Kota";

  const readyRuns =
    activities?.runs.filter((r) => r.status === "ready").length ?? 0;
  const hasFreeSlot =
    (activities?.runs.length ?? 0) < (activities?.slots_total ?? 2);

  return (
    <aside className="side-panel">
      <div className="panel-header">
        <h2>{title}</h2>
        <button type="button" className="btn-ghost" onClick={onClose}>
          Peta
        </button>
      </div>

      <div className="panel-scroll">
        {tab === "city" && city && me.profile && (
          <>
            <div className="card">
              <h3>Profil Pemain</h3>
              <div className="kv-row">
                <span>Nama</span>
                <strong>{me.username}</strong>
              </div>
              <div className="kv-row">
                <span>Level & XP</span>
                <span className="mono">
                  Lv {me.profile.level} ({me.profile.xp}/{me.profile.xp_needed})
                </span>
              </div>
              <div className="kv-row">
                <span>Wallet Pribadi</span>
                <strong className="mono">{me.profile.balance} Bac</strong>
              </div>
              <div className="kv-row">
                <span>Bahasa</span>
                <span>{me.profile.lang === "id" ? "Indonesia" : "English"}</span>
              </div>
            </div>

            <div className="card">
              <h3>Ekonomi Kota ({city.stage_name})</h3>
              <div className="kv-row">
                <span>City Treasury</span>
                <strong className="mono">{city.treasury} Bac</strong>
              </div>
              <div className="kv-row">
                <span>Tarif Pajak</span>
                <span className="mono">{city.tax_rate_pct}%</span>
              </div>
              <div className="kv-row">
                <span>Estimasi Pajak / Biaya</span>
                <span className="mono">
                  +{city.est_tax_income} / -{city.est_expense}
                </span>
              </div>
              <div className="kv-row">
                <span>Welfare & Efektivitas</span>
                <span className="mono">
                  {city.welfare}% / {city.effectiveness_pct}%
                </span>
              </div>
              <div className="kv-row">
                <span>Bonus Yield Kota</span>
                <span className="mono ok-text">+{city.yield_bonus_pct}%</span>
              </div>
              <div className="kv-row">
                <span>Kru Bertugas</span>
                <span className="mono">{city.assigned_crews}</span>
              </div>
            </div>

            <div className="card">
              <h3>Atur Tarif Pajak</h3>
              <div className="tax-grid">
                {TAX_OPTIONS.map((pct) => (
                  <button
                    key={pct}
                    type="button"
                    disabled={busy}
                    className={`tax-btn ${city.tax_rate_pct === pct ? "active" : ""}`}
                    onClick={() => onSetTaxRate(pct)}
                  >
                    {pct}%
                  </button>
                ))}
              </div>
            </div>

            <div className="card">
              <h3>Kelola City Treasury</h3>
              <div className="action-row">
                <input
                  type="number"
                  min={1}
                  step="1"
                  className="input-field"
                  style={{ flex: 1 }}
                  value={treasuryAmount}
                  onChange={(e) => setTreasuryAmount(e.target.value)}
                  placeholder="Jumlah Bac"
                />
              </div>
              <div className="action-row">
                <button
                  type="button"
                  className="btn-success btn-sm"
                  disabled={busy}
                  onClick={() =>
                    onDepositTreasury(treasuryAmount, newRequestId())
                  }
                >
                  Setor ke Treasury
                </button>
                <button
                  type="button"
                  className="btn-ghost btn-sm"
                  disabled={busy}
                  onClick={() =>
                    onWithdrawTreasury(treasuryAmount, newRequestId())
                  }
                >
                  Tarik ke Wallet
                </button>
              </div>
            </div>

            <div className="card">
              <h3>Fasilitas Terbangun ({city.facilities.length})</h3>
              {city.facilities.length === 0 ? (
                <p className="muted">Belum ada fasilitas terbangun.</p>
              ) : (
                city.facilities.map((f) => (
                  <div className="list-item" key={f.id}>
                    <span>{f.name}</span>
                    <span className="pill pill-green">Lv {f.level}</span>
                  </div>
                ))
              )}
            </div>
          </>
        )}

        {tab === "inventory" && inventory && (
          <>
            <div className="card">
              <div className="kv-row">
                <span>Kapasitas Slot</span>
                <strong className="mono">
                  {inventory.slots_used} / {inventory.slot_capacity}
                </strong>
              </div>
            </div>

            <div className="card">
              <div className="kv-row">
                <h3>Pending Claim ({inventory.pending.length})</h3>
                {inventory.pending.length > 0 && (
                  <button
                    type="button"
                    className="btn-primary btn-sm"
                    disabled={busy}
                    onClick={onClaimPending}
                  >
                    Klaim Pending
                  </button>
                )}
              </div>
              {inventory.pending.length === 0 ? (
                <p className="muted">Tidak ada item menunggu klaim.</p>
              ) : (
                inventory.pending.map((p) => (
                  <div className="list-item" key={p.item_id}>
                    <div>
                      <strong>{p.name}</strong>
                      <div className="muted">{p.rarity}</div>
                    </div>
                    <strong className="mono warn-text">x{p.quantity}</strong>
                  </div>
                ))
              )}
            </div>

            <div className="card">
              <h3>Item Inventory</h3>
              {inventory.items.length === 0 ? (
                <p className="muted">Belum ada item di inventory.</p>
              ) : (
                inventory.items.map((it) => (
                  <div
                    className="list-item"
                    key={it.item_id}
                    style={{ flexDirection: "column", alignItems: "stretch" }}
                  >
                    <div className="kv-row">
                      <div>
                        <strong>{it.name}</strong>
                        <div className="muted">
                          {it.rarity} • NPC {it.npc_unit_price} Bac
                        </div>
                      </div>
                      <strong className="mono">x{it.quantity}</strong>
                    </div>
                    <div className="action-row">
                      <button
                        type="button"
                        className="btn-success btn-sm"
                        disabled={busy}
                        onClick={() =>
                          onSellToNpc({
                            itemId: it.item_id,
                            quantity: 1,
                            requestId: newRequestId(),
                          })
                        }
                      >
                        Jual 1
                      </button>
                      <button
                        type="button"
                        className="btn-primary btn-sm"
                        disabled={busy}
                        onClick={() =>
                          onSellToNpc({
                            itemId: it.item_id,
                            sellAll: true,
                            requestId: newRequestId(),
                          })
                        }
                      >
                        Jual Semua
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
          </>
        )}

        {tab === "activities" && activities && (
          <>
            <div className="card">
              <div className="kv-row">
                <h3>
                  Slot Aktivitas ({activities.runs.length} /{" "}
                  {activities.slots_total})
                </h3>
                {readyRuns > 0 && (
                  <button
                    type="button"
                    className="btn-primary btn-sm"
                    disabled={busy}
                    onClick={onCollectActivities}
                  >
                    Ambil Semua ({readyRuns})
                  </button>
                )}
              </div>
              {activities.runs.length === 0 ? (
                <p className="muted">Semua slot aktivitas sedang kosong.</p>
              ) : (
                activities.runs.map((r) => (
                  <div className="list-item" key={r.id}>
                    <span>{r.name}</span>
                    {r.status === "ready" ? (
                      <span className="pill pill-gold">SIAP</span>
                    ) : (
                      <span className="mono warn-text">
                        {formatCountdown(r.ends_at, nowMs, clockOffsetMs)}
                      </span>
                    )}
                  </div>
                ))
              )}
            </div>

            <div className="card">
              <h3>Mulai Aktivitas</h3>
              {activities.catalog.map((cat) => (
                <div className="list-item" key={cat.kind}>
                  <div>
                    <strong>{cat.name}</strong>
                    <div className="muted">
                      {cat.unlocked
                        ? `${cat.base_duration_secs}d • +${cat.xp_reward} XP`
                        : `Butuh ${cat.min_stage_name}`}
                    </div>
                  </div>
                  <button
                    type="button"
                    className="btn-success btn-sm"
                    disabled={!cat.unlocked || !hasFreeSlot || busy}
                    onClick={() => onStartActivity(cat.kind)}
                  >
                    Mulai
                  </button>
                </div>
              ))}
            </div>

            <div className="card">
              <h3>Progres Level Aktivitas</h3>
              {activities.progress.map((p) => (
                <div className="list-item" key={p.kind}>
                  <div>
                    <strong>{p.name}</strong>
                    <div className="muted">
                      XP {p.xp}/{p.xp_needed}
                      {p.overflow > 0 ? ` • Overflow ${p.overflow}` : ""}
                    </div>
                  </div>
                  <span className="pill pill-sky">
                    Lv {p.level}/{p.cap}
                  </span>
                </div>
              ))}
            </div>

            <div className="card">
              <h3>Daftar Proyek Kota</h3>
              {(quotes ?? []).map((q) => (
                <div className="list-item" key={q.project}>
                  <div>
                    <strong>{q.name}</strong>
                    <div className="muted">
                      Lv {q.current_level}/{q.max_level} •{" "}
                      {q.next_level === null
                        ? "Maksimal"
                        : `${q.cost_bac} Bac`}
                    </div>
                  </div>
                  <button
                    type="button"
                    className="btn-ghost btn-sm"
                    onClick={() => onInspectSlot(q.project)}
                  >
                    {q.next_level === null
                      ? "MAX"
                      : q.can_start
                        ? "Bangun"
                        : "Detail"}
                  </button>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </aside>
  );
}
