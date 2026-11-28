import type { Texture } from "pixi.js";
import type { CityStageKey } from "../api/types";
import type { MapSlotId, MapSlotSpec } from "./assetRegistry";
import { PixelCanvas, PixelTextureCache, SDV } from "./pixelArt";

type EraTier = "rural" | "town" | "urban";

function toEraTier(stage: CityStageKey): EraTier {
  if (stage === "kota_dagang" || stage === "metropolis") return "urban";
  if (stage === "kota_kecil") return "town";
  return "rural";
}

export type SlotRenderStateKind =
  | "locked"
  | "unbuilt"
  | "building"
  | "running"
  | "ready"
  | "built";

export interface SlotSpriteAsset {
  texture: Texture;
  /** Native pixel anchor X (0..1) */
  anchorX: number;
  /** Native pixel anchor Y (0..1), aligned to slot ground origin (0, 0) */
  anchorY: number;
  /** World Y offset above slot center where status badges/bars sit */
  badgeOffsetY: number;
}

interface BuildingBox {
  w: number;
  h: number;
  originX: number;
  originY: number;
}

const BOX: BuildingBox = {
  w: 96,
  h: 96,
  originX: 48,
  originY: 78,
};

export function getSelectionRingTexture(
  cache: PixelTextureCache,
  widthWorld: number,
  depthWorld: number,
): Texture {
  const rx = Math.round(widthWorld / 4) + 6;
  const ry = Math.round(depthWorld / 4) + 4;
  const w = rx * 2 + 8;
  const h = ry * 2 + 8;
  return cache.get(`ui:select_ring:${rx}x${ry}`, () => {
    const c = new PixelCanvas(w, h);
    const cx = Math.floor(w / 2);
    const cy = Math.floor(h / 2);
    c.ellipse(cx, cy, rx + 1, ry + 1, SDV.ink);
    c.ellipse(cx, cy, rx, ry, SDV.goldBase);
    c.ellipse(cx, cy, rx - 2, ry - 2, "rgba(0,0,0,0)");
    // Cut out interior cleanly
    for (let dy = -(ry - 2); dy <= ry - 2; dy++) {
      const ratio = dy / (ry - 2);
      const halfSpan = Math.floor((rx - 2) * Math.sqrt(Math.max(0, 1 - ratio * ratio)));
      c.clearRect(cx - halfSpan, cy + dy, halfSpan * 2 + 1, 1);
    }
    // Crisp golden corner brackets
    const bx1 = cx - rx - 2;
    const bx2 = cx + rx + 2;
    const by1 = cy - ry - 1;
    const by2 = cy + ry + 1;
    c.rect(bx1, by1, 5, 1, SDV.goldHi).rect(bx1, by1, 1, 4, SDV.goldHi);
    c.rect(bx2 - 4, by1, 5, 1, SDV.goldHi).rect(bx2, by1, 1, 4, SDV.goldHi);
    c.rect(bx1, by2, 5, 1, SDV.goldHi).rect(bx1, by2 - 3, 1, 4, SDV.goldHi);
    c.rect(bx2 - 4, by2, 5, 1, SDV.goldHi).rect(bx2, by2 - 3, 1, 4, SDV.goldHi);
    return c.toTexture(false);
  });
}

function drawGroundShadow(c: PixelCanvas, cx: number, cy: number, rx: number, ry: number): void {
  c.ellipse(cx, cy, rx, ry, SDV.shadow);
}

function drawWindow(
  c: PixelCanvas,
  x: number,
  y: number,
  w = 6,
  h = 7,
  lit = true,
): void {
  c.strokeRect(x, y, w, h, SDV.woodDark);
  c.rect(x + 1, y + 1, w - 2, h - 2, lit ? SDV.winAmber : SDV.winDark);
  if (lit && w >= 5 && h >= 5) {
    c.rect(x + 1, y + 1, w - 3, Math.max(1, h - 4), SDV.winGlow);
    // Mullion cross
    c.rect(x + Math.floor(w / 2), y + 1, 1, h - 2, SDV.woodDark);
    c.rect(x + 1, y + Math.floor(h / 2), w - 2, 1, SDV.woodDark);
  }
}

function drawDoor(c: PixelCanvas, cx: number, bottomY: number, w = 10, h = 14): void {
  const x = Math.round(cx - w / 2);
  const y = bottomY - h;
  c.strokeRect(x, y, w, h, SDV.ink);
  c.rect(x + 1, y + 1, w - 2, h - 2, SDV.woodMid);
  c.rect(x + 2, y + 2, w - 4, h - 3, SDV.woodBase);
  // Plank lines & brass knob
  c.rect(cx, y + 2, 1, h - 3, SDV.woodDark);
  c.px(x + w - 3, y + Math.floor(h / 2), SDV.goldHi);
}

function drawTimberWall(
  c: PixelCanvas,
  x: number,
  y: number,
  w: number,
  h: number,
  wallBase: string,
  wallShade: string,
  trim: string = SDV.woodDark,
): void {
  c.rect(x, y, w, h, wallBase);
  c.strokeRect(x, y, w, h, SDV.ink);
  // 3/4 underside shadow strip & timber beams
  c.rect(x + 1, y + 1, w - 2, 2, wallShade);
  c.rect(x + 1, y + h - 3, w - 2, 2, wallShade);
  c.rect(x + 1, y + 1, 2, h - 2, trim);
  c.rect(x + w - 3, y + 1, 2, h - 2, trim);
  // Horizontal plank seams
  for (let py = y + 5; py < y + h - 3; py += 4) {
    c.rect(x + 3, py, w - 6, 1, wallShade);
  }
}

function drawChimney(c: PixelCanvas, x: number, y: number, h = 11): void {
  c.rect(x, y, 7, h, SDV.stoneMid);
  c.strokeRect(x, y, 7, h, SDV.ink);
  c.rect(x + 1, y + 1, 2, h - 2, SDV.stoneHi);
  c.rect(x - 1, y, 9, 2, SDV.stoneDark);
  c.rect(x, y, 7, 1, SDV.stoneHi);
}

function drawFlowerBox(c: PixelCanvas, x: number, y: number): void {
  c.rect(x, y, 8, 3, SDV.woodMid);
  c.strokeRect(x, y, 8, 3, SDV.ink);
  c.rect(x + 1, y - 1, 6, 1, SDV.pineBase);
  c.px(x + 2, y - 2, SDV.flowerPink);
  c.px(x + 4, y - 2, SDV.flowerYellow);
  c.px(x + 6, y - 2, SDV.flowerPink);
}

function drawLockedPlot(spec: MapSlotSpec): { c: PixelCanvas; topY: number } {
  const c = new PixelCanvas(BOX.w, BOX.h);
  const cx = BOX.originX;
  const cy = BOX.originY;
  const rx = Math.min(34, Math.round(spec.width / 4) + 2);
  const ry = Math.min(18, Math.round(spec.depth / 4) + 2);

  drawGroundShadow(c, cx, cy + 2, rx, ry);
  c.ellipse(cx, cy, rx, ry, SDV.stoneDark);
  c.ellipse(cx, cy - 1, rx - 2, ry - 2, SDV.grassDeep);

  // Overgrown bushes & weathered fence barrier
  c.ellipse(cx - 12, cy - 5, 7, 5, SDV.pineDark);
  c.ellipse(cx - 12, cy - 6, 5, 4, SDV.pineBase);
  c.ellipse(cx + 12, cy - 4, 6, 5, SDV.pineDark);
  c.ellipse(cx + 12, cy - 5, 4, 3, SDV.pineBase);

  // Wooden barricade X-fence
  c.rect(cx - 16, cy - 14, 3, 14, SDV.woodDark);
  c.rect(cx + 13, cy - 14, 3, 14, SDV.woodDark);
  c.rect(cx - 15, cy - 12, 30, 3, SDV.woodMid);
  c.rect(cx - 15, cy - 7, 30, 3, SDV.woodMid);
  c.strokeRect(cx - 16, cy - 12, 32, 3, SDV.ink);
  c.strokeRect(cx - 16, cy - 7, 32, 3, SDV.ink);

  // Small iron padlock on fence center
  c.strokeRect(cx - 3, cy - 16, 6, 5, SDV.stoneHi);
  c.rect(cx - 4, cy - 12, 8, 7, SDV.goldBase);
  c.strokeRect(cx - 4, cy - 12, 8, 7, SDV.ink);
  c.px(cx, cy - 9, SDV.ink);

  return { c, topY: cy - 22 };
}

function drawUnbuiltPlot(spec: MapSlotSpec): { c: PixelCanvas; topY: number } {
  const c = new PixelCanvas(BOX.w, BOX.h);
  const cx = BOX.originX;
  const cy = BOX.originY;
  const hw = Math.min(32, Math.round(spec.width / 4));
  const hd = Math.min(16, Math.round(spec.depth / 4));

  // Cleared raked earth plot
  c.bevelBox(
    cx - hw,
    cy - hd,
    hw * 2,
    hd * 2,
    SDV.dirtBase,
    SDV.dirtEdge,
    SDV.dirtLight,
    SDV.dirtDark,
  );
  // Subtle rake lines
  for (let ry = cy - hd + 4; ry < cy + hd - 3; ry += 4) {
    c.rect(cx - hw + 5, ry, hw * 2 - 10, 1, SDV.dirtDark);
  }

  // 4 corner timber stakes + rope perimeter
  const corners: Array<[number, number]> = [
    [cx - hw + 2, cy - hd + 1],
    [cx + hw - 4, cy - hd + 1],
    [cx - hw + 2, cy + hd - 4],
    [cx + hw - 4, cy + hd - 4],
  ];
  c.strokeRect(cx - hw + 3, cy - hd - 2, hw * 2 - 6, hd * 2 - 2, SDV.plasterShade);
  for (const [sx, sy] of corners) {
    c.rect(sx, sy - 6, 3, 8, SDV.woodDark);
    c.px(sx + 1, sy - 5, SDV.woodLight);
  }

  // Tiny blueprint signpost in center
  c.rect(cx - 1, cy - 10, 2, 10, SDV.woodDark);
  c.bevelBox(cx - 7, cy - 17, 14, 9, SDV.parchmentBase, SDV.ink, SDV.parchmentHi, SDV.parchmentShade);
  c.rect(cx - 5, cy - 15, 10, 5, SDV.slateBase);
  c.rect(cx - 4, cy - 14, 8, 3, SDV.slateHi);

  return { c, topY: cy - 22 };
}

function drawBuildingPlot(spec: MapSlotSpec): { c: PixelCanvas; topY: number } {
  const c = new PixelCanvas(BOX.w, BOX.h);
  const cx = BOX.originX;
  const cy = BOX.originY;
  const w = Math.min(62, Math.round(spec.width * 0.42));
  const h = Math.max(24, Math.min(44, Math.round(spec.wallHeight * 0.48)));
  const x = cx - Math.floor(w / 2);
  const y = cy - h;

  drawGroundShadow(c, cx, cy + 3, Math.floor(w / 2) + 4, 10);

  // Unfinished stone & timber foundation + walls
  c.rect(x, y + 6, w, h - 4, SDV.woodMid);
  c.strokeRect(x, y + 6, w, h - 4, SDV.ink);
  c.rect(x + 2, cy - 8, w - 4, 9, SDV.stoneBase);

  // Timber scaffolding posts & planks
  c.rect(x - 3, y - 2, 3, h + 5, SDV.woodDark);
  c.rect(x + w, y - 2, 3, h + 5, SDV.woodDark);
  c.rect(cx - 1, y - 4, 2, h + 6, SDV.woodDark);
  c.rect(x - 4, y + Math.floor(h * 0.35), w + 8, 3, SDV.woodLight);
  c.strokeRect(x - 4, y + Math.floor(h * 0.35), w + 8, 3, SDV.ink);
  c.rect(x - 4, y + Math.floor(h * 0.72), w + 8, 3, SDV.woodLight);
  c.strokeRect(x - 4, y + Math.floor(h * 0.72), w + 8, 3, SDV.ink);

  // Leaning wooden ladder & stacked lumber
  c.line(x + w - 10, y + 2, x + w - 4, cy + 3, SDV.woodHi, 1);
  c.line(x + w - 6, y + 2, x + w, cy + 3, SDV.woodHi, 1);
  c.rect(x - 8, cy, 10, 4, SDV.woodBase);
  c.strokeRect(x - 8, cy, 10, 4, SDV.ink);

  return { c, topY: y - 6 };
}

function drawBuiltSlot(
  spec: MapSlotSpec,
  level: number,
  era: EraTier,
  stage: CityStageKey,
): { c: PixelCanvas; topY: number } {
  const c = new PixelCanvas(BOX.w, BOX.h);
  const cx = BOX.originX;
  const cy = BOX.originY;
  let topY = cy - 40;

  switch (spec.id as MapSlotId) {
    case "mining": {
      drawGroundShadow(c, cx, cy + 2, 36, 13);
      // Rocky hillside mound
      c.ellipse(cx, cy - 16, 33, 24, SDV.ink);
      c.ellipse(cx, cy - 16, 31, 22, SDV.stoneDark);
      c.ellipse(cx - 2, cy - 19, 27, 17, SDV.stoneMid);
      c.ellipse(cx - 4, cy - 24, 18, 9, SDV.stoneHi);
      // Grassy moss cap on top of hill
      c.ellipse(cx, cy - 33, 20, 6, SDV.grassDeep);
      c.ellipse(cx - 2, cy - 34, 16, 4, SDV.grassBase);

      // Timber mine portal & dark tunnel opening
      c.rect(cx - 15, cy - 23, 30, 24, SDV.woodDark);
      c.strokeRect(cx - 15, cy - 23, 30, 24, SDV.ink);
      c.rect(cx - 14, cy - 22, 28, 4, SDV.woodBase);
      c.rect(cx - 11, cy - 18, 22, 19, SDV.ink);
      c.rect(cx - 9, cy - 16, 18, 17, "#1a1314");

      // Iron rails exiting tunnel
      c.rect(cx - 5, cy - 4, 2, 13, SDV.stoneHi);
      c.rect(cx + 3, cy - 4, 2, 13, SDV.stoneHi);
      for (let ry = cy - 2; ry <= cy + 7; ry += 3) {
        c.rect(cx - 7, ry, 14, 1, SDV.woodMid);
      }

      // Ore-laden minecart with sparkling cyan/gold gems
      c.rect(cx - 7, cy - 8, 14, 7, SDV.stoneMid);
      c.strokeRect(cx - 7, cy - 8, 14, 7, SDV.ink);
      c.rect(cx - 5, cy - 11, 10, 3, SDV.stoneDark);
      c.px(cx - 3, cy - 10, SDV.oreCyan);
      c.px(cx, cy - 11, SDV.oreGold);
      c.px(cx + 2, cy - 10, SDV.oreCyan);

      // Hanging glowing mine lantern & ore veins
      c.px(cx - 13, cy - 15, SDV.winAmber);
      c.px(cx - 13, cy - 14, SDV.winGlow);
      c.px(cx + 20, cy - 18, SDV.oreCyan);
      c.px(cx - 22, cy - 14, SDV.oreGold);
      topY = cy - 40;
      break;
    }

    case "gathering": {
      drawGroundShadow(c, cx, cy + 3, 34, 13);
      // Flanking background pine trees
      c.ellipse(cx - 24, cy - 18, 9, 14, SDV.pineDark);
      c.ellipse(cx - 24, cy - 21, 7, 10, SDV.pineBase);
      c.ellipse(cx + 24, cy - 17, 9, 13, SDV.pineDark);
      c.ellipse(cx + 24, cy - 20, 7, 9, SDV.pineBase);

      // Log cabin walls
      drawTimberWall(c, cx - 22, cy - 24, 44, 24, SDV.woodBase, SDV.woodMid, SDV.woodDark);
      drawDoor(c, cx - 4, cy, 9, 13);
      drawWindow(c, cx + 8, cy - 16, 7, 7, true);
      drawChimney(c, cx + 11, cy - 44, 12);
      // Mossy pine-green roof
      c.roofTrapezoid(cx, cy - 39, 34, 52, 16, {
        base: SDV.pineBase,
        hi: SDV.pineHi,
        lo: SDV.pineDark,
        border: SDV.ink,
      });

      // Firewood pile & stump with axe on right/left
      c.rect(cx - 30, cy - 6, 8, 6, SDV.woodMid);
      c.strokeRect(cx - 30, cy - 6, 8, 6, SDV.ink);
      c.px(cx - 28, cy - 4, SDV.woodHi);
      c.px(cx - 25, cy - 4, SDV.woodHi);
      c.rect(cx + 21, cy - 4, 6, 5, SDV.woodMid);
      c.rect(cx + 22, cy - 5, 4, 2, SDV.woodHi);
      c.line(cx + 24, cy - 5, cx + 27, cy - 10, SDV.woodLight, 1);
      c.rect(cx + 26, cy - 11, 3, 2, SDV.stoneHi);
      topY = cy - 45;
      break;
    }

    case "farming": {
      drawGroundShadow(c, cx, cy + 3, 38, 14);
      // Tilled soil garden bed on left-front with golden wheat & leafy crops
      c.bevelBox(cx - 36, cy - 10, 34, 18, SDV.soilMid, SDV.ink, SDV.dirtDark, SDV.soilDark);
      for (let ry = cy - 7; ry <= cy + 4; ry += 4) {
        c.rect(cx - 34, ry, 30, 1, SDV.soilDark);
        for (let rx = cx - 32; rx <= cx - 7; rx += 5) {
          c.px(rx, ry - 1, SDV.pineBase);
          c.px(rx + 1, ry - 2, ry < cy ? SDV.goldHi : SDV.grassBright);
        }
      }

      // Classic Stardew red barn on right
      drawTimberWall(c, cx - 2, cy - 23, 36, 23, SDV.barnBase, SDV.barnDark, SDV.plasterHi);
      // White X-braced barn doors
      c.rect(cx + 9, cy - 14, 14, 14, SDV.barnDark);
      c.strokeRect(cx + 9, cy - 14, 14, 14, SDV.plasterHi);
      c.line(cx + 10, cy - 13, cx + 21, cy - 2, SDV.plasterHi, 1);
      c.line(cx + 21, cy - 13, cx + 10, cy - 2, SDV.plasterHi, 1);
      // Barn loft window & pitched red roof
      c.roofTrapezoid(cx + 16, cy - 38, 22, 42, 16, {
        base: SDV.barnBase,
        hi: SDV.barnHi,
        lo: SDV.barnDark,
        border: SDV.ink,
      });
      c.rect(cx + 13, cy - 31, 6, 5, SDV.winDark);
      c.strokeRect(cx + 13, cy - 31, 6, 5, SDV.plasterHi);

      // Golden hay bale beside barn
      c.bevelBox(cx + 30, cy - 5, 9, 7, SDV.goldBase, SDV.ink, SDV.goldHi, SDV.goldDark);
      topY = cy - 40;
      break;
    }

    case "fishing": {
      // Timber pier stretching over eastern water
      c.rect(cx - 28, cy - 4, 48, 8, SDV.woodBase);
      c.strokeRect(cx - 28, cy - 4, 48, 8, SDV.ink);
      // Plank lines & pilings dipping into water foam
      for (let px = cx - 24; px <= cx + 16; px += 8) {
        c.rect(px, cy - 3, 1, 6, SDV.woodDark);
        c.rect(px - 1, cy + 4, 3, 7, SDV.woodDark);
        c.rect(px - 2, cy + 10, 5, 1, SDV.waterFoam);
      }

      // Cozy teal-roofed bait shack on pier
      drawTimberWall(c, cx - 24, cy - 21, 28, 17, SDV.plasterBase, SDV.plasterShade, SDV.woodMid);
      drawDoor(c, cx - 15, cy - 4, 7, 10);
      drawWindow(c, cx - 6, cy - 16, 6, 6, true);
      c.roofTrapezoid(cx - 10, cy - 32, 20, 34, 12, {
        base: SDV.tealBase,
        hi: SDV.tealHi,
        lo: SDV.tealDark,
        border: SDV.ink,
      });

      // Fishing rod, fish crate & moored wooden rowboat
      c.line(cx + 14, cy - 4, cx + 25, cy - 18, SDV.woodHi, 1);
      c.line(cx + 25, cy - 18, cx + 25, cy + 6, SDV.plasterHi, 1);
      c.bevelBox(cx + 5, cy - 7, 7, 5, SDV.woodLight, SDV.ink, SDV.woodHi, SDV.woodMid);

      // Tiny moored rowboat
      c.ellipse(cx + 28, cy + 8, 10, 4, SDV.ink);
      c.ellipse(cx + 28, cy + 7, 9, 3, SDV.woodBase);
      c.ellipse(cx + 28, cy + 7, 6, 2, SDV.woodDark);
      c.rect(cx + 22, cy + 11, 12, 1, SDV.waterFoam);
      topY = cy - 34;
      break;
    }

    case "market_npc": {
      drawGroundShadow(c, cx, cy + 3, 34, 12);
      if (era === "urban") {
        // Grand Covered Bazaar Arcade in Trade City / Metropolis
        drawTimberWall(c, cx - 26, cy - 26, 52, 26, SDV.stoneBase, SDV.stoneMid, SDV.stoneHi);
        c.rect(cx - 24, cy - 12, 48, 12, SDV.plasterBase);
        c.rect(cx - 24, cy - 23, 3, 23, SDV.stoneHi);
        c.rect(cx + 21, cy - 23, 3, 23, SDV.stoneHi);
        for (let ax = cx - 28; ax < cx + 28; ax++) {
          const stripe = Math.floor((ax - (cx - 28)) / 4) % 2 === 0;
          c.rect(ax, cy - 35, 1, 9, stripe ? SDV.goldBase : SDV.barnHi);
          c.px(ax, cy - 26, SDV.ink);
        }
        c.strokeRect(cx - 28, cy - 36, 56, 11, SDV.ink);
        c.bevelBox(cx - 23, cy - 11, 46, 11, SDV.woodLight, SDV.ink, SDV.woodHi, SDV.woodMid);
        c.rect(cx - 19, cy - 14, 7, 4, SDV.barnHi);
        c.rect(cx - 10, cy - 14, 7, 4, SDV.goldHi);
        c.rect(cx - 1, cy - 14, 7, 4, SDV.pineHi);
        c.rect(cx + 8, cy - 14, 7, 4, SDV.tealHi);
        c.rect(cx + 16, cy - 15, 5, 5, SDV.winGlow);
        topY = cy - 37;
      } else {
        // Rustic open-air village market stall
        drawTimberWall(c, cx - 24, cy - 22, 48, 22, SDV.woodBase, SDV.woodMid, SDV.woodDark);
        for (let ax = cx - 27; ax < cx + 27; ax++) {
          const stripe = Math.floor((ax - (cx - 27)) / 4) % 2 === 0;
          const fill = stripe ? SDV.barnHi : SDV.plasterHi;
          const shade = stripe ? SDV.barnBase : SDV.plasterShade;
          c.rect(ax, cy - 32, 1, 10, fill);
          c.px(ax, cy - 22, shade);
          c.px(ax, cy - 21, SDV.ink);
        }
        c.strokeRect(cx - 27, cy - 33, 54, 12, SDV.ink);

        c.bevelBox(cx - 25, cy - 11, 50, 11, SDV.woodLight, SDV.ink, SDV.woodHi, SDV.woodMid);
        c.rect(cx - 21, cy - 14, 7, 4, SDV.barnHi);
        c.rect(cx - 12, cy - 14, 7, 4, SDV.goldHi);
        c.rect(cx - 3, cy - 14, 7, 4, SDV.pineHi);
        c.rect(cx + 6, cy - 14, 7, 4, SDV.tealHi);
        c.rect(cx + 15, cy - 15, 5, 5, SDV.plasterHi);

        c.bevelBox(cx + 26, cy - 9, 8, 10, SDV.woodBase, SDV.ink, SDV.woodLight, SDV.woodDark);
        c.rect(cx + 26, cy - 6, 8, 1, SDV.stoneDark);
        topY = cy - 34;
      }
      break;
    }

    case "well": {
      drawGroundShadow(c, cx, cy + 2, 22, 9);
      // Cobblestone circular base
      c.ellipse(cx, cy - 3, 15, 8, SDV.ink);
      c.ellipse(cx, cy - 4, 14, 7, SDV.stoneMid);
      c.ellipse(cx, cy - 8, 14, 6, SDV.stoneHi);
      c.ellipse(cx, cy - 8, 10, 4, SDV.waterDeep);
      c.rect(cx - 5, cy - 8, 7, 1, SDV.waterFoam);

      // Timber uprights & rope crank
      c.rect(cx - 12, cy - 24, 3, 16, SDV.woodDark);
      c.rect(cx + 9, cy - 24, 3, 16, SDV.woodDark);
      c.rect(cx - 10, cy - 18, 20, 2, SDV.woodLight);
      c.rect(cx, cy - 17, 1, 7, SDV.plasterShade);
      c.rect(cx - 2, cy - 11, 4, 3, SDV.woodBase);

      // Pitched terracotta shingle cap roof
      c.roofTrapezoid(cx, cy - 35, 16, 32, 12, {
        base: SDV.terraBase,
        hi: SDV.terraHi,
        lo: SDV.terraDark,
        border: SDV.ink,
      });
      topY = cy - 37;
      break;
    }

    case "granary": {
      const extraFloor = level > 1 ? Math.min(2, level - 1) * 8 : 0;
      const wallH = 26 + extraFloor;
      drawGroundShadow(c, cx, cy + 3, 30, 12);

      // Stone masonry in urban stages, warm timber in rural/town
      const wallBase = era === "urban" ? SDV.stoneBase : SDV.woodBase;
      const wallShade = era === "urban" ? SDV.stoneMid : SDV.woodMid;
      const wallTrim = era === "urban" ? SDV.stoneDark : SDV.woodDark;
      drawTimberWall(c, cx - 22, cy - wallH, 44, wallH, wallBase, wallShade, wallTrim);
      c.rect(cx - 22, cy - 6, 44, 6, SDV.stoneMid);
      c.strokeRect(cx - 22, cy - 6, 44, 6, SDV.ink);

      drawDoor(c, cx, cy, 10, 13);
      drawWindow(c, cx - 16, cy - 20, 6, 6, true);
      drawWindow(c, cx + 10, cy - 20, 6, 6, true);
      if (extraFloor > 0) {
        drawWindow(c, cx - 4, cy - wallH + 6, 8, 7, true);
      }

      c.roofTrapezoid(cx, cy - wallH - 15, 24, 50, 16, era === "urban"
        ? {
            base: SDV.slateBase,
            hi: SDV.slateHi,
            lo: SDV.slateDark,
            border: SDV.ink,
          }
        : {
            base: SDV.terraBase,
            hi: SDV.terraHi,
            lo: SDV.terraDark,
            border: SDV.ink,
          });

      c.bevelBox(cx - 28, cy - 6, 8, 7, SDV.parchmentBase, SDV.ink, SDV.parchmentHi, SDV.parchmentShade);
      c.bevelBox(cx + 20, cy - 5, 7, 6, SDV.goldBase, SDV.ink, SDV.goldHi, SDV.goldDark);
      topY = cy - wallH - 17;
      break;
    }

    case "town_hall": {
      drawGroundShadow(c, cx, cy + 3, 38, 14);
      const urbanHall = era === "urban";
      drawTimberWall(
        c,
        cx - 29,
        cy - 27,
        58,
        27,
        urbanHall ? SDV.stoneHi : SDV.plasterBase,
        urbanHall ? SDV.stoneBase : SDV.plasterShade,
        urbanHall ? SDV.stoneDark : SDV.woodDark,
      );
      c.rect(cx - 13, cy - 26, 2, 25, urbanHall ? SDV.stoneMid : SDV.woodMid);
      c.rect(cx + 11, cy - 26, 2, 25, urbanHall ? SDV.stoneMid : SDV.woodMid);

      c.rect(cx - 9, cy - 1, 18, 3, SDV.stoneHi);
      drawDoor(c, cx, cy - 1, 12, 14);

      drawWindow(c, cx - 24, cy - 19, 8, 8, true);
      drawWindow(c, cx + 16, cy - 19, 8, 8, true);
      if (urbanHall) {
        // Festive crimson/gold civic banners
        c.rect(cx - 16, cy - 24, 3, 9, SDV.barnHi);
        c.rect(cx + 13, cy - 24, 3, 9, SDV.goldBase);
      } else {
        drawFlowerBox(c, cx - 24, cy - 11);
        drawFlowerBox(c, cx + 16, cy - 11);
      }

      drawChimney(c, cx + 16, cy - 50, 14);
      c.roofTrapezoid(cx, cy - 43, 38, 66, 17, urbanHall
        ? {
            base: SDV.slateBase,
            hi: SDV.slateHi,
            lo: SDV.slateDark,
            border: SDV.ink,
          }
        : {
            base: SDV.terraBase,
            hi: SDV.terraHi,
            lo: SDV.terraDark,
            border: SDV.ink,
          });
      c.bevelBox(cx - 9, cy - 41, 18, 12, urbanHall ? SDV.stoneHi : SDV.plasterBase, SDV.ink, SDV.plasterHi, SDV.plasterShade);
      c.roofTrapezoid(cx, cy - 48, 6, 22, 8, urbanHall
        ? {
            base: SDV.goldBase,
            hi: SDV.goldHi,
            lo: SDV.goldDark,
            border: SDV.ink,
          }
        : {
            base: SDV.terraBase,
            hi: SDV.terraHi,
            lo: SDV.terraDark,
            border: SDV.ink,
          }, false);
      c.rect(cx - 3, cy - 38, 6, 6, SDV.goldBase);
      c.strokeRect(cx - 3, cy - 38, 6, 6, SDV.ink);
      topY = cy - 51;
      break;
    }

    case "market_hall": {
      drawGroundShadow(c, cx, cy + 3, 35, 13);
      drawTimberWall(c, cx - 26, cy - 25, 52, 25, SDV.plasterBase, SDV.plasterShade, SDV.woodMid);
      drawDoor(c, cx, cy, 12, 14);
      drawWindow(c, cx - 21, cy - 19, 7, 7, true);
      drawWindow(c, cx + 14, cy - 19, 7, 7, true);

      // Twin side awnings
      c.rect(cx - 24, cy - 11, 13, 4, SDV.barnHi);
      c.strokeRect(cx - 24, cy - 11, 13, 4, SDV.ink);
      c.rect(cx + 11, cy - 11, 13, 4, SDV.tealHi);
      c.strokeRect(cx + 11, cy - 11, 13, 4, SDV.ink);

      // Golden-ochre guild roof + hanging coin trade sign
      c.roofTrapezoid(cx, cy - 40, 32, 60, 16, {
        base: SDV.goldBase,
        hi: SDV.goldHi,
        lo: SDV.goldDark,
        border: SDV.ink,
      });
      c.bevelBox(cx - 6, cy - 34, 12, 9, SDV.woodDark, SDV.ink, SDV.woodBase, SDV.woodDark);
      c.ellipse(cx, cy - 30, 3, 3, SDV.goldHi);
      topY = cy - 42;
      break;
    }

    case "city_hall": {
      drawGroundShadow(c, cx, cy + 4, 36, 14);
      // Stately dressed stone civic hall
      drawTimberWall(c, cx - 27, cy - 31, 54, 31, SDV.stoneBase, SDV.stoneMid, SDV.stoneDark);
      // Twin entrance pillars & stone stairs
      c.rect(cx - 11, cy - 18, 3, 18, SDV.stoneHi);
      c.rect(cx + 8, cy - 18, 3, 18, SDV.stoneHi);
      c.rect(cx - 10, cy - 1, 20, 3, SDV.stoneHi);
      drawDoor(c, cx, cy - 1, 10, 14);

      drawWindow(c, cx - 22, cy - 23, 7, 9, true);
      drawWindow(c, cx + 15, cy - 23, 7, 9, true);

      // Slate-blue hipped roof + golden clock pediment
      c.roofTrapezoid(cx, cy - 48, 34, 62, 18, {
        base: SDV.slateBase,
        hi: SDV.slateHi,
        lo: SDV.slateDark,
        border: SDV.ink,
      });
      c.bevelBox(cx - 9, cy - 44, 18, 14, SDV.stoneHi, SDV.ink, SDV.plasterHi, SDV.stoneMid);
      c.ellipse(cx, cy - 37, 4, 4, SDV.goldHi);
      c.px(cx, cy - 37, SDV.ink);
      c.px(cx + 1, cy - 38, SDV.ink);
      topY = cy - 50;
      break;
    }

    case "trade_port": {
      const stoneQuay = era === "urban";
      c.rect(cx - 32, cy - 5, 64, 10, stoneQuay ? SDV.stoneBase : SDV.woodBase);
      c.strokeRect(cx - 32, cy - 5, 64, 10, SDV.ink);
      if (stoneQuay) {
        c.rect(cx - 31, cy - 4, 62, 2, SDV.stoneHi);
      }
      for (let px = cx - 28; px <= cx + 28; px += 11) {
        c.rect(px, cy + 5, 3, 7, stoneQuay ? SDV.stoneDark : SDV.woodDark);
        c.rect(px - 1, cy + 11, 5, 1, SDV.waterFoam);
      }

      drawTimberWall(
        c,
        cx - 29,
        cy - 27,
        34,
        22,
        stoneQuay ? SDV.terraBase : SDV.woodMid,
        stoneQuay ? SDV.terraDark : SDV.woodDark,
        stoneQuay ? SDV.stoneHi : SDV.woodDark,
      );
      drawDoor(c, cx - 12, cy - 5, 10, 13);
      drawWindow(c, cx - 24, cy - 21, 6, 6, true);
      c.roofTrapezoid(cx - 12, cy - 39, 22, 40, 13, {
        base: SDV.slateBase,
        hi: SDV.slateHi,
        lo: SDV.slateDark,
        border: SDV.ink,
      });

      // Cargo crane & shipping crates on right dock
      c.rect(cx + 10, cy - 28, 3, 23, stoneQuay ? SDV.stoneDark : SDV.woodDark);
      c.rect(cx + 10, cy - 28, 14, 2, stoneQuay ? SDV.goldBase : SDV.woodLight);
      c.rect(cx + 21, cy - 26, 1, 7, SDV.plasterShade);
      c.bevelBox(cx + 18, cy - 19, 7, 7, SDV.woodLight, SDV.ink, SDV.woodHi, SDV.woodMid);

      // Moored sailboat silhouette on right edge
      c.ellipse(cx + 25, cy + 7, 13, 4, SDV.ink);
      c.ellipse(cx + 25, cy + 6, 12, 3, stoneQuay ? SDV.slateDark : SDV.woodBase);
      c.rect(cx + 24, cy - 15, 2, 20, SDV.woodDark);
      c.rect(cx + 15, cy - 13, 9, 12, SDV.plasterHi);
      c.strokeRect(cx + 15, cy - 13, 9, 12, SDV.ink);
      c.rect(cx + 17, cy - 8, 6, 2, SDV.goldBase);
      topY = cy - 41;
      break;
    }

    case "metropolis_tower": {
      drawGroundShadow(c, cx, cy + 3, 32, 12);
      drawTimberWall(c, cx - 22, cy - 28, 44, 28, SDV.stoneBase, SDV.stoneMid, SDV.stoneDark);
      drawDoor(c, cx, cy, 11, 15);
      drawWindow(c, cx - 16, cy - 20, 6, 8, true);
      drawWindow(c, cx + 10, cy - 20, 6, 8, true);

      c.rect(cx - 24, cy - 30, 48, 3, SDV.goldBase);
      c.strokeRect(cx - 24, cy - 30, 48, 3, SDV.ink);
      drawTimberWall(c, cx - 17, cy - 54, 34, 24, SDV.stoneHi, SDV.stoneBase, SDV.stoneMid);
      drawWindow(c, cx - 12, cy - 47, 6, 8, true);
      drawWindow(c, cx + 6, cy - 47, 6, 8, true);

      drawTimberWall(c, cx - 12, cy - 71, 24, 17, SDV.slateBase, SDV.slateDark, SDV.stoneHi);
      c.ellipse(cx, cy - 63, 5, 5, SDV.ink);
      c.ellipse(cx, cy - 63, 4, 4, SDV.goldHi);
      c.px(cx, cy - 63, SDV.ink);
      c.px(cx, cy - 65, SDV.ink);
      c.px(cx + 2, cy - 63, SDV.ink);

      c.roofTrapezoid(cx, cy - 84, 4, 28, 13, stage === "metropolis"
        ? {
            base: SDV.goldBase,
            hi: SDV.goldHi,
            lo: SDV.goldDark,
            border: SDV.ink,
          }
        : {
            base: SDV.tealBase,
            hi: SDV.tealHi,
            lo: SDV.tealDark,
            border: SDV.ink,
          });
      c.rect(cx - 1, cy - 90, 2, 6, SDV.goldBase);
      c.rect(cx - 2, cy - 91, 4, 2, stage === "metropolis" ? "#ffffff" : SDV.goldHi);
      topY = cy - 92;
      break;
    }
  }

  return { c, topY };
}

const slotBadgeOffsetCache = new Map<string, number>();

export function getSlotSpriteAsset(
  cache: PixelTextureCache,
  spec: MapSlotSpec,
  stateKind: SlotRenderStateKind,
  level = 1,
  stage: CityStageKey = "kampung",
): SlotSpriteAsset {
  const levelBucket =
    spec.id === "granary" && stateKind === "built" ? Math.min(3, Math.max(1, level)) : 1;
  const era = toEraTier(stage);
  const metroKey = spec.id === "metropolis_tower" && stage === "metropolis" ? "metro" : era;
  const key = `slot:${spec.id}:${stateKind}:L${levelBucket}:${metroKey}`;

  const texture = cache.get(key, () => {
    const built =
      stateKind === "locked"
        ? drawLockedPlot(spec)
        : stateKind === "unbuilt"
          ? drawUnbuiltPlot(spec)
          : stateKind === "building"
            ? drawBuildingPlot(spec)
            : drawBuiltSlot(spec, levelBucket, era, stage);

    const badgeOffsetY = (built.topY - BOX.originY) * 2 - 14;
    slotBadgeOffsetCache.set(key, badgeOffsetY);
    return built.c.toTexture(false);
  });

  return {
    texture,
    anchorX: BOX.originX / BOX.w,
    anchorY: BOX.originY / BOX.h,
    badgeOffsetY: slotBadgeOffsetCache.get(key) ?? -70,
  };
}

/** Tiny crisp 16-bit padlock icon for locked badges */
export function getPadlockIconTexture(cache: PixelTextureCache): Texture {
  return cache.get("icon:padlock", () => {
    const c = new PixelCanvas(9, 10);
    c.stamp(
      0,
      0,
      [
        "..kkkkk..",
        ".ksssssk.",
        ".ks...sk.",
        "kgggggggk",
        "kgGGGGggk",
        "kggkggggk",
        "kggkggggk",
        "kgggggggk",
        ".kkkkkkk.",
      ],
      {
        k: SDV.ink,
        s: SDV.stoneHi,
        g: SDV.goldBase,
        G: SDV.goldHi,
      },
    );
    return c.toTexture(false);
  });
}

/** Cozy 16-bit bobbing activity/status micro-icons (12x12) */
export function getStatusIconTexture(
  cache: PixelTextureCache,
  kind: "ready" | "pickaxe" | "axe" | "sprout" | "fish" | "hammer",
): Texture {
  return cache.get(`icon:status:${kind}`, () => {
    const c = new PixelCanvas(12, 12);
    const pal: Record<string, string> = {
      k: SDV.ink,
      g: SDV.goldBase,
      G: SDV.goldHi,
      w: SDV.plasterHi,
      b: SDV.woodBase,
      d: SDV.woodDark,
      s: SDV.stoneHi,
      m: SDV.stoneMid,
      p: SDV.pineHi,
      P: SDV.grassBright,
      t: SDV.tealHi,
      T: SDV.tealBase,
      c: SDV.oreCyan,
    };

    if (kind === "ready") {
      c.stamp(
        0,
        0,
        [
          ".....kk.....",
          "....kGGk....",
          "...kGwwGk...",
          ".kkGwwwwGkk.",
          "kGGwwwwwwGGk",
          "kggGGGGGGggk",
          ".kkGGGGGGkk.",
          "...kGggGk...",
          "....kggk....",
          ".....kk.....",
        ],
        pal,
      );
    } else if (kind === "pickaxe") {
      c.stamp(
        0,
        0,
        [
          "....kkkkk...",
          "..kkssssskk.",
          ".ksskkcckssk",
          ".kk..kbbk...",
          ".....kbbk...",
          "....kbbk....",
          "...kbbk.....",
          "..kbbk......",
          ".kbbk.......",
          "..kk........",
        ],
        pal,
      );
    } else if (kind === "axe") {
      c.stamp(
        0,
        0,
        [
          "...kkkkk....",
          "..ksssssk...",
          "..kssmmkk...",
          "...kkkbbk...",
          ".....kbbk...",
          "....kbbk....",
          "...kbbk.....",
          "..kbbk......",
          ".kbbk.......",
          "..kk........",
        ],
        pal,
      );
    } else if (kind === "sprout") {
      c.stamp(
        0,
        0,
        [
          "..kkk..kkk..",
          ".kPPPk.kPPk.",
          ".kPppk.kppk.",
          "..kkkpkpkk..",
          "....kpk.....",
          "....kpk.....",
          "..kkkdkkk...",
          ".kbbbbbbbk..",
          "..kkkkkkk...",
        ],
        pal,
      );
    } else if (kind === "fish") {
      c.stamp(
        0,
        0,
        [
          "............",
          "....kkkkk...",
          "..kktTTttk..",
          "kktTTwwwwtk.",
          "kTtkTTwkkwk.",
          "kttTTTTTTtk.",
          "..kkTTTTkk..",
          "....kkkk....",
        ],
        pal,
      );
    } else {
      c.stamp(
        0,
        0,
        [
          "..kkkkkkk...",
          ".ksssmmmmk..",
          ".ksssmmmmk..",
          "..kkkbbkk...",
          "....kbbk....",
          "....kbbk....",
          "....kbbk....",
          "....kbbk....",
          "....kkkk....",
        ],
        pal,
      );
    }
    return c.toTexture(false);
  });
}
