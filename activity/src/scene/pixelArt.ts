import { CanvasSource, Texture } from "pixi.js";

/** Native pixel art scale factor: 1 native pixel = 2x2 world units */
export const PIXEL_SCALE = 2;

/**
 * Cozy 16-bit Stardew-inspired palette with warm umber outlines,
 * sunlit meadow greens, rich oak timber, terracotta shingles, and cerulean water.
 */
export const SDV = {
  // Warm dark-umber outlines & shadows (avoid harsh pure black)
  ink: "#2d1a0e",
  inkSoft: "#472916",
  shadow: "rgba(38, 24, 14, 0.34)",
  shadowDark: "rgba(30, 18, 10, 0.48)",

  // Meadow & foliage
  grassBase: "#6ca646",
  grassLight: "#85bd54",
  grassBright: "#a2d467",
  grassDark: "#538736",
  grassDeep: "#3f6a29",
  moss: "#4f7c3b",

  // Dirt paths & tilled soil
  dirtBase: "#c79a61",
  dirtLight: "#dfb87e",
  dirtDark: "#9e7242",
  dirtEdge: "#77522c",
  soilDark: "#6a4224",
  soilMid: "#825431",

  // Quarry stone & slate
  stoneHi: "#b4c0cb",
  stoneBase: "#8896a6",
  stoneMid: "#677586",
  stoneDark: "#4a5666",
  stoneDeep: "#343e4c",

  // River & harbor water
  waterDeep: "#256488",
  waterMid: "#3685ab",
  waterLight: "#53adc4",
  waterFoam: "#b7eff6",
  sandBase: "#e4c98e",
  sandDark: "#c7a668",

  // Oak & timber
  woodDark: "#4d2b16",
  woodMid: "#7d4927",
  woodBase: "#a46638",
  woodLight: "#c88952",
  woodHi: "#e2b07a",

  // Plaster & parchment
  plasterHi: "#fff3d6",
  plasterBase: "#f2dcb1",
  plasterShade: "#d9b983",
  parchmentHi: "#fff4d9",
  parchmentBase: "#f5ddb0",
  parchmentShade: "#d8b47b",

  // Roofs (terracotta, red barn, pine, slate, gold)
  terraHi: "#ea8859",
  terraBase: "#cf623c",
  terraDark: "#9e4124",
  barnHi: "#dc5a44",
  barnBase: "#b53c2c",
  barnDark: "#7d251c",
  pineHi: "#6da94f",
  pineBase: "#4c8038",
  pineDark: "#335825",
  slateHi: "#7d9db8",
  slateBase: "#53728f",
  slateDark: "#364f69",
  goldHi: "#ffe680",
  goldBase: "#f5c242",
  goldDark: "#c2871e",
  tealHi: "#70c9d4",
  tealBase: "#419ba8",
  tealDark: "#296975",

  // Accents (windows, flowers, ores)
  winGlow: "#ffe688",
  winAmber: "#ffba47",
  winDark: "#382924",
  flowerPink: "#f28aa8",
  flowerYellow: "#ffe36e",
  flowerWhite: "#fff8e8",
  flowerBlue: "#7dc7ff",
  oreCyan: "#6fe5f7",
  oreGold: "#ffd24a",
} as const;

/** Deterministic seeded PRNG (mulberry32) so re-renders are pixel-identical */
export function createRng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Offscreen integer-grid pixel canvas builder */
export class PixelCanvas {
  readonly canvas: HTMLCanvasElement;
  readonly ctx: CanvasRenderingContext2D;
  readonly width: number;
  readonly height: number;

  constructor(width: number, height: number) {
    this.width = Math.max(1, Math.round(width));
    this.height = Math.max(1, Math.round(height));
    this.canvas = document.createElement("canvas");
    this.canvas.width = this.width;
    this.canvas.height = this.height;
    const ctx = this.canvas.getContext("2d");
    if (!ctx) {
      throw new Error("2D canvas context unavailable");
    }
    ctx.imageSmoothingEnabled = false;
    this.ctx = ctx;
  }

  px(x: number, y: number, color: string): this {
    this.ctx.fillStyle = color;
    this.ctx.fillRect(Math.round(x), Math.round(y), 1, 1);
    return this;
  }

  rect(x: number, y: number, w: number, h: number, color: string): this {
    const rw = Math.round(w);
    const rh = Math.round(h);
    if (rw <= 0 || rh <= 0) return this;
    this.ctx.fillStyle = color;
    this.ctx.fillRect(Math.round(x), Math.round(y), rw, rh);
    return this;
  }

  clearRect(x: number, y: number, w: number, h: number): this {
    const rw = Math.round(w);
    const rh = Math.round(h);
    if (rw <= 0 || rh <= 0) return this;
    this.ctx.clearRect(Math.round(x), Math.round(y), rw, rh);
    return this;
  }

  strokeRect(x: number, y: number, w: number, h: number, color: string): this {
    const rx = Math.round(x);
    const ry = Math.round(y);
    const rw = Math.round(w);
    const rh = Math.round(h);
    if (rw <= 0 || rh <= 0) return this;
    this.ctx.fillStyle = color;
    this.ctx.fillRect(rx, ry, rw, 1);
    this.ctx.fillRect(rx, ry + rh - 1, rw, 1);
    this.ctx.fillRect(rx, ry, 1, rh);
    this.ctx.fillRect(rx + rw - 1, ry, 1, rh);
    return this;
  }

  /** 16-bit beveled box with 1px outer border and inner highlight/shade edges */
  bevelBox(
    x: number,
    y: number,
    w: number,
    h: number,
    fill: string,
    border: string,
    hi: string,
    lo: string,
  ): this {
    const rx = Math.round(x);
    const ry = Math.round(y);
    const rw = Math.round(w);
    const rh = Math.round(h);
    if (rw < 4 || rh < 4) return this.rect(rx, ry, rw, rh, fill);
    this.rect(rx + 1, ry + 1, rw - 2, rh - 2, fill);
    this.strokeRect(rx, ry, rw, rh, border);
    this.rect(rx + 1, ry + 1, rw - 2, 1, hi);
    this.rect(rx + 1, ry + 1, 1, rh - 2, hi);
    this.rect(rx + 1, ry + rh - 2, rw - 2, 1, lo);
    this.rect(rx + rw - 2, ry + 1, 1, rh - 2, lo);
    return this;
  }

  /** Subtle checkerboard/stepped pixel dithering */
  ditherRect(
    x: number,
    y: number,
    w: number,
    h: number,
    c1: string,
    c2: string,
    step: 2 | 4 = 2,
  ): this {
    const rx = Math.round(x);
    const ry = Math.round(y);
    const rw = Math.round(w);
    const rh = Math.round(h);
    this.rect(rx, ry, rw, rh, c1);
    this.ctx.fillStyle = c2;
    for (let dy = 0; dy < rh; dy++) {
      for (let dx = 0; dx < rw; dx++) {
        if ((dx + dy) % step === 0 && (step === 2 || dx % 2 === dy % 2)) {
          this.ctx.fillRect(rx + dx, ry + dy, 1, 1);
        }
      }
    }
    return this;
  }

  /** Scanline integer pixel ellipse */
  ellipse(cx: number, cy: number, rx: number, ry: number, color: string): this {
    const icx = Math.round(cx);
    const icy = Math.round(cy);
    const irx = Math.max(1, Math.round(rx));
    const iry = Math.max(1, Math.round(ry));
    this.ctx.fillStyle = color;
    for (let dy = -iry; dy <= iry; dy++) {
      const ratio = dy / iry;
      const halfSpan = Math.floor(irx * Math.sqrt(Math.max(0, 1 - ratio * ratio)));
      this.ctx.fillRect(icx - halfSpan, icy + dy, halfSpan * 2 + 1, 1);
    }
    return this;
  }

  /** Bresenham integer line with optional square brush thickness */
  line(
    x0: number,
    y0: number,
    x1: number,
    y1: number,
    color: string,
    thickness = 1,
  ): this {
    let ix0 = Math.round(x0);
    let iy0 = Math.round(y0);
    const ix1 = Math.round(x1);
    const iy1 = Math.round(y1);
    const dx = Math.abs(ix1 - ix0);
    const dy = Math.abs(iy1 - iy0);
    const sx = ix0 < ix1 ? 1 : -1;
    const sy = iy0 < iy1 ? 1 : -1;
    let err = dx - dy;
    const half = Math.floor(thickness / 2);
    this.ctx.fillStyle = color;

    while (true) {
      this.ctx.fillRect(ix0 - half, iy0 - half, thickness, thickness);
      if (ix0 === ix1 && iy0 === iy1) break;
      const e2 = 2 * err;
      if (e2 > -dy) {
        err -= dy;
        ix0 += sx;
      }
      if (e2 < dx) {
        err += dx;
        iy0 += sy;
      }
    }
    return this;
  }

  /**
   * 3/4 shingled trapezoid roof centered horizontally at `cx`, top edge at `topY`.
   */
  roofTrapezoid(
    cx: number,
    topY: number,
    topW: number,
    botW: number,
    h: number,
    colors: { base: string; hi: string; lo: string; border: string },
    shingleCourses = true,
  ): this {
    const icx = Math.round(cx);
    const itop = Math.round(topY);
    const ih = Math.max(3, Math.round(h));
    for (let row = 0; row < ih; row++) {
      const t = ih === 1 ? 1 : row / (ih - 1);
      const rowW = Math.round(topW + (botW - topW) * t);
      const left = icx - Math.floor(rowW / 2);
      const y = itop + row;

      // Outer border left/right/top/bottom
      if (row === 0 || row === ih - 1) {
        this.rect(left, y, rowW, 1, colors.border);
        continue;
      }
      this.px(left, y, colors.border);
      this.px(left + rowW - 1, y, colors.border);

      const innerW = Math.max(0, rowW - 2);
      if (innerW === 0) continue;

      let fill = colors.base;
      if (row === 1) {
        fill = colors.hi;
      } else if (row === ih - 2) {
        fill = colors.lo;
      } else if (shingleCourses && row % 3 === 0) {
        fill = colors.lo;
      }
      this.rect(left + 1, y, innerW, 1, fill);

      // Shingle staggered notches
      if (shingleCourses && row > 1 && row < ih - 2 && row % 3 === 1) {
        for (let sx = left + 2 + (row % 2) * 2; sx < left + rowW - 2; sx += 4) {
          this.px(sx, y, colors.hi);
        }
      }
    }
    return this;
  }

  /** Draw a compact ASCII sprite stamp at (x, y) */
  stamp(x: number, y: number, rows: string[], palette: Record<string, string>): this {
    const ox = Math.round(x);
    const oy = Math.round(y);
    for (let ry = 0; ry < rows.length; ry++) {
      const line = rows[ry];
      for (let rx = 0; rx < line.length; rx++) {
        const ch = line[rx];
        const color = palette[ch];
        if (color) {
          this.ctx.fillStyle = color;
          this.ctx.fillRect(ox + rx, oy + ry, 1, 1);
        }
      }
    }
    return this;
  }

  /** Bake to a nearest-neighbor PixiJS v8 Texture */
  toTexture(repeat = false): Texture {
    const source = new CanvasSource({
      resource: this.canvas,
      scaleMode: "nearest",
      antialias: false,
      resolution: 1,
      addressMode: repeat ? "repeat" : "clamp-to-edge",
    });
    return new Texture({ source });
  }
}

/** Caches baked pixel textures across re-renders and destroys them on unmount */
export class PixelTextureCache {
  private readonly cache = new Map<string, Texture>();

  get(key: string, builder: () => Texture): Texture {
    const existing = this.cache.get(key);
    if (existing) return existing;
    const created = builder();
    this.cache.set(key, created);
    return created;
  }

  destroyAll(): void {
    for (const tex of this.cache.values()) {
      tex.destroy(true);
    }
    this.cache.clear();
  }
}
