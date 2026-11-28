import type { MapSlotId } from "./scene/assetRegistry";

const SLOT_LABELS: Record<MapSlotId, string> = {
  mining: "Mine",
  gathering: "Forest",
  farming: "Farm",
  fishing: "Pier",
  market_npc: "NPC Market",
  metropolis_tower: "Metro Tower",
  city_hall: "Town Hall",
  town_hall: "Village Hall",
  well: "Well",
  granary: "Granary",
  market_hall: "Market Hall",
  trade_port: "Trade Port",
};

export function slotLabel(id: MapSlotId): string {
  return SLOT_LABELS[id] ?? id;
}

export interface UiStrings {
  refresh: string;
  refreshing: string;
  liveOn: string;
  liveReconnecting: string;
  tabs: {
    city: string;
    inventory: string;
    activities: string;
    market: string;
    crafting: string;
    crew: string;
  };
}

export const UI_STRINGS: UiStrings = {
  refresh: "Refresh",
  refreshing: "Loading...",
  liveOn: "LIVE",
  liveReconnecting: "SYNC...",
  tabs: {
    city: "City",
    inventory: "Bag",
    activities: "Activities",
    market: "Market",
    crafting: "Craft",
    crew: "Crew",
  },
};

export function tr(): UiStrings {
  return UI_STRINGS;
}
