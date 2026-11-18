/**
 * Building massing (M1-052b, MAPS 3.7): footprint polygon -> extruded walls + flat roof, merged per cell into plain
 * vertex arrays. Pure (no Babylon), so it is unit-tested; `world-scene.ts` uploads the arrays. This is the placeholder
 * massing stage: no openings/interiors/stairs yet (enterable buildings are drawn as solid blocks, see PI-110).
 * Coordinates are map-local metres, x east, z north, y up.
 */
import { LEVEL_HEIGHT_M, type Ring } from './building-math';

export interface BuildingInput {
  ring: Ring;
  levels: number;
  style: string;
  enterable: boolean;
}

export interface MeshArrays {
  positions: number[];
  normals: number[];
  colors: number[];
  indices: number[];
}

export const newArrays = (): MeshArrays => ({ positions: [], normals: [], colors: [], indices: [] });

/** Signed area (shoelace); > 0 = counter-clockwise in the x/z plane (x east, z north). */
export function signedArea(ring: Ring): number {
  let a = 0;
  for (let i = 0; i < ring.length; i++) {
    const [x0, z0] = ring[i] as readonly [number, number];
    const [x1, z1] = ring[(i + 1) % ring.length] as readonly [number, number];
    a += x0 * z1 - x1 * z0;
  }
  return a / 2;
}

/** Drops the closing duplicate vertex and consecutive duplicates. */
export function cleanRing(ring: Ring): [number, number][] {
  const out: [number, number][] = [];
  for (const p of ring) {
    const last = out[out.length - 1];
    if (!last || Math.hypot(p[0] - last[0], p[1] - last[1]) > 1e-6) out.push([p[0], p[1]]);
  }
  const f = out[0];
  const l = out[out.length - 1];
  if (f && l && out.length > 1 && Math.hypot(f[0] - l[0], f[1] - l[1]) <= 1e-6) out.pop();
  return out;
}

const cross = (ax: number, az: number, bx: number, bz: number, cx: number, cz: number): number => (bx - ax) * (cz - az) - (bz - az) * (cx - ax);

function inTri(px: number, pz: number, a: readonly number[], b: readonly number[], c: readonly number[]): boolean {
  const d1 = cross(a[0]!, a[1]!, b[0]!, b[1]!, px, pz);
  const d2 = cross(b[0]!, b[1]!, c[0]!, c[1]!, px, pz);
  const d3 = cross(c[0]!, c[1]!, a[0]!, a[1]!, px, pz);
  return d1 >= -1e-9 && d2 >= -1e-9 && d3 >= -1e-9;
}

/**
 * Ear clipping of a simple polygon (any winding). Returns triangles as index triples into `ring` (counter-clockwise),
 * or null when the polygon is degenerate / self-intersecting and no ear can be found.
 */
export function triangulate(ring: readonly (readonly [number, number])[]): [number, number, number][] | null {
  const n = ring.length;
  if (n < 3) return null;
  const idx = Array.from({ length: n }, (_, i) => i);
  if (signedArea(ring) < 0) idx.reverse();
  const tris: [number, number, number][] = [];
  let guard = 0;
  while (idx.length > 3 && guard++ < n * n) {
    let clipped = false;
    for (let k = 0; k < idx.length; k++) {
      const ia = idx[(k + idx.length - 1) % idx.length]!;
      const ib = idx[k]!;
      const ic = idx[(k + 1) % idx.length]!;
      const a = ring[ia]!;
      const b = ring[ib]!;
      const c = ring[ic]!;
      if (cross(a[0], a[1], b[0], b[1], c[0], c[1]) <= 1e-9) continue; // reflex or collinear
      let ear = true;
      for (const j of idx) {
        if (j === ia || j === ib || j === ic) continue;
        const p = ring[j]!;
        if (inTri(p[0], p[1], a, b, c)) {
          ear = false;
          break;
        }
      }
      if (!ear) continue;
      tris.push([ia, ib, ic]);
      idx.splice(k, 1);
      clipped = true;
      break;
    }
    if (!clipped) return null;
  }
  if (idx.length === 3) {
    const a = ring[idx[0]!]!;
    const b = ring[idx[1]!]!;
    const c = ring[idx[2]!]!;
    if (cross(a[0], a[1], b[0], b[1], c[0], c[1]) > 1e-9) tris.push([idx[0]!, idx[1]!, idx[2]!]);
  }
  return tris.length > 0 ? tris : null;
}

const STYLE_COLORS: Record<string, [number, number, number]> = {
  stone_townhouse: [0.66, 0.62, 0.54],
  farmhouse: [0.58, 0.5, 0.4],
  church: [0.72, 0.7, 0.66],
  industrial: [0.5, 0.52, 0.54],
};
const DEFAULT_WALL: [number, number, number] = [0.6, 0.57, 0.5];
export const wallColor = (style: string): [number, number, number] => STYLE_COLORS[style] ?? DEFAULT_WALL;
const ROOF_COLOR: [number, number, number] = [0.32, 0.27, 0.25];

export interface BuildingStats {
  buildings: number;
  triangles: number;
  roofFailures: number;
}

/**
 * Appends one extruded building to `out`. `groundAt` is the terrain height; walls start 0.4 m below the lowest footprint
 * ground sample (no floating on slopes) and rise to the highest ground + levels * 3 m.
 */
export function extrudeBuilding(b: BuildingInput, groundAt: (x: number, z: number) => number, out: MeshArrays, stats?: BuildingStats): void {
  const ring = cleanRing(b.ring);
  if (ring.length < 3) return;
  if (signedArea(ring) < 0) ring.reverse();
  let lo = Infinity;
  let hi = -Infinity;
  for (const [x, z] of ring) {
    const g = groundAt(x, z);
    lo = Math.min(lo, g);
    hi = Math.max(hi, g);
  }
  const yBot = lo - 0.4;
  const yTop = hi + Math.max(1, b.levels) * LEVEL_HEIGHT_M;
  const wc = wallColor(b.style);
  let tri = 0;
  const pushV = (x: number, y: number, z: number, nx: number, ny: number, nz: number, c: readonly number[]): number => {
    const i = out.positions.length / 3;
    out.positions.push(x, y, z);
    out.normals.push(nx, ny, nz);
    out.colors.push(c[0]!, c[1]!, c[2]!, 1);
    return i;
  };
  for (let i = 0; i < ring.length; i++) {
    const [x0, z0] = ring[i]!;
    const [x1, z1] = ring[(i + 1) % ring.length]!;
    const dx = x1 - x0;
    const dz = z1 - z0;
    const L = Math.hypot(dx, dz);
    if (L < 1e-6) continue;
    const nx = dz / L; // outward normal for a counter-clockwise ring (x east, z north, y up)
    const nz = -dx / L;
    const a = pushV(x0, yBot, z0, nx, 0, nz, wc);
    const bq = pushV(x1, yBot, z1, nx, 0, nz, wc);
    const c = pushV(x1, yTop, z1, nx, 0, nz, wc);
    const d = pushV(x0, yTop, z0, nx, 0, nz, wc);
    out.indices.push(a, c, bq, a, d, c);
    tri += 2;
  }
  const roof = triangulate(ring);
  if (roof) {
    const base = out.positions.length / 3;
    for (const [x, z] of ring) pushV(x, yTop, z, 0, 1, 0, ROOF_COLOR);
    for (const [i, j, k] of roof) out.indices.push(base + i, base + k, base + j);
    tri += roof.length;
  } else if (stats) stats.roofFailures++;
  if (stats) {
    stats.buildings++;
    stats.triangles += tri;
  }
}
