/** M1-052 asset-independent layout, baked alongside the map and also consumed by sim. */
import type { Ring } from './building-math';
import { type MeshArrays, wallColor } from './building-mesh';

export type Point3 = readonly [number, number, number];
type Point2 = readonly [number, number];
export interface BuildingWall {
  a: Point2;
  b: Point2;
  bottom: number;
  top: number;
  width: number;
}
export interface BuildingSurface {
  kind: 'floor' | 'roof' | 'ramp';
  vertices: readonly Point3[];
  indices: readonly number[];
}
export interface BuildingInterior {
  id: string;
  ring: Ring;
  holes: readonly Ring[];
  base_y: number;
  roof_y: number;
  walls: readonly BuildingWall[];
  surfaces: readonly BuildingSurface[];
  probes: readonly Point3[];
  door: { center: Point2; outward: Point2; width: number };
  ramp?: { start: Point3; end: Point3 };
}
export interface BuildingLayouts {
  version: 1;
  buildings: BuildingInterior[];
}

export function containsPoint(ring: Ring, x: number, z: number): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [ax, az] = ring[j]!;
    const [bx, bz] = ring[i]!;
    if ((az > z) !== (bz > z) && x < ax + (bx - ax) * (z - az) / (bz - az)) inside = !inside;
  }
  return inside;
}

/** Footprint (minus courtyards) and vertical extent, not camera heading or visibility. */
export function buildingAt(buildings: readonly BuildingInterior[], x: number, y: number, z: number): BuildingInterior | undefined {
  return buildings.find((b) => y >= b.base_y && y < b.roof_y && containsPoint(b.ring, x, z) && !b.holes.some((r) => containsPoint(r, x, z)));
}

/** Four bottom corners CCW in x/z. The sim uses the same oriented box (not its AABB). */
export function wallCorners(wall: BuildingWall): readonly Point2[] {
  const [ax, az] = wall.a;
  const [bx, bz] = wall.b;
  const half = wall.width / (2 * Math.hypot(bx - ax, bz - az));
  const dx = -(bz - az) * half;
  const dz = (bx - ax) * half;
  return [[ax - dx, az - dz], [bx - dx, bz - dz], [bx + dx, bz + dz], [ax + dx, az + dz]];
}

function appendFace(out: MeshArrays, points: readonly Point3[], indices: readonly number[], color: Point3): void {
  const base = out.positions.length / 3;
  const [a, b, c] = [points[indices[0]!]!, points[indices[1]!]!, points[indices[2]!]!];
  const ux = b[0] - a[0], uy = b[1] - a[1], uz = b[2] - a[2];
  const vx = c[0] - a[0], vy = c[1] - a[1], vz = c[2] - a[2];
  const nx = uy * vz - uz * vy, ny = uz * vx - ux * vz, nz = ux * vy - uy * vx;
  const norm = Math.hypot(nx, ny, nz);
  for (const p of points) {
    out.positions.push(...p);
    out.normals.push(nx / norm, ny / norm, nz / norm);
    out.colors.push(...color, 1);
  }
  for (const i of indices) out.indices.push(base + i);
}

export function appendWall(out: MeshArrays, wall: BuildingWall, style: string): void {
  const corners = wallCorners(wall);
  const floor = corners.map(([x, z]): Point3 => [x, wall.bottom, z]);
  const ceil = corners.map(([x, z]): Point3 => [x, wall.top, z]);
  const color = wallColor(style);
  for (let i = 0; i < 4; i++) {
    const j = (i + 1) % 4;
    appendFace(out, [floor[i]!, floor[j]!, ceil[j]!, ceil[i]!], [0, 2, 1, 0, 3, 2], color);
  }
  appendFace(out, floor, [0, 1, 2, 0, 2, 3], color);
  appendFace(out, ceil, [0, 2, 1, 0, 3, 2], color);
}

export function appendSurface(out: MeshArrays, surface: BuildingSurface, style: string): void {
  // Every indexed triangle of a floor, roof or ramp lies on the same plane.
  if (surface.indices.length) appendFace(out, surface.vertices, surface.indices, wallColor(style));
}
