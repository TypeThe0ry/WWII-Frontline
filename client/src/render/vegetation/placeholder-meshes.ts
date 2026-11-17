/**
 * Procedural grey-box vegetation geometry (M0-024). No external assets: crossed blades, ellipsoid lumps and
 * cylinders with flat colours stand in for V-NAT-002 (trees) and V-NAT-004 (grass, shrubs) until the real scans arrive
 * (M1-051). The triangle counts are constants, asserted against ARCHITECTURE §9.6 in placeholder-meshes.test.ts.
 *
 * Vertex colour: rgb = albedo, a = wind sway weight (0 = fixed trunk, 1 = free tip), see wind.ts.
 */
import { hash4, unit } from './scatter';

export interface MeshData {
  positions: Float32Array;
  normals: Float32Array;
  /** RGBA float, a = wind weight. */
  colors: Float32Array;
  indices: Uint16Array;
  triangles: number;
}

export interface MeshBounds {
  minY: number;
  maxY: number;
  /** Centre of the bounding sphere used by the impostor (on the vertical axis). */
  centerY: number;
  /** Radius of the sphere around (0, centerY, 0) that contains every vertex. */
  radius: number;
}

type RGB = readonly [number, number, number];

class Builder {
  readonly pos: number[] = [];
  readonly col: number[] = [];
  readonly idx: number[] = [];

  vert(x: number, y: number, z: number, c: RGB, w: number): number {
    this.pos.push(x, y, z);
    this.col.push(c[0], c[1], c[2], w);
    return this.pos.length / 3 - 1;
  }

  tri(a: number, b: number, c: number): void {
    this.idx.push(a, b, c);
  }

  /**
   * Lumpy ellipsoid: `seg` columns x `rings` latitude bands -> seg * (2 * rings - 2) triangles. Vertices are
   * pushed radially by hash noise (`bump` fraction) so crowns are not perfect spheres.
   */
  ellipsoid(cx: number, cy: number, cz: number, rx: number, ry: number, rz: number, seg: number, rings: number, color: RGB, weight: (y: number) => number, bump: number, salt: number): void {
    const base = this.pos.length / 3;
    const top = this.vert(cx, cy + ry * (1 + bump * (unit(hash4(salt, 0, 0, 7)) - 0.5)), cz, color, weight(cy + ry));
    for (let r = 1; r < rings; r++) {
      const phi = (r / rings) * Math.PI;
      const sy = Math.cos(phi);
      const sr = Math.sin(phi);
      for (let s = 0; s < seg; s++) {
        const th = (s / seg) * Math.PI * 2;
        const n = 1 + bump * (unit(hash4(salt, r, s, 3)) - 0.5);
        const y = cy + ry * sy * n;
        this.vert(cx + rx * sr * Math.cos(th) * n, y, cz + rz * sr * Math.sin(th) * n, color, weight(y));
      }
    }
    const bot = this.vert(cx, cy - ry * (1 + bump * (unit(hash4(salt, 1, 0, 7)) - 0.5)), cz, color, weight(cy - ry));
    const ring = (r: number, s: number) => base + 1 + (r - 1) * seg + (s % seg);
    for (let s = 0; s < seg; s++) this.tri(top, ring(1, s + 1), ring(1, s));
    for (let r = 1; r < rings - 1; r++) {
      for (let s = 0; s < seg; s++) {
        this.tri(ring(r, s), ring(r, s + 1), ring(r + 1, s));
        this.tri(ring(r, s + 1), ring(r + 1, s + 1), ring(r + 1, s));
      }
    }
    for (let s = 0; s < seg; s++) this.tri(bot, ring(rings - 1, s), ring(rings - 1, s + 1));
  }

  /** Open tapered cylinder from y0 (radius r0) to y1 (r1) in `sections` rings of `seg` sides: seg * 2 * sections triangles. */
  trunk(x: number, z: number, y0: number, y1: number, r0: number, r1: number, seg: number, sections: number, color: RGB): void {
    const base = this.pos.length / 3;
    for (let k = 0; k <= sections; k++) {
      const t = k / sections;
      const y = y0 + (y1 - y0) * t;
      const r = r0 + (r1 - r0) * t;
      for (let s = 0; s < seg; s++) {
        const th = (s / seg) * Math.PI * 2;
        this.vert(x + r * Math.cos(th), y, z + r * Math.sin(th), color, 0);
      }
    }
    for (let k = 0; k < sections; k++) {
      for (let s = 0; s < seg; s++) {
        const a = base + k * seg + s;
        const b = base + k * seg + ((s + 1) % seg);
        this.tri(a, b, a + seg);
        this.tri(b, b + seg, a + seg);
      }
    }
  }

  /**
   * Grass blade: width `w` at the root tapering to a point at height `h`, bending by `lean` metres along (dx, dz) at the
   * tip. `segs` quads (2 tris each) + 1 tip triangle. Sway weight = (y/h)^2 so roots stay put.
   */
  blade(x: number, z: number, yaw: number, h: number, w: number, lean: number, segs: number, c0: RGB, c1: RGB): void {
    const cx = Math.cos(yaw);
    const sx = Math.sin(yaw);
    // blade plane spanned by (cx, sx) horizontally; lean along the plane normal
    const nx = -sx;
    const nz = cx;
    const rows: [number, number][] = [];
    for (let k = 0; k < segs; k++) {
      const t = k / segs;
      const half = (w / 2) * (1 - t * 0.6);
      const y = h * t;
      const off = lean * t * t;
      const col: RGB = [c0[0] + (c1[0] - c0[0]) * t, c0[1] + (c1[1] - c0[1]) * t, c0[2] + (c1[2] - c0[2]) * t];
      const wt = t * t;
      const a = this.vert(x - cx * half + nx * off, y, z - sx * half + nz * off, col, wt);
      const b = this.vert(x + cx * half + nx * off, y, z + sx * half + nz * off, col, wt);
      rows.push([a, b]);
    }
    const tip = this.vert(x + nx * lean, h, z + nz * lean, c1, 1);
    for (let k = 0; k < segs - 1; k++) {
      const [a0, b0] = rows[k]!;
      const [a1, b1] = rows[k + 1]!;
      this.tri(a0, b0, a1);
      this.tri(b0, b1, a1);
    }
    const [al, bl] = rows[segs - 1]!;
    this.tri(al, bl, tip);
  }

  build(): MeshData {
    const n = this.pos.length / 3;
    const positions = new Float32Array(this.pos);
    const normals = new Float32Array(n * 3);
    const indices = new Uint16Array(this.idx);
    if (n > 65535) throw new Error('placeholder mesh exceeds 16-bit indices');
    for (let i = 0; i < indices.length; i += 3) {
      const a = indices[i]! * 3;
      const b = indices[i + 1]! * 3;
      const c = indices[i + 2]! * 3;
      const ux = positions[b]! - positions[a]!;
      const uy = positions[b + 1]! - positions[a + 1]!;
      const uz = positions[b + 2]! - positions[a + 2]!;
      const vx = positions[c]! - positions[a]!;
      const vy = positions[c + 1]! - positions[a + 1]!;
      const vz = positions[c + 2]! - positions[a + 2]!;
      // area-weighted face normal (outward for the winding used above, see the winding test)
      const fx = uy * vz - uz * vy;
      const fy = uz * vx - ux * vz;
      const fz = ux * vy - uy * vx;
      for (const v of [a, b, c]) {
        normals[v] = normals[v]! + fx;
        normals[v + 1] = normals[v + 1]! + fy;
        normals[v + 2] = normals[v + 2]! + fz;
      }
    }
    for (let i = 0; i < n; i++) {
      const l = Math.hypot(normals[3 * i]!, normals[3 * i + 1]!, normals[3 * i + 2]!) || 1;
      normals[3 * i] = normals[3 * i]! / l;
      normals[3 * i + 1] = normals[3 * i + 1]! / l;
      normals[3 * i + 2] = normals[3 * i + 2]! / l;
    }
    return { positions, normals, colors: new Float32Array(this.col), indices, triangles: indices.length / 3 };
  }
}

const TRUNK: RGB = [0.33, 0.22, 0.12];
const LEAF_A: RGB = [0.2, 0.4, 0.12];
const LEAF_B: RGB = [0.26, 0.46, 0.14];
const LEAF_C: RGB = [0.16, 0.34, 0.11];
const SHRUB_A: RGB = [0.22, 0.38, 0.13];
const SHRUB_B: RGB = [0.28, 0.42, 0.15];
const GRASS_ROOT: RGB = [0.12, 0.25, 0.07];
const GRASS_TIP: RGB = [0.42, 0.56, 0.17];

/** Tree crown lumps: [x, y, z, rx, ry, rz, colour index]. Slightly asymmetric on purpose (impostor mirror checks). */
const TREE_LUMPS: readonly (readonly [number, number, number, number, number, number, number])[] = [
  [0.0, 6.4, 0.0, 2.3, 2.1, 2.3, 0],
  [1.7, 5.3, 0.5, 1.8, 1.6, 1.8, 1],
  [-1.6, 5.6, -0.6, 1.9, 1.7, 1.8, 2],
  [0.4, 5.2, -1.8, 1.7, 1.5, 1.7, 1],
  [-0.3, 5.0, 1.9, 1.8, 1.5, 1.8, 0],
  [0.9, 7.7, 0.7, 1.5, 1.4, 1.5, 1],
  [-0.9, 7.4, -0.8, 1.4, 1.3, 1.4, 2],
];
const LEAF = [LEAF_A, LEAF_B, LEAF_C] as const;

const SHRUB_LUMPS: readonly (readonly [number, number, number, number, number, number, number])[] = [
  [0.0, 0.55, 0.0, 0.7, 0.55, 0.7, 0],
  [0.55, 0.45, 0.2, 0.55, 0.45, 0.55, 1],
  [-0.5, 0.45, -0.25, 0.55, 0.42, 0.55, 1],
  [0.1, 0.4, -0.55, 0.5, 0.4, 0.5, 0],
];
const SHRUB = [SHRUB_A, SHRUB_B] as const;

/** Triangle counts, asserted in the tests against ARCHITECTURE §9.6. */
export const TREE_LOD0 = { seg: 20, rings: 12, trunkSeg: 10, trunkSections: 3 } as const;
export const TREE_LOD1 = { seg: 12, rings: 6, trunkSeg: 6, trunkSections: 1 } as const;
export const SHRUB_LOD0 = { seg: 12, rings: 7 } as const;
export const SHRUB_LOD1 = { seg: 5, rings: 4 } as const;

export function buildTree(lod: 0 | 1): MeshData {
  const p = lod === 0 ? TREE_LOD0 : TREE_LOD1;
  const b = new Builder();
  b.trunk(0, 0, 0, 4.4, 0.3, 0.17, p.trunkSeg, p.trunkSections, TRUNK);
  const weight = (y: number) => Math.min(1, Math.max(0.35, 0.35 + ((y - 3.6) / 5) * 0.65));
  TREE_LUMPS.forEach(([x, y, z, rx, ry, rz, c], i) => b.ellipsoid(x, y, z, rx, ry, rz, p.seg, p.rings, LEAF[c]!, weight, lod === 0 ? 0.22 : 0.12, 100 + i));
  return b.build();
}

export function buildShrub(lod: 0 | 1): MeshData {
  const p = lod === 0 ? SHRUB_LOD0 : SHRUB_LOD1;
  const b = new Builder();
  const weight = (y: number) => Math.min(1, Math.max(0.2, y / 1.0));
  SHRUB_LUMPS.forEach(([x, y, z, rx, ry, rz, c], i) => b.ellipsoid(x, y, z, rx, ry, rz, p.seg, p.rings, SHRUB[c]!, weight, lod === 0 ? 0.2 : 0.1, 200 + i));
  return b.build();
}

/** Grass tuft: LOD0 = 7 blades x 5 triangles, LOD1 = 3 single-triangle blades. */
export function buildGrass(lod: 0 | 1): MeshData {
  const b = new Builder();
  const blades = lod === 0 ? 7 : 3;
  for (let i = 0; i < blades; i++) {
    const a = unit(hash4(300, i, lod, 1));
    const r = lod === 0 ? 0.06 + 0.1 * unit(hash4(300, i, lod, 2)) : 0.05;
    const th = (i / blades) * Math.PI * 2 + a;
    const h = lod === 0 ? 0.38 + 0.25 * unit(hash4(300, i, lod, 3)) : 0.45;
    const lean = (unit(hash4(300, i, lod, 4)) - 0.35) * 0.18;
    b.blade(Math.cos(th) * r, Math.sin(th) * r, a * Math.PI * 2, h, lod === 0 ? 0.055 : 0.12, lean, lod === 0 ? 3 : 1, GRASS_ROOT, GRASS_TIP);
  }
  return b.build();
}

export function boundsOf(m: MeshData): MeshBounds {
  let minY = Infinity;
  let maxY = -Infinity;
  for (let i = 1; i < m.positions.length; i += 3) {
    minY = Math.min(minY, m.positions[i]!);
    maxY = Math.max(maxY, m.positions[i]!);
  }
  const centerY = (minY + maxY) / 2;
  let r2 = 0;
  for (let i = 0; i < m.positions.length; i += 3) {
    const dx = m.positions[i]!;
    const dy = m.positions[i + 1]! - centerY;
    const dz = m.positions[i + 2]!;
    r2 = Math.max(r2, dx * dx + dy * dy + dz * dz);
  }
  return { minY, maxY, centerY, radius: Math.sqrt(r2) };
}
