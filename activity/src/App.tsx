import { useCallback, useEffect, useRef, useState } from "react";
import {
  assignCrew,
  cancelMarketListing,
  claimPending,
  collectActivities,
  createMarketListing,
  depositTreasury,
  fetchSnapshot,
  fulfillCraftOrder,
  recruitCrew,
  refreshCraftOrders,
  registerPlayer,
  sellToNpc,
  setCityTaxRate,
  setPlayerLang,
  startActivity,
  startCityProject,
  startCraft,
  toggleCraftAutoSell,
  upgradeCrew,
  withdrawTreasury,
} from "./api/client";
import { initDiscordAuth, type AuthSession } from "./api/discordAuth";
import { resetMockPreset } from "./api/mockData";
import { connectEventStream, type StreamStatus } from "./api/sse";
import type {
  ActivityKindKey,
  CitySnapshot,
  MockPreset,
  ProjectKey,
} from "./api/types";
import { AdvancedPanels } from "./components/AdvancedPanels";
import { CityPanels, type NavTab } from "./components/CityPanels";
import { InspectorPanel } from "./components/InspectorPanel";
import { TopHud } from "./components/TopHud";
import { tr, type UiLang } from "./i18n";
import { CityCanvas } from "./scene/CityCanvas";
import type { MapSlotId } from "./scene/assetRegistry";

export function App() {
  const [session, setSession] = useState<AuthSession | null>(null);
  const [mockPreset, setMockPreset] = useState<MockPreset>("kampung");
  const [snapshot, setSnapshot] = useState<CitySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [streamStatus, setStreamStatus] = useState<StreamStatus>("offline");
  const [toast, setToast] = useState<{ kind: "ok" | "err"; text: string } | null>(
    null,
  );

  const [activeTab, setActiveTab] = useState<NavTab>("map");
  const [selectedSlot, setSelectedSlot] = useState<MapSlotId | null>(null);
  const [nowMs, setNowMs] = useState(() => Date.now());
  const lastAutoRefreshRef = useRef(0);

  const showToast = useCallback((kind: "ok" | "err", text: string) => {
    setToast({ kind, text });
  }, []);

  useEffect(() => {
    if (!toast) return;
    const id = window.setTimeout(() => setToast(null), 4500);
    return () => window.clearTimeout(id);
  }, [toast]);

  const loadData = useCallback(
    async (activeSession: AuthSession, preset: MockPreset) => {
      setRefreshing(true);
      try {
        const snap = await fetchSnapshot(activeSession, preset);
        setSnapshot(snap);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Gagal memuat data kota");
      } finally {
        setRefreshing(false);
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const authSession = await initDiscordAuth();
        if (cancelled) return;
        setSession(authSession);
        await loadData(authSession, mockPreset);
      } catch (err) {
        if (cancelled) return;
        setError(
          err instanceof Error ? err.message : "Gagal menghubungkan ke Discord",
        );
        setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const id = window.setInterval(() => setNowMs(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, []);

  // Koneksi SSE real-time untuk menerima selesai aktivitas/proyek/crafting/market tick
  const registered = Boolean(snapshot?.me.registered);
  useEffect(() => {
    if (!session || !registered) return;
    return connectEventStream(
      session,
      (ev) => {
        if (ev.kind !== "connected") {
          void loadData(session, mockPreset);
        }
        if (ev.message) {
          showToast("ok", ev.message);
        }
      },
      setStreamStatus,
    );
  }, [session, registered, mockPreset, loadData, showToast]);

  // Refresh otomatis saat timer lokal aktivitas, proyek, atau crafting mencapai 0
  useEffect(() => {
    if (!session || !snapshot || !snapshot.me.registered || refreshing) return;
    const serverNow = nowMs + snapshot.clockOffsetMs;
    const dueActivity = snapshot.activities?.runs.some(
      (r) => r.status === "running" && Date.parse(r.ends_at) <= serverNow + 400,
    );
    const dueProject =
      snapshot.city?.running_project &&
      Date.parse(snapshot.city.running_project.ends_at) <= serverNow + 400;
    const dueCraft = snapshot.crafting?.running.some(
      (c) => Date.parse(c.ends_at) <= serverNow + 400,
    );

    if (
      (dueActivity || dueProject || dueCraft) &&
      nowMs - lastAutoRefreshRef.current >= 2500
    ) {
      lastAutoRefreshRef.current = nowMs;
      void loadData(session, mockPreset);
    }
  }, [nowMs, session, snapshot, mockPreset, refreshing, loadData]);

  const runAction = useCallback(
    async <T,>(
      fn: (activeSession: AuthSession) => Promise<T>,
      formatOk: (res: T) => string,
    ) => {
      if (!session || busy) return;
      setBusy(true);
      try {
        const res = await fn(session);
        await loadData(session, mockPreset);
        showToast("ok", formatOk(res));
      } catch (err) {
        showToast(
          "err",
          err instanceof Error ? err.message : "Aksi gagal dijalankan",
        );
      } finally {
        setBusy(false);
      }
    },
    [session, busy, mockPreset, loadData, showToast],
  );

  const handlePresetChange = (next: MockPreset) => {
    setMockPreset(next);
    setSelectedSlot(null);
    setActiveTab("map");
    if (session) {
      void loadData(session, next);
    }
  };

  if (loading) {
    return (
      <div className="center-screen">
        <div className="center-card">
          <h1>Urbaniac</h1>
          <p className="muted">Menghubungkan kota visual ke Discord...</p>
        </div>
      </div>
    );
  }

  if (error && !snapshot) {
    return (
      <div className="center-screen">
        <div className="center-card">
          <h1>Koneksi Gagal</h1>
          <p className="danger-text" style={{ fontSize: 13 }}>
            {error}
          </p>
          <button
            type="button"
            className="btn-ghost"
            onClick={() => window.location.reload()}
          >
            Coba Lagi
          </button>
        </div>
      </div>
    );
  }

  if (!snapshot) return null;

  if (!snapshot.me.registered) {
    return (
      <div className="center-screen">
        <div className="center-card">
          <h1>Selamat Datang, {snapshot.me.username}!</h1>
          <p className="muted">
            Kotamu belum terdaftar. Buat kota Kampung pertamamu sekarang dan
            dapatkan modal awal <strong>100.00 Bac</strong>.
          </p>
          <div
            style={{
              display: "flex",
              gap: 8,
              justifyContent: "center",
              flexWrap: "wrap",
            }}
          >
            <button
              type="button"
              className="btn-primary"
              disabled={busy}
              onClick={() =>
                void runAction(
                  (s) => registerPlayer(s, mockPreset),
                  () => "Kota Kampung berhasil dibuat! Modal awal 100.00 Bac.",
                )
              }
            >
              {busy ? "Mendaftar..." : "Mulai Kota Baru (100 Bac)"}
            </button>
            <button
              type="button"
              className="btn-ghost"
              disabled={refreshing}
              onClick={() => session && void loadData(session, mockPreset)}
            >
              Segarkan
            </button>
            {session?.mode === "mock" && (
              <button
                type="button"
                className="btn-ghost"
                onClick={() => handlePresetChange("kampung")}
              >
                Lihat Mock Kampung
              </button>
            )}
          </div>
        </div>
      </div>
    );
  }

  const lang: UiLang = snapshot.me.profile?.lang ?? "id";
  const labels = tr(lang);
  const readyCount =
    snapshot.activities?.runs.filter((r) => r.status === "ready").length ?? 0;
  const pendingCount = snapshot.inventory?.pending.length ?? 0;

  return (
    <div className="app-shell">
      <TopHud
        snapshot={snapshot}
        refreshing={refreshing}
        streamStatus={streamStatus}
        lang={lang}
        busy={busy}
        onRefresh={() => session && void loadData(session, mockPreset)}
        onToggleLang={() =>
          void runAction(
            (s) => setPlayerLang(s, mockPreset),
            (r) => `Bahasa diubah ke ${r.label}.`,
          )
        }
      />

      <main className="workspace">
        <section className="map-stage">
          {session?.mode === "mock" && (
            <div className="mock-toolbar">
              <span className="pill pill-gold">MOCK DEV</span>
              <label htmlFor="mock-preset">Preset:</label>
              <select
                id="mock-preset"
                value={mockPreset}
                onChange={(e) =>
                  handlePresetChange(e.target.value as MockPreset)
                }
              >
                <option value="kampung">Kampung (Awal)</option>
                <option value="desa">Desa (Fishing)</option>
                <option value="metropolis">Metropolis (Lengkap)</option>
                <option value="unregistered">Belum /start</option>
              </select>
              <button
                type="button"
                className="btn-ghost btn-sm"
                onClick={() => {
                  resetMockPreset(mockPreset);
                  void loadData(session, mockPreset);
                  showToast("ok", "Data mock direset ke awal.");
                }}
              >
                Reset
              </button>
            </div>
          )}

          <CityCanvas
            snapshot={snapshot}
            selectedSlot={selectedSlot}
            onSelectSlot={(slot) => {
              setSelectedSlot(slot);
              setActiveTab("map");
            }}
          />

          {toast && (
            <div className={`toast-banner ${toast.kind}`}>
              <span>{toast.text}</span>
              <button
                type="button"
                className="btn-ghost btn-sm"
                onClick={() => setToast(null)}
              >
                ×
              </button>
            </div>
          )}
        </section>

        {activeTab === "city" ||
        activeTab === "inventory" ||
        activeTab === "activities" ? (
          <CityPanels
            tab={activeTab}
            snapshot={snapshot}
            nowMs={nowMs}
            busy={busy}
            onClose={() => setActiveTab("map")}
            onDepositTreasury={(amount, rid) =>
              void runAction(
                (s) => depositTreasury(s, mockPreset, amount, rid),
                (r) =>
                  `Setor berhasil. Treasury: ${r.treasury} Bac • Wallet: ${r.wallet_balance} Bac`,
              )
            }
            onWithdrawTreasury={(amount, rid) =>
              void runAction(
                (s) => withdrawTreasury(s, mockPreset, amount, rid),
                (r) =>
                  `Tarik berhasil. Treasury: ${r.treasury} Bac • Wallet: ${r.wallet_balance} Bac`,
              )
            }
            onSetTaxRate={(pct) =>
              void runAction(
                (s) => setCityTaxRate(s, mockPreset, pct),
                (r) => `Tarif pajak kota diubah menjadi ${r.tax_rate_pct}%.`,
              )
            }
            onClaimPending={() =>
              void runAction(
                (s) => claimPending(s, mockPreset),
                (r) => {
                  const total = r.claimed.reduce((acc, x) => acc + x.claimed, 0);
                  return total > 0
                    ? `Berhasil mengklaim ${total} item dari Pending Claim.`
                    : "Slot inventory masih penuh.";
                },
              )
            }
            onSellToNpc={(params) =>
              void runAction(
                (s) => sellToNpc(s, mockPreset, params),
                (r) =>
                  r.duplicate
                    ? "Permintaan jual sudah diproses sebelumnya."
                    : `Terjual ${r.qty}× ${r.name} (+${r.total_bac} Bac).`,
              )
            }
            onStartActivity={(kind: ActivityKindKey) =>
              void runAction(
                (s) => startActivity(s, mockPreset, kind),
                (r) => `Aktivitas ${r.name} dimulai!`,
              )
            }
            onCollectActivities={() =>
              void runAction(
                (s) => collectActivities(s, mockPreset),
                (r) => {
                  if (r.collected.length === 0) {
                    return "Belum ada reward aktivitas yang siap diambil.";
                  }
                  const summary = r.collected
                    .map(
                      (c) =>
                        `${c.name} (+${c.xp_gain} XP): ` +
                        c.items
                          .map((i) => `${i.stored + i.pending}× ${i.name}`)
                          .join(", "),
                    )
                    .join(" | ");
                  return `Reward diambil: ${summary}`;
                },
              )
            }
            onInspectSlot={(slotId) => {
              setSelectedSlot(slotId);
              setActiveTab("map");
            }}
          />
        ) : activeTab === "market" ||
          activeTab === "crafting" ||
          activeTab === "crew" ? (
          <AdvancedPanels
            tab={activeTab}
            snapshot={snapshot}
            nowMs={nowMs}
            busy={busy}
            onClose={() => setActiveTab("map")}
            onCreateListing={(params) =>
              void runAction(
                (s) => createMarketListing(s, mockPreset, params),
                (r) =>
                  `Listing ${r.name} ×${r.quantity} dipasang (fee ${r.listing_fee} Bac).`,
              )
            }
            onCancelListing={(listingId) =>
              void runAction(
                (s) => cancelMarketListing(s, mockPreset, listingId),
                (r) => `Listing ${r.name} dibatalkan (kembali ${r.stored + r.pending}).`,
              )
            }
            onStartCraft={(recipeId, batchQty) =>
              void runAction(
                (s) => startCraft(s, mockPreset, recipeId, batchQty),
                (r) => `Crafting ${r.name} ×${r.quantity} dimulai!`,
              )
            }
            onToggleAutoSell={() =>
              void runAction(
                (s) => toggleCraftAutoSell(s, mockPreset),
                (r) => `Auto-Sell Craft: ${r.auto_sell ? "ON" : "OFF"}.`,
              )
            }
            onFulfillOrder={(orderId) =>
              void runAction(
                (s) => fulfillCraftOrder(s, mockPreset, orderId),
                (r) =>
                  `Pesanan ${r.name} ×${r.quantity} selesai (+${r.reward_bac} Bac, +${r.rep_xp_gain} Rep).`,
              )
            }
            onRefreshOrders={() =>
              void runAction(
                (s) => refreshCraftOrders(s, mockPreset),
                () => "Daftar pesanan NPC diperbarui.",
              )
            }
            onRecruitCrew={(rid) =>
              void runAction(
                (s) => recruitCrew(s, mockPreset, rid),
                (r) => `Berhasil merekrut ${r.name} (${r.rarity})!`,
              )
            }
            onUpgradeCrew={(crewId, rid) =>
              void runAction(
                (s) => upgradeCrew(s, mockPreset, crewId, rid),
                (r) => `${r.name} naik ke Lv ${r.level}!`,
              )
            }
            onAssignCrew={(crewId, activity, autoEnabled) =>
              void runAction(
                (s) => assignCrew(s, mockPreset, crewId, activity, autoEnabled),
                (r) =>
                  r.assigned_name
                    ? `${r.name} ditugaskan ke ${r.assigned_name} (Auto: ${r.auto_enabled ? "ON" : "OFF"}).`
                    : `${r.name} dilepas dari tugas.`,
              )
            }
          />
        ) : (
          selectedSlot && (
            <InspectorPanel
              slotId={selectedSlot}
              snapshot={snapshot}
              nowMs={nowMs}
              busy={busy}
              lang={lang}
              onClose={() => setSelectedSlot(null)}
              onStartActivity={(kind: ActivityKindKey) =>
                void runAction(
                  (s) => startActivity(s, mockPreset, kind),
                  (r) => `Aktivitas ${r.name} dimulai!`,
                )
              }
              onCollectActivities={() =>
                void runAction(
                  (s) => collectActivities(s, mockPreset),
                  (r) =>
                    r.collected.length === 0
                      ? "Belum ada reward yang siap diambil."
                      : `Berhasil mengambil ${r.collected.length} reward aktivitas.`,
                )
              }
              onSellToNpc={(params) =>
                void runAction(
                  (s) => sellToNpc(s, mockPreset, params),
                  (r) =>
                    r.duplicate
                      ? "Permintaan jual sudah diproses sebelumnya."
                      : `Terjual ${r.qty}× ${r.name} (+${r.total_bac} Bac).`,
                )
              }
              onStartProject={(facility: ProjectKey, rid: string) =>
                void runAction(
                  (s) => startCityProject(s, mockPreset, facility, rid),
                  (r) => `Proyek ${r.name} dimulai!`,
                )
              }
              onOpenMarket={() => {
                setSelectedSlot(null);
                setActiveTab("market");
              }}
            />
          )
        )}
      </main>

      <nav className="bottom-nav">
        <button
          type="button"
          className={activeTab === "map" ? "active" : ""}
          onClick={() => setActiveTab("map")}
        >
          {labels.tabs.map}
        </button>
        <button
          type="button"
          className={activeTab === "city" ? "active" : ""}
          onClick={() => {
            setSelectedSlot(null);
            setActiveTab("city");
          }}
        >
          {labels.tabs.city}
        </button>
        <button
          type="button"
          className={activeTab === "inventory" ? "active" : ""}
          onClick={() => {
            setSelectedSlot(null);
            setActiveTab("inventory");
          }}
        >
          {labels.tabs.inventory}
          {pendingCount > 0 && <span className="nav-badge">{pendingCount}</span>}
        </button>
        <button
          type="button"
          className={activeTab === "activities" ? "active" : ""}
          onClick={() => {
            setSelectedSlot(null);
            setActiveTab("activities");
          }}
        >
          {labels.tabs.activities}
          {readyCount > 0 && <span className="nav-badge">{readyCount}</span>}
        </button>
        <button
          type="button"
          className={activeTab === "market" ? "active" : ""}
          onClick={() => {
            setSelectedSlot(null);
            setActiveTab("market");
          }}
        >
          {labels.tabs.market}
        </button>
        <button
          type="button"
          className={activeTab === "crafting" ? "active" : ""}
          onClick={() => {
            setSelectedSlot(null);
            setActiveTab("crafting");
          }}
        >
          {labels.tabs.crafting}
        </button>
        <button
          type="button"
          className={activeTab === "crew" ? "active" : ""}
          onClick={() => {
            setSelectedSlot(null);
            setActiveTab("crew");
          }}
        >
          {labels.tabs.crew}
        </button>
      </nav>
    </div>
  );
}
