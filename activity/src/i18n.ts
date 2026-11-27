import type { MapSlotId } from "./scene/assetRegistry";

export type UiLang = "id" | "en";

const SLOT_LABELS: Record<MapSlotId, { id: string; en: string }> = {
  mining: { id: "Tambang", en: "Mine" },
  gathering: { id: "Hutan", en: "Forest" },
  farming: { id: "Ladang", en: "Farm" },
  fishing: { id: "Dermaga", en: "Pier" },
  market_npc: { id: "Pasar NPC", en: "NPC Market" },
  metropolis_tower: { id: "Menara Metro", en: "Metro Tower" },
  city_hall: { id: "Balai Kota", en: "Town Hall" },
  town_hall: { id: "Balai Kampung", en: "Village Hall" },
  well: { id: "Sumur", en: "Well" },
  granary: { id: "Lumbung", en: "Granary" },
  market_hall: { id: "Pasar Desa", en: "Market Hall" },
  trade_port: { id: "Pelabuhan", en: "Trade Port" },
};

export function slotLabel(id: MapSlotId, lang: UiLang): string {
  return SLOT_LABELS[id]?.[lang] ?? id;
}

export interface UiStrings {
  refresh: string;
  refreshing: string;
  liveOn: string;
  liveReconnecting: string;
  tabs: {
    map: string;
    city: string;
    inventory: string;
    activities: string;
    market: string;
    crafting: string;
    crew: string;
  };
}

const STRINGS: Record<UiLang, UiStrings> = {
  id: {
    refresh: "Segarkan",
    refreshing: "Memuat...",
    liveOn: "LIVE",
    liveReconnecting: "SYNC...",
    tabs: {
      map: "Peta",
      city: "Kota",
      inventory: "Tas",
      activities: "Aktivitas",
      market: "Pasar",
      crafting: "Craft",
      crew: "Kru",
    },
  },
  en: {
    refresh: "Refresh",
    refreshing: "Loading...",
    liveOn: "LIVE",
    liveReconnecting: "SYNC...",
    tabs: {
      map: "Map",
      city: "City",
      inventory: "Bag",
      activities: "Activities",
      market: "Market",
      crafting: "Craft",
      crew: "Crew",
    },
  },
};

export function tr(lang: UiLang): UiStrings {
  return STRINGS[lang];
}
