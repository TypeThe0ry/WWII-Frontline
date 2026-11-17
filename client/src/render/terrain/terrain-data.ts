/**
 * Terrain data: meta.json + the two `.r16` height maps (little-endian u16, row = z, column = x,
 * `h = hmin + u16 / 65535 * (hmax - hmin)`; see tools/map-pipeline/README.md) and the CPU height query.
 *
 * The same height function is evaluated in the vertex shader (terrain-material.*), so keep them in step:
 *  - inside the near block: bilinear over the 1 m map (this is what sim's `height_at` returns, M0-010);
 *  - outside: bilinear over the 8 m far map, blended toward the clamped near-block edge value over
 *    `NEAR_BLEND_M` metres so near/far meet without a height jump (acceptance 7).
 */

export interface MapMeta {
  map_id: string;
  height: { res_m: number; samples: number; hmin_m: number; hmax_m: number };
  far_ring: { res_m: number; samples: number; extent_m: number };
  crs: { size_m: number };
}

/** Width (m) of the blend band outside the near block in which the far map fades into the near-edge value. */
export const NEAR_BLEND_M = 16;

/** Decode little-endian u16 samples (explicit byte order, independent of host endianness). */
export function decodeR16(bytes: Uint8Array, expectedSamples?: number): Uint16Array {
  if (bytes.byteLength % 2 !== 0) throw new Error(`r16: odd byte length ${bytes.byteLength}`);
  const n = bytes.byteLength / 2;
  if (expectedSamples !== undefined && n !== expectedSamples) {
    throw new Error(`r16: expected ${expectedSamples} samples, got ${n}`);
  }
  const out = new Uint16Array(n);
  for (let i = 0; i < n; i++) out[i] = bytes[2 * i]! | (bytes[2 * i + 1]! << 8);
  return out;
}

export function dequantize(q: Uint16Array, hmin: number, hmax: number): Float32Array {
  const out = new Float32Array(q.length);
  const k = (hmax - hmin) / 65535;
  for (let i = 0; i < q.length; i++) out[i] = hmin + q[i]! * k;
  return out;
}

const smooth01 = (t: number): number => {
  const c = Math.min(1, Math.max(0, t));
  return c * c * (3 - 2 * c);
};

/** One square regular height grid, sample (i,j) at world (originX + i*res, originZ + j*res). */
export class HeightGrid {
  constructor(
    readonly heights: Float32Array,
    readonly samples: number,
    readonly res: number,
    readonly originX: number,
    readonly originZ: number,
  ) {
    if (heights.length !== samples * samples) throw new Error('HeightGrid: size mismatch');
  }

  /** Bilinear sample, clamped to the grid. */
  at(x: number, z: number): number {
    const n = this.samples;
    let u = (x - this.originX) / this.res;
    let v = (z - this.originZ) / this.res;
    u = Math.min(Math.max(u, 0), n - 1);
    v = Math.min(Math.max(v, 0), n - 1);
    const i0 = Math.min(Math.floor(u), n - 2);
    const j0 = Math.min(Math.floor(v), n - 2);
    const fx = u - i0;
    const fz = v - j0;
    const h = this.heights;
    const a = h[j0 * n + i0]!;
    const b = h[j0 * n + i0 + 1]!;
    const c = h[(j0 + 1) * n + i0]!;
    const d = h[(j0 + 1) * n + i0 + 1]!;
    return (a + (b - a) * fx) * (1 - fz) + (c + (d - c) * fx) * fz;
  }

  /** Min/max of every sample whose cell touches the rectangle (conservative for bilinear surfaces). */
  minMaxRect(x0: number, z0: number, x1: number, z1: number): [number, number] {
    const n = this.samples;
    const i0 = Math.max(0, Math.floor((x0 - this.originX) / this.res));
    const i1 = Math.min(n - 1, Math.ceil((x1 - this.originX) / this.res));
    const j0 = Math.max(0, Math.floor((z0 - this.originZ) / this.res));
    const j1 = Math.min(n - 1, Math.ceil((z1 - this.originZ) / this.res));
    let lo = Infinity;
    let hi = -Infinity;
    for (let j = j0; j <= j1; j++) {
      for (let i = i0; i <= i1; i++) {
        const v = this.heights[j * n + i]!;
        if (v < lo) lo = v;
        if (v > hi) hi = v;
      }
    }
    if (lo > hi) {
      // Rectangle entirely outside the grid: clamped sampling returns the nearest edge.
      const e = this.at(x0, z0);
      return [e, e];
    }
    return [lo, hi];
  }
}

/** Decoded map: raw u16 (for GPU upload as RG8) plus dequantised f32 grids (for CPU queries). */
export class TerrainData {
  readonly near: HeightGrid;
  readonly far: HeightGrid;
  readonly nearMin: number;
  readonly nearMax: number;

  constructor(
    readonly meta: MapMeta,
    /** Raw little-endian bytes of the two maps (R = low byte, G = high byte when uploaded as RG8). */
    readonly nearBytes: Uint8Array,
    readonly farBytes: Uint8Array,
  ) {
    const { hmin_m: hmin, hmax_m: hmax, samples: ns } = meta.height;
    const fs = meta.far_ring.samples;
    const nq = decodeR16(nearBytes, ns * ns);
    const fq = decodeR16(farBytes, fs * fs);
    const size = meta.crs.size_m;
    const extent = meta.far_ring.extent_m;
    this.near = new HeightGrid(dequantize(nq, hmin, hmax), ns, meta.height.res_m, 0, 0);
    const c = size / 2 - extent / 2;
    this.far = new HeightGrid(dequantize(fq, hmin, hmax), fs, meta.far_ring.res_m, c, c);
    let lo = Infinity;
    let hi = -Infinity;
    for (const v of this.near.heights) {
      if (v < lo) lo = v;
      if (v > hi) hi = v;
    }
    this.nearMin = lo;
    this.nearMax = hi;
  }

  get hmin(): number {
    return this.meta.height.hmin_m;
  }
  get hmax(): number {
    return this.meta.height.hmax_m;
  }
  get nearSize(): number {
    return this.meta.crs.size_m;
  }
  get farExtent(): number {
    return this.meta.far_ring.extent_m;
  }
  /** Bytes uploaded to the GPU (both maps as RG8). */
  get uploadBytes(): number {
    return this.nearBytes.byteLength + this.farBytes.byteLength;
  }

  /** CPU surface height at world (x,z); mirrors the vertex shader. */
  heightAt(x: number, z: number): number {
    const s = this.nearSize;
    const dx = Math.max(-x, x - s, 0);
    const dz = Math.max(-z, z - s, 0);
    const d = Math.max(dx, dz);
    const nearH = this.near.at(x, z); // clamped to the block edge outside
    if (d <= 0) return nearH;
    const farH = this.far.at(x, z);
    const w = 1 - smooth01(d / NEAR_BLEND_M);
    return farH + (nearH - farH) * w;
  }

  /** Conservative height bounds of a level-0 node rectangle (used for AABB culling/selection). */
  leafRange(x0: number, z0: number, x1: number, z1: number): [number, number] {
    let [lo, hi] = this.far.minMaxRect(x0, z0, x1, z1);
    const s = this.nearSize;
    if (x0 < s + NEAR_BLEND_M && x1 > -NEAR_BLEND_M && z0 < s + NEAR_BLEND_M && z1 > -NEAR_BLEND_M) {
      lo = Math.min(lo, this.nearMin);
      hi = Math.max(hi, this.nearMax);
    }
    return [lo, hi];
  }
}

export interface LoadedTerrainFiles {
  meta: MapMeta;
  nearBytes: Uint8Array;
  farBytes: Uint8Array;
}

/** Fetch meta.json and the two height maps from `baseUrl` (e.g. `/maps/m0/`). */
export async function fetchTerrainFiles(baseUrl: string, fetchFn: typeof fetch = fetch): Promise<LoadedTerrainFiles> {
  const base = baseUrl.endsWith('/') ? baseUrl : `${baseUrl}/`;
  const get = async (name: string): Promise<Response> => {
    const r = await fetchFn(base + name);
    if (!r.ok) throw new Error(`terrain: ${base}${name} -> HTTP ${r.status}`);
    return r;
  };
  const [metaRes, nearRes, farRes] = await Promise.all([get('meta.json'), get('heightmap_1m.r16'), get('heightmap_far_8m.r16')]);
  const meta = (await metaRes.json()) as MapMeta;
  const [nearBuf, farBuf] = await Promise.all([nearRes.arrayBuffer(), farRes.arrayBuffer()]);
  return { meta, nearBytes: new Uint8Array(nearBuf), farBytes: new Uint8Array(farBuf) };
}

export async function loadTerrainData(baseUrl: string, fetchFn: typeof fetch = fetch): Promise<TerrainData> {
  const f = await fetchTerrainFiles(baseUrl, fetchFn);
  return new TerrainData(f.meta, f.nearBytes, f.farBytes);
}
