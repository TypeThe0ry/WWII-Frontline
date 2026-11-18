/** Hedgerow segment geometry (M1-051, MAPS 3.7): polylines -> instance segments, boxes, LOD. */

export interface HedgeSegment {
  x: number;
  z: number;
  /** 0 = +z, growing toward +x (same convention as the bots / sim). */
  yaw: number;
  len: number;
  height: number;
  moundWidth: number;
  gap: boolean;
  vehicleBlocking: boolean;
}

export interface HedgeProps {
  height_m: number;
  mound_width_m: number;
  gap?: boolean;
  vehicle_blocking?: boolean;
}

/** Longest single instance; longer edges are split so every instance keeps the model's aspect. */
export const MAX_SEG_LEN_M = 6;

/** Converts one polyline (x east, z north) into instance segments, preserving total length. */
export function polylineToSegments(pts: readonly (readonly [number, number])[], p: HedgeProps, maxLen = MAX_SEG_LEN_M): HedgeSegment[] {
  const out: HedgeSegment[] = [];
  for (let i = 0; i + 1 < pts.length; i++) {
    const [ax, az] = pts[i] as readonly [number, number];
    const [bx, bz] = pts[i + 1] as readonly [number, number];
    const dx = bx - ax;
    const dz = bz - az;
    const L = Math.hypot(dx, dz);
    if (L < 1e-6) continue;
    const n = Math.max(1, Math.ceil(L / maxLen));
    const len = L / n;
    const yaw = Math.atan2(dx, dz);
    for (let k = 0; k < n; k++) {
      const t = (k + 0.5) / n;
      out.push({
        x: ax + dx * t,
        z: az + dz * t,
        yaw,
        len,
        height: p.height_m,
        moundWidth: p.mound_width_m,
        gap: p.gap === true,
        vehicleBlocking: p.vehicle_blocking === true,
      });
    }
  }
  return out;
}

/** Axis-aligned bounds (x/z) of a segment's oriented box (length x mound width). */
export function segmentAabb(s: HedgeSegment): { minX: number; maxX: number; minZ: number; maxZ: number } {
  const sx = Math.sin(s.yaw);
  const cz = Math.cos(s.yaw);
  const hl = s.len / 2;
  const hw = s.moundWidth / 2;
  const ex = Math.abs(sx) * hl + Math.abs(cz) * hw;
  const ez = Math.abs(cz) * hl + Math.abs(sx) * hw;
  return { minX: s.x - ex, maxX: s.x + ex, minZ: s.z - ez, maxZ: s.z + ez };
}

/** The four corners of the oriented box (independent route to the same bounds; sim-side shape). */
export function segmentCorners(s: HedgeSegment): [number, number][] {
  const fx = Math.sin(s.yaw);
  const fz = Math.cos(s.yaw);
  const rx = fz; // right-hand perpendicular
  const rz = -fx;
  const hl = s.len / 2;
  const hw = s.moundWidth / 2;
  return [
    [s.x + fx * hl + rx * hw, s.z + fz * hl + rz * hw],
    [s.x + fx * hl - rx * hw, s.z + fz * hl - rz * hw],
    [s.x - fx * hl - rx * hw, s.z - fz * hl - rz * hw],
    [s.x - fx * hl + rx * hw, s.z - fz * hl + rz * hw],
  ];
}

/** Gap segments neither block movement nor give cover. */
export const blocksMovement = (s: HedgeSegment): boolean => !s.gap;

/** Beyond this distance a hedge is drawn as one low-poly strip per polyline [tbd]. */
export const HEDGE_FAR_LOD_M = 150;
export const useFarLod = (distM: number): boolean => distM > HEDGE_FAR_LOD_M;

export const totalLength = (segs: readonly HedgeSegment[]): number => segs.reduce((a, s) => a + s.len, 0);
