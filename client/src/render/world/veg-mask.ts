/**
 * Vegetation placement mask (M1-051b): rasterises the Carentan map layers onto a 1 m grid so the scatter can stay out of
 * buildings, roads, hedgerows and water and can concentrate trees in forests. Pure (no Babylon, no DOM), deterministic.
 * Coordinates are map-local metres (x east, z north), cell (i, j) covers [i, i+1) x [j, j+1).
 */
export const BLOCK = 1;
export const FOREST = 2;
/** Surface kinds for the terrain splat (M1-050b): they also set BLOCK (vegetation stays off them). */
export const ROAD = 4;
export const WATER = 8;
export const BUILDING = 16;

export type Pt = readonly [number, number];

export interface VegMask {
  size: number;
  blocked(x: number, z: number): boolean;
  forest(x: number, z: number): boolean;
  /** Raw flag byte of the cell at (x, z) (BLOCK | FOREST | ROAD | WATER | BUILDING); 0 outside the grid. */
  flags(x: number, z: number): number;
  /** Cells carrying each flag (diagnostics). */
  counts: { blocked: number; forest: number };
}

export interface MaskLayers {
  size: number;
  /** Building footprints (outer rings). */
  buildings?: readonly (readonly Pt[])[];
  roads?: readonly { pts: readonly Pt[]; width_m: number }[];
  hedges?: readonly { pts: readonly Pt[]; width_m: number }[];
  /** Water polygons and water lines (canals, streams) with their width. */
  waterPolys?: readonly (readonly Pt[])[];
  waterLines?: readonly { pts: readonly Pt[]; width_m: number }[];
  forests?: readonly (readonly Pt[])[];
}

/** Margins (m) added around each blocking feature. */
export const MARGIN = { building: 1.5, road: 0.4, hedge: 0.6, water: 0.5 };

function setCell(m: Uint8Array, n: number, i: number, j: number, flag: number): void {
  if (i >= 0 && j >= 0 && i < n && j < n) m[j * n + i]! |= flag;
}

/** Even-odd scanline fill of a ring at cell centres. */
export function fillPolygon(m: Uint8Array, n: number, ring: readonly Pt[], flag: number): void {
  if (ring.length < 3) return;
  let zmin = Infinity;
  let zmax = -Infinity;
  for (const p of ring) {
    zmin = Math.min(zmin, p[1]);
    zmax = Math.max(zmax, p[1]);
  }
  const j0 = Math.max(0, Math.floor(zmin - 0.5));
  const j1 = Math.min(n - 1, Math.ceil(zmax - 0.5));
  const xs: number[] = [];
  for (let j = j0; j <= j1; j++) {
    const zc = j + 0.5;
    xs.length = 0;
    for (let k = 0; k < ring.length; k++) {
      const a = ring[k]!;
      const b = ring[(k + 1) % ring.length]!;
      if ((a[1] <= zc && b[1] > zc) || (b[1] <= zc && a[1] > zc)) xs.push(a[0] + ((zc - a[1]) / (b[1] - a[1])) * (b[0] - a[0]));
    }
    xs.sort((p, q) => p - q);
    for (let k = 0; k + 1 < xs.length; k += 2) {
      const i0 = Math.max(0, Math.ceil(xs[k]! - 0.5));
      const i1 = Math.min(n - 1, Math.floor(xs[k + 1]! - 0.5));
      for (let i = i0; i <= i1; i++) m[j * n + i]! |= flag;
    }
  }
}

/** Marks every cell whose centre lies within `halfW` of the polyline (capsule per segment). */
export function strokePolyline(m: Uint8Array, n: number, pts: readonly Pt[], halfW: number, flag: number): void {
  for (let s = 0; s + 1 < pts.length; s++) {
    const [ax, az] = pts[s]!;
    const [bx, bz] = pts[s + 1]!;
    const i0 = Math.floor(Math.min(ax, bx) - halfW - 1);
    const i1 = Math.ceil(Math.max(ax, bx) + halfW + 1);
    const j0 = Math.floor(Math.min(az, bz) - halfW - 1);
    const j1 = Math.ceil(Math.max(az, bz) + halfW + 1);
    const dx = bx - ax;
    const dz = bz - az;
    const L2 = dx * dx + dz * dz;
    for (let j = j0; j <= j1; j++) {
      for (let i = i0; i <= i1; i++) {
        const px = i + 0.5 - ax;
        const pz = j + 0.5 - az;
        const t = L2 > 0 ? Math.min(1, Math.max(0, (px * dx + pz * dz) / L2)) : 0;
        const ex = px - t * dx;
        const ez = pz - t * dz;
        if (ex * ex + ez * ez <= halfW * halfW) setCell(m, n, i, j, flag);
      }
    }
  }
}

export function buildVegMask(layers: MaskLayers): VegMask {
  const n = Math.ceil(layers.size);
  const m = new Uint8Array(n * n);
  for (const ring of layers.forests ?? []) fillPolygon(m, n, ring, FOREST);
  for (const ring of layers.buildings ?? []) {
    fillPolygon(m, n, ring, BLOCK | BUILDING);
    strokePolyline(m, n, [...ring, ring[0]!], MARGIN.building, BLOCK | BUILDING);
  }
  for (const r of layers.roads ?? []) strokePolyline(m, n, r.pts, r.width_m / 2 + MARGIN.road, BLOCK | ROAD);
  for (const r of layers.hedges ?? []) strokePolyline(m, n, r.pts, r.width_m / 2 + MARGIN.hedge, BLOCK);
  for (const ring of layers.waterPolys ?? []) {
    fillPolygon(m, n, ring, BLOCK | WATER);
    strokePolyline(m, n, [...ring, ring[0]!], MARGIN.water, BLOCK | WATER);
  }
  for (const r of layers.waterLines ?? []) strokePolyline(m, n, r.pts, r.width_m / 2 + MARGIN.water, BLOCK | WATER);
  let blocked = 0;
  let forest = 0;
  for (let k = 0; k < m.length; k++) {
    if (m[k]! & BLOCK) blocked++;
    if (m[k]! & FOREST) forest++;
  }
  const at = (x: number, z: number): number => {
    const i = Math.floor(x);
    const j = Math.floor(z);
    return i >= 0 && j >= 0 && i < n && j < n ? m[j * n + i]! : 0;
  };
  return { size: n, blocked: (x, z) => (at(x, z) & BLOCK) !== 0, forest: (x, z) => (at(x, z) & FOREST) !== 0, flags: at, counts: { blocked, forest } };
}

interface Feat {
  geometry: { type: string; coordinates: unknown };
  properties: Record<string, unknown>;
}

async function layer(base: string, name: string): Promise<Feat[]> {
  try {
    const r = await fetch(`${base}${name}.geojson`);
    return r.ok ? ((await r.json()) as { features: Feat[] }).features : [];
  } catch {
    return [];
  }
}

const num = (v: unknown, d: number): number => (typeof v === 'number' && Number.isFinite(v) ? v : d);

/** Loads roads/buildings/hedgerows/water/forests of a map (`base` = `/maps/<id>/`) and builds the placement mask; null when none exist. */
export async function loadVegMask(base: string, size: number): Promise<VegMask | null> {
  const [buildings, roads, hedges, water, forests] = await Promise.all(['buildings', 'roads', 'hedgerows', 'water', 'forests'].map((n) => layer(base, n)));
  if (!buildings!.length && !roads!.length && !hedges!.length && !water!.length && !forests!.length) return null;
  const rings = (fs: Feat[]): Pt[][] => fs.filter((f) => f.geometry.type === 'Polygon').map((f) => (f.geometry.coordinates as Pt[][])[0]!);
  const lines = (fs: Feat[], w: (p: Record<string, unknown>) => number): { pts: Pt[]; width_m: number }[] =>
    fs.filter((f) => f.geometry.type === 'LineString').map((f) => ({ pts: f.geometry.coordinates as Pt[], width_m: w(f.properties) }));
  return buildVegMask({
    size,
    buildings: rings(buildings!),
    roads: lines(roads!, (p) => num(p.width_m, 4)),
    hedges: lines(hedges!, (p) => num(p.mound_width_m, 1)),
    waterPolys: rings(water!),
    waterLines: lines(water!, (p) => num(p.width_m, 2)),
    forests: rings(forests!),
  });
}
