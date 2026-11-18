/** Building geometry rules (M1-052, MAPS 3.7, ARCHITECTURE 9.7): openings, stairs, probes, LOD, stats. */

export const DOOR_MIN_W_M = 1.0;
export const DOOR_MIN_H_M = 2.0;
export const STAIR_MAX_SLOPE_DEG = 35;
export const STAIR_MAX_STEP_M = 0.4;
export const LEVEL_HEIGHT_M = 3;
export const ENTERABLE_MAX_FRACTION = 0.25;
/** LOD switch distances [tbd]. */
export const BUILDING_LOD_M = [60, 150, 400] as const;

export interface Opening {
  type: 'door' | 'window';
  wall: number;
  offset: number;
  w: number;
  h: number;
  sill: number;
}

/** Returns a list of problems (empty = valid) for one opening on a wall of length `wallLen`. */
export function checkOpening(o: Opening, wallLen: number): string[] {
  const e: string[] = [];
  if (o.type === 'door') {
    if (o.w < DOOR_MIN_W_M) e.push('door too narrow');
    if (o.h < DOOR_MIN_H_M) e.push('door too low');
    if (o.sill > 0.05) e.push('door sill above floor');
  }
  if (o.offset < 0 || o.offset + o.w > wallLen) e.push('opening outside wall');
  if (o.sill + o.h > LEVEL_HEIGHT_M) e.push('opening above ceiling');
  return e;
}

/** A stair is walkable when it is a ramp <= 35 deg or consists of steps <= 0.4 m. */
export function stairWalkable(rise: number, run: number, stepM?: number): boolean {
  if (stepM !== undefined) return stepM <= STAIR_MAX_STEP_M;
  return (Math.atan2(rise, run) * 180) / Math.PI <= STAIR_MAX_SLOPE_DEG;
}

/** Number of steps and the run needed to climb one level with steps of at most `maxStep`. */
export function stairPlan(levelH = LEVEL_HEIGHT_M, maxStep = STAIR_MAX_STEP_M, treadM = 0.28): { steps: number; run: number } {
  const steps = Math.ceil(levelH / maxStep);
  return { steps, run: steps * treadM };
}

export type Ring = readonly (readonly [number, number])[];

/** Polygon centroid (shoelace); falls back to the vertex mean for a degenerate ring. */
export function centroid(ring: Ring): [number, number] {
  let a = 0;
  let cx = 0;
  let cz = 0;
  for (let i = 0; i < ring.length; i++) {
    const [x0, z0] = ring[i] as readonly [number, number];
    const [x1, z1] = ring[(i + 1) % ring.length] as readonly [number, number];
    const w = x0 * z1 - x1 * z0;
    a += w;
    cx += (x0 + x1) * w;
    cz += (z0 + z1) * w;
  }
  if (Math.abs(a) < 1e-9) {
    const n = ring.length || 1;
    return [ring.reduce((s, p) => s + p[0], 0) / n, ring.reduce((s, p) => s + p[1], 0) / n];
  }
  return [cx / (3 * a), cz / (3 * a)];
}

export interface ProbePoint {
  x: number;
  y: number;
  z: number;
  level: number;
}

/** One probe per level at the footprint centroid, 1.2 m above that floor. */
export function probesFor(ring: Ring, levels: number, groundY = 0): ProbePoint[] {
  const [x, z] = centroid(ring);
  return Array.from({ length: Math.max(1, levels) }, (_, level) => ({ x, z, level, y: groundY + level * LEVEL_HEIGHT_M + 1.2 }));
}

/** LOD index 0..3 for a distance (3 = shell-only beyond 400 m). */
export const buildingLod = (distM: number): number => BUILDING_LOD_M.findIndex((t) => distM <= t) === -1 ? 3 : BUILDING_LOD_M.findIndex((t) => distM <= t);

/** Fraction of enterable buildings (0 for an empty set). */
export function enterableFraction(flags: readonly boolean[]): number {
  return flags.length === 0 ? 0 : flags.filter(Boolean).length / flags.length;
}

/** Axis-aligned bounds of a wall segment box of thickness `t` (render side of `building_align`). */
export function wallAabb(a: readonly [number, number], b: readonly [number, number], t: number): { minX: number; maxX: number; minZ: number; maxZ: number } {
  const dx = b[0] - a[0];
  const dz = b[1] - a[1];
  const L = Math.hypot(dx, dz) || 1;
  const nx = (-dz / L) * (t / 2);
  const nz = (dx / L) * (t / 2);
  const xs = [a[0] + nx, a[0] - nx, b[0] + nx, b[0] - nx];
  const zs = [a[1] + nz, a[1] - nz, b[1] + nz, b[1] - nz];
  return { minX: Math.min(...xs), maxX: Math.max(...xs), minZ: Math.min(...zs), maxZ: Math.max(...zs) };
}
