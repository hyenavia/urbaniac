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
      ? "City Profile & Economy"
      : tab === "inventory"
        ? "Inventory & Pending Claim"
        : "Activities & City Projects";

  const readyRuns =
    activities?.runs.filter((r) => r.status === "ready").length ?? 0;
  const hasFreeSlot =
    (activities?.runs.length ?? 0) < (activities?.slots_total ?? 2);

  return (
    <aside className="side-panel">
      <div className="panel-header">
        <h2>{title}</h2>
        <button type="button" className="btn-ghost" onClick={onClose}>
          Close
        </button>
      </div>

      <div className="panel-scroll">
        {tab === "city" && city && me.profile && (
          <>
            <div className="card">
              <h3>Player Profile</h3>
              <div className="kv-row">
                <span>Name</span>
                <strong>{me.username}</strong>
              </div>
              <div className="kv-row">
                <span>Level & XP</span>
                <span className="mono">
                  Lv {me.profile.level} ({me.profile.xp}/{me.profile.xp_needed})
                </span>
              </div>
              <div className="kv-row">
                <span>Personal Wallet</span>
                <strong className="mono">{me.profile.balance} Bac</strong>
              </div>
            </div>

            <div className="card">
              <h3>City Economy ({city.stage_name})</h3>
              <div className="kv-row">
                <span>City Treasury</span>
                <strong className="mono">{city.treasury} Bac</strong>
              </div>
              <div className="kv-row">
                <span>Tax Rate</span>
                <span className="mono">{city.tax_rate_pct}%</span>
              </div>
              <div className="kv-row">
                <span>Est. Tax / Upkeep</span>
                <span className="mono">
                  +{city.est_tax_income} / -{city.est_expense}
                </span>
              </div>
              <div className="kv-row">
                <span>Welfare & Effectiveness</span>
                <span className="mono">
                  {city.welfare}% / {city.effectiveness_pct}%
                </span>
              </div>
              <div className="kv-row">
                <span>City Yield Bonus</span>
                <span className="mono ok-text">+{city.yield_bonus_pct}%</span>
              </div>
              <div className="kv-row">
                <span>Assigned Crew</span>
                <span className="mono">{city.assigned_crews}</span>
              </div>
            </div>

            <div className="card">
              <h3>Set Tax Rate</h3>
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
              <h3>Manage City Treasury</h3>
              <div className="action-row">
                <input
                  type="number"
                  min={1}
                  step="1"
                  className="input-field"
                  style={{ flex: 1 }}
                  value={treasuryAmount}
                  onChange={(e) => setTreasuryAmount(e.target.value)}
                  placeholder="Bac amount"
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
                  Deposit to Treasury
                </button>
                <button
                  type="button"
                  className="btn-ghost btn-sm"
                  disabled={busy}
                  onClick={() =>
                    onWithdrawTreasury(treasuryAmount, newRequestId())
                  }
                >
                  Withdraw to Wallet
                </button>
              </div>
            </div>

            <div className="card">
              <h3>Built Facilities ({city.facilities.length})</h3>
              {city.facilities.length === 0 ? (
                <p className="muted">No facilities built yet.</p>
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
                <span>Slot Capacity</span>
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
                    Claim Pending
                  </button>
                )}
              </div>
              {inventory.pending.length === 0 ? (
                <p className="muted">No items waiting to be claimed.</p>
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
              <h3>Inventory Items</h3>
              {inventory.items.length === 0 ? (
                <p className="muted">No items in inventory yet.</p>
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
                        Sell 1
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
                        Sell All
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
                  Activity Slots ({activities.runs.length} /{" "}
                  {activities.slots_total})
                </h3>
                {readyRuns > 0 && (
                  <button
                    type="button"
                    className="btn-primary btn-sm"
                    disabled={busy}
                    onClick={onCollectActivities}
                  >
                    Collect All ({readyRuns})
                  </button>
                )}
              </div>
              {activities.runs.length === 0 ? (
                <p className="muted">All activity slots are currently idle.</p>
              ) : (
                activities.runs.map((r) => (
                  <div className="list-item" key={r.id}>
                    <span>{r.name}</span>
                    {r.status === "ready" ? (
                      <span className="pill pill-gold">READY</span>
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
              <h3>Start Activity</h3>
              {activities.catalog.map((cat) => (
                <div className="list-item" key={cat.kind}>
                  <div>
                    <strong>{cat.name}</strong>
                    <div className="muted">
                      {cat.unlocked
                        ? `${cat.base_duration_secs}s • +${cat.xp_reward} XP`
                        : `Requires ${cat.min_stage_name}`}
                    </div>
                  </div>
                  <button
                    type="button"
                    className="btn-success btn-sm"
                    disabled={!cat.unlocked || !hasFreeSlot || busy}
                    onClick={() => onStartActivity(cat.kind)}
                  >
                    Start
                  </button>
                </div>
              ))}
            </div>

            <div className="card">
              <h3>Activity Level Progress</h3>
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
              <h3>City Projects</h3>
              {(quotes ?? []).map((q) => (
                <div className="list-item" key={q.project}>
                  <div>
                    <strong>{q.name}</strong>
                    <div className="muted">
                      Lv {q.current_level}/{q.max_level} •{" "}
                      {q.next_level === null
                        ? "Maxed"
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
                        ? "Build"
                        : "Details"}
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
