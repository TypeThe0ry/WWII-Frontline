/** Terrain splat maths (M1-050, ARCHITECTURE 9.2); the terrain shader must match these. */

export const LAYERS = 8;

/** Normalises 8 raw weights to sum 1 (all-zero input falls back to layer 0). */
export function normalizeSplat(w: ArrayLike<number>, out: Float32Array): void {
  let sum = 0;
  for (let i = 0; i < LAYERS; i++) sum += Math.max(0, w[i] as number);
  if (sum <= 0) {
    out.fill(0);
    out[0] = 1;
    return;
  }
  for (let i = 0; i < LAYERS; i++) out[i] = Math.max(0, w[i] as number) / sum;
}

/** Layer with the largest weight of the two RGBA splat texels (ties: lowest index). */
export function dominantLayer(a: ArrayLike<number>, b: ArrayLike<number>): number {
  let best = 0;
  let bw = -1;
  for (let i = 0; i < LAYERS; i++) {
    const v = (i < 4 ? a[i] : b[i - 4]) as number;
    if (v > bw) {
      bw = v;
      best = i;
    }
  }
  return best;
}

export const TRIPLANAR_START_DEG = 40;
export const TRIPLANAR_RANGE_DEG = 10;
export const DETAIL_FADE_M = [80, 200] as const;
export const TRIPLANAR_FADE_M = [40, 100] as const;
export const MACRO_FADE_M = [160, 450] as const;

const smoothstep = (e0: number, e1: number, x: number): number => {
  const t = Math.min(Math.max((x - e0) / (e1 - e0), 0), 1);
  return t * t * (3 - 2 * t);
};

/** Triplanar blend factor from the surface slope: 0 below `startDeg`, 1 at `startDeg + rangeDeg`. */
export function triplanarWeight(slopeDeg: number, startDeg = TRIPLANAR_START_DEG, rangeDeg = TRIPLANAR_RANGE_DEG): number {
  return smoothstep(startDeg, startDeg + rangeDeg, slopeDeg);
}

/** Slope (deg) from a unit surface normal's y component. */
export const slopeDeg = (ny: number): number => (Math.acos(Math.min(Math.max(ny, -1), 1)) * 180) / Math.PI;

/** Repeating texture coordinate at `scale` repeats per metre. */
export function tileUv(u: number, scale: number): number {
  const t = u * scale;
  return t - Math.floor(t);
}

/** GPU block footprints. `etc2` here is opaque RGB8; the worker's ETC2 RGBA8 output is 8 bpp. */
const BLOCKS = { bc7: [4, 16], astc6x6: [6, 16], astc4x4: [4, 16], etc2: [4, 8], etc2rgba8: [4, 16] } as const;
export type LayerFormat = keyof typeof BLOCKS;

/** Tightly packed compressed bytes, including whole edge blocks and a full mip chain. Excludes driver overhead. */
export function layerArrayBytes(layers: number, size: number, maps: number, fmt: LayerFormat): number {
  if (!Number.isSafeInteger(size) || size < 1) throw new RangeError('Terrain layer size must be a positive integer');
  const [blockEdge, blockBytes] = BLOCKS[fmt];
  let bytes = 0;
  for (let edge = size; edge >= 1; edge = Math.floor(edge / 2)) {
    bytes += Math.ceil(edge / blockEdge) ** 2 * blockBytes;
  }
  return bytes * layers * maps;
}

export const LAYER_BUDGET_BYTES = 24 * 1024 * 1024;

/** PI-102: for 8-bpp formats, retain the albedo base mip and drop the first normal and ORM mip. */
export function terrainLayerSizes(fmt: LayerFormat): { albedo: number; normal: number; orm: number } {
  const [edge, bytes] = BLOCKS[fmt];
  const detail = bytes * 8 / edge ** 2 >= 8 ? 512 : 1024;
  return { albedo: 1024, normal: detail, orm: detail };
}

export function terrainLayerBytes(fmt: LayerFormat, layers = LAYERS): number {
  return Object.values(terrainLayerSizes(fmt)).reduce((sum, size) => sum + layerArrayBytes(layers, size, 1, fmt), 0);
}
