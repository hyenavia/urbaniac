import type { ActivityKindKey, ProjectKey } from "../api/types";

export const WORLD_WIDTH = 1120;
export const WORLD_HEIGHT = 720;

export type MapSlotId = ActivityKindKey | ProjectKey | "market_npc";

export interface MapSlotSpec {
  id: MapSlotId;
  category: "activity" | "facility" | "market";
  label: string;
  shortCode: string;
  x: number;
  y: number;
  width: number;
  depth: number;
  wallHeight: number;
  roofHeight: number;
  roofColor: number;
  wallColor: number;
  accentColor: number;
  /** Disiapkan untuk Tahap 3+/aset final tanpa mengubah logika scene */
  spriteUrl?: string;
}

export const MAP_SLOTS: MapSlotSpec[] = [
  // Zona Aktivitas
  {
    id: "mining",
    category: "activity",
    label: "Tambang",
    shortCode: "MINE",
    x: 245,
    y: 155,
    width: 146,
    depth: 74,
    wallHeight: 48,
    roofHeight: 28,
    roofColor: 0x677586,
    wallColor: 0x465160,
    accentColor: 0x55bce8,
  },
  {
    id: "gathering",
    category: "activity",
    label: "Hutan",
    shortCode: "WOOD",
    x: 215,
    y: 355,
    width: 152,
    depth: 78,
    wallHeight: 38,
    roofHeight: 42,
    roofColor: 0x5d9d45,
    wallColor: 0x6f4e37,
    accentColor: 0x8cd06e,
  },
  {
    id: "farming",
    category: "activity",
    label: "Ladang",
    shortCode: "FARM",
    x: 245,
    y: 555,
    width: 164,
    depth: 82,
    wallHeight: 20,
    roofHeight: 16,
    roofColor: 0x8db64c,
    wallColor: 0x8b623f,
    accentColor: 0xffd166,
  },
  {
    id: "fishing",
    category: "activity",
    label: "Dermaga",
    shortCode: "FISH",
    x: 915,
    y: 410,
    width: 144,
    depth: 74,
    wallHeight: 26,
    roofHeight: 22,
    roofColor: 0x55bce8,
    wallColor: 0x8a6446,
    accentColor: 0x9be2ff,
  },

  // Pasar NPC
  {
    id: "market_npc",
    category: "market",
    label: "Pasar NPC",
    shortCode: "NPC",
    x: 755,
    y: 555,
    width: 150,
    depth: 76,
    wallHeight: 40,
    roofHeight: 28,
    roofColor: 0xff914d,
    wallColor: 0xa87955,
    accentColor: 0xffd166,
  },

  // 7 Slot Fasilitas Kota
  {
    id: "metropolis_tower",
    category: "facility",
    label: "Menara Metro",
    shortCode: "MTR",
    x: 560,
    y: 165,
    width: 126,
    depth: 70,
    wallHeight: 96,
    roofHeight: 38,
    roofColor: 0x55bce8,
    wallColor: 0x33455e,
    accentColor: 0xffd166,
  },
  {
    id: "city_hall",
    category: "facility",
    label: "Balai Kota",
    shortCode: "CITY",
    x: 735,
    y: 235,
    width: 138,
    depth: 72,
    wallHeight: 58,
    roofHeight: 32,
    roofColor: 0x627792,
    wallColor: 0x3b4a62,
    accentColor: 0xffd166,
  },
  {
    id: "town_hall",
    category: "facility",
    label: "Balai Kampung",
    shortCode: "HALL",
    x: 560,
    y: 345,
    width: 154,
    depth: 80,
    wallHeight: 54,
    roofHeight: 32,
    roofColor: 0xd98045,
    wallColor: 0x946b4b,
    accentColor: 0xffd166,
  },
  {
    id: "well",
    category: "facility",
    label: "Sumur",
    shortCode: "WELL",
    x: 410,
    y: 445,
    width: 108,
    depth: 58,
    wallHeight: 28,
    roofHeight: 24,
    roofColor: 0x7c8a99,
    wallColor: 0x596675,
    accentColor: 0x55bce8,
  },
  {
    id: "granary",
    category: "facility",
    label: "Lumbung",
    shortCode: "GRAN",
    x: 425,
    y: 255,
    width: 124,
    depth: 66,
    wallHeight: 48,
    roofHeight: 30,
    roofColor: 0xc26d42,
    wallColor: 0x9a6d48,
    accentColor: 0xffd166,
  },
  {
    id: "market_hall",
    category: "facility",
    label: "Pasar Desa",
    shortCode: "MKTH",
    x: 725,
    y: 390,
    width: 134,
    depth: 70,
    wallHeight: 44,
    roofHeight: 28,
    roofColor: 0xe6a844,
    wallColor: 0x8f6646,
    accentColor: 0x79b85c,
  },
  {
    id: "trade_port",
    category: "facility",
    label: "Pelabuhan",
    shortCode: "PORT",
    x: 925,
    y: 575,
    width: 148,
    depth: 76,
    wallHeight: 46,
    roofHeight: 26,
    roofColor: 0x3f8fb5,
    wallColor: 0x6f533c,
    accentColor: 0xffd166,
  },
];

export function findSlotSpec(id: MapSlotId): MapSlotSpec | undefined {
  return MAP_SLOTS.find((s) => s.id === id);
}
