/**
 * CDLOD quadtree (ARCHITECTURE §9.2). Pure TypeScript, no Babylon imports, so it is unit-testable in node.
 *
 * World convention (sim): x east, z north, y up. Near block SW corner = (0,0); the 8 km far ring is
 * centred on the block, so the root node (8192 m) has its SW corner at blockCentre - 4096.
 *
 * Every drawn node uses the same 65x65 vertex grid (64x64 cells, 8 192 triangles). LOD L has
 * cell size 2^L m, node edge 64 * 2^L m and selection range LOD0_RANGE * 2^L m.
 */

export const GRID_CELLS = 64;
export const GRID_VERTS = GRID_CELLS + 1;
/** Finest level: 1 m cells / 64 m nodes. Coarsest drawn level: LOD6 (64 m cells / 4 096 m nodes). */
export const MAX_LOD = 6;
/** The 8 192 m root is a virtual level-7 node that is always split (never drawn). */
export const ROOT_LEVEL = MAX_LOD + 1;
export const LOD_COUNT = MAX_LOD + 1;
/** Selection range of LOD0 in metres; doubles per level (§9.2: 110 / 220 / 440 / ... / 7 040). */
export const LOD0_RANGE = 110;
/** `morph_k = smoothstep(range * MORPH_START_RATIO, range, dist)` (§9.2: last 30 % of each range). */
export const MORPH_START_RATIO = 0.7;
/** Floats per instance in the buffer produced by `select`. */
export const INSTANCE_STRIDE = 8;
/** Bits of the edge masks (W = -x, E = +x, S = -z, N = +z). */
export const EDGE_W = 1;
export const EDGE_E = 2;
export const EDGE_S = 4;
export const EDGE_N = 8;
const MAX_LEAVES = 4096;

export const nodeSize = (lod: number): number => GRID_CELLS * 2 ** lod;
export const lodRange = (lod: number): number => LOD0_RANGE * 2 ** lod;

export function smoothstep(e0: number, e1: number, x: number): number {
  const t = Math.min(1, Math.max(0, (x - e0) / (e1 - e0)));
  return t * t * (3 - 2 * t);
}

/** Distance-based morph factor of LOD `lod` (0 = full detail, 1 = fully collapsed onto the next-coarser grid). */
export function morphK(lod: number, dist: number): number {
  if (lod >= MAX_LOD) return 0; // nothing coarser to morph to
  const r = lodRange(lod);
  return smoothstep(r * MORPH_START_RATIO, r, dist);
}

export interface QuadtreeLayout {
  /** SW corner of the 8 192 m root. */
  originX: number;
  originZ: number;
  /** Near block (1 m height map) rectangle, world metres. */
  nearMinX: number;
  nearMinZ: number;
  nearMaxX: number;
  nearMaxZ: number;
  /** Nodes that do not overlap the near block never get finer than this level (§9.2 task note: LOD3 = 8 m). */
  minLodOutsideNear: number;
}

/** Layout for a map: near block at (0,0)..(size,size), far ring centred on it. */
export function makeLayout(nearSizeM: number, farExtentM: number, minLodOutsideNear = 3): QuadtreeLayout {
  const c = nearSizeM / 2;
  return {
    originX: c - farExtentM / 2,
    originZ: c - farExtentM / 2,
    nearMinX: 0,
    nearMinZ: 0,
    nearMaxX: nearSizeM,
    nearMaxZ: nearSizeM,
    minLodOutsideNear,
  };
}

export interface NodeSelection {
  /** Visible instances, `INSTANCE_STRIDE` floats each: ox, oz, size, lod, coarserMask, finerMask, 0, 0. */
  data: Float32Array;
  /** Visible node count (after frustum culling). */
  count: number;
  /** Leaf count before frustum culling (the orientation-independent LOD cut). */
  leafCount: number;
  /** Visible node count per LOD. */
  perLod: Int32Array;
  /** Wall-clock selection time of the last call (ms). */
  selectMs: number;
}

export function makeSelection(capacity = MAX_LEAVES): NodeSelection {
  return {
    data: new Float32Array(capacity * INSTANCE_STRIDE),
    count: 0,
    leafCount: 0,
    perLod: new Int32Array(LOD_COUNT),
    selectMs: 0,
  };
}

/** Per-node (level, ix, iz) helpers over the implicit full quadtree (levels 0..7). */
const NODES_PER_SIDE: number[] = [];
const LEVEL_OFFSET: number[] = [];
{
  let off = 0;
  for (let l = 0; l <= ROOT_LEVEL; l++) {
    const n = 2 ** (ROOT_LEVEL - l);
    NODES_PER_SIDE.push(n);
    LEVEL_OFFSET.push(off);
    off += n * n;
  }
}
export const TOTAL_NODES = LEVEL_OFFSET[ROOT_LEVEL]! + 1;
const nodeId = (l: number, ix: number, iz: number): number => LEVEL_OFFSET[l]! + iz * NODES_PER_SIDE[l]! + ix;

/** Node height bounds: arrays of length `TOTAL_NODES`, indexed by `nodeId`. */
export interface NodeBounds {
  minH: Float32Array;
  maxH: Float32Array;
}

/**
 * Build bounds bottom-up. `leafRange(x0, z0, x1, z1)` returns a conservative [min, max] of the surface
 * heights over the level-0 node rectangle.
 */
export function buildNodeBounds(
  layout: QuadtreeLayout,
  leafRange: (x0: number, z0: number, x1: number, z1: number) => readonly [number, number],
): NodeBounds {
  const minH = new Float32Array(TOTAL_NODES);
  const maxH = new Float32Array(TOTAL_NODES);
  const n0 = NODES_PER_SIDE[0]!;
  const s0 = nodeSize(0);
  for (let iz = 0; iz < n0; iz++) {
    for (let ix = 0; ix < n0; ix++) {
      const x0 = layout.originX + ix * s0;
      const z0 = layout.originZ + iz * s0;
      const [lo, hi] = leafRange(x0, z0, x0 + s0, z0 + s0);
      const id = nodeId(0, ix, iz);
      minH[id] = lo;
      maxH[id] = hi;
    }
  }
  for (let l = 1; l <= ROOT_LEVEL; l++) {
    const n = NODES_PER_SIDE[l]!;
    for (let iz = 0; iz < n; iz++) {
      for (let ix = 0; ix < n; ix++) {
        let lo = Infinity;
        let hi = -Infinity;
        for (let c = 0; c < 4; c++) {
          const cid = nodeId(l - 1, ix * 2 + (c & 1), iz * 2 + (c >> 1));
          lo = Math.min(lo, minH[cid]!);
          hi = Math.max(hi, maxH[cid]!);
        }
        const id = nodeId(l, ix, iz);
        minH[id] = lo;
        maxH[id] = hi;
      }
    }
  }
  return { minH, maxH };
}

/** Flat bounds (every node gets the same range); used by tests and as a fallback. */
export function flatBounds(lo: number, hi: number): NodeBounds {
  return { minH: new Float32Array(TOTAL_NODES).fill(lo), maxH: new Float32Array(TOTAL_NODES).fill(hi) };
}

/** True when the AABB lies fully outside one of the (inward-normal, normalised) frustum planes (a,b,c,d per plane). */
export function aabbOutsideFrustum(
  planes: Float32Array,
  minX: number,
  minY: number,
  minZ: number,
  maxX: number,
  maxY: number,
  maxZ: number,
): boolean {
  for (let p = 0; p < planes.length; p += 4) {
    const nx = planes[p]!;
    const ny = planes[p + 1]!;
    const nz = planes[p + 2]!;
    const d = planes[p + 3]!;
    const px = nx >= 0 ? maxX : minX;
    const py = ny >= 0 ? maxY : minY;
    const pz = nz >= 0 ? maxZ : minZ;
    if (nx * px + ny * py + nz * pz + d < 0) return true;
  }
  return false;
}

export class CdlodQuadtree {
  readonly layout: QuadtreeLayout;
  private readonly bounds: NodeBounds;
  private readonly split = new Uint8Array(TOTAL_NODES);
  private readonly splitList = new Int32Array(TOTAL_NODES);
  private splitCount = 0;
  private readonly leafL = new Uint8Array(MAX_LEAVES);
  private readonly leafX = new Int32Array(MAX_LEAVES);
  private readonly leafZ = new Int32Array(MAX_LEAVES);
  private leafN = 0;
  private cx = 0;
  private cy = 0;
  private cz = 0;
  // Result of the last `findLeaf` call.
  private foundL = -1;
  private foundX = 0;
  private foundZ = 0;

  constructor(layout: QuadtreeLayout, bounds: NodeBounds) {
    this.layout = layout;
    this.bounds = bounds;
  }

  /**
   * Select nodes for a camera at (x,y,z). `planes` = 6 frustum planes (24 floats) or null for no culling.
   * The LOD cut ignores the frustum (so edge flags stay valid); culling only drops leaves from the output.
   * No allocation per call.
   */
  select(x: number, y: number, z: number, planes: Float32Array | null, out: NodeSelection): void {
    const t0 = performance.now();
    this.cx = x;
    this.cy = y;
    this.cz = z;
    this.clearSplits();
    this.markSplit(nodeId(ROOT_LEVEL, 0, 0));
    this.visitChildren(ROOT_LEVEL, 0, 0);
    this.balance();
    this.collectLeaves();
    this.emit(planes, out);
    out.selectMs = performance.now() - t0;
  }

  /** Leaf at world position (x,z) from the last `select`: level, or -1 outside the root. Test/diagnostic helper. */
  leafLevelAt(x: number, z: number): number {
    return this.findLeaf(x, z) ? this.foundL : -1;
  }

  private clearSplits(): void {
    for (let i = 0; i < this.splitCount; i++) this.split[this.splitList[i]!] = 0;
    this.splitCount = 0;
  }

  private markSplit(id: number): void {
    if (this.split[id]) return;
    this.split[id] = 1;
    this.splitList[this.splitCount++] = id;
  }

  private overlapsNear(ox: number, oz: number, s: number): boolean {
    const l = this.layout;
    return ox < l.nearMaxX && ox + s > l.nearMinX && oz < l.nearMaxZ && oz + s > l.nearMinZ;
  }

  /** Squared 3D distance from the camera to the node AABB. */
  private dist2(l: number, ix: number, iz: number): number {
    const s = nodeSize(l);
    const x0 = this.layout.originX + ix * s;
    const z0 = this.layout.originZ + iz * s;
    const id = nodeId(l, ix, iz);
    const dx = Math.max(x0 - this.cx, 0, this.cx - (x0 + s));
    const dz = Math.max(z0 - this.cz, 0, this.cz - (z0 + s));
    const dy = Math.max(this.bounds.minH[id]! - this.cy, 0, this.cy - this.bounds.maxH[id]!);
    return dx * dx + dy * dy + dz * dz;
  }

  private shouldSplit(l: number, ix: number, iz: number): boolean {
    if (l === 0) return false;
    const s = nodeSize(l);
    const ox = this.layout.originX + ix * s;
    const oz = this.layout.originZ + iz * s;
    if (l - 1 < this.layout.minLodOutsideNear && !this.overlapsNear(ox, oz, s)) return false;
    const r = lodRange(l - 1);
    return this.dist2(l, ix, iz) < r * r;
  }

  private visitChildren(l: number, ix: number, iz: number): void {
    for (let c = 0; c < 4; c++) {
      const cx = ix * 2 + (c & 1);
      const cz = iz * 2 + (c >> 1);
      const cl = l - 1;
      if (this.shouldSplit(cl, cx, cz)) {
        this.markSplit(nodeId(cl, cx, cz));
        this.visitChildren(cl, cx, cz);
      }
    }
  }

  /** Descend from the root to the leaf covering (x,z). Sets foundL/X/Z. */
  private findLeaf(x: number, z: number): boolean {
    const lx = x - this.layout.originX;
    const lz = z - this.layout.originZ;
    const root = nodeSize(ROOT_LEVEL);
    if (lx < 0 || lz < 0 || lx >= root || lz >= root) return false;
    let l = ROOT_LEVEL;
    let ix = 0;
    let iz = 0;
    while (this.split[nodeId(l, ix, iz)]) {
      l--;
      const s = nodeSize(l);
      ix = Math.floor(lx / s);
      iz = Math.floor(lz / s);
    }
    this.foundL = l;
    this.foundX = ix;
    this.foundZ = iz;
    return true;
  }

  private collectLeaves(): void {
    this.leafN = 0;
    // Iterative DFS over split nodes.
    const stackL: number[] = [ROOT_LEVEL];
    const stackX: number[] = [0];
    const stackZ: number[] = [0];
    while (stackL.length) {
      const l = stackL.pop()!;
      const ix = stackX.pop()!;
      const iz = stackZ.pop()!;
      if (this.split[nodeId(l, ix, iz)]) {
        for (let c = 0; c < 4; c++) {
          stackL.push(l - 1);
          stackX.push(ix * 2 + (c & 1));
          stackZ.push(iz * 2 + (c >> 1));
        }
      } else {
        if (this.leafN >= MAX_LEAVES) throw new Error('CdlodQuadtree: leaf capacity exceeded');
        this.leafL[this.leafN] = l;
        this.leafX[this.leafN] = ix;
        this.leafZ[this.leafN] = iz;
        this.leafN++;
      }
    }
  }

  /** Probe point just outside side `side` of a leaf, at 1/4 of the side (inside one half of a finer neighbour). */
  private probe(l: number, ix: number, iz: number, side: number): boolean {
    const s = nodeSize(l);
    const ox = this.layout.originX + ix * s;
    const oz = this.layout.originZ + iz * s;
    const eps = 0.5; // smaller than any node (64 m) so we never skip a neighbour
    switch (side) {
      case 0:
        return this.findLeaf(ox - eps, oz + 0.25 * s);
      case 1:
        return this.findLeaf(ox + s + eps, oz + 0.25 * s);
      case 2:
        return this.findLeaf(ox + 0.25 * s, oz - eps);
      default:
        return this.findLeaf(ox + 0.25 * s, oz + s + eps);
    }
  }

  /** Restrict the tree so that adjacent leaves differ by at most one level (split the coarser one). */
  private balance(): void {
    for (let iter = 0; iter < 32; iter++) {
      this.collectLeaves();
      let changed = false;
      for (let i = 0; i < this.leafN; i++) {
        const l = this.leafL[i]!;
        for (let side = 0; side < 4; side++) {
          if (!this.probe(l, this.leafX[i]!, this.leafZ[i]!, side)) continue;
          if (this.foundL >= l + 2) {
            this.markSplit(nodeId(this.foundL, this.foundX, this.foundZ));
            changed = true;
          }
        }
      }
      if (!changed) return;
    }
    throw new Error('CdlodQuadtree: balance did not converge');
  }

  private emit(planes: Float32Array | null, out: NodeSelection): void {
    out.perLod.fill(0);
    out.leafCount = this.leafN;
    let n = 0;
    const { minH, maxH } = this.bounds;
    for (let i = 0; i < this.leafN; i++) {
      const l = this.leafL[i]!;
      const ix = this.leafX[i]!;
      const iz = this.leafZ[i]!;
      const s = nodeSize(l);
      const ox = this.layout.originX + ix * s;
      const oz = this.layout.originZ + iz * s;
      if (planes) {
        const id = nodeId(l, ix, iz);
        if (aabbOutsideFrustum(planes, ox, minH[id]!, oz, ox + s, maxH[id]!, oz + s)) continue;
      }
      let coarser = 0;
      let finer = 0;
      for (let side = 0; side < 4; side++) {
        if (!this.probe(l, ix, iz, side)) continue;
        if (this.foundL > l) coarser |= 1 << side;
        else if (this.foundL < l) finer |= 1 << side;
      }
      const o = n * INSTANCE_STRIDE;
      out.data[o] = ox;
      out.data[o + 1] = oz;
      out.data[o + 2] = s;
      out.data[o + 3] = l;
      out.data[o + 4] = coarser;
      out.data[o + 5] = finer;
      out.data[o + 6] = 0;
      out.data[o + 7] = 0;
      out.perLod[l]!++;
      n++;
    }
    out.count = n;
  }
}

/**
 * CPU reference of the vertex-shader morph (terrain-material.glsl/wgsl must mirror it exactly).
 *
 * Vertex (i,j) in 0..64 of a node at origin (ox,oz), edge `size`, level `lod`. `H(x,z)` is the surface height
 * function. Odd vertices move toward the average of their two even neighbours (the coarser grid; cells are all
 * triangulated along the BL-TR diagonal, which is what the odd-odd diagonal average assumes).
 *
 * Edge rule (needed on top of the distance-only formula: with 2x ranges and 0.7 start a coarse node's own morph
 * can already be running at a shared edge, see PLAN_ISSUES): on an edge whose neighbour is finer
 * (`finerMask`) the coarse node does not morph, on an edge whose neighbour is coarser (`coarserMask`) the fine
 * node is fully morphed. Corner/even vertices never move, so both sides land on the same polyline.
 */
export function morphVertexHeight(
  H: (x: number, z: number) => number,
  cam: readonly [number, number, number],
  ox: number,
  oz: number,
  size: number,
  lod: number,
  coarserMask: number,
  finerMask: number,
  i: number,
  j: number,
): number {
  const cell = size / GRID_CELLS;
  const px = ox + i * cell;
  const pz = oz + j * cell;
  const h = H(px, pz);
  const oi = i & 1;
  const oj = j & 1;
  if (!oi && !oj) return h;
  let k: number;
  const onW = i === 0;
  const onE = i === GRID_CELLS;
  const onS = j === 0;
  const onN = j === GRID_CELLS;
  if ((onW && finerMask & EDGE_W) || (onE && finerMask & EDGE_E) || (onS && finerMask & EDGE_S) || (onN && finerMask & EDGE_N)) {
    k = 0;
  } else if (
    (onW && coarserMask & EDGE_W) ||
    (onE && coarserMask & EDGE_E) ||
    (onS && coarserMask & EDGE_S) ||
    (onN && coarserMask & EDGE_N)
  ) {
    k = 1;
  } else {
    const dx = px - cam[0];
    const dy = h - cam[1];
    const dz = pz - cam[2];
    k = morphK(lod, Math.sqrt(dx * dx + dy * dy + dz * dz));
  }
  if (k === 0) return h;
  let ha: number;
  let hb: number;
  if (oi && !oj) {
    ha = H(px - cell, pz);
    hb = H(px + cell, pz);
  } else if (!oi && oj) {
    ha = H(px, pz - cell);
    hb = H(px, pz + cell);
  } else {
    ha = H(px - cell, pz - cell);
    hb = H(px + cell, pz + cell);
  }
  return h + (0.5 * (ha + hb) - h) * k;
}
