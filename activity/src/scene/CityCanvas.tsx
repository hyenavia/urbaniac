import { useEffect, useRef, useState } from "react";
import {
  Application,
  Container,
  Graphics,
  Sprite,
  Text,
  TextStyle,
  TilingSprite,
} from "pixi.js";
import type {
  ActivityCatalogDto,
  ActivityKindKey,
  ActivityRunDto,
  CitySnapshot,
  CityStageKey,
  ProjectQuoteDto,
} from "../api/types";
import { slotLabel } from "../i18n";
import {
  MAP_SLOTS,
  WORLD_HEIGHT,
  WORLD_WIDTH,
  type MapSlotId,
  type MapSlotSpec,
} from "./assetRegistry";
import { PIXEL_SCALE, PixelTextureCache } from "./pixelArt";
import {
  getPadlockIconTexture,
  getSelectionRingTexture,
  getSlotSpriteAsset,
  getStatusIconTexture,
} from "./pixelBuildings";
import {
  CORE_TERRAIN_H,
  CORE_TERRAIN_OFFSET_X,
  CORE_TERRAIN_OFFSET_Y,
  RIVER_TILE_LEFT_WORLD_X,
  getCoreTerrainTexture,
  getGrassTileTexture,
  getPropTextures,
  getRiverBankTileTexture,
  getStageProps,
  getWaterTileTexture,
} from "./pixelTerrain";
import {
  type RoadNodeId,
  type VillagerAgent,
  getVillagerTexture,
  reconcileVillagers,
  stepVillagers,
} from "./pixelVillagers";

interface CityCanvasProps {
  snapshot: CitySnapshot;
  selectedSlot: MapSlotId | null;
  onSelectSlot: (slotId: MapSlotId) => void;
}

const STAGE_ORDER: CityStageKey[] = [
  "kampung",
  "desa",
  "kota_kecil",
  "kota_dagang",
  "metropolis",
];

function stageGte(current: CityStageKey, min: CityStageKey): boolean {
  return STAGE_ORDER.indexOf(current) >= STAGE_ORDER.indexOf(min);
}

type SlotVisualState =
  | { kind: "locked"; subLabel: string }
  | { kind: "unbuilt"; subLabel: string }
  | { kind: "building"; subLabel: string; endsAtMs: number; totalMs: number }
  | { kind: "running"; subLabel: string; endsAtMs: number; totalMs: number }
  | { kind: "ready"; subLabel: string }
  | { kind: "built"; level: number; subLabel: string };

function resolveSlotState(
  spec: MapSlotSpec,
  snapshot: CitySnapshot,
): SlotVisualState {
  const city = snapshot.city;
  const activities = snapshot.activities;
  const quotes = snapshot.quotes ?? [];

  if (spec.category === "market") {
    const count = snapshot.inventory?.items.length ?? 0;
    return {
      kind: "built",
      level: 1,
      subLabel: `${count} item${count === 1 ? "" : "s"}`,
    };
  }

  if (spec.category === "activity") {
    const cat: ActivityCatalogDto | undefined = activities?.catalog.find(
      (c) => c.kind === spec.id,
    );
    if (cat && !cat.unlocked) {
      return { kind: "locked", subLabel: cat.min_stage_name };
    }
    const matchingRuns: ActivityRunDto[] =
      activities?.runs.filter((r) => r.kind === spec.id) ?? [];
    const readyRun = matchingRuns.find((r) => r.status === "ready");
    if (readyRun) {
      return { kind: "ready", subLabel: "READY!" };
    }
    const runningRun = matchingRuns.find((r) => r.status === "running");
    if (runningRun) {
      const endsAtMs = Date.parse(runningRun.ends_at);
      const totalMs = Math.max(1000, (cat?.base_duration_secs ?? 300) * 1000);
      return { kind: "running", subLabel: "Active", endsAtMs, totalMs };
    }
    const prog = activities?.progress.find((p) => p.kind === spec.id);
    return {
      kind: "built",
      level: prog?.level ?? 1,
      subLabel: `Lv ${prog?.level ?? 1}`,
    };
  }

  // Facility slot
  const fac = city?.facilities.find((f) => f.id === spec.id);
  const quote: ProjectQuoteDto | undefined = quotes.find(
    (q) => q.project === spec.id,
  );

  if (city?.running_project?.project === spec.id) {
    const endsAtMs = Date.parse(city.running_project.ends_at);
    const totalMs = Math.max(1000, (quote?.duration_secs ?? 300) * 1000);
    return {
      kind: "building",
      subLabel: "Building",
      endsAtMs,
      totalMs,
    };
  }

  if (fac && fac.level > 0) {
    return {
      kind: "built",
      level: fac.level,
      subLabel: `Lv ${fac.level}`,
    };
  }

  if (city && quote && !stageGte(city.stage, quote.min_stage)) {
    return { kind: "locked", subLabel: quote.min_stage_name };
  }

  return { kind: "unbuilt", subLabel: "Empty" };
}

interface LiveBar {
  fill: Graphics;
  hi: Graphics;
  x: number;
  y: number;
  width: number;
  endsAtMs: number;
  totalMs: number;
  color: number;
  hiColor: number;
}

interface BobbingNode {
  node: Container;
  baseY: number;
  phase: number;
}

interface AmbientDrift {
  gfx: Graphics;
  originX: number;
  originY: number;
  phase: number;
  kind: "butterfly" | "smoke";
}

/** 16-bit Stardew parchment/wood plaque with crisp pixel bevel */
function drawPixelPlaque(
  x: number,
  y: number,
  w: number,
  h: number,
  colors: {
    fill: number;
    border: number;
    hi: number;
    lo: number;
  },
): Graphics {
  const g = new Graphics();
  // Outer dark-umber border
  g.rect(x, y, w, h).fill({ color: colors.border });
  // Inner fill
  g.rect(x + 2, y + 2, w - 4, h - 4).fill({ color: colors.fill });
  // Top & left highlight bevel
  g.rect(x + 2, y + 2, w - 4, 2).fill({ color: colors.hi });
  g.rect(x + 2, y + 2, 2, h - 4).fill({ color: colors.hi });
  // Bottom & right shade bevel
  g.rect(x + 2, y + h - 4, w - 4, 2).fill({ color: colors.lo });
  g.rect(x + w - 4, y + 2, 2, h - 4).fill({ color: colors.lo });
  // Pixel corner rivets
  g.rect(x + 4, y + 4, 2, 2).fill({ color: colors.lo });
  g.rect(x + w - 6, y + 4, 2, 2).fill({ color: colors.lo });
  return g;
}

function drawTerrain(
  world: Container,
  stage: CityStageKey,
  cache: PixelTextureCache,
): { riverBg: TilingSprite; ambients: AmbientDrift[] } {
  const bleedX = 1800;
  const bleedY = 1400;

  // 1. Full-bleed meadow grass TilingSprite
  const grassBg = new TilingSprite({
    texture: getGrassTileTexture(cache, stage),
    width: WORLD_WIDTH + bleedX * 2,
    height: WORLD_HEIGHT + bleedY * 2,
  });
  grassBg.position.set(-bleedX, -bleedY);
  grassBg.tileScale.set(PIXEL_SCALE, PIXEL_SCALE);
  grassBg.zIndex = -1000;
  grassBg.roundPixels = true;
  world.addChild(grassBg);

  // 2. Full-bleed eastern animated river TilingSprite
  const riverBg = new TilingSprite({
    texture: getWaterTileTexture(cache, 0),
    width: WORLD_WIDTH + bleedX - RIVER_TILE_LEFT_WORLD_X,
    height: WORLD_HEIGHT + bleedY * 2,
  });
  riverBg.position.set(RIVER_TILE_LEFT_WORLD_X, -bleedY);
  riverBg.tileScale.set(PIXEL_SCALE, PIXEL_SCALE);
  riverBg.zIndex = -900;
  riverBg.roundPixels = true;
  world.addChild(riverBg);

  // 3. Seamless river shoreline strips beyond core terrain vertical bounds (tall viewports)
  const coreTopY = CORE_TERRAIN_OFFSET_Y;
  const coreBotY = CORE_TERRAIN_OFFSET_Y + CORE_TERRAIN_H * PIXEL_SCALE;
  const bankTex = getRiverBankTileTexture(cache, stage);
  const bankX = 852 - 12 * PIXEL_SCALE;

  const northBank = new TilingSprite({
    texture: bankTex,
    width: 16 * PIXEL_SCALE,
    height: coreTopY - -bleedY,
  });
  northBank.position.set(bankX, -bleedY);
  northBank.tileScale.set(PIXEL_SCALE, PIXEL_SCALE);
  northBank.zIndex = -810;
  world.addChild(northBank);

  const southBank = new TilingSprite({
    texture: bankTex,
    width: 16 * PIXEL_SCALE,
    height: WORLD_HEIGHT + bleedY - coreBotY,
  });
  southBank.position.set(bankX, coreBotY);
  southBank.tileScale.set(PIXEL_SCALE, PIXEL_SCALE);
  southBank.zIndex = -810;
  world.addChild(southBank);

  // 4. Baked Stardew 3/4 core terrain (quarry cliff, shoreline, plaza, dirt/stone roads)
  const coreSprite = new Sprite(getCoreTerrainTexture(cache, stage));
  coreSprite.position.set(CORE_TERRAIN_OFFSET_X, CORE_TERRAIN_OFFSET_Y);
  coreSprite.scale.set(PIXEL_SCALE, PIXEL_SCALE);
  coreSprite.zIndex = -800;
  coreSprite.roundPixels = true;
  world.addChild(coreSprite);

  // 5. Y-sorted stage scenery props (trees, cottages, townhouses, skyline towers, harbor ships)
  const propTextures = getPropTextures(cache);
  for (const spot of getStageProps(stage)) {
    const prop = new Sprite(propTextures[spot.kind]);
    const isWaterProp = spot.kind === "sailboat" || spot.kind === "steamship";
    prop.anchor.set(0.5, isWaterProp ? 0.78 : 0.9);
    prop.scale.set(PIXEL_SCALE, PIXEL_SCALE);
    prop.position.set(spot.x, spot.y);
    prop.zIndex = isWaterProp ? -500 + spot.y : spot.y;
    prop.roundPixels = true;
    world.addChild(prop);
  }

  // 6. Lightweight ambient meadow butterflies & cozy chimney smoke puffs
  const ambients: AmbientDrift[] = [];
  const butterflySpots: Array<[number, number, number]> = [
    [335, 480, 0xffe680],
    [480, 290, 0x9be2ff],
    [640, 495, 0xf28aa8],
  ];
  butterflySpots.forEach(([bx, by, col], idx) => {
    const g = new Graphics();
    g.rect(-2, -1, 2, 2).fill({ color: col });
    g.rect(1, -1, 2, 2).fill({ color: col });
    g.rect(0, 0, 1, 2).fill({ color: 0x2d1a0e });
    g.position.set(bx, by);
    g.zIndex = 2000;
    world.addChild(g);
    ambients.push({
      gfx: g,
      originX: bx,
      originY: by,
      phase: idx * 1.9,
      kind: "butterfly",
    });
  });

  const smokeSpots: Array<[number, number]> = [
    [560 + 34, 345 - 98],
    [215 + 24, 355 - 86],
  ];
  smokeSpots.forEach(([sx, sy], idx) => {
    const g = new Graphics();
    g.rect(-3, -2, 6, 4).fill({ color: 0xf7f1e1, alpha: 0.72 });
    g.rect(-2, -4, 4, 2).fill({ color: 0xffffff, alpha: 0.55 });
    g.position.set(sx, sy);
    g.zIndex = 2000;
    world.addChild(g);
    ambients.push({
      gfx: g,
      originX: sx,
      originY: sy,
      phase: idx * 2.4,
      kind: "smoke",
    });
  });

  return { riverBg, ambients };
}

function drawSlotNode(
  spec: MapSlotSpec,
  state: SlotVisualState,
  selected: boolean,
  liveBars: LiveBar[],
  bobbing: BobbingNode[],
  cache: PixelTextureCache,
  stage: CityStageKey,
): Container {
  const root = new Container();
  root.x = spec.x;
  root.y = spec.y;
  root.zIndex = spec.y;
  root.eventMode = "static";
  root.cursor = "pointer";

  if (selected) {
    const ring = new Sprite(getSelectionRingTexture(cache, spec.width, spec.depth));
    ring.anchor.set(0.5, 0.5);
    ring.scale.set(PIXEL_SCALE, PIXEL_SCALE);
    ring.position.set(0, 4);
    root.addChild(ring);
  }

  const level = state.kind === "built" ? state.level : 1;
  const asset = getSlotSpriteAsset(cache, spec, state.kind, level, stage);
  const bSprite = new Sprite(asset.texture);
  bSprite.anchor.set(asset.anchorX, asset.anchorY);
  bSprite.scale.set(PIXEL_SCALE, PIXEL_SCALE);
  bSprite.roundPixels = true;
  if (state.kind === "locked") {
    bSprite.alpha = 0.86;
  }
  root.addChild(bSprite);

  // Cozy Stardew wooden/parchment nameplate plaque below building
  const titleText = slotLabel(spec.id);
  const topLabelY = 26;
  const plaqueW = Math.max(98, Math.round(titleText.length * 8.2 + 26));
  const plaque = drawPixelPlaque(
    -Math.round(plaqueW / 2),
    topLabelY,
    plaqueW,
    24,
    selected
      ? {
          fill: 0xffe494,
          border: 0x2d1a0e,
          hi: 0xfff8dc,
          lo: 0xd99b2e,
        }
      : state.kind === "locked"
        ? {
            fill: 0x52463f,
            border: 0x2d1a0e,
            hi: 0x706158,
            lo: 0x3b312c,
          }
        : {
            fill: 0xf5ddb0,
            border: 0x3b2010,
            hi: 0xfff4d9,
            lo: 0xc99f65,
          },
  );
  root.addChild(plaque);

  const labelText = new Text({
    text: titleText,
    style: new TextStyle({
      fontFamily: "system-ui, -apple-system, Segoe UI, sans-serif",
      fontSize: 12,
      fontWeight: "800",
      fill: state.kind === "locked" ? 0xd9cdb8 : 0x3b2010,
    }),
  });
  labelText.anchor.set(0.5, 0.5);
  labelText.position.set(0, topLabelY + 12);
  root.addChild(labelText);

  // Status badge / progress meter above building
  const badgeY = asset.badgeOffsetY;

  if (state.kind === "ready") {
    const bobWrap = new Container();
    bobWrap.position.set(0, badgeY);

    const badge = drawPixelPlaque(-42, -13, 84, 24, {
      fill: 0xf5c242,
      border: 0x2d1a0e,
      hi: 0xfff0a3,
      lo: 0xc2871e,
    });
    bobWrap.addChild(badge);

    const star = new Sprite(getStatusIconTexture(cache, "ready"));
    star.anchor.set(0.5, 0.5);
    star.scale.set(1.5, 1.5);
    star.position.set(-28, -1);
    bobWrap.addChild(star);

    const t = new Text({
      text: "READY!",
      style: new TextStyle({
        fontFamily: "system-ui, -apple-system, Segoe UI, sans-serif",
        fontSize: 11,
        fontWeight: "900",
        fill: 0x2d1a0e,
      }),
    });
    t.anchor.set(0.5, 0.5);
    t.position.set(9, -1);
    bobWrap.addChild(t);

    root.addChild(bobWrap);
    bobbing.push({ node: bobWrap, baseY: badgeY, phase: spec.x * 0.02 });
  } else if (state.kind === "running" || state.kind === "building") {
    const barW = 90;
    const barH = 14;
    const barX = -barW / 2;
    const barY = badgeY - 6;

    // Wooden-framed chunky pixel meter
    const frame = drawPixelPlaque(barX, barY, barW, barH, {
      fill: 0x422414,
      border: 0x2d1a0e,
      hi: 0x8c522d,
      lo: 0x2d1a0e,
    });
    root.addChild(frame);

    const fill = new Graphics();
    const hi = new Graphics();
    root.addChild(fill);
    root.addChild(hi);

    liveBars.push({
      fill,
      hi,
      x: barX + 3,
      y: barY + 3,
      width: barW - 6,
      endsAtMs: state.endsAtMs,
      totalMs: state.totalMs,
      color: state.kind === "building" ? 0xf5b83d : 0x6eb846,
      hiColor: state.kind === "building" ? 0xffe680 : 0xa4d66b,
    });

    // Bobbing activity tool micro-icon above progress bar
    const iconKind =
      state.kind === "building"
        ? "hammer"
        : spec.id === "mining"
          ? "pickaxe"
          : spec.id === "gathering"
            ? "axe"
            : spec.id === "farming"
              ? "sprout"
              : "fish";
    const toolWrap = new Container();
    const toolBaseY = barY - 12;
    toolWrap.position.set(0, toolBaseY);
    const toolIcon = new Sprite(getStatusIconTexture(cache, iconKind));
    toolIcon.anchor.set(0.5, 0.5);
    toolIcon.scale.set(PIXEL_SCALE, PIXEL_SCALE);
    toolWrap.addChild(toolIcon);
    root.addChild(toolWrap);
    bobbing.push({ node: toolWrap, baseY: toolBaseY, phase: spec.y * 0.03 });
  } else if (state.kind === "locked") {
    const txt = state.subLabel;
    const lw = Math.max(76, Math.round(txt.length * 6.8 + 26));
    const lockPlaque = drawPixelPlaque(-Math.round(lw / 2), badgeY - 10, lw, 20, {
      fill: 0x4b5666,
      border: 0x2d1a0e,
      hi: 0x677586,
      lo: 0x343e4c,
    });
    root.addChild(lockPlaque);

    const lockIcon = new Sprite(getPadlockIconTexture(cache));
    lockIcon.anchor.set(0, 0.5);
    lockIcon.scale.set(1.3, 1.3);
    lockIcon.position.set(-Math.round(lw / 2) + 6, badgeY);
    root.addChild(lockIcon);

    const t = new Text({
      text: txt,
      style: new TextStyle({
        fontFamily: "system-ui, -apple-system, Segoe UI, sans-serif",
        fontSize: 10.5,
        fontWeight: "700",
        fill: 0xf2dcb1,
      }),
    });
    t.anchor.set(0.5, 0.5);
    t.position.set(7, badgeY);
    root.addChild(t);
  } else if (state.kind === "built" && spec.category === "facility") {
    const lvPlaque = drawPixelPlaque(-25, badgeY - 10, 50, 20, {
      fill: 0x4c8038,
      border: 0x2d1a0e,
      hi: 0x85bd54,
      lo: 0x335825,
    });
    root.addChild(lvPlaque);

    const t = new Text({
      text: state.subLabel,
      style: new TextStyle({
        fontFamily: "system-ui, -apple-system, Segoe UI, sans-serif",
        fontSize: 11,
        fontWeight: "800",
        fill: 0xfff4d9,
      }),
    });
    t.anchor.set(0.5, 0.5);
    t.position.set(0, badgeY);
    root.addChild(t);
  }

  return root;
}

const PAN_MARGIN_X = 80;
const PAN_MARGIN_Y = 60;

function getFitScale(vw: number, vh: number): number {
  if (vw <= 0 || vh <= 0) return 0.85;
  return Math.max(
    0.35,
    Math.min(vw / (WORLD_WIDTH + 40), vh / (WORLD_HEIGHT + 40), 1.35),
  );
}

export function CityCanvas({
  snapshot,
  selectedSlot,
  onSelectSlot,
}: CityCanvasProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const appRef = useRef<Application | null>(null);
  const worldRef = useRef<Container | null>(null);
  const cacheRef = useRef<PixelTextureCache | null>(null);
  const liveBarsRef = useRef<LiveBar[]>([]);
  const bobbingRef = useRef<BobbingNode[]>([]);
  const ambientsRef = useRef<AmbientDrift[]>([]);
  const villagersRef = useRef<VillagerAgent[]>([]);
  const activeRoadNodesRef = useRef<Set<RoadNodeId>>(
    new Set(["plaza_c", "plaza_w", "plaza_e"]),
  );
  const riverBgRef = useRef<TilingSprite | null>(null);
  const animTickRef = useRef(0);
  const onSelectRef = useRef(onSelectSlot);
  onSelectRef.current = onSelectSlot;

  const [canPan, setCanPan] = useState(false);
  const lastFitScaleRef = useRef(1);
  const camRef = useRef({ x: 0, y: 0, scale: 1, initialized: false });
  const dragRef = useRef({
    active: false,
    startX: 0,
    startY: 0,
    originX: 0,
    originY: 0,
    moved: false,
  });

  const applyCamera = () => {
    const world = worldRef.current;
    if (!world) return;
    world.position.set(
      Math.round(camRef.current.x),
      Math.round(camRef.current.y),
    );
    world.scale.set(camRef.current.scale);
  };

  const clampCamera = () => {
    const el = hostRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const vw = rect.width;
    const vh = rect.height;
    if (vw <= 0 || vh <= 0) return;

    const fitScale = getFitScale(vw, vh);
    const maxScale = Math.max(1.8, fitScale * 2.2);
    const scale = Math.max(fitScale, Math.min(maxScale, camRef.current.scale));
    camRef.current.scale = scale;

    const scaledW = WORLD_WIDTH * scale;
    const scaledH = WORLD_HEIGHT * scale;

    if (scale <= fitScale + 0.005) {
      camRef.current.x = (vw - scaledW) / 2;
      camRef.current.y = (vh - scaledH) / 2;
      setCanPan((prev) => (prev ? false : prev));
      applyCamera();
      return;
    }

    const totalPanW = (WORLD_WIDTH + PAN_MARGIN_X * 2) * scale;
    const totalPanH = (WORLD_HEIGHT + PAN_MARGIN_Y * 2) * scale;

    if (totalPanW <= vw) {
      camRef.current.x = (vw - scaledW) / 2;
    } else {
      const minX = vw - (WORLD_WIDTH + PAN_MARGIN_X) * scale;
      const maxX = PAN_MARGIN_X * scale;
      camRef.current.x = Math.max(minX, Math.min(maxX, camRef.current.x));
    }

    if (totalPanH <= vh) {
      camRef.current.y = (vh - scaledH) / 2;
    } else {
      const minY = vh - (WORLD_HEIGHT + PAN_MARGIN_Y) * scale;
      const maxY = PAN_MARGIN_Y * scale;
      camRef.current.y = Math.max(minY, Math.min(maxY, camRef.current.y));
    }

    const pannable = totalPanW > vw + 1 || totalPanH > vh + 1;
    setCanPan((prev) => (prev === pannable ? prev : pannable));
    applyCamera();
  };

  const resetCamera = () => {
    const el = hostRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const fitScale = getFitScale(rect.width, rect.height);
    lastFitScaleRef.current = fitScale;
    camRef.current = {
      scale: fitScale,
      x: (rect.width - WORLD_WIDTH * fitScale) / 2,
      y: (rect.height - WORLD_HEIGHT * fitScale) / 2,
      initialized: true,
    };
    clampCamera();
  };

  const zoomBy = (factor: number, focusX?: number, focusY?: number) => {
    const el = hostRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const cx = focusX ?? rect.width / 2;
    const cy = focusY ?? rect.height / 2;
    const fitScale = getFitScale(rect.width, rect.height);
    const maxScale = Math.max(1.8, fitScale * 2.2);
    const oldScale = camRef.current.scale;
    const nextScale = Math.max(fitScale, Math.min(maxScale, oldScale * factor));
    const ratio = nextScale / oldScale;
    camRef.current.x = cx - (cx - camRef.current.x) * ratio;
    camRef.current.y = cy - (cy - camRef.current.y) * ratio;
    camRef.current.scale = nextScale;
    clampCamera();
  };

  const updateLiveBars = (clockOffsetMs: number) => {
    const serverNow = Date.now() + clockOffsetMs;
    for (const bar of liveBarsRef.current) {
      const rem = Math.max(0, bar.endsAtMs - serverNow);
      const pct = Math.max(
        0.08,
        Math.min(1, 1 - rem / Math.max(1000, bar.totalMs)),
      );
      const w = Math.max(4, Math.round(bar.width * pct));
      bar.fill.clear();
      bar.fill.rect(bar.x, bar.y, w, 8).fill({ color: bar.color });
      bar.hi.clear();
      bar.hi.rect(bar.x, bar.y, w, 2).fill({ color: bar.hiColor });
    }
  };

  const renderWorld = (snap: CitySnapshot, sel: MapSlotId | null) => {
    const world = worldRef.current;
    const cache = cacheRef.current;
    if (!world || !cache) return;

    const removed = world.removeChildren();
    for (const child of removed) {
      child.destroy({ children: true, texture: false });
    }
    liveBarsRef.current = [];
    bobbingRef.current = [];
    ambientsRef.current = [];
    riverBgRef.current = null;

    const stage = snap.city?.stage ?? "kampung";
    const { riverBg, ambients } = drawTerrain(world, stage, cache);
    riverBgRef.current = riverBg;
    ambientsRef.current = ambients;

    for (const spec of MAP_SLOTS) {
      const st = resolveSlotState(spec, snap);
      const node = drawSlotNode(
        spec,
        st,
        sel === spec.id,
        liveBarsRef.current,
        bobbingRef.current,
        cache,
        stage,
      );
      node.on("pointerup", () => {
        if (!dragRef.current.moved) {
          onSelectRef.current(spec.id);
        }
      });
      world.addChild(node);
    }

    // Reconcile walking townsfolk & running activity workers without resetting positions
    const activeNodes = new Set<RoadNodeId>([
      "plaza_c",
      "plaza_w",
      "plaza_e",
      "town_hall",
      "market_hall",
      "market_npc",
      "mining",
      "gathering",
      "farming",
    ]);
    const fishingCat = snap.activities?.catalog.find((c) => c.kind === "fishing");
    if (fishingCat?.unlocked) {
      activeNodes.add("fishing");
    }
    for (const fac of snap.city?.facilities ?? []) {
      if (fac.level > 0) {
        activeNodes.add(fac.id as RoadNodeId);
      }
    }
    activeRoadNodesRef.current = activeNodes;

    const runningKinds = new Set<ActivityKindKey>(
      (snap.activities?.runs ?? [])
        .filter((r) => r.status === "running")
        .map((r) => r.kind),
    );
    villagersRef.current = reconcileVillagers(
      villagersRef.current,
      stage,
      activeNodes,
      runningKinds,
    );
    for (const agent of villagersRef.current) {
      const vSprite = new Sprite(
        getVillagerTexture(cache, agent.role, agent.variant, 0),
      );
      vSprite.anchor.set(0.5, 0.92);
      vSprite.position.set(Math.round(agent.x), Math.round(agent.y));
      vSprite.scale.set(agent.facing * PIXEL_SCALE, PIXEL_SCALE);
      vSprite.zIndex = Math.round(agent.y);
      vSprite.roundPixels = true;
      vSprite.eventMode = "none";
      agent.sprite = vSprite;
      world.addChild(vSprite);
    }

    updateLiveBars(snap.clockOffsetMs);
  };

  // Keep latest snapshot accessible inside init & timer callbacks
  const latestRef = useRef({ snapshot, selectedSlot });
  latestRef.current = { snapshot, selectedSlot };

  useEffect(() => {
    const el = hostRef.current;
    if (!el) return;

    let disposed = false;
    const app = new Application();
    const cache = new PixelTextureCache();
    cacheRef.current = cache;

    (async () => {
      await app.init({
        resizeTo: el,
        background: 0x6ca646,
        antialias: false,
        roundPixels: true,
        resolution: Math.min(window.devicePixelRatio || 1, 2),
        autoDensity: true,
      });

      if (disposed) {
        app.destroy(true, { children: true });
        cache.destroyAll();
        return;
      }

      appRef.current = app;
      el.appendChild(app.canvas);

      const world = new Container();
      world.sortableChildren = true;
      worldRef.current = world;
      app.stage.addChild(world);

      if (!camRef.current.initialized) {
        resetCamera();
      } else {
        clampCamera();
      }
      renderWorld(latestRef.current.snapshot, latestRef.current.selectedSlot);
    })();

    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry || !camRef.current.initialized) return;
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0 && appRef.current) {
        appRef.current.resize();
      }
      const nextFit = getFitScale(width, height);
      if (Math.abs(camRef.current.scale - lastFitScaleRef.current) < 0.015) {
        camRef.current.scale = nextFit;
      }
      lastFitScaleRef.current = nextFit;
      clampCamera();
    });
    ro.observe(el);

    // Stepped 16-bit animation & progress ticker (every 100ms)
    const timer = window.setInterval(() => {
      animTickRef.current += 1;
      const tick = animTickRef.current;

      if (cacheRef.current && villagersRef.current.length > 0) {
        stepVillagers(
          villagersRef.current,
          activeRoadNodesRef.current,
          cacheRef.current,
        );
      }

      if (tick % 10 === 0) {
        updateLiveBars(latestRef.current.snapshot.clockOffsetMs);
      }

      // River 2-frame pixel shimmer every 700ms
      if (riverBgRef.current && cacheRef.current && tick % 7 === 0) {
        const frame = (Math.floor(tick / 7) % 2) as 0 | 1;
        riverBgRef.current.texture = getWaterTileTexture(cacheRef.current, frame);
      }

      if (tick % 2 === 0) {
        const step = Math.floor(tick / 2);
        // Stepped integer bobbing for READY! and active tool micro-icons
        const bobOffsets = [0, -2, -3, -2];
        for (const b of bobbingRef.current) {
          const idx = Math.floor(step + b.phase) % bobOffsets.length;
          b.node.y = b.baseY + bobOffsets[(idx + bobOffsets.length) % bobOffsets.length];
        }

        // Gentle ambient meadow butterflies & chimney smoke
        for (const amb of ambientsRef.current) {
          const t = step * 0.45 + amb.phase;
          if (amb.kind === "butterfly") {
            amb.gfx.position.set(
              Math.round(amb.originX + Math.sin(t * 0.7) * 14),
              Math.round(amb.originY + Math.cos(t * 1.1) * 7),
            );
          } else {
            const cycle = (step * 0.22 + amb.phase) % 6;
            amb.gfx.position.set(
              Math.round(amb.originX + Math.sin(cycle * 1.2) * 4),
              Math.round(amb.originY - cycle * 4),
            );
            amb.gfx.alpha = Math.max(0.15, 0.75 - cycle * 0.1);
          }
        }
      }
    }, 100);

    return () => {
      disposed = true;
      ro.disconnect();
      window.clearInterval(timer);
      if (appRef.current) {
        appRef.current.destroy(true, { children: true });
        appRef.current = null;
      }
      if (cacheRef.current) {
        cacheRef.current.destroyAll();
        cacheRef.current = null;
      }
      worldRef.current = null;
      riverBgRef.current = null;
      liveBarsRef.current = [];
      bobbingRef.current = [];
      ambientsRef.current = [];
      villagersRef.current = [];
    };
  }, []);

  useEffect(() => {
    renderWorld(snapshot, selectedSlot);
  }, [snapshot, selectedSlot]);

  return (
    <div
      className={`city-canvas-wrap${canPan ? " pannable" : ""}`}
      ref={hostRef}
      onPointerDown={(e) => {
        dragRef.current = {
          active: true,
          startX: e.clientX,
          startY: e.clientY,
          originX: camRef.current.x,
          originY: camRef.current.y,
          moved: false,
        };
      }}
      onPointerMove={(e) => {
        if (!dragRef.current.active) return;
        const dx = e.clientX - dragRef.current.startX;
        const dy = e.clientY - dragRef.current.startY;
        if (Math.hypot(dx, dy) > 6) {
          dragRef.current.moved = true;
        }
        if (dragRef.current.moved) {
          camRef.current.x = dragRef.current.originX + dx;
          camRef.current.y = dragRef.current.originY + dy;
          clampCamera();
        }
      }}
      onPointerUp={() => {
        dragRef.current.active = false;
      }}
      onPointerCancel={() => {
        dragRef.current.active = false;
      }}
      onWheel={(e) => {
        const rect = e.currentTarget.getBoundingClientRect();
        const factor = e.deltaY < 0 ? 1.1 : 0.9;
        zoomBy(factor, e.clientX - rect.left, e.clientY - rect.top);
      }}
    >
      <div className="camera-controls" onPointerDown={(e) => e.stopPropagation()}>
        <button type="button" onClick={() => zoomBy(1.15)} title="Zoom In">
          +
        </button>
        <button type="button" onClick={() => zoomBy(0.87)} title="Zoom Out">
          -
        </button>
        <button type="button" onClick={resetCamera} title="Reset Camera">
          Reset
        </button>
      </div>
    </div>
  );
}
