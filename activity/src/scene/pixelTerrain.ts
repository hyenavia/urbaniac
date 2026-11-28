import type { Texture } from "pixi.js";
import type { CityStageKey } from "../api/types";
import { MAP_SLOTS, WORLD_HEIGHT, WORLD_WIDTH } from "./assetRegistry";
import {
  PIXEL_SCALE,
  PixelCanvas,
  PixelTextureCache,
  SDV,
  createRng,
} from "./pixelArt";

export const CORE_TERRAIN_OFFSET_X = -160;
export const CORE_TERRAIN_OFFSET_Y = -120;
export const CORE_TERRAIN_W = 720; // 1440 world px
export const CORE_TERRAIN_H = 480; // 960 world px
export const RIVER_TILE_LEFT_WORLD_X = 760;

const STAGE_ORDER: CityStageKey[] = [
  "kampung",
  "desa",
  "kota_kecil",
  "kota_dagang",
  "metropolis",
];

function stageAtLeast(current: CityStageKey, min: CityStageKey): boolean {
  return STAGE_ORDER.indexOf(current) >= STAGE_ORDER.indexOf(min);
}

function toCoreX(worldX: number): number {
  return Math.round((worldX - CORE_TERRAIN_OFFSET_X) / PIXEL_SCALE);
}

function toCoreY(worldY: number): number {
  return Math.round((worldY - CORE_TERRAIN_OFFSET_Y) / PIXEL_SCALE);
}

/** Shoreline world X coordinate as a smooth organic curve from north to south */
function shorelineWorldX(worldY: number, stage: CityStageKey): number {
  const edgeFade = Math.min(
    1,
    Math.max(0, (worldY - CORE_TERRAIN_OFFSET_Y) / 90),
    Math.max(0, (CORE_TERRAIN_OFFSET_Y + CORE_TERRAIN_H * PIXEL_SCALE - worldY) / 90),
  );
  // Urban stages straighten the quay slightly while keeping gentle organic contour
  const amp = stageAtLeast(stage, "kota_dagang") ? 0.45 : 1;
  const wave1 = Math.sin(worldY * 0.009) * 18 * amp;
  const wave2 = Math.cos(worldY * 0.021 + 0.8) * 8 * amp;
  const harborCurve = worldY > 470 ? -((worldY - 470) * 0.04 * amp) : 0;
  return 852 + (wave1 + wave2 + harborCurve) * edgeFade;
}

/** Seamless 64x64 native grass tile for the full-bleed meadow TilingSprite */
export function getGrassTileTexture(
  cache: PixelTextureCache,
  stage: CityStageKey = "kampung",
): Texture {
  const urban = stage === "metropolis";
  return cache.get(`terrain:grass_tile:${urban ? "metro" : "meadow"}`, () => {
    const c = new PixelCanvas(64, 64);
    const rng = createRng(1042);

    c.rect(0, 0, 64, 64, urban ? "#5d9241" : SDV.grassBase);

    for (let y = 0; y < 64; y += 2) {
      for (let x = 0; x < 64; x += 2) {
        const n = Math.sin(x * 0.19) + Math.cos(y * 0.19);
        if (n > 0.75) {
          c.px(x, y, urban ? "#70a64c" : SDV.grassLight);
        } else if (n < -0.9) {
          c.px(x, y, urban ? "#4b7b32" : SDV.grassDark);
        }
      }
    }

    for (let i = 0; i < (urban ? 24 : 36); i++) {
      const gx = Math.floor(rng() * 60) + 2;
      const gy = Math.floor(rng() * 60) + 2;
      const tone = rng() > 0.45 ? SDV.grassLight : SDV.grassDark;
      c.px(gx, gy, tone);
      c.px(gx + 1, gy - 1, rng() > 0.5 ? SDV.grassBright : tone);
      c.px(gx + 2, gy, tone);
    }

    if (!urban) {
      const flowers = [SDV.flowerYellow, SDV.flowerWhite, SDV.flowerPink];
      for (let i = 0; i < 7; i++) {
        const fx = Math.floor(rng() * 60) + 2;
        const fy = Math.floor(rng() * 60) + 2;
        c.px(fx, fy + 1, SDV.grassDeep);
        c.px(fx, fy, flowers[i % flowers.length]);
      }
    }

    return c.toTexture(true);
  });
}

/** Seamless 64x64 animated river/harbor tile (2 calm pixel-shimmer frames) */
export function getWaterTileTexture(
  cache: PixelTextureCache,
  frame: 0 | 1,
): Texture {
  return cache.get(`terrain:water_tile:${frame}`, () => {
    const c = new PixelCanvas(64, 64);
    const rng = createRng(2088);

    c.rect(0, 0, 64, 64, SDV.waterMid);

    for (let y = 0; y < 64; y += 4) {
      for (let x = 0; x < 64; x += 2) {
        if ((x + y) % 8 === 0) {
          c.rect(x, y, 2, 1, SDV.waterDeep);
        }
      }
    }

    const shift = frame === 0 ? 0 : 4;
    for (let i = 0; i < 24; i++) {
      const rx = (Math.floor(rng() * 60) + shift) % 60;
      const ry = Math.floor(rng() * 62) + 1;
      const len = 3 + Math.floor(rng() * 4);
      c.rect(rx, ry, len, 1, SDV.waterLight);
      if (i % 4 === 0) {
        c.rect((rx + 1) % 62, ry, 2, 1, SDV.waterFoam);
      }
    }

    return c.toTexture(true);
  });
}

/** Seamless 16x64 shoreline bank tile for vertical bleed beyond core terrain */
export function getRiverBankTileTexture(
  cache: PixelTextureCache,
  stage: CityStageKey,
): Texture {
  const quay = stageAtLeast(stage, "kota_kecil");
  return cache.get(`terrain:river_bank_tile:${quay ? "quay" : "beach"}`, () => {
    const c = new PixelCanvas(16, 64);
    for (let y = 0; y < 64; y++) {
      if (quay) {
        c.rect(0, y, 4, 1, stage === "metropolis" ? "#6d7885" : SDV.grassBase);
        c.rect(4, y, 6, 1, SDV.stoneHi);
        c.rect(10, y, 2, 1, SDV.stoneDark);
        c.px(12, y, SDV.waterFoam);
        if (y % 8 === 0) {
          c.rect(4, y, 6, 1, SDV.stoneMid);
          c.rect(8, y, 2, 2, SDV.woodDark);
        }
      } else {
        c.rect(0, y, 5, 1, SDV.grassBase);
        c.rect(5, y, 3, 1, SDV.grassDeep);
        c.rect(8, y, 4, 1, SDV.sandBase);
        c.px(12, y, SDV.sandDark);
        c.px(13, y, SDV.waterFoam);
        if (y % 5 === 0) c.px(6, y, SDV.grassBright);
      }
    }
    return c.toTexture(true);
  });
}

function drawLampPost(
  c: PixelCanvas,
  cx: number,
  cy: number,
  ironStyle: boolean,
): void {
  const postCol = ironStyle ? SDV.stoneDeep : SDV.woodDark;
  const trimCol = ironStyle ? SDV.stoneHi : SDV.woodMid;
  c.rect(cx - 1, cy - 9, 2, 10, postCol);
  c.px(cx, cy - 9, trimCol);
  c.rect(cx - 2, cy - 12, 4, 3, SDV.stoneDark);
  c.rect(cx - 1, cy - 11, 2, 2, SDV.winGlow);
  c.rect(cx - 1, cy - 13, 2, 1, ironStyle ? SDV.goldBase : SDV.stoneMid);
}

function drawPlaza(
  c: PixelCanvas,
  cx: number,
  cy: number,
  rx: number,
  ry: number,
  stage: CityStageKey,
): void {
  const rng = createRng(3311);

  if (stage === "metropolis") {
    // Grand Marble & Gold Metropolitan Fountain Square
    c.ellipse(cx, cy, rx + 4, ry + 3, SDV.inkSoft);
    c.ellipse(cx, cy, rx + 2, ry + 1, SDV.stoneDark);
    c.ellipse(cx, cy, rx, ry, "#b8c1ca");
    for (let dy = -ry + 2; dy <= ry - 2; dy++) {
      for (let dx = -rx + 2; dx <= rx - 2; dx++) {
        const norm = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry);
        if (norm > 0.94) continue;
        const bx = cx + dx;
        const by = cy + dy;
        const checker = (Math.floor((dx + rx) / 5) + Math.floor((dy + ry) / 4)) % 2 === 0;
        if ((dx + rx) % 5 === 0 || (dy + ry) % 4 === 0) {
          c.px(bx, by, "#8f9aa6");
        } else if (norm > 0.78) {
          c.px(bx, by, SDV.goldBase);
        } else {
          c.px(bx, by, checker ? "#dfe5ec" : "#c5cfd8");
        }
      }
    }
    // Centerpiece water fountain slightly below Town/Village Hall front
    const fy = cy + 12;
    c.ellipse(cx, fy, 16, 8, SDV.ink);
    c.ellipse(cx, fy, 15, 7, SDV.stoneHi);
    c.ellipse(cx, fy, 12, 5, SDV.waterMid);
    c.rect(cx - 6, fy - 1, 12, 1, SDV.waterFoam);
    c.rect(cx - 2, fy - 6, 4, 6, SDV.stoneHi);
    c.rect(cx - 3, fy - 7, 6, 2, SDV.goldHi);
    c.px(cx - 1, fy - 9, SDV.waterFoam);
    c.px(cx + 1, fy - 9, SDV.waterFoam);
    c.px(cx, fy - 10, "#ffffff");
    return;
  }

  if (stage === "kota_dagang" || stage === "kota_kecil") {
    const trade = stage === "kota_dagang";
    c.ellipse(cx, cy, rx + 3, ry + 2, SDV.stoneDark);
    c.ellipse(cx, cy, rx, ry, trade ? "#b89f78" : SDV.stoneMid);
    for (let dy = -ry + 2; dy <= ry - 2; dy++) {
      for (let dx = -rx + 2; dx <= rx - 2; dx++) {
        const norm = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry);
        if (norm > 0.92) continue;
        const bx = cx + dx;
        const by = cy + dy;
        const row = Math.floor((dy + ry) / 4);
        const offset = (row % 2) * 3;
        if ((dy + ry) % 4 === 0 || (dx + rx + offset) % 6 === 0) {
          c.px(bx, by, trade ? "#8c724d" : SDV.stoneDark);
        } else if (trade && norm > 0.76) {
          c.px(bx, by, SDV.terraHi);
        } else if ((bx + by) % 7 === 0 || rng() > 0.85) {
          c.px(bx, by, trade ? SDV.parchmentBase : SDV.stoneHi);
        } else {
          c.px(bx, by, trade ? "#d8be90" : SDV.stoneBase);
        }
      }
    }
    c.ellipse(cx, cy, 11, 6, SDV.dirtDark);
    c.ellipse(cx, cy, 9, 4, SDV.goldBase);
    c.px(cx, cy, SDV.plasterHi);
    return;
  }

  // Kampung / Desa rustic dirt plaza
  c.ellipse(cx, cy, rx + 2, ry + 2, SDV.dirtEdge);
  c.ellipse(cx, cy, rx + 1, ry + 1, SDV.dirtDark);
  c.ellipse(cx, cy, rx - 1, ry - 1, SDV.dirtBase);
  for (let i = 0; i < 45; i++) {
    const a = rng() * Math.PI * 2;
    const r = Math.sqrt(rng()) * 0.86;
    const px = Math.round(cx + Math.cos(a) * rx * r);
    const py = Math.round(cy + Math.sin(a) * ry * r);
    c.px(px, py, rng() > 0.4 ? SDV.dirtLight : SDV.dirtDark);
  }
}

/** Draw natural stepped rocky quarry terrace (no hard rectangle edges) */
function drawQuarryTerrace(out: PixelCanvas, stage: CityStageKey): void {
  const rng = createRng(5521);
  const qLeft = toCoreX(-40);
  const qTop = toCoreY(-55);
  const qRight = toCoreX(395);
  const qBot = toCoreY(205);
  const urbanQuarry = stageAtLeast(stage, "kota_kecil");

  // Irregular rock plateau silhouette
  const cx = toCoreX(195);
  const cy = toCoreY(95);
  out.ellipse(cx, cy + 10, 145, 76, SDV.stoneDeep);
  out.ellipse(cx - 10, cy + 4, 142, 72, SDV.stoneDark);
  out.ellipse(cx - 12, cy - 2, 138, 66, SDV.stoneMid);
  out.ellipse(cx - 16, cy - 8, 126, 56, SDV.stoneBase);

  // Fill NW corner seamlessly for wide camera angles
  out.rect(qLeft, qTop, 170, 96, SDV.stoneBase);

  // Southern cliff drop face along curved bottom
  for (let x = qLeft + 10; x < qRight - 14; x += 6) {
    const curveY = qBot - 14 - Math.round(Math.max(0, x - (cx + 10)) * 0.28);
    if (curveY < qTop + 30) continue;
    out.rect(x, curveY, 7, 13, SDV.stoneDeep);
    out.rect(x + 1, curveY + 1, 5, 9, urbanQuarry ? SDV.stoneMid : SDV.stoneDark);
    out.rect(x, curveY - 2, 6, 3, SDV.stoneHi);
    if (urbanQuarry && x % 12 === 0) {
      out.rect(x + 2, curveY + 4, 3, 6, "#2c3440");
    }
  }

  // Sunlit gravel apron right around Mine entrance
  out.ellipse(toCoreX(245), toCoreY(155), 54, 24, SDV.stoneHi);
  out.ellipse(toCoreX(245), toCoreY(158), 42, 18, SDV.dirtDark);

  // Scattered boulders & glinting ore specks
  for (let i = 0; i < 36; i++) {
    const bx = qLeft + 18 + Math.floor(rng() * (qRight - qLeft - 45));
    const by = qTop + 14 + Math.floor(rng() * (qBot - qTop - 42));
    out.rect(bx, by, 3, 2, SDV.stoneDark);
    out.px(bx, by, SDV.stoneHi);
    if (i % 6 === 0) {
      out.px(bx + 1, by, i % 12 === 0 ? SDV.oreGold : SDV.oreCyan);
    }
  }
}

/**
 * Baked core 3/4 Stardew terrain layer with 5 distinct stage evolutions:
 * Kampung -> Desa -> Kota Kecil -> Kota Dagang -> Metropolis.
 */
export function getCoreTerrainTexture(
  cache: PixelTextureCache,
  stage: CityStageKey,
): Texture {
  return cache.get(`terrain:core:${stage}`, () => {
    const rng = createRng(9107);
    const out = new PixelCanvas(CORE_TERRAIN_W, CORE_TERRAIN_H);

    // 1. Base ground zones by stage
    if (stage === "metropolis") {
      // Grand Urban City District (paved granite/slate city blocks)
      const cityX1 = toCoreX(295);
      const cityY1 = toCoreY(25);
      const cityX2 = toCoreX(856);
      const cityY2 = toCoreY(675);
      out.rect(cityX1, cityY1, cityX2 - cityX1, cityY2 - cityY1, "#7b8490");
      // Granite sidewalk tiles grid across metropolis blocks
      for (let py = cityY1 + 2; py < cityY2 - 2; py += 6) {
        out.rect(cityX1, py, cityX2 - cityX1, 1, "#666f7a");
      }
      for (let px = cityX1 + 2; px < cityX2 - 2; px += 8) {
        out.rect(px, cityY1, 1, cityY2 - cityY1, "#6b7480");
      }
      // Stone perimeter curb around city blocks
      out.strokeRect(cityX1, cityY1, cityX2 - cityX1, cityY2 - cityY1, SDV.stoneDark);
      out.strokeRect(cityX1 + 1, cityY1 + 1, cityX2 - cityX1 - 2, cityY2 - cityY1 - 2, SDV.stoneHi);

      // Preserve green Botanical & Agricultural Park in SW (Forest & Farm)
      out.bevelBox(
        toCoreX(75),
        toCoreY(245),
        toCoreX(365) - toCoreX(75),
        toCoreY(640) - toCoreY(245),
        SDV.grassBase,
        SDV.stoneDark,
        SDV.stoneHi,
        SDV.stoneMid,
      );
      out.ellipse(toCoreX(245), toCoreY(545), 76, 48, SDV.grassLight);
      out.ellipse(toCoreX(215), toCoreY(355), 68, 48, SDV.grassDark);
    } else if (stage === "kota_dagang") {
      // Trade City: paved warm cobblestone commercial heart on east & center
      out.ellipse(toCoreX(645), toCoreY(385), 145, 130, "#a89270");
      out.ellipse(toCoreX(645), toCoreY(385), 142, 126, "#c2ab83");
      for (let py = toCoreY(210); py < toCoreY(600); py += 5) {
        out.rect(toCoreX(450), py, toCoreX(850) - toCoreX(450), 1, "#9a825f");
      }
      out.ellipse(toCoreX(245), toCoreY(545), 96, 58, SDV.grassLight);
      out.ellipse(toCoreX(215), toCoreY(355), 86, 60, SDV.grassDark);
    } else {
      // Kampung / Desa / Kota Kecil: lush sunlit meadow clearings
      out.ellipse(toCoreX(560), toCoreY(350), 165, 108, SDV.grassLight);
      out.ellipse(toCoreX(245), toCoreY(545), 96, 58, SDV.grassLight);
      out.ellipse(toCoreX(215), toCoreY(355), 88, 62, SDV.grassDark);
    }

    // Wildflowers in natural grass areas
    const flowerColors = [SDV.flowerYellow, SDV.flowerWhite, SDV.flowerPink, SDV.flowerBlue];
    const flowerCount = stage === "metropolis" ? 65 : stage === "kota_dagang" ? 110 : 220;
    for (let i = 0; i < flowerCount; i++) {
      const wx = 40 + rng() * (stage === "metropolis" ? 280 : WORLD_WIDTH - 240);
      const wy = 240 + rng() * (WORLD_HEIGHT - 280);
      const cx = toCoreX(wx);
      const cy = toCoreY(wy);
      if (i % 4 === 0) {
        out.px(cx, cy + 1, SDV.grassDeep);
        out.px(cx, cy, flowerColors[i % flowerColors.length]);
      } else {
        out.px(cx, cy, i % 2 === 0 ? SDV.grassBright : SDV.grassDark);
      }
    }

    // 2. Northern Rocky Quarry Terrace
    drawQuarryTerrace(out, stage);

    // 3. Connecting Roads evolved by stage
    const plazaX = 560;
    const plazaY = 360;
    const roads: Array<[number, number, number, number]> = [
      [plazaX, plazaY, 245, 555],
      [plazaX, plazaY, 215, 355],
      [plazaX, plazaY, 245, 155],
      [plazaX, plazaY, 410, 445],
      [plazaX, plazaY, 425, 255],
      [plazaX, plazaY, 560, 165],
      [plazaX, plazaY, 725, 390],
      [plazaX, plazaY, 735, 235],
      [725, 390, 755, 555],
      [725, 390, 880, 410],
      [755, 555, 885, 575],
    ];

    const isMetro = stage === "metropolis";
    const isTrade = stage === "kota_dagang";
    const isTown = stage === "kota_kecil";

    if (isMetro) {
      // Grand dark-slate urban boulevards + granite sidewalks + crosswalk dashes
      for (const [x1, y1, x2, y2] of roads) {
        out.line(toCoreX(x1), toCoreY(y1), toCoreX(x2), toCoreY(y2), SDV.stoneDark, 13);
      }
      for (const [x1, y1, x2, y2] of roads) {
        out.line(toCoreX(x1), toCoreY(y1), toCoreX(x2), toCoreY(y2), SDV.stoneHi, 11);
        out.line(toCoreX(x1), toCoreY(y1), toCoreX(x2), toCoreY(y2), "#47515e", 7);
      }
      // Dashed center road lines
      for (const [x1, y1, x2, y2] of roads) {
        for (let s = 1; s < 9; s += 2) {
          const t = s / 10;
          const px = toCoreX(x1 + (x2 - x1) * t);
          const py = toCoreY(y1 + (y2 - y1) * t);
          out.rect(px, py, 2, 1, SDV.goldHi);
        }
      }
      // Extra straight North-South & East-West Metro Avenue stripes
      out.rect(toCoreX(544), toCoreY(35), 16, toCoreY(670) - toCoreY(35), "#47515e");
      out.rect(toCoreX(542), toCoreY(35), 2, toCoreY(670) - toCoreY(35), SDV.stoneHi);
      out.rect(toCoreX(560), toCoreY(35), 2, toCoreY(670) - toCoreY(35), SDV.stoneHi);
    } else if (isTrade || isTown) {
      // Cobblestone paved roads with stone curbs
      for (const [x1, y1, x2, y2] of roads) {
        out.line(toCoreX(x1), toCoreY(y1), toCoreX(x2), toCoreY(y2), SDV.stoneDeep, 10);
      }
      for (const [x1, y1, x2, y2] of roads) {
        out.line(toCoreX(x1), toCoreY(y1), toCoreX(x2), toCoreY(y2), isTrade ? "#b9a17b" : SDV.stoneBase, 8);
        out.line(toCoreX(x1), toCoreY(y1) - 1, toCoreX(x2), toCoreY(y2) - 1, isTrade ? SDV.parchmentShade : SDV.stoneHi, 3);
      }
    } else {
      // Kampung / Desa rustic dirt trails
      const width = stage === "desa" ? 8 : 6;
      for (const [x1, y1, x2, y2] of roads) {
        out.line(toCoreX(x1), toCoreY(y1), toCoreX(x2), toCoreY(y2), SDV.dirtEdge, width + 2);
      }
      for (const [x1, y1, x2, y2] of roads) {
        out.line(toCoreX(x1), toCoreY(y1), toCoreX(x2), toCoreY(y2), SDV.dirtBase, width);
        out.line(toCoreX(x1), toCoreY(y1) - 1, toCoreX(x2), toCoreY(y2) - 1, SDV.dirtLight, Math.max(2, width - 4));
      }
    }

    // Ground pads under slots
    for (const slot of MAP_SLOTS) {
      if (slot.id === "fishing" || slot.id === "trade_port") continue;
      const sx = toCoreX(slot.x);
      const sy = toCoreY(slot.y + 6);
      const rx = Math.round(slot.width / 4) + 3;
      const ry = Math.round(slot.depth / 4) + 2;
      if (isMetro && slot.id !== "farming" && slot.id !== "gathering") {
        out.ellipse(sx, sy, rx + 2, ry + 1, SDV.stoneDark);
        out.ellipse(sx, sy, rx, ry - 1, "#9aa4b0");
      } else if (isTrade || isTown) {
        out.ellipse(sx, sy, rx, ry, SDV.stoneDark);
        out.ellipse(sx, sy, rx - 1, ry - 1, isTrade ? "#c4ab83" : SDV.stoneBase);
      } else {
        out.ellipse(sx, sy, rx, ry, SDV.dirtDark);
        out.ellipse(sx, sy, rx - 2, ry - 1, SDV.dirtBase);
      }
    }

    // 4. Central Town Plaza
    const plazaRx = isMetro ? 74 : isTrade ? 68 : isTown ? 62 : stage === "desa" ? 54 : 46;
    const plazaRy = isMetro ? 45 : isTrade ? 41 : isTown ? 38 : stage === "desa" ? 33 : 28;
    drawPlaza(out, toCoreX(plazaX), toCoreY(plazaY), plazaRx, plazaRy, stage);

    // Streetlamps around plaza
    const ironLamps = stageAtLeast(stage, "kota_kecil");
    const lampDist = plazaRx - 5;
    drawLampPost(out, toCoreX(plazaX) - lampDist, toCoreY(plazaY) - 14, ironLamps);
    drawLampPost(out, toCoreX(plazaX) + lampDist, toCoreY(plazaY) - 14, ironLamps);
    drawLampPost(out, toCoreX(plazaX) - 26, toCoreY(plazaY) + plazaRy - 4, ironLamps);
    drawLampPost(out, toCoreX(plazaX) + 26, toCoreY(plazaY) + plazaRy - 4, ironLamps);

    if (isMetro) {
      drawLampPost(out, toCoreX(495), toCoreY(255), true);
      drawLampPost(out, toCoreX(640), toCoreY(255), true);
      drawLampPost(out, toCoreX(495), toCoreY(485), true);
      drawLampPost(out, toCoreX(640), toCoreY(485), true);
    }

    // Farm split-rail wooden fence
    const farmX = toCoreX(245);
    const farmY = toCoreY(555);
    for (let fx = farmX - 48; fx <= farmX + 46; fx += 8) {
      out.rect(fx, farmY + 24, 2, 5, SDV.woodDark);
      out.px(fx, farmY + 23, SDV.woodLight);
      if (fx + 8 <= farmX + 46) {
        out.rect(fx + 2, farmY + 25, 6, 1, SDV.woodBase);
        out.rect(fx + 2, farmY + 27, 6, 1, SDV.woodMid);
      }
    }

    // 5. Eastern River Shoreline / Stone Quay / Metropolis Promenade
    for (let cy = 0; cy < CORE_TERRAIN_H; cy++) {
      const worldY = CORE_TERRAIN_OFFSET_Y + cy * PIXEL_SCALE;
      const shoreCx = toCoreX(shorelineWorldX(worldY, stage));
      if (shoreCx < CORE_TERRAIN_W) {
        out.clearRect(shoreCx, cy, CORE_TERRAIN_W - shoreCx, 1);
      }

      if (isMetro) {
        // Grand stone waterfront promenade + balustrade rail
        out.rect(shoreCx - 12, cy, 10, 1, "#c5cfd8");
        out.rect(shoreCx - 12, cy, 1, 1, SDV.stoneDark);
        out.rect(shoreCx - 2, cy, 2, 1, SDV.stoneDark);
        out.px(shoreCx, cy, SDV.waterFoam);
        if (cy % 6 === 0) {
          out.rect(shoreCx - 12, cy, 10, 1, "#9aa4b0");
          out.rect(shoreCx - 3, cy, 2, 2, SDV.stoneHi);
        }
        if (cy % 24 === 0) {
          out.rect(shoreCx - 4, cy - 2, 2, 3, SDV.goldBase);
        }
      } else if (isTrade || isTown) {
        // Dressed stone trade quay & wooden bollards
        out.rect(shoreCx - 8, cy, 6, 1, SDV.stoneHi);
        out.rect(shoreCx - 2, cy, 2, 1, SDV.stoneDark);
        out.px(shoreCx, cy, SDV.waterFoam);
        if (cy % 8 === 0) {
          out.rect(shoreCx - 8, cy, 6, 1, SDV.stoneMid);
          out.rect(shoreCx - 3, cy, 2, 2, SDV.woodDark);
        }
      } else {
        // Kampung / Desa sandy beach & reeds
        out.rect(shoreCx - 7, cy, 3, 1, SDV.grassDeep);
        out.rect(shoreCx - 4, cy, 3, 1, SDV.sandBase);
        out.px(shoreCx - 1, cy, SDV.sandDark);
        out.px(shoreCx, cy, SDV.waterFoam);
        if (cy % 5 === 0) out.px(shoreCx - 5, cy, SDV.grassBright);
        if (cy % 11 === 0) {
          out.px(shoreCx - 2, cy, SDV.woodDark);
          out.px(shoreCx - 2, cy - 1, SDV.pineHi);
        }
      }
    }

    return out.toTexture(false);
  });
}

export type PropKind =
  | "oak"
  | "pine"
  | "planter_tree"
  | "cottage"
  | "townhouse"
  | "merchant_house"
  | "metro_midrise"
  | "metro_highrise"
  | "sailboat"
  | "steamship";

export function getPropTextures(
  cache: PixelTextureCache,
): Record<PropKind, Texture> {
  const oak = cache.get("prop:oak", () => {
    const c = new PixelCanvas(28, 36);
    c.ellipse(14, 32, 9, 3, SDV.shadow);
    c.rect(11, 21, 6, 11, SDV.woodDark);
    c.rect(12, 22, 4, 10, SDV.woodMid);
    c.rect(12, 23, 2, 8, SDV.woodBase);
    c.ellipse(14, 14, 11, 10, SDV.inkSoft);
    c.ellipse(14, 14, 10, 9, SDV.grassDeep);
    c.ellipse(14, 12, 9, 8, SDV.pineBase);
    c.ellipse(11, 13, 5, 5, SDV.grassBase);
    c.ellipse(17, 13, 5, 5, SDV.grassBase);
    c.ellipse(12, 9, 6, 4, SDV.grassLight);
    c.ellipse(16, 10, 4, 3, SDV.grassLight);
    c.rect(10, 6, 4, 2, SDV.grassBright);
    return c.toTexture(false);
  });

  const pine = cache.get("prop:pine", () => {
    const c = new PixelCanvas(26, 38);
    c.ellipse(13, 34, 8, 3, SDV.shadow);
    c.rect(11, 26, 4, 9, SDV.woodDark);
    c.rect(12, 27, 2, 7, SDV.woodMid);
    const tiers = [
      { topY: 16, h: 12, botW: 22 },
      { topY: 9, h: 11, botW: 18 },
      { topY: 2, h: 10, botW: 12 },
    ];
    for (const tier of tiers) {
      for (let r = 0; r < tier.h; r++) {
        const t = r / Math.max(1, tier.h - 1);
        const w = Math.max(2, Math.round(2 + (tier.botW - 2) * t));
        const lx = 13 - Math.floor(w / 2);
        const y = tier.topY + r;
        c.rect(lx, y, w, 1, SDV.inkSoft);
        if (w > 2) {
          c.rect(lx + 1, y, w - 2, 1, r >= tier.h - 2 ? SDV.pineDark : SDV.pineBase);
          if (r > 0 && r < tier.h - 2) {
            c.rect(lx + 1, y, Math.max(1, Math.floor((w - 2) * 0.55)), 1, SDV.pineHi);
          }
        }
      }
    }
    return c.toTexture(false);
  });

  const planter_tree = cache.get("prop:planter_tree", () => {
    const c = new PixelCanvas(24, 34);
    c.ellipse(12, 30, 7, 3, SDV.shadow);
    c.bevelBox(6, 25, 12, 6, SDV.stoneBase, SDV.ink, SDV.stoneHi, SDV.stoneDark);
    c.rect(10, 17, 4, 9, SDV.woodMid);
    c.ellipse(12, 12, 8, 8, SDV.inkSoft);
    c.ellipse(12, 12, 7, 7, SDV.pineBase);
    c.ellipse(11, 10, 5, 4, SDV.grassLight);
    c.px(9, 8, SDV.grassBright);
    return c.toTexture(false);
  });

  const cottage = cache.get("prop:cottage", () => {
    const c = new PixelCanvas(32, 32);
    c.ellipse(16, 28, 13, 4, SDV.shadow);
    c.bevelBox(5, 16, 22, 12, SDV.plasterBase, SDV.ink, SDV.plasterHi, SDV.plasterShade);
    c.rect(13, 20, 6, 8, SDV.woodMid);
    c.strokeRect(13, 20, 6, 8, SDV.ink);
    c.rect(7, 19, 4, 4, SDV.winAmber);
    c.strokeRect(7, 19, 4, 4, SDV.ink);
    c.rect(21, 19, 4, 4, SDV.winAmber);
    c.strokeRect(21, 19, 4, 4, SDV.ink);
    c.roofTrapezoid(16, 7, 14, 26, 10, {
      base: SDV.terraBase,
      hi: SDV.terraHi,
      lo: SDV.terraDark,
      border: SDV.ink,
    });
    return c.toTexture(false);
  });

  const townhouse = cache.get("prop:townhouse", () => {
    const c = new PixelCanvas(34, 38);
    c.ellipse(17, 34, 14, 4, SDV.shadow);
    c.bevelBox(5, 15, 24, 19, SDV.plasterBase, SDV.ink, SDV.plasterHi, SDV.plasterShade);
    c.rect(6, 24, 22, 2, SDV.woodMid);
    c.rect(14, 26, 6, 8, SDV.woodDark);
    for (const wy of [17, 26]) {
      c.rect(8, wy, 4, 5, SDV.winGlow);
      c.strokeRect(8, wy, 4, 5, SDV.ink);
      c.rect(22, wy, 4, 5, SDV.winGlow);
      c.strokeRect(22, wy, 4, 5, SDV.ink);
    }
    c.roofTrapezoid(17, 6, 16, 28, 10, {
      base: SDV.slateBase,
      hi: SDV.slateHi,
      lo: SDV.slateDark,
      border: SDV.ink,
    });
    return c.toTexture(false);
  });

  const merchant_house = cache.get("prop:merchant_house", () => {
    const c = new PixelCanvas(38, 42);
    c.ellipse(19, 38, 16, 4, SDV.shadow);
    c.bevelBox(5, 14, 28, 24, "#e6c38d", SDV.ink, "#fae5bc", "#ba9057");
    c.rect(5, 31, 28, 7, SDV.stoneBase);
    c.strokeRect(5, 31, 28, 7, SDV.ink);
    c.rect(16, 29, 6, 9, SDV.woodDark);
    c.rect(7, 27, 24, 3, SDV.barnHi);
    for (const wx of [9, 17, 25]) {
      c.rect(wx, 18, 4, 5, SDV.winGlow);
      c.strokeRect(wx, 18, 4, 5, SDV.ink);
    }
    c.roofTrapezoid(19, 5, 20, 32, 10, {
      base: SDV.terraBase,
      hi: SDV.terraHi,
      lo: SDV.terraDark,
      border: SDV.ink,
    });
    return c.toTexture(false);
  });

  const metro_midrise = cache.get("prop:metro_midrise", () => {
    const c = new PixelCanvas(42, 48);
    c.ellipse(21, 44, 18, 4, SDV.shadow);
    c.bevelBox(5, 11, 32, 33, "#b96a4b", SDV.ink, "#d68a69", "#8c4830");
    c.rect(5, 37, 32, 7, SDV.stoneBase);
    c.strokeRect(5, 37, 32, 7, SDV.ink);
    c.rect(18, 36, 6, 8, SDV.woodDark);
    for (let row = 0; row < 3; row++) {
      const wy = 14 + row * 7;
      for (const wx of [9, 16, 23, 30]) {
        c.rect(wx, wy, 4, 4, (wx + row) % 2 === 0 ? SDV.winGlow : SDV.winAmber);
        c.strokeRect(wx, wy, 4, 4, SDV.ink);
      }
    }
    c.bevelBox(4, 7, 34, 5, SDV.stoneHi, SDV.ink, "#eef3f8", SDV.stoneMid);
    return c.toTexture(false);
  });

  const metro_highrise = cache.get("prop:metro_highrise", () => {
    const c = new PixelCanvas(42, 62);
    c.ellipse(21, 58, 18, 4, SDV.shadow);
    c.bevelBox(6, 12, 30, 46, SDV.stoneBase, SDV.ink, SDV.stoneHi, SDV.stoneDark);
    c.rect(5, 50, 32, 8, SDV.stoneMid);
    c.strokeRect(5, 50, 32, 8, SDV.ink);
    c.rect(17, 49, 8, 9, SDV.goldBase);
    c.strokeRect(17, 49, 8, 9, SDV.ink);
    for (let row = 0; row < 5; row++) {
      const wy = 16 + row * 7;
      for (const wx of [10, 17, 24, 30]) {
        c.rect(wx, wy, 4, 4, (row + wx) % 3 === 0 ? SDV.winAmber : SDV.winGlow);
        c.strokeRect(wx, wy, 4, 4, SDV.ink);
      }
    }
    // Art-deco crown & rooftop spire
    c.bevelBox(9, 7, 24, 6, SDV.slateBase, SDV.ink, SDV.slateHi, SDV.slateDark);
    c.rect(19, 1, 4, 6, SDV.goldBase);
    c.px(20, 0, SDV.goldHi);
    c.px(21, 0, SDV.goldHi);
    return c.toTexture(false);
  });

  const sailboat = cache.get("prop:sailboat", () => {
    const c = new PixelCanvas(28, 28);
    c.rect(5, 22, 18, 1, SDV.waterFoam);
    c.ellipse(14, 20, 10, 3, SDV.ink);
    c.ellipse(14, 19, 9, 2, SDV.woodBase);
    c.rect(13, 5, 2, 14, SDV.woodDark);
    c.rect(6, 7, 7, 10, SDV.plasterHi);
    c.strokeRect(6, 7, 7, 10, SDV.ink);
    c.rect(15, 9, 6, 8, SDV.parchmentBase);
    c.strokeRect(15, 9, 6, 8, SDV.ink);
    return c.toTexture(false);
  });

  const steamship = cache.get("prop:steamship", () => {
    const c = new PixelCanvas(42, 34);
    c.rect(4, 28, 34, 1, SDV.waterFoam);
    c.ellipse(21, 25, 16, 4, SDV.ink);
    c.ellipse(21, 24, 15, 3, SDV.slateDark);
    c.rect(9, 22, 24, 2, SDV.barnHi);
    // White cabin deck + smokestack
    c.bevelBox(12, 15, 18, 7, SDV.plasterHi, SDV.ink, "#ffffff", SDV.plasterShade);
    c.rect(14, 17, 3, 3, SDV.winGlow);
    c.rect(19, 17, 3, 3, SDV.winGlow);
    c.rect(24, 17, 3, 3, SDV.winGlow);
    c.rect(18, 7, 5, 8, SDV.barnBase);
    c.strokeRect(18, 7, 5, 8, SDV.ink);
    c.rect(18, 7, 5, 2, SDV.ink);
    c.ellipse(23, 4, 4, 2, "#eef3f8");
    return c.toTexture(false);
  });

  return {
    oak,
    pine,
    planter_tree,
    cottage,
    townhouse,
    merchant_house,
    metro_midrise,
    metro_highrise,
    sailboat,
    steamship,
  };
}

export interface StagePropSpot {
  x: number;
  y: number;
  kind: PropKind;
}

export function getStageProps(stage: CityStageKey): StagePropSpot[] {
  const forestTrees: StagePropSpot[] = [
    { x: 95, y: 295, kind: "pine" },
    { x: 125, y: 420, kind: "oak" },
    { x: 98, y: 490, kind: "pine" },
    { x: 318, y: 298, kind: "oak" },
    { x: 138, y: 630, kind: "oak" },
    { x: 45, y: 695, kind: "pine" },
  ];

  if (stage === "kampung") {
    return [
      ...forestTrees,
      { x: 452, y: 632, kind: "oak" },
      { x: -35, y: 345, kind: "pine" },
      { x: -65, y: 520, kind: "oak" },
      { x: 365, y: 715, kind: "oak" },
      { x: 625, y: 695, kind: "oak" },
      { x: 485, y: 112, kind: "pine" },
      { x: 645, y: 122, kind: "oak" },
      { x: 785, y: 125, kind: "pine" },
    ];
  }

  if (stage === "desa") {
    return [
      ...forestTrees,
      { x: 78, y: 268, kind: "cottage" },
      { x: 430, y: 652, kind: "cottage" },
      { x: 792, y: 112, kind: "cottage" },
      { x: 485, y: 112, kind: "pine" },
      { x: 645, y: 118, kind: "oak" },
      { x: 625, y: 690, kind: "oak" },
    ];
  }

  if (stage === "kota_kecil") {
    return [
      ...forestTrees,
      { x: 470, y: 92, kind: "townhouse" },
      { x: 655, y: 96, kind: "townhouse" },
      { x: 795, y: 115, kind: "townhouse" },
      { x: 430, y: 652, kind: "townhouse" },
      { x: 630, y: 658, kind: "townhouse" },
      { x: 78, y: 268, kind: "oak" },
      { x: 1035, y: 260, kind: "sailboat" },
    ];
  }

  if (stage === "kota_dagang") {
    return [
      ...forestTrees,
      { x: 468, y: 92, kind: "merchant_house" },
      { x: 655, y: 94, kind: "merchant_house" },
      { x: 792, y: 115, kind: "merchant_house" },
      { x: 348, y: 132, kind: "townhouse" },
      { x: 430, y: 652, kind: "merchant_house" },
      { x: 635, y: 656, kind: "merchant_house" },
      { x: 1032, y: 255, kind: "sailboat" },
      { x: 1042, y: 498, kind: "sailboat" },
    ];
  }

  // Metropolis: urban skyline towers + avenue planter trees + harbor ships
  return [
    { x: 95, y: 295, kind: "pine" },
    { x: 122, y: 420, kind: "oak" },
    { x: 98, y: 492, kind: "pine" },
    { x: 136, y: 626, kind: "oak" },
    { x: 348, y: 128, kind: "metro_midrise" },
    { x: 468, y: 92, kind: "metro_highrise" },
    { x: 655, y: 94, kind: "metro_highrise" },
    { x: 792, y: 114, kind: "metro_midrise" },
    { x: 74, y: 262, kind: "metro_midrise" },
    { x: 430, y: 656, kind: "metro_midrise" },
    { x: 542, y: 656, kind: "metro_highrise" },
    { x: 652, y: 660, kind: "metro_midrise" },
    { x: 486, y: 280, kind: "planter_tree" },
    { x: 642, y: 282, kind: "planter_tree" },
    { x: 482, y: 472, kind: "planter_tree" },
    { x: 642, y: 472, kind: "planter_tree" },
    { x: 1036, y: 255, kind: "sailboat" },
    { x: 1044, y: 502, kind: "steamship" },
  ];
}
