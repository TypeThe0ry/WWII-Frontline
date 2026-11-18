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

const smoothstep = (e0: number, e1: number, x: number): number => {
  const t = Math.min(Math.max((x - e0) / (e1 - e0), 0), 1);
  return t * t * (3 - 2 * t);
};

/** Triplanar blend factor from the surface slope: 0 below `startDeg`, 1 at `startDeg + rangeDeg`. */
export function triplanarWeight(slopeDeg: number, startDeg = 40, rangeDeg = 10): number {
  return smoothstep(startDeg, startDeg + rangeDeg, slopeDeg);
}

/** Slope (deg) from a unit surface normal's y component. */
export const slopeDeg = (ny: number): number => (Math.acos(Math.min(Math.max(ny, -1), 1)) * 180) / Math.PI;

/** Two-scale tiling blend that hides repetition: mixes the layer at `scale` and at `scale * ratio`. */
export function tileUv(u: number, scale: number): number {
  const t = u * scale;
  return t - Math.floor(t);
}

/** Bits per pixel of the GPU block formats a UASTC transcode may land in. */
export const BPP = { bc7: 8, astc6x6: 3.5556, astc4x4: 8, etc2: 4 } as const;
export type LayerFormat = keyof typeof BPP;

/** VRAM bytes of `layers` layers of `size`^2 textures, `maps` maps per layer, with a full mip chain. */
export function layerArrayBytes(layers: number, size: number, maps: number, fmt: LayerFormat): number {
  return layers * maps * ((size * size * BPP[fmt]) / 8) * (4 / 3);
}

export const LAYER_BUDGET_BYTES = 24 * 1024 * 1024;
