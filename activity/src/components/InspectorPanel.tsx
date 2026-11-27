import { useEffect, useState } from "react";
import { newRequestId } from "../api/client";
import type { ActivityKindKey, CitySnapshot, ProjectKey } from "../api/types";
import { slotLabel, type UiLang } from "../i18n";
import { findSlotSpec, type MapSlotId } from "../scene/assetRegistry";

interface InspectorPanelProps {
  slotId: MapSlotId;
  snapshot: CitySnapshot;
  nowMs: number;
  busy: boolean;
  lang: UiLang;
  onClose: () => void;
  onStartActivity: (kind: ActivityKindKey) => void;
  onCollectActivities: () => void;
  onSellToNpc: (params: {
    itemId: string;
    quantity?: number;
    sellAll?: boolean;
    requestId: string;
  }) => void;
  onStartProject: (facility: ProjectKey, requestId: string) => void;
  onOpenMarket: () => void;
}

export function formatCountdown(
  endsAtIso: string,
  nowMs: number,
  offsetMs: number,
): string {
  const diffSec = Math.max(
    0,
    Math.ceil((Date.parse(endsAtIso) - (nowMs + offsetMs)) / 1000),
  );
  const mins = Math.floor(diffSec / 60);
  const secs = diffSec % 60;
  return `${mins}m ${String(secs).padStart(2, "0")}d`;
}

export function InspectorPanel({
  slotId,
  snapshot,
  nowMs,
  busy,
  lang,
  onClose,
  onStartActivity,
  onCollectActivities,
  onSellToNpc,
  onStartProject,
  onOpenMarket,
}: InspectorPanelProps) {
  const spec = findSlotSpec(slotId);
  const [confirmProjReqId, setConfirmProjReqId] = useState<string | null>(null);
  const [sellQtys, setSellQtys] = useState<Record<string, string>>({});

  useEffect(() => {
    setConfirmProjReqId(null);
  }, [slotId]);

  if (!spec) return null;

  const { city, quotes, activities, inventory, clockOffsetMs } = snapshot;

  return (
    <aside className="side-panel">
      <div className="panel-header">
        <h2>{slotLabel(spec.id, lang)}</h2>
        <button type="button" className="btn-ghost" onClick={onClose}>
          Tutup
        </button>
      </div>

      <div className="panel-scroll">
        {spec.category === "facility" &&
          (() => {
            const quote = quotes?.find((q) => q.project === spec.id);
            const running =
              city?.running_project?.project === spec.id
                ? city.running_project
                : null;
            const fac = city?.facilities.find((f) => f.id === spec.id);
            const lvl = fac?.level ?? 0;

            return (
              <>
                <div className="card">
                  <div className="kv-row">
                    <span className="muted">Level Saat Ini</span>
                    <strong className="mono">
                      {lvl} / {quote?.max_level ?? 1}
                    </strong>
                  </div>
                  {quote && <p className="muted">{quote.description}</p>}
                  {spec.id === "market_hall" && (
                    <button
                      type="button"
                      className="btn-ghost btn-sm"
                      onClick={onOpenMarket}
                    >
                      Buka Pasar & Makelar
                    </button>
                  )}
                </div>

                {running && (
                  <div className="card">
                    <h3>Sedang Dibangun</h3>
                    <div className="kv-row">
                      <span>Sisa Waktu</span>
                      <strong className="mono warn-text">
                        {formatCountdown(running.ends_at, nowMs, clockOffsetMs)}
                      </strong>
                    </div>
                  </div>
                )}

                {quote && quote.next_level !== null && (
                  <div className="card">
                    <h3>
                      {quote.current_level === 0
                        ? "Rincian Pembangunan"
                        : `Upgrade ke Lv ${quote.next_level}`}
                    </h3>
                    <div className="kv-row">
                      <span>Stage Minimum</span>
                      <span>{quote.min_stage_name}</span>
                    </div>
                    <div className="kv-row">
                      <span>Biaya Bac</span>
                      <span className="mono">
                        {quote.cost_bac} Bac (tersedia {quote.available_bac})
                      </span>
                    </div>
                    <div className="kv-row">
                      <span>Durasi</span>
                      <span className="mono">{quote.duration_secs} detik</span>
                    </div>
                    {quote.items.map((it) => (
                      <div className="list-item" key={it.item_id}>
                        <span>{it.name}</span>
                        <span
                          className={`mono ${
                            it.have >= it.need ? "ok-text" : "danger-text"
                          }`}
                        >
                          {it.have} / {it.need}
                        </span>
                      </div>
                    ))}
                    {quote.block_reason ? (
                      <p className="warn-text" style={{ fontSize: 12 }}>
                        Belum bisa dimulai: {quote.block_reason}
                      </p>
                    ) : (
                      <p className="ok-text" style={{ fontSize: 12 }}>
                        Semua syarat terpenuhi.
                      </p>
                    )}

                    {!running &&
                      (confirmProjReqId ? (
                        <div className="confirm-box">
                          <strong>Konfirmasi Biaya Proyek</strong>
                          <p className="muted">
                            Akan memotong <strong>{quote.cost_bac} Bac</strong>{" "}
                            (Treasury terlebih dahulu, sisa dari Wallet) beserta
                            resource yang tercantum di atas.
                          </p>
                          <div className="action-row">
                            <button
                              type="button"
                              className="btn-primary"
                              disabled={busy}
                              onClick={() => {
                                const rid = confirmProjReqId;
                                setConfirmProjReqId(null);
                                onStartProject(quote.project, rid);
                              }}
                            >
                              Konfirmasi & Mulai
                            </button>
                            <button
                              type="button"
                              className="btn-ghost"
                              disabled={busy}
                              onClick={() => setConfirmProjReqId(null)}
                            >
                              Batal
                            </button>
                          </div>
                        </div>
                      ) : (
                        <button
                          type="button"
                          className="btn-primary"
                          disabled={!quote.can_start || busy}
                          onClick={() => setConfirmProjReqId(newRequestId())}
                        >
                          {quote.current_level === 0
                            ? `Bangun ${quote.name}`
                            : `Upgrade ke Lv ${quote.next_level}`}
                        </button>
                      ))}
                  </div>
                )}
              </>
            );
          })()}

        {spec.category === "activity" &&
          (() => {
            const cat = activities?.catalog.find((c) => c.kind === spec.id);
            const prog = activities?.progress.find((p) => p.kind === spec.id);
            const runs =
              activities?.runs.filter((r) => r.kind === spec.id) ?? [];
            const readyCount = runs.filter((r) => r.status === "ready").length;
            const totalBusy = activities?.runs.length ?? 0;
            const totalSlots = activities?.slots_total ?? 2;
            const hasFreeSlot = totalBusy < totalSlots;

            return (
              <>
                <div className="card">
                  <div className="kv-row">
                    <span>Status Zona</span>
                    <span className={cat?.unlocked ? "ok-text" : "warn-text"}>
                      {cat?.unlocked
                        ? "Terbuka"
                        : `Terkunci (Butuh ${cat?.min_stage_name ?? "-"})`}
                    </span>
                  </div>
                  <div className="kv-row">
                    <span>Activity Slot</span>
                    <span className="mono">
                      {totalBusy} / {totalSlots} terpakai
                    </span>
                  </div>
                  {cat && (
                    <>
                      <div className="kv-row">
                        <span>Durasi Dasar</span>
                        <span className="mono">
                          {cat.base_duration_secs} detik
                        </span>
                      </div>
                      <div className="kv-row">
                        <span>Hadiah XP</span>
                        <span className="mono">+{cat.xp_reward} XP</span>
                      </div>
                    </>
                  )}
                  <div className="action-row" style={{ marginTop: 4 }}>
                    {cat && (
                      <button
                        type="button"
                        className="btn-success"
                        disabled={!cat.unlocked || !hasFreeSlot || busy}
                        onClick={() => onStartActivity(cat.kind)}
                      >
                        Mulai {cat.name}
                      </button>
                    )}
                    {readyCount > 0 && (
                      <button
                        type="button"
                        className="btn-primary"
                        disabled={busy}
                        onClick={onCollectActivities}
                      >
                        Ambil Reward ({readyCount})
                      </button>
                    )}
                  </div>
                </div>

                {prog && (
                  <div className="card">
                    <h3>Progres Aktivitas</h3>
                    <div className="kv-row">
                      <span>Level</span>
                      <strong className="mono">
                        Lv {prog.level} / {prog.cap}
                      </strong>
                    </div>
                    <div className="kv-row">
                      <span>XP</span>
                      <span className="mono">
                        {prog.xp} / {prog.xp_needed}
                      </span>
                    </div>
                    {prog.overflow > 0 && (
                      <div className="kv-row">
                        <span>XP Overflow</span>
                        <span className="mono warn-text">{prog.overflow}</span>
                      </div>
                    )}
                  </div>
                )}

                <div className="card">
                  <h3>Slot Berjalan ({runs.length})</h3>
                  {runs.length === 0 ? (
                    <p className="muted">
                      Belum ada aktivitas berjalan di zona ini.
                    </p>
                  ) : (
                    runs.map((r) => (
                      <div className="list-item" key={r.id}>
                        <span>{r.name}</span>
                        {r.status === "ready" ? (
                          <span className="pill pill-gold">SIAP DIKLAIM</span>
                        ) : (
                          <span className="mono warn-text">
                            {formatCountdown(r.ends_at, nowMs, clockOffsetMs)}
                          </span>
                        )}
                      </div>
                    ))
                  )}
                </div>
              </>
            );
          })()}

        {spec.category === "market" && (
          <div className="card">
            <div className="kv-row">
              <h3>Jual Langsung ke NPC</h3>
              <button
                type="button"
                className="btn-primary btn-sm"
                onClick={onOpenMarket}
              >
                Pasar & Makelar
              </button>
            </div>
            <p className="muted">
              Pilih jumlah item dari inventory untuk dijual pada harga NPC saat
              ini.
            </p>
            {!inventory || inventory.items.length === 0 ? (
              <p className="muted">Inventory masih kosong.</p>
            ) : (
              inventory.items.map((it) => {
                const rawQty = sellQtys[it.item_id] ?? "1";
                const parsedQty = Math.max(
                  1,
                  Math.min(it.quantity, Math.floor(Number(rawQty) || 1)),
                );
                return (
                  <div
                    className="list-item"
                    key={it.item_id}
                    style={{ flexDirection: "column", alignItems: "stretch" }}
                  >
                    <div className="kv-row">
                      <div>
                        <strong>{it.name}</strong>
                        <div className="muted">
                          {it.rarity} • Stok {it.quantity}
                        </div>
                      </div>
                      <strong className="mono">{it.npc_unit_price} Bac</strong>
                    </div>
                    <div className="action-row">
                      <input
                        type="number"
                        className="input-field"
                        style={{ width: 76 }}
                        min={1}
                        max={it.quantity}
                        value={rawQty}
                        onChange={(e) =>
                          setSellQtys((prev) => ({
                            ...prev,
                            [it.item_id]: e.target.value,
                          }))
                        }
                      />
                      <button
                        type="button"
                        className="btn-success btn-sm"
                        disabled={busy}
                        onClick={() =>
                          onSellToNpc({
                            itemId: it.item_id,
                            quantity: parsedQty,
                            requestId: newRequestId(),
                          })
                        }
                      >
                        Jual
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
                        Semua ({it.quantity})
                      </button>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        )}
      </div>
    </aside>
  );
}
