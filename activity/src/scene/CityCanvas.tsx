import { useEffect, useRef, useState } from "react";
import { Application, Container, Graphics, Text, TextStyle } from "pixi.js";
import type {
  ActivityCatalogDto,
  ActivityRunDto,
  CitySnapshot,
  CityStageKey,
  ProjectQuoteDto,
} from "../api/types";
import { slotLabel, type UiLang } from "../i18n";
import {
  MAP_SLOTS,
  WORLD_HEIGHT,
  WORLD_WIDTH,
  type MapSlotId,
  type MapSlotSpec,
} from "./assetRegistry";

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
    return { kind: "built", level: 1, subLabel: `${count} item` };
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
      return { kind: "ready", subLabel: "SIAP!" };
    }
    const runningRun = matchingRuns.find((r) => r.status === "running");
    if (runningRun) {
      const endsAtMs = Date.parse(runningRun.ends_at);
      const totalMs = Math.max(1000, (cat?.base_duration_secs ?? 300) * 1000);
      return { kind: "running", subLabel: "Aktif", endsAtMs, totalMs };
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
      subLabel: "Dibangun",
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

  if (
    city &&
    quote &&
    !stageGte(city.stage, quote.min_stage)
  ) {
    return { kind: "locked", subLabel: quote.min_stage_name };
  }

  return { kind: "unbuilt", subLabel: "Kosong" };
}

function drawTerrain(world: Container, stage: CityStageKey): void {
  const bg = new Graphics();
  const bleedX = 1400;
  const bleedY = 1200;

  // Full-bleed meadow so wide/tall screens never show dark borders
  bg.rect(
    -bleedX,
    -bleedY,
    WORLD_WIDTH + bleedX * 2,
    WORLD_HEIGHT + bleedY * 2,
  ).fill({ color: 0x264430 });

  // Subtle grass contour patches
  bg.ellipse(220, 410, 420, 310).fill({ color: 0x2c4f37, alpha: 0.72 });
  bg.ellipse(570, 335, 260, 180).fill({ color: 0x32593e, alpha: 0.65 });
  bg.ellipse(-180, 260, 260, 220).fill({ color: 0x223e2b, alpha: 0.7 });
  bg.ellipse(160, 860, 380, 200).fill({ color: 0x223e2b, alpha: 0.65 });

  // Northern rocky quarry terrace (Mining zone, extended north-west)
  bg.roundRect(-240, -260, 710, 470, 36)
    .fill({ color: 0x333e4c })
    .stroke({ color: 0x4b596b, width: 2 });
  bg.ellipse(245, 130, 145, 58).fill({ color: 0x3e4b5b, alpha: 0.85 });

  // Eastern river & harbor water (Fishing & Trade Port, full-bleed east)
  bg.poly([
    850, -bleedY,
    WORLD_WIDTH + bleedX, -bleedY,
    WORLD_WIDTH + bleedX, WORLD_HEIGHT + bleedY,
    810, WORLD_HEIGHT + bleedY,
    815, WORLD_HEIGHT,
    835, 510,
    875, 340,
    830, 160,
  ])
    .fill({ color: 0x1d4d6d })
    .stroke({ color: 0x3d82a8, width: 3 });
  bg.poly([
    890, -bleedY,
    WORLD_WIDTH + bleedX, -bleedY,
    WORLD_WIDTH + bleedX, WORLD_HEIGHT + bleedY,
    860, WORLD_HEIGHT + bleedY,
    890, 510,
    920, 340,
    875, 160,
  ]).fill({ color: 0x25628a, alpha: 0.55 });

  // Central town plaza
  const plazaRadius = stageGte(stage, "kota_kecil") ? 145 : 108;
  bg.ellipse(560, 360, plazaRadius, plazaRadius * 0.62)
    .fill({ color: 0x5c5346, alpha: 0.85 })
    .stroke({ color: 0x7b6f5e, width: 2 });

  // Connecting dirt / stone paths
  const roads: Array<[number, number, number, number]> = [
    [560, 360, 245, 555],
    [560, 360, 215, 355],
    [560, 360, 245, 155],
    [560, 360, 410, 445],
    [560, 360, 425, 255],
    [560, 360, 560, 165],
    [560, 360, 725, 390],
    [560, 360, 735, 235],
    [725, 390, 755, 555],
    [725, 390, 915, 410],
    [755, 555, 925, 575],
  ];
  for (const [x1, y1, x2, y2] of roads) {
    bg.moveTo(x1, y1)
      .lineTo(x2, y2)
      .stroke({ color: 0x6f604e, width: 14, cap: "round", alpha: 0.82 });
  }

  // Decorative trees near forest & outer meadow
  const treeSpots = [
    [95, 290],
    [130, 420],
    [105, 480],
    [320, 295],
    [140, 625],
    [450, 625],
    [-40, 340],
    [-70, 520],
    [40, 690],
    [360, 720],
    [620, 690],
  ];
  for (const [tx, ty] of treeSpots) {
    bg.ellipse(tx, ty + 14, 18, 8).fill({ color: 0x0e1812, alpha: 0.35 });
    bg.roundRect(tx - 4, ty - 6, 8, 20, 2).fill({ color: 0x5e412c });
    bg.circle(tx, ty - 12, 16).fill({ color: 0x3b7638 });
    bg.circle(tx - 5, ty - 16, 11).fill({ color: 0x56964c });
  }

  world.addChild(bg);
}

interface LiveBar {
  fill: Graphics;
  x: number;
  y: number;
  width: number;
  endsAtMs: number;
  totalMs: number;
  color: number;
}

function drawSlotNode(
  spec: MapSlotSpec,
  state: SlotVisualState,
  selected: boolean,
  liveBars: LiveBar[],
  lang: UiLang,
): Container {
  const root = new Container();
  root.x = spec.x;
  root.y = spec.y;
  root.zIndex = spec.y;
  root.eventMode = "static";
  root.cursor = "pointer";

  const g = new Graphics();
  const halfW = spec.width / 2;
  const halfD = spec.depth / 2;

  // Ground shadow / footprint
  g.ellipse(0, 6, halfW * 0.96, halfD * 0.72).fill({
    color: 0x0b111b,
    alpha: state.kind === "locked" ? 0.25 : 0.48,
  });

  if (selected) {
    g.ellipse(0, 4, halfW * 1.08, halfD * 0.84).stroke({
      color: 0xffd166,
      width: 3.5,
      alpha: 0.95,
    });
  }

  if (state.kind === "locked") {
    g.roundRect(-halfW * 0.82, -spec.wallHeight * 0.65, spec.width * 0.82, spec.wallHeight * 0.65 + halfD * 0.35, 10)
      .fill({ color: 0x26303d, alpha: 0.65 })
      .stroke({ color: 0x4b586b, width: 1.5, alpha: 0.7 });
  } else if (state.kind === "unbuilt") {
    g.roundRect(-halfW * 0.84, -halfD * 0.56, spec.width * 0.84, spec.depth * 0.62, 10)
      .fill({ color: 0x3b4439, alpha: 0.55 })
      .stroke({ color: 0xa8b6c8, width: 2, alpha: 0.65 });
    // Corner stakes
    for (const [sx, sy] of [
      [-halfW * 0.74, -halfD * 0.44],
      [halfW * 0.74, -halfD * 0.44],
      [-halfW * 0.74, halfD * 0.36],
      [halfW * 0.74, halfD * 0.36],
    ]) {
      g.roundRect(sx - 3, sy - 10, 6, 12, 2).fill({ color: 0xa87955 });
    }
  } else if (state.kind === "building") {
    const h = Math.max(36, spec.wallHeight * 0.85);
    const w = spec.width * 0.82;
    g.roundRect(-w / 2, -h, w, h + 10, 8)
      .fill({ color: 0x624d3b, alpha: 0.9 })
      .stroke({ color: 0xffd166, width: 2, alpha: 0.85 });
    // Scaffold crossbars
    g.moveTo(-w / 2 + 8, -h + 8)
      .lineTo(w / 2 - 8, 2)
      .moveTo(w / 2 - 8, -h + 8)
      .lineTo(-w / 2 + 8, 2)
      .stroke({ color: 0xffd166, width: 2, alpha: 0.7 });
  } else {
    // 3/4 View building (built / running / ready)
    const levelBoost =
      spec.id === "granary" && state.kind === "built"
        ? (state.level - 1) * 9
        : 0;
    const wallH = spec.wallHeight + levelBoost;
    const w = spec.width * 0.86;
    const roofH = spec.roofHeight;

    // Front wall (darker shade in 3/4 view)
    g.roundRect(-w / 2, -wallH, w, wallH + 10, 6)
      .fill({ color: spec.wallColor })
      .stroke({ color: 0x121824, width: 2, alpha: 0.65 });

    // Roof overhang (angled top face)
    g.poly([
      -w / 2 - 8, -wallH + 2,
      -w / 2 + 12, -wallH - roofH,
      w / 2 - 12, -wallH - roofH,
      w / 2 + 8, -wallH + 2,
    ])
      .fill({ color: spec.roofColor })
      .stroke({ color: 0x121824, width: 2, alpha: 0.6 });

    // Roof ridge highlight
    g.moveTo(-w / 2 + 14, -wallH - roofH + 5)
      .lineTo(w / 2 - 14, -wallH - roofH + 5)
      .stroke({ color: 0xffffff, width: 2, alpha: 0.25 });

    // Door & accent window
    const doorW = Math.min(24, w * 0.22);
    const doorH = Math.min(26, wallH * 0.55);
    g.roundRect(-doorW / 2, 10 - doorH, doorW, doorH, 4).fill({
      color: 0x241c18,
      alpha: 0.85,
    });
    if (w >= 100 && wallH >= 32) {
      g.roundRect(-w * 0.33, -wallH * 0.6, 14, 12, 2).fill({
        color: spec.accentColor,
        alpha: 0.85,
      });
      g.roundRect(w * 0.33 - 14, -wallH * 0.6, 14, 12, 2).fill({
        color: spec.accentColor,
        alpha: 0.85,
      });
    }
  }

  root.addChild(g);

  // Label pill underneath
  const titleText = slotLabel(spec.id, lang);
  const topLabelY = 24;
  const pill = new Graphics();
  const pillW = Math.max(102, titleText.length * 8.2 + 26);
  pill.roundRect(-pillW / 2, topLabelY, pillW, 24, 12)
    .fill({ color: 0x121824, alpha: 0.9 })
    .stroke({
      color: selected ? 0xffd166 : 0x3b4d66,
      width: selected ? 2 : 1.2,
    });
  root.addChild(pill);

  const labelText = new Text({
    text: titleText,
    style: new TextStyle({
      fontFamily: "system-ui, -apple-system, Segoe UI, sans-serif",
      fontSize: 12,
      fontWeight: "700",
      fill: state.kind === "locked" ? 0x8d9aad : 0xeef2f6,
    }),
  });
  labelText.anchor.set(0.5, 0.5);
  labelText.x = 0;
  labelText.y = topLabelY + 12;
  root.addChild(labelText);

  // Status badge above building
  const badgeY =
    state.kind === "unbuilt"
      ? -34
      : state.kind === "locked"
        ? -spec.wallHeight * 0.65 - 16
        : -spec.wallHeight - spec.roofHeight - 16;

  if (state.kind === "ready") {
    const badge = new Graphics();
    badge
      .roundRect(-36, badgeY - 12, 72, 22, 11)
      .fill({ color: 0xffd166 })
      .stroke({ color: 0x121824, width: 1.5 });
    root.addChild(badge);
    const t = new Text({
      text: "SIAP!",
      style: new TextStyle({
        fontFamily: "system-ui, -apple-system, Segoe UI, sans-serif",
        fontSize: 11,
        fontWeight: "800",
        fill: 0x121824,
      }),
    });
    t.anchor.set(0.5, 0.5);
    t.y = badgeY - 1;
    root.addChild(t);
  } else if (state.kind === "running" || state.kind === "building") {
    const barW = 86;
    const barX = -barW / 2;
    const barY = badgeY - 6;
    const track = new Graphics();
    track
      .roundRect(barX, barY, barW, 11, 5)
      .fill({ color: 0x121824, alpha: 0.92 })
      .stroke({ color: 0x4b5e78, width: 1 });
    root.addChild(track);

    const fill = new Graphics();
    root.addChild(fill);
    liveBars.push({
      fill,
      x: barX + 1.5,
      y: barY + 1.5,
      width: barW - 3,
      endsAtMs: state.endsAtMs,
      totalMs: state.totalMs,
      color: state.kind === "building" ? 0xffd166 : 0xff914d,
    });
  } else if (state.kind === "locked") {
    const lockBg = new Graphics();
    const txt = `🔒 ${state.subLabel}`;
    const lw = Math.max(72, txt.length * 7 + 16);
    lockBg
      .roundRect(-lw / 2, badgeY - 10, lw, 20, 10)
      .fill({ color: 0x17202e, alpha: 0.92 })
      .stroke({ color: 0x46566e, width: 1 });
    root.addChild(lockBg);
    const t = new Text({
      text: txt,
      style: new TextStyle({
        fontFamily: "system-ui, -apple-system, Segoe UI, sans-serif",
        fontSize: 10.5,
        fontWeight: "600",
        fill: 0xa8b6c8,
      }),
    });
    t.anchor.set(0.5, 0.5);
    t.y = badgeY;
    root.addChild(t);
  } else if (state.kind === "built" && spec.category === "facility") {
    const lvBg = new Graphics();
    lvBg
      .roundRect(-24, badgeY - 10, 48, 20, 10)
      .fill({ color: 0x1f3329, alpha: 0.95 })
      .stroke({ color: 0x79b85c, width: 1.3 });
    root.addChild(lvBg);
    const t = new Text({
      text: state.subLabel,
      style: new TextStyle({
        fontFamily: "system-ui, -apple-system, Segoe UI, sans-serif",
        fontSize: 11,
        fontWeight: "700",
        fill: 0x8cd06e,
      }),
    });
    t.anchor.set(0.5, 0.5);
    t.y = badgeY;
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
  const liveBarsRef = useRef<LiveBar[]>([]);
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
    world.position.set(camRef.current.x, camRef.current.y);
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

  const renderWorld = (snap: CitySnapshot, sel: MapSlotId | null) => {
    const world = worldRef.current;
    if (!world) return;
    world.removeChildren();
    liveBarsRef.current = [];

    const stage = snap.city?.stage ?? "kampung";
    const lang: UiLang = snap.me.profile?.lang ?? "id";
    drawTerrain(world, stage);

    for (const spec of MAP_SLOTS) {
      const st = resolveSlotState(spec, snap);
      const node = drawSlotNode(
        spec,
        st,
        sel === spec.id,
        liveBarsRef.current,
        lang,
      );
      node.on("pointerup", () => {
        if (!dragRef.current.moved) {
          onSelectRef.current(spec.id);
        }
      });
      world.addChild(node);
    }

    updateLiveBars(snap.clockOffsetMs);
  };

  const updateLiveBars = (clockOffsetMs: number) => {
    const serverNow = Date.now() + clockOffsetMs;
    for (const bar of liveBarsRef.current) {
      const rem = Math.max(0, bar.endsAtMs - serverNow);
      const pct = Math.max(
        0.08,
        Math.min(1, 1 - rem / Math.max(1000, bar.totalMs)),
      );
      bar.fill.clear();
      bar.fill
        .roundRect(bar.x, bar.y, bar.width * pct, 8, 4)
        .fill({ color: bar.color });
    }
  };

  // Keep latest snapshot accessible inside init callback
  const latestRef = useRef({ snapshot, selectedSlot });
  latestRef.current = { snapshot, selectedSlot };

  useEffect(() => {
    const el = hostRef.current;
    if (!el) return;

    let disposed = false;
    const app = new Application();

    (async () => {
      await app.init({
        resizeTo: el,
        background: 0x264430,
        antialias: true,
        resolution: Math.min(window.devicePixelRatio || 1, 2),
        autoDensity: true,
      });

      if (disposed) {
        app.destroy(true, { children: true });
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
      const nextFit = getFitScale(width, height);
      if (Math.abs(camRef.current.scale - lastFitScaleRef.current) < 0.015) {
        camRef.current.scale = nextFit;
      }
      lastFitScaleRef.current = nextFit;
      clampCamera();
    });
    ro.observe(el);

    const timer = window.setInterval(() => {
      updateLiveBars(latestRef.current.snapshot.clockOffsetMs);
    }, 1000);

    return () => {
      disposed = true;
      ro.disconnect();
      window.clearInterval(timer);
      if (appRef.current) {
        appRef.current.destroy(true, { children: true });
        appRef.current = null;
      }
      worldRef.current = null;
      liveBarsRef.current = [];
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
        <button type="button" onClick={() => zoomBy(1.15)} title="Perbesar">
          +
        </button>
        <button type="button" onClick={() => zoomBy(0.87)} title="Perkecil">
          -
        </button>
        <button type="button" onClick={resetCamera} title="Reset Kamera">
          Reset
        </button>
      </div>
    </div>
  );
}
