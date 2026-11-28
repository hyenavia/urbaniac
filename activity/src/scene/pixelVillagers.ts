import type { Sprite, Texture } from "pixi.js";
import type { ActivityKindKey, CityStageKey } from "../api/types";
import { PIXEL_SCALE, PixelCanvas, PixelTextureCache, SDV } from "./pixelArt";

export type VillagerRole = "townsfolk" | ActivityKindKey;
export type WalkFrame = 0 | 1 | 2;

export type RoadNodeId =
  | "plaza_c"
  | "plaza_w"
  | "plaza_e"
  | "mining"
  | "gathering"
  | "farming"
  | "fishing"
  | "market_npc"
  | "well"
  | "granary"
  | "town_hall"
  | "market_hall"
  | "city_hall"
  | "trade_port"
  | "metropolis_tower";

interface RoadNode {
  x: number;
  y: number;
  neighbors: RoadNodeId[];
}

const ROAD_GRAPH: Record<RoadNodeId, RoadNode> = {
  plaza_c: {
    x: 560,
    y: 382,
    neighbors: ["plaza_w", "plaza_e", "town_hall", "well", "market_hall"],
  },
  plaza_w: {
    x: 512,
    y: 368,
    neighbors: ["plaza_c", "town_hall", "granary", "gathering", "farming", "well"],
  },
  plaza_e: {
    x: 608,
    y: 368,
    neighbors: ["plaza_c", "town_hall", "metropolis_tower", "city_hall", "market_hall"],
  },
  town_hall: {
    x: 560,
    y: 360,
    neighbors: ["plaza_c", "plaza_w", "plaza_e", "metropolis_tower", "granary"],
  },
  granary: {
    x: 435,
    y: 272,
    neighbors: ["plaza_w", "town_hall", "mining"],
  },
  mining: {
    x: 258,
    y: 174,
    neighbors: ["granary", "plaza_w"],
  },
  gathering: {
    x: 235,
    y: 372,
    neighbors: ["plaza_w", "farming"],
  },
  farming: {
    x: 262,
    y: 570,
    neighbors: ["plaza_w", "well", "gathering"],
  },
  well: {
    x: 418,
    y: 458,
    neighbors: ["plaza_c", "plaza_w", "farming"],
  },
  metropolis_tower: {
    x: 560,
    y: 184,
    neighbors: ["plaza_e", "town_hall", "city_hall"],
  },
  city_hall: {
    x: 730,
    y: 254,
    neighbors: ["plaza_e", "metropolis_tower", "market_hall"],
  },
  market_hall: {
    x: 722,
    y: 408,
    neighbors: ["plaza_c", "plaza_e", "city_hall", "market_npc", "fishing"],
  },
  market_npc: {
    x: 755,
    y: 572,
    neighbors: ["market_hall", "trade_port"],
  },
  fishing: {
    x: 872,
    y: 418,
    neighbors: ["market_hall"],
  },
  trade_port: {
    x: 882,
    y: 578,
    neighbors: ["market_npc"],
  },
};

const WORKER_PATROL_LINE: Record<
  ActivityKindKey,
  { x1: number; y1: number; x2: number; y2: number }
> = {
  mining: { x1: 236, y1: 174, x2: 274, y2: 176 },
  gathering: { x1: 196, y1: 374, x2: 240, y2: 374 },
  farming: { x1: 196, y1: 572, x2: 252, y2: 574 },
  fishing: { x1: 882, y1: 412, x2: 916, y2: 412 },
};

export const VILLAGER_COUNT_BY_STAGE: Record<CityStageKey, number> = {
  kampung: 4,
  desa: 6,
  kota_kecil: 8,
  kota_dagang: 10,
  metropolis: 12,
};

interface VillagerStyle {
  skin: string;
  hair: string;
  shirt: string;
  shirtShade: string;
  pants: string;
  hat?: string;
  hatBand?: string;
  lamp?: string;
}

const TOWNSFOLK_STYLES: VillagerStyle[] = [
  {
    skin: "#f4c79c",
    hair: "#5a321c",
    shirt: "#d95d47",
    shirtShade: "#9e3c2c",
    pants: "#3c5878",
  },
  {
    skin: "#e9b584",
    hair: "#2d1a0e",
    shirt: "#56964c",
    shirtShade: "#386930",
    pants: "#4d3524",
    hat: "#e2b568",
    hatBand: "#b53c2c",
  },
  {
    skin: "#f8d3ab",
    hair: "#c47435",
    shirt: "#5295b8",
    shirtShade: "#356582",
    pants: "#3a4559",
  },
  {
    skin: "#d89c6a",
    hair: "#241610",
    shirt: "#e6b84c",
    shirtShade: "#b38422",
    pants: "#4b382a",
  },
  {
    skin: "#f5c99f",
    hair: "#8c4a27",
    shirt: "#b66494",
    shirtShade: "#7f3e66",
    pants: "#403048",
    hat: "#f2dcb1",
    hatBand: "#5b993d",
  },
  {
    skin: "#e3ab79",
    hair: "#d9b97a",
    shirt: "#e88a45",
    shirtShade: "#ad5c22",
    pants: "#3e596e",
  },
];

const URBAN_TOWNSFOLK_STYLES: VillagerStyle[] = [
  {
    skin: "#f4c79c",
    hair: "#3b2010",
    shirt: "#2d4563",
    shirtShade: "#1c2d42",
    pants: "#232a36",
    hat: "#3a3f4a",
    hatBand: "#f5c242",
  },
  {
    skin: "#e9b584",
    hair: "#5a321c",
    shirt: "#8c333b",
    shirtShade: "#611f26",
    pants: "#2e3440",
    hat: "#4b382a",
    hatBand: "#d9b983",
  },
  {
    skin: "#f8d3ab",
    hair: "#8c4a27",
    shirt: "#386b5e",
    shirtShade: "#24473e",
    pants: "#29303b",
  },
];

const WORKER_STYLES: Record<ActivityKindKey, VillagerStyle> = {
  mining: {
    skin: "#efc094",
    hair: "#472916",
    shirt: "#677586",
    shirtShade: "#4a5666",
    pants: "#343e4c",
    hat: "#f5c242",
    hatBand: "#bd811a",
    lamp: "#ffe680",
  },
  gathering: {
    skin: "#ebba89",
    hair: "#5a321c",
    shirt: "#b53c2c",
    shirtShade: "#7d251c",
    pants: "#4d2b16",
    hat: "#7d4927",
    hatBand: "#4d2b16",
  },
  farming: {
    skin: "#f4c79c",
    hair: "#8c522d",
    shirt: "#4c8038",
    shirtShade: "#335825",
    pants: "#3b5e8c",
    hat: "#e6c373",
    hatBand: "#b53c2c",
  },
  fishing: {
    skin: "#e9b584",
    hair: "#3b2010",
    shirt: "#419ba8",
    shirtShade: "#296975",
    pants: "#364f69",
    hat: "#c88952",
    hatBand: "#296975",
  },
};

/** 12x16 native pixel Stardew-style villager sprite (3 walk frames) */
export function getVillagerTexture(
  cache: PixelTextureCache,
  role: VillagerRole,
  variant: number,
  frame: WalkFrame,
): Texture {
  const combinedTownStyles = [...TOWNSFOLK_STYLES, ...URBAN_TOWNSFOLK_STYLES];
  const safeVariant =
    role === "townsfolk" ? Math.abs(variant) % combinedTownStyles.length : 0;
  const key = `villager:${role}:${safeVariant}:${frame}`;

  return cache.get(key, () => {
    const c = new PixelCanvas(12, 16);
    const st =
      role === "townsfolk"
        ? combinedTownStyles[safeVariant]
        : WORKER_STYLES[role];

    const bobY = frame === 0 ? 0 : -1;

    // Ground shadow
    c.ellipse(6, 14, 4, 1, SDV.shadow);

    // Legs (idle, left step, right step)
    if (frame === 0) {
      c.rect(4, 11, 2, 3, st.pants);
      c.rect(6, 11, 2, 3, st.pants);
      c.px(4, 13, SDV.ink);
      c.px(7, 13, SDV.ink);
    } else if (frame === 1) {
      c.rect(3, 11, 2, 3, st.pants);
      c.rect(7, 11, 2, 2, st.pants);
      c.px(3, 13, SDV.ink);
      c.px(8, 12, SDV.ink);
    } else {
      c.rect(3, 11, 2, 2, st.pants);
      c.rect(7, 11, 2, 3, st.pants);
      c.px(3, 12, SDV.ink);
      c.px(8, 13, SDV.ink);
    }

    // Torso & swinging arms
    c.rect(3, 7 + bobY, 6, 4, SDV.ink);
    c.rect(4, 7 + bobY, 4, 4, st.shirt);
    c.rect(4, 10 + bobY, 4, 1, st.shirtShade);
    // Arms
    const armL = frame === 1 ? 8 : frame === 2 ? 6 : 7;
    const armR = frame === 1 ? 6 : frame === 2 ? 8 : 7;
    c.rect(2, armL + bobY, 2, 3, st.shirtShade);
    c.px(2, armL + 2 + bobY, st.skin);
    c.rect(8, armR + bobY, 2, 3, st.shirtShade);
    c.px(9, armR + 2 + bobY, st.skin);

    // Head outline + face
    c.rect(3, 2 + bobY, 6, 5, SDV.ink);
    c.rect(4, 3 + bobY, 4, 4, st.skin);
    // Eyes
    c.px(5, 5 + bobY, SDV.ink);
    c.px(7, 5 + bobY, SDV.ink);

    // Hair / Hat
    if (st.hat) {
      c.rect(2, 2 + bobY, 8, 1, SDV.ink);
      c.rect(3, 1 + bobY, 6, 2, st.hat);
      c.rect(3, 2 + bobY, 6, 1, st.hatBand ?? st.hat);
      c.rect(4, 0 + bobY, 4, 1, SDV.ink);
      if (st.lamp) {
        c.rect(5, 1 + bobY, 2, 1, st.lamp);
      }
    } else {
      c.rect(3, 1 + bobY, 6, 2, st.hair);
      c.px(3, 3 + bobY, st.hair);
      c.px(8, 3 + bobY, st.hair);
    }

    return c.toTexture(false);
  });
}

export interface VillagerAgent {
  id: string;
  role: VillagerRole;
  variant: number;
  x: number;
  y: number;
  facing: -1 | 1;
  fromNode: RoadNodeId;
  toNode: RoadNodeId;
  progress: number;
  speed: number;
  pauseSteps: number;
  stepAccum: number;
  patrolDir: -1 | 1;
  sprite: Sprite | null;
}

function randomPick<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

function pickNextNode(
  current: RoadNodeId,
  prev: RoadNodeId,
  activeNodes: Set<RoadNodeId>,
): RoadNodeId {
  const candidates = ROAD_GRAPH[current].neighbors.filter((n) =>
    activeNodes.has(n),
  );
  if (candidates.length === 0) return "plaza_c";
  const nonBacktrack = candidates.filter((n) => n !== prev);
  if (nonBacktrack.length > 0) return randomPick(nonBacktrack);
  return randomPick(candidates);
}

function createTownsfolk(
  index: number,
  activeNodes: Set<RoadNodeId>,
  stage: CityStageKey,
): VillagerAgent {
  const pool = Array.from(activeNodes);
  const start = pool.length > 0 ? pool[index % pool.length] : "plaza_c";
  const next = pickNextNode(start, start, activeNodes);
  const p1 = ROAD_GRAPH[start];
  const p2 = ROAD_GRAPH[next];
  const t = 0.2 + ((index * 0.37) % 0.6);
  const x = p1.x + (p2.x - p1.x) * t;
  const y = p1.y + (p2.y - p1.y) * t;
  const urbanStage = stage === "kota_dagang" || stage === "metropolis";
  const variant =
    urbanStage && index % 2 === 1
      ? TOWNSFOLK_STYLES.length + (index % URBAN_TOWNSFOLK_STYLES.length)
      : index % TOWNSFOLK_STYLES.length;
  return {
    id: `town_${index}`,
    role: "townsfolk",
    variant,
    x,
    y,
    facing: p2.x >= p1.x ? 1 : -1,
    fromNode: start,
    toNode: next,
    progress: t,
    speed: 2.0 + (index % 3) * 0.35, // world px per 100ms tick (~20-27 px/s)
    pauseSteps: (index * 3) % 10,
    stepAccum: index * 0.7,
    patrolDir: 1,
    sprite: null,
  };
}

function createActivityWorker(kind: ActivityKindKey): VillagerAgent {
  const line = WORKER_PATROL_LINE[kind];
  const x = (line.x1 + line.x2) / 2;
  const y = (line.y1 + line.y2) / 2;
  return {
    id: `worker_${kind}`,
    role: kind,
    variant: 0,
    x,
    y,
    facing: 1,
    fromNode: kind,
    toNode: kind,
    progress: 0.5,
    speed: 1.5,
    pauseSteps: 4,
    stepAccum: 0,
    patrolDir: 1,
    sprite: null,
  };
}

/**
 * Reconciles active townsfolk & profession workers with current city stage and
 * running activities while preserving existing agents' positions & paths.
 */
export function reconcileVillagers(
  existing: VillagerAgent[],
  stage: CityStageKey,
  activeNodes: Set<RoadNodeId>,
  runningKinds: Set<ActivityKindKey>,
): VillagerAgent[] {
  const targetTownCount = VILLAGER_COUNT_BY_STAGE[stage] ?? 4;
  const byId = new Map(existing.map((a) => [a.id, a]));
  const nextList: VillagerAgent[] = [];

  for (let i = 0; i < targetTownCount; i++) {
    const id = `town_${i}`;
    const prev = byId.get(id);
    if (prev) {
      prev.sprite = null;
      const urbanStage = stage === "kota_dagang" || stage === "metropolis";
      prev.variant =
        urbanStage && i % 2 === 1
          ? TOWNSFOLK_STYLES.length + (i % URBAN_TOWNSFOLK_STYLES.length)
          : i % TOWNSFOLK_STYLES.length;
      if (!activeNodes.has(prev.toNode)) {
        prev.toNode = pickNextNode(prev.fromNode, prev.fromNode, activeNodes);
        prev.progress = 0;
      }
      nextList.push(prev);
    } else {
      nextList.push(createTownsfolk(i, activeNodes, stage));
    }
  }

  for (const kind of runningKinds) {
    const id = `worker_${kind}`;
    const prev = byId.get(id);
    if (prev) {
      prev.sprite = null;
      nextList.push(prev);
    } else {
      nextList.push(createActivityWorker(kind));
    }
  }

  return nextList;
}

/** Advance all villagers one 100ms tick and update their Pixi sprites */
export function stepVillagers(
  agents: VillagerAgent[],
  activeNodes: Set<RoadNodeId>,
  cache: PixelTextureCache,
): void {
  for (const a of agents) {
    let walking = false;

    if (a.pauseSteps > 0) {
      a.pauseSteps -= 1;
    } else if (a.role !== "townsfolk") {
      // Activity worker patrols short work yard line
      const line = WORKER_PATROL_LINE[a.role];
      const dx = line.x2 - line.x1;
      const dy = line.y2 - line.y1;
      const dist = Math.max(1, Math.hypot(dx, dy));
      a.progress += (a.speed / dist) * a.patrolDir;

      if (a.progress >= 1) {
        a.progress = 1;
        a.patrolDir = -1;
        a.pauseSteps = 8 + Math.floor(Math.random() * 12);
      } else if (a.progress <= 0) {
        a.progress = 0;
        a.patrolDir = 1;
        a.pauseSteps = 8 + Math.floor(Math.random() * 12);
      } else {
        walking = true;
      }

      a.x = line.x1 + dx * a.progress;
      a.y = line.y1 + dy * a.progress;
      a.facing = a.patrolDir >= 0 ? 1 : -1;
    } else {
      // Townsfolk stroll along road graph
      const p1 = ROAD_GRAPH[a.fromNode];
      const p2 = ROAD_GRAPH[a.toNode];
      const dx = p2.x - p1.x;
      const dy = p2.y - p1.y;
      const dist = Math.max(1, Math.hypot(dx, dy));
      a.progress += a.speed / dist;

      if (Math.abs(dx) > 1) {
        a.facing = dx >= 0 ? 1 : -1;
      }

      if (a.progress >= 1) {
        const arrived = a.toNode;
        const prev = a.fromNode;
        a.fromNode = arrived;
        a.toNode = pickNextNode(arrived, prev, activeNodes);
        a.progress = 0;
        a.x = p2.x;
        a.y = p2.y;
        // Cozy pause at plaza or building fronts
        if (arrived.startsWith("plaza") || Math.random() < 0.55) {
          a.pauseSteps = 6 + Math.floor(Math.random() * 16);
        }
      } else {
        a.x = p1.x + dx * a.progress;
        a.y = p1.y + dy * a.progress;
        walking = true;
      }
    }

    if (a.sprite) {
      let frame: WalkFrame = 0;
      if (walking) {
        a.stepAccum += 0.45;
        frame = Math.floor(a.stepAccum) % 2 === 0 ? 1 : 2;
      }
      a.sprite.texture = getVillagerTexture(cache, a.role, a.variant, frame);
      a.sprite.position.set(Math.round(a.x), Math.round(a.y));
      a.sprite.scale.set(a.facing * PIXEL_SCALE, PIXEL_SCALE);
      a.sprite.zIndex = Math.round(a.y);
    }
  }
}
