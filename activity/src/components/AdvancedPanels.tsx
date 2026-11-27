import { useEffect, useState } from "react";
import { newRequestId } from "../api/client";
import type { ActivityKindKey, CitySnapshot } from "../api/types";
import { formatCountdown } from "./InspectorPanel";

interface AdvancedPanelsProps {
  tab: "market" | "crafting" | "crew";
  snapshot: CitySnapshot;
  nowMs: number;
  busy: boolean;
  onClose: () => void;
  onCreateListing: (params: {
    itemId: string;
    quantity: number;
    targetUnitPrice: string;
    duration: "2h" | "6h" | "24h";
    requestId: string;
  }) => void;
  onCancelListing: (listingId: number) => void;
  onStartCraft: (recipeId: string, batchQty: number) => void;
  onToggleAutoSell: () => void;
  onFulfillOrder: (orderId: number) => void;
  onRefreshOrders: () => void;
  onRecruitCrew: (requestId: string) => void;
  onUpgradeCrew: (crewId: number, requestId: string) => void;
  onAssignCrew: (
    crewId: number,
    activity: ActivityKindKey | null,
    autoEnabled: boolean,
  ) => void;
}

export function AdvancedPanels({
  tab,
  snapshot,
  nowMs,
  busy,
  onClose,
  onCreateListing,
  onCancelListing,
  onStartCraft,
  onToggleAutoSell,
  onFulfillOrder,
  onRefreshOrders,
  onRecruitCrew,
  onUpgradeCrew,
  onAssignCrew,
}: AdvancedPanelsProps) {
  const { market, crafting, crew, inventory, activities, clockOffsetMs } = snapshot;

  const firstItem = inventory?.items[0];
  const [listItemId, setListItemId] = useState(firstItem?.item_id ?? "");
  const [listQty, setListQty] = useState("1");
  const [listPrice, setListPrice] = useState(firstItem?.npc_unit_price ?? "5.00");
  const [listDur, setListDur] = useState<"2h" | "6h" | "24h">("2h");
  const [craftBatches, setCraftBatches] = useState<Record<string, string>>({});

  useEffect(() => {
    const items = inventory?.items ?? [];
    if (items.length > 0 && !items.some((i) => i.item_id === listItemId)) {
      setListItemId(items[0].item_id);
      setListPrice(items[0].npc_unit_price);
    }
  }, [inventory, listItemId]);

  const title =
    tab === "market"
      ? "Pasar Dinamis & Makelar"
      : tab === "crafting"
        ? "Crafting & Pesanan NPC"
        : "Kru & Automasi";

  const selectedInvItem = inventory?.items.find((i) => i.item_id === listItemId);
  const parsedListQty = Math.max(1, Math.floor(Number(listQty) || 1));
  const parsedListPrice = Math.max(0.01, Number(listPrice) || 0);
  const estFee = Math.max(
    0.5,
    (parsedListQty * parsedListPrice * (market?.fee_pct ?? 1)) / 100,
  ).toFixed(2);

  return (
    <aside className="side-panel">
      <div className="panel-header">
        <h2>{title}</h2>
        <button type="button" className="btn-ghost" onClick={onClose}>
          Peta
        </button>
      </div>

      <div className="panel-scroll">
        {tab === "market" && market && (
          <>
            <div className="card">
              <h3>
                Kalender Dunia — Hari {market.calendar.day} ({market.calendar.season_name})
              </h3>
              <p className="muted">Bonus Musim: {market.calendar.season_hint}</p>
              {market.calendar.active_event_name && (
                <div className="kv-row">
                  <span className="pill pill-gold">
                    {market.calendar.active_event_name}
                  </span>
                  <span className="muted">{market.calendar.event_hint}</span>
                </div>
              )}
              {market.calendar.trend_item_name && (
                <div className="kv-row">
                  <span>Item Tren (+15%)</span>
                  <strong className="ok-text">
                    {market.calendar.trend_item_name}
                  </strong>
                </div>
              )}
            </div>

            <div className="card">
              <h3>
                Buat Listing Makelar ({market.listings.length} / {market.max_listings})
              </h3>
              <p className="muted">
                Fee {market.fee_pct}% di muka (min 0.50 Bac) • Komisi{" "}
                {market.commission_pct}% saat terjual.
              </p>
              {!inventory || inventory.items.length === 0 ? (
                <p className="muted">Inventory kosong; tidak ada item untuk dititipkan.</p>
              ) : (
                <>
                  <select
                    className="input-field"
                    value={listItemId}
                    onChange={(e) => {
                      const nextId = e.target.value;
                      setListItemId(nextId);
                      const found = inventory.items.find((x) => x.item_id === nextId);
                      if (found) setListPrice(found.npc_unit_price);
                    }}
                  >
                    {inventory.items.map((it) => (
                      <option key={it.item_id} value={it.item_id}>
                        {it.name} (Stok {it.quantity} • NPC {it.npc_unit_price})
                      </option>
                    ))}
                  </select>
                  <div className="action-row">
                    <input
                      type="number"
                      className="input-field"
                      style={{ width: 78 }}
                      min={1}
                      max={selectedInvItem?.quantity ?? 999}
                      value={listQty}
                      onChange={(e) => setListQty(e.target.value)}
                      placeholder="Qty"
                    />
                    <input
                      type="number"
                      step="0.1"
                      min="0.01"
                      className="input-field"
                      style={{ flex: 1 }}
                      value={listPrice}
                      onChange={(e) => setListPrice(e.target.value)}
                      placeholder="Harga target"
                    />
                    <select
                      className="input-field"
                      value={listDur}
                      onChange={(e) =>
                        setListDur(e.target.value as "2h" | "6h" | "24h")
                      }
                    >
                      <option value="2h">2 Jam</option>
                      <option value="6h">6 Jam</option>
                      <option value="24h">24 Jam</option>
                    </select>
                  </div>
                  <div className="kv-row">
                    <span className="muted">Estimasi Fee</span>
                    <span className="mono">{estFee} Bac</span>
                  </div>
                  <button
                    type="button"
                    className="btn-primary"
                    disabled={
                      busy ||
                      !selectedInvItem ||
                      market.listings.length >= market.max_listings
                    }
                    onClick={() =>
                      onCreateListing({
                        itemId: listItemId,
                        quantity: parsedListQty,
                        targetUnitPrice: parsedListPrice.toFixed(2),
                        duration: listDur,
                        requestId: newRequestId(),
                      })
                    }
                  >
                    Pasang Listing
                  </button>
                </>
              )}
            </div>

            <div className="card">
              <h3>Listing Aktif ({market.listings.length})</h3>
              {market.listings.length === 0 ? (
                <p className="muted">Belum ada listing aktif.</p>
              ) : (
                market.listings.map((l) => (
                  <div className="list-item" key={l.id}>
                    <div>
                      <strong>
                        {l.name} ×{l.quantity}
                      </strong>
                      <div className="muted">
                        Target {l.target_unit_price} Bac •{" "}
                        {formatCountdown(l.expires_at, nowMs, clockOffsetMs)}
                      </div>
                    </div>
                    <button
                      type="button"
                      className="btn-ghost btn-sm"
                      disabled={busy}
                      onClick={() => onCancelListing(l.id)}
                    >
                      Batalkan
                    </button>
                  </div>
                ))
              )}
            </div>

            <div className="card">
              <h3>Harga Pasar NPC Terkini</h3>
              {market.prices.map((p) => (
                <div className="list-item" key={p.item_id}>
                  <div>
                    <strong>{p.name}</strong>
                    <div className="muted">
                      Dasar {p.base_price} • ×{p.multiplier}
                    </div>
                  </div>
                  <strong className="mono">{p.current_price} Bac</strong>
                </div>
              ))}
            </div>
          </>
        )}

        {tab === "crafting" && crafting && (
          <>
            <div className="card">
              <div className="kv-row">
                <h3>Reputasi Toko (Lv {crafting.rep_level})</h3>
                <button
                  type="button"
                  disabled={busy}
                  className={`btn-sm ${
                    crafting.auto_sell ? "btn-primary" : "btn-ghost"
                  }`}
                  onClick={onToggleAutoSell}
                >
                  Auto-Sell: {crafting.auto_sell ? "ON" : "OFF"}
                </button>
              </div>
              <div className="kv-row">
                <span>XP Reputasi</span>
                <span className="mono">
                  {crafting.rep_xp} / {crafting.rep_xp_needed}
                </span>
              </div>
              <div className="kv-row">
                <span>Bonus Pesanan NPC</span>
                <span className="mono ok-text">+{crafting.rep_bonus_pct}%</span>
              </div>
              <div className="kv-row">
                <span>Pesanan Selesai</span>
                <span className="mono">{crafting.completed_orders}</span>
              </div>
            </div>

            <div className="card">
              <h3>
                Antrean Crafting ({crafting.running.length} / {crafting.max_concurrent})
              </h3>
              {crafting.running.length === 0 ? (
                <p className="muted">Tidak ada crafting berjalan.</p>
              ) : (
                crafting.running.map((r) => (
                  <div className="list-item" key={r.id}>
                    <div>
                      <strong>
                        {r.name} ×{r.quantity}
                      </strong>
                      {r.auto_sell && (
                        <div className="muted">Auto-sell aktif</div>
                      )}
                    </div>
                    <span className="mono warn-text">
                      {formatCountdown(r.ends_at, nowMs, clockOffsetMs)}
                    </span>
                  </div>
                ))
              )}
            </div>

            <div className="card">
              <div className="kv-row">
                <h3>Pesanan NPC</h3>
                <button
                  type="button"
                  className="btn-ghost btn-sm"
                  disabled={busy}
                  onClick={onRefreshOrders}
                >
                  Ganti Pesanan
                </button>
              </div>
              {crafting.orders.map((o) => (
                <div className="list-item" key={o.id}>
                  <div>
                    <strong>
                      {o.name} ×{o.quantity}
                    </strong>
                    <div className="muted">
                      Stok {o.have}/{o.quantity} • +{o.reward_bac} Bac • +{o.rep_xp} Rep
                    </div>
                  </div>
                  <button
                    type="button"
                    className="btn-primary btn-sm"
                    disabled={busy || !o.can_fulfill}
                    onClick={() => onFulfillOrder(o.id)}
                  >
                    Penuhi
                  </button>
                </div>
              ))}
            </div>

            <div className="card">
              <h3>Katalog Resep</h3>
              {crafting.recipes.map((r) => {
                const rawBatch = craftBatches[r.id] ?? "1";
                const batch = Math.max(
                  1,
                  Math.min(Math.max(1, r.max_batches), Math.floor(Number(rawBatch) || 1)),
                );
                return (
                  <div
                    className="list-item"
                    key={r.id}
                    style={{ flexDirection: "column", alignItems: "stretch" }}
                  >
                    <div className="kv-row">
                      <div>
                        <strong>{r.name}</strong>
                        <div className="muted">
                          {r.tier_name} • {r.duration_secs}d • +{r.xp_per_unit} XP
                        </div>
                      </div>
                      <span className="mono muted">Maks {r.max_batches}×</span>
                    </div>
                    <div className="muted">
                      Bahan:{" "}
                      {r.inputs
                        .map((inp) => `${inp.name} ${inp.have}/${inp.need}`)
                        .join(", ")}
                    </div>
                    <div className="action-row">
                      <input
                        type="number"
                        className="input-field"
                        style={{ width: 70 }}
                        min={1}
                        max={Math.max(1, r.max_batches)}
                        value={rawBatch}
                        onChange={(e) =>
                          setCraftBatches((prev) => ({
                            ...prev,
                            [r.id]: e.target.value,
                          }))
                        }
                      />
                      <button
                        type="button"
                        className="btn-success btn-sm"
                        disabled={
                          busy ||
                          r.max_batches <= 0 ||
                          crafting.running.length >= crafting.max_concurrent
                        }
                        onClick={() => onStartCraft(r.id, batch)}
                      >
                        Craft
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </>
        )}

        {tab === "crew" && crew && (
          <>
            <div className="card">
              <div className="kv-row">
                <h3>
                  Daftar Kru ({crew.members.length} / {crew.max_slots})
                </h3>
                <button
                  type="button"
                  className="btn-primary btn-sm"
                  disabled={busy || crew.members.length >= crew.max_slots}
                  onClick={() => onRecruitCrew(newRequestId())}
                >
                  Rekrut ({crew.recruit_cost} Bac)
                </button>
              </div>
              <p className="muted">
                Kru memberi bonus Efficiency, Yield, & Luck serta otomasi aktivitas
                tanpa memakai Activity Slot tambahan.
              </p>
            </div>

            {crew.members.length === 0 ? (
              <div className="card">
                <p className="muted">Belum ada kru. Rekrut kru pertamamu di atas.</p>
              </div>
            ) : (
              crew.members.map((m) => {
                const unlockedActs =
                  activities?.catalog.filter((c) => c.unlocked) ?? [];
                return (
                  <div className="card" key={m.id}>
                    <div className="kv-row">
                      <div>
                        <strong>
                          {m.name} (Lv {m.level}/{crew.max_level})
                        </strong>
                        <div className="muted">
                          {m.rarity} • Spesialis {m.specialty_name}
                        </div>
                      </div>
                      {m.upgrade_cost && (
                        <button
                          type="button"
                          className="btn-ghost btn-sm"
                          disabled={busy}
                          onClick={() => onUpgradeCrew(m.id, newRequestId())}
                        >
                          Upgrade ({m.upgrade_cost} Bac)
                        </button>
                      )}
                    </div>

                    <div className="muted mono">
                      Eff +{m.efficiency_pct}% • Yield +{m.yield_pct}% • Luck +
                      {m.luck_pct}%
                    </div>

                    <div className="action-row">
                      <select
                        className="input-field"
                        style={{ flex: 1 }}
                        disabled={busy}
                        value={m.assigned ?? ""}
                        onChange={(e) => {
                          const next = e.target.value
                            ? (e.target.value as ActivityKindKey)
                            : null;
                          onAssignCrew(m.id, next, next ? m.auto_enabled : false);
                        }}
                      >
                        <option value="">Tidak Ditugaskan</option>
                        {unlockedActs.map((a) => (
                          <option key={a.kind} value={a.kind}>
                            Tugas: {a.name}
                          </option>
                        ))}
                      </select>

                      <button
                        type="button"
                        disabled={busy || !m.assigned}
                        className={`btn-sm ${
                          m.auto_enabled ? "btn-primary" : "btn-ghost"
                        }`}
                        onClick={() =>
                          onAssignCrew(m.id, m.assigned, !m.auto_enabled)
                        }
                      >
                        Auto: {m.auto_enabled ? "ON" : "OFF"}
                      </button>
                    </div>
                  </div>
                );
              })
            )}
          </>
        )}
      </div>
    </aside>
  );
}
