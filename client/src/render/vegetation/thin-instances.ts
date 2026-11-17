/**
 * CPU side of the per-(chunk x species) LOD buckets (M0-024, ARCHITECTURE §9.6): each LOD owns one matrix buffer that
 * a Babylon thin-instance mesh draws (one draw call per chunk x species x LOD). A frame only touches instances that
 * changed LOD, so the cost is proportional to what crossed a distance threshold, not to the instance count:
 *  - a bucket whose whole bounding box lies in a single distance band that every instance already occupies is skipped
 *    after two distance computations;
 *  - otherwise instances are classified (one squared distance each) and only changed ones are moved between buffers
 *    (swap-remove from the old LOD, append to the new one).
 * No Babylon dependency: unit-testable in node. The buffers use the 16-float thin-instance matrix layout, written by
 * `writeInstanceMatrix` (row-vector convention: x' = c x + s z, z' = -s x + c z, i.e. Babylon's Matrix.RotationY).
 */

/** Per-instance state shared by all LODs of a bucket. */
export interface CompactInstances {
  count: number;
  /** x, y, z per instance. */
  pos: Float32Array;
  scale: Float32Array;
  cos: Float32Array;
  sin: Float32Array;
  /** Stable per-instance rank 0..255 for density thinning: shown iff rank < rankLimit. */
  rank: Uint8Array;
}

export function makeCompact(count: number): CompactInstances {
  return {
    count,
    pos: new Float32Array(count * 3),
    scale: new Float32Array(count),
    cos: new Float32Array(count),
    sin: new Float32Array(count),
    rank: new Uint8Array(count),
  };
}

/** Write instance `i` as a thin-instance world matrix (uniform scale, yaw about Y, translation) at float offset `o`. */
export function writeInstanceMatrix(out: Float32Array, o: number, c: CompactInstances, i: number): void {
  const s = c.scale[i]!;
  const cs = c.cos[i]!;
  const sn = c.sin[i]!;
  out[o] = s * cs;
  out[o + 1] = 0;
  out[o + 2] = -s * sn;
  out[o + 3] = 0;
  out[o + 4] = 0;
  out[o + 5] = s;
  out[o + 6] = 0;
  out[o + 7] = 0;
  out[o + 8] = s * sn;
  out[o + 9] = 0;
  out[o + 10] = s * cs;
  out[o + 11] = 0;
  out[o + 12] = c.pos[i * 3]!;
  out[o + 13] = c.pos[i * 3 + 1]!;
  out[o + 14] = c.pos[i * 3 + 2]!;
  out[o + 15] = 1;
}

export interface Box {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  minZ: number;
  maxZ: number;
}

const UNASSIGNED = -2;
const HIDDEN = -1;
/** A bucket is re-evaluated only after the camera moved this far since its last evaluation (m). */
export const REEVAL_MOVE_M = 0.2;

export class LodBuckets {
  readonly lodCount: number;
  /** One matrix buffer per visible LOD; capacity = instance count (any LOD can hold all of them). */
  readonly buffers: Float32Array[];
  readonly counts: Int32Array;
  /** LODs whose buffer changed since the caller last cleared the flag. */
  readonly dirty: boolean[];
  private readonly lodOf: Int8Array;
  private readonly slot: Int32Array;
  private readonly idAt: Int32Array[];
  private uniformLod = UNASSIGNED;
  private lastX = NaN;
  private lastY = NaN;
  private lastZ = NaN;
  private lastRankLimit = -1;
  private lastThrKey = '';

  constructor(
    readonly inst: CompactInstances,
    lodCount: number,
    readonly box: Box,
  ) {
    this.lodCount = lodCount;
    const n = inst.count;
    this.buffers = Array.from({ length: lodCount }, () => new Float32Array(n * 16));
    this.counts = new Int32Array(lodCount);
    this.dirty = new Array<boolean>(lodCount).fill(false);
    this.lodOf = new Int8Array(n).fill(UNASSIGNED);
    this.slot = new Int32Array(n);
    this.idAt = Array.from({ length: lodCount }, () => new Int32Array(n));
  }

  /** Forget the cached state (quality or threshold change): the next update re-classifies everything. */
  invalidate(): void {
    this.lastX = NaN;
    this.uniformLod = UNASSIGNED;
  }

  lodOfInstance(i: number): number {
    return this.lodOf[i]!;
  }

  private band(d2: number, thr2: readonly number[]): number {
    for (let l = 0; l < thr2.length; l++) if (d2 < thr2[l]!) return l;
    return HIDDEN;
  }

  private remove(i: number): void {
    const l = this.lodOf[i]!;
    if (l < 0) return;
    const s = this.slot[i]!;
    const last = this.counts[l]! - 1;
    if (s !== last) {
      const j = this.idAt[l]![last]!;
      this.buffers[l]!.copyWithin(s * 16, last * 16, last * 16 + 16);
      this.idAt[l]![s] = j;
      this.slot[j] = s;
    }
    this.counts[l] = last;
    this.dirty[l] = true;
  }

  private place(i: number, l: number): void {
    this.lodOf[i] = l;
    if (l < 0) return;
    const s = this.counts[l]!;
    writeInstanceMatrix(this.buffers[l]!, s * 16, this.inst, i);
    this.idAt[l]![s] = i;
    this.slot[i] = s;
    this.counts[l] = s + 1;
    this.dirty[l] = true;
  }

  /**
   * Classify instances for camera (cx, cy, cz). `thr` = ascending upper distance bound of every visible LOD (instances
   * beyond the last are hidden); `rankLimit` thins the set (instance shown iff rank < rankLimit).
   * Returns the number of instances that changed LOD, or -1 if the call was skipped (camera hardly moved).
   */
  update(cx: number, cy: number, cz: number, thr: readonly number[], rankLimit: number): number {
    const thrKey = thr.join(',');
    const unchanged = thrKey === this.lastThrKey && rankLimit === this.lastRankLimit;
    const dx0 = cx - this.lastX;
    const dy0 = cy - this.lastY;
    const dz0 = cz - this.lastZ;
    if (unchanged && dx0 * dx0 + dy0 * dy0 + dz0 * dz0 < REEVAL_MOVE_M * REEVAL_MOVE_M) return -1;

    const thr2 = thr.map((t) => t * t);
    if (unchanged && this.uniformLod !== UNASSIGNED) {
      // Whole-box shortcut: nearest and farthest box point in the same band that every eligible instance occupies.
      const b = this.box;
      const nx = Math.max(b.minX - cx, 0, cx - b.maxX);
      const ny = Math.max(b.minY - cy, 0, cy - b.maxY);
      const nz = Math.max(b.minZ - cz, 0, cz - b.maxZ);
      const fx = Math.max(Math.abs(cx - b.minX), Math.abs(cx - b.maxX));
      const fy = Math.max(Math.abs(cy - b.minY), Math.abs(cy - b.maxY));
      const fz = Math.max(Math.abs(cz - b.minZ), Math.abs(cz - b.maxZ));
      const lo = this.band(nx * nx + ny * ny + nz * nz, thr2);
      const hi = this.band(fx * fx + fy * fy + fz * fz, thr2);
      if (lo === hi && lo === this.uniformLod) {
        this.lastX = cx;
        this.lastY = cy;
        this.lastZ = cz;
        return 0;
      }
    }

    const inst = this.inst;
    let moves = 0;
    let allSame = true;
    let common = UNASSIGNED; // band shared by all rank-eligible instances so far
    for (let i = 0; i < inst.count; i++) {
      let nl: number;
      if (inst.rank[i]! >= rankLimit) {
        nl = HIDDEN;
      } else {
        const dx = inst.pos[i * 3]! - cx;
        const dy = inst.pos[i * 3 + 1]! - cy;
        const dz = inst.pos[i * 3 + 2]! - cz;
        nl = this.band(dx * dx + dy * dy + dz * dz, thr2);
        if (common === UNASSIGNED) common = nl;
        else if (common !== nl) allSame = false;
      }
      if (this.lodOf[i]! !== nl) {
        this.remove(i);
        this.place(i, nl);
        moves++;
      }
    }
    this.uniformLod = allSame ? common : UNASSIGNED;
    this.lastX = cx;
    this.lastY = cy;
    this.lastZ = cz;
    this.lastRankLimit = rankLimit;
    this.lastThrKey = thrKey;
    return moves;
  }
}
