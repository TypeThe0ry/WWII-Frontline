/**
 * Remote entity interpolation (ARCHITECTURE 6.4/6.5): every remote entity keeps its last `RING` snapshot
 * samples (struct of arrays, no per-sample objects); rendering samples them at
 * `render_tick = estimated_server_tick - 3 + alpha` (100 ms behind). Positions are interpolated linearly,
 * yaw along the shortest arc. When the buffer runs dry the entity is extrapolated with its last velocity
 * for at most 150 ms and then held. Entities silent for more than 1.5 s time out.
 */
export const MAX_ENTITIES = 256; // wire entity ids are 8 bit
export const RING = 8;
export const INTERP_DELAY_TICKS = 3;
export const MAX_EXTRAPOLATION_MS = 150;
export const ENTITY_TIMEOUT_MS = 1500;
/** A jump larger than this between two samples is a teleport (respawn): no interpolation across it. */
export const TELEPORT_M = 25;
const TAU = Math.PI * 2;

export const shortestArc = (from: number, to: number): number => {
  const d = (((to - from + Math.PI) % TAU) + TAU) % TAU;
  return d - Math.PI;
};

/** Smooth render clock: advances with real time and slews towards the (jittery) clock estimate. */
export class RenderTimeline {
  /** Continuous server tick being rendered; NaN until the first target. */
  tick = NaN;
  /** Hard-resync when further than this many ticks away. */
  resyncTicks = 6;
  /** Largest rate change (fraction of real time) used to slew towards the target. */
  maxSlew = 0.03;
  resyncs = 0;

  constructor(readonly tickMs: number) {}

  /** `target` = clock estimate of the render time (already minus the interpolation delay). */
  update(dtMs: number, target: number): number {
    if (!Number.isFinite(target)) return this.tick;
    if (!Number.isFinite(this.tick)) {
      this.tick = target;
      return this.tick;
    }
    const adv = dtMs / this.tickMs;
    const err = target - (this.tick + adv);
    if (Math.abs(err) > this.resyncTicks) {
      this.tick = target;
      this.resyncs++;
    } else {
      const lim = this.maxSlew * adv;
      this.tick += adv + Math.max(-lim, Math.min(lim, err));
    }
    return this.tick;
  }
}

export class EntityStore {
  /** Live entity count. */
  count = 0;
  readonly alive = new Uint8Array(MAX_ENTITIES);
  private readonly n = new Uint8Array(MAX_ENTITIES);
  private readonly tick = new Float64Array(MAX_ENTITIES * RING);
  private readonly px = new Float32Array(MAX_ENTITIES * RING);
  private readonly py = new Float32Array(MAX_ENTITIES * RING);
  private readonly pz = new Float32Array(MAX_ENTITIES * RING);
  private readonly yaw = new Float32Array(MAX_ENTITIES * RING);
  private readonly pitch = new Float32Array(MAX_ENTITIES * RING);
  // latest non-interpolated attributes
  readonly anim = new Uint8Array(MAX_ENTITIES);
  readonly stance = new Uint8Array(MAX_ENTITIES);
  readonly weapon = new Uint8Array(MAX_ENTITIES);
  readonly flags = new Uint8Array(MAX_ENTITIES);
  readonly health = new Uint8Array(MAX_ENTITIES);
  private readonly lastUpdateMs = new Float64Array(MAX_ENTITIES);
  /** Samples rejected because they were older than the whole buffer. */
  droppedStale = 0;

  /**
   * Adds a sample. Returns `true` when the entity is new (caller emits `entity.spawned`).
   * Samples may arrive out of order; duplicates of a tick overwrite.
   */
  push(id: number, tick: number, x: number, y: number, z: number, yaw: number, pitch: number, nowMs: number): boolean {
    const isNew = this.alive[id] === 0;
    if (isNew) {
      this.alive[id] = 1;
      this.count++;
      this.n[id] = 0;
    }
    this.lastUpdateMs[id] = nowMs;
    const base = id * RING;
    let n = this.n[id] as number;
    // find insertion point (ascending by tick)
    let at = n;
    while (at > 0 && (this.tick[base + at - 1] as number) > tick) at--;
    if (at > 0 && this.tick[base + at - 1] === tick) {
      this.write(base + at - 1, tick, x, y, z, yaw, pitch);
      return isNew;
    }
    if (n === RING) {
      if (at === 0) {
        this.droppedStale++;
        return isNew;
      }
      // drop the oldest, shift left
      for (let i = 1; i < RING; i++) this.copy(base + i, base + i - 1);
      at--;
      n--;
    }
    for (let i = n; i > at; i--) this.copy(base + i - 1, base + i);
    this.write(base + at, tick, x, y, z, yaw, pitch);
    this.n[id] = n + 1;
    return isNew;
  }

  private write(i: number, tick: number, x: number, y: number, z: number, yaw: number, pitch: number): void {
    this.tick[i] = tick;
    this.px[i] = x;
    this.py[i] = y;
    this.pz[i] = z;
    this.yaw[i] = yaw;
    this.pitch[i] = pitch;
  }

  private copy(from: number, to: number): void {
    this.tick[to] = this.tick[from] as number;
    this.px[to] = this.px[from] as number;
    this.py[to] = this.py[from] as number;
    this.pz[to] = this.pz[from] as number;
    this.yaw[to] = this.yaw[from] as number;
    this.pitch[to] = this.pitch[from] as number;
  }

  setAttrs(id: number, anim: number, stance: number, weapon: number, flags: number, health: number): void {
    this.anim[id] = anim;
    this.stance[id] = stance;
    this.weapon[id] = weapon;
    this.flags[id] = flags;
    this.health[id] = health;
  }

  remove(id: number): boolean {
    if (this.alive[id] === 0) return false;
    this.alive[id] = 0;
    this.n[id] = 0;
    this.count--;
    return true;
  }

  samples(id: number): number {
    return this.n[id] as number;
  }

  /** Ids silent for more than `ENTITY_TIMEOUT_MS`; removes them and calls `cb(id)`. */
  expire(nowMs: number, cb: (id: number) => void): void {
    for (let id = 0; id < MAX_ENTITIES; id++) {
      if (this.alive[id] !== 0 && nowMs - (this.lastUpdateMs[id] as number) > ENTITY_TIMEOUT_MS) {
        this.remove(id);
        cb(id);
      }
    }
  }

  /**
   * Interpolated pose of entity `id` at continuous server tick `rt` written to `out`
   * (x, y, z, yaw, pitch). Returns 0 interpolated, 1 extrapolated, 2 held, 3 before the first sample.
   */
  sample(id: number, rt: number, tickMs: number, out: Float32Array | number[]): 0 | 1 | 2 | 3 {
    const n = this.n[id] as number;
    const b = id * RING;
    if (n === 0) return 3;
    let j = 0;
    while (j < n && (this.tick[b + j] as number) <= rt) j++;
    if (j === 0) {
      this.pose(b, out);
      return 3;
    }
    if (j < n) {
      const i0 = b + j - 1;
      const i1 = b + j;
      const t0 = this.tick[i0] as number;
      const t1 = this.tick[i1] as number;
      if (Math.hypot((this.px[i1] as number) - (this.px[i0] as number), (this.pz[i1] as number) - (this.pz[i0] as number)) > TELEPORT_M) {
        this.pose(i0, out);
        return 2;
      }
      const a = (rt - t0) / (t1 - t0);
      out[0] = (this.px[i0] as number) + ((this.px[i1] as number) - (this.px[i0] as number)) * a;
      out[1] = (this.py[i0] as number) + ((this.py[i1] as number) - (this.py[i0] as number)) * a;
      out[2] = (this.pz[i0] as number) + ((this.pz[i1] as number) - (this.pz[i0] as number)) * a;
      out[3] = (this.yaw[i0] as number) + shortestArc(this.yaw[i0] as number, this.yaw[i1] as number) * a;
      out[4] = (this.pitch[i0] as number) + ((this.pitch[i1] as number) - (this.pitch[i0] as number)) * a;
      return 0;
    }
    // buffer ran dry: extrapolate with the last segment's velocity, at most 150 ms, then hold
    const last = b + n - 1;
    this.pose(last, out);
    if (n < 2) return 2;
    const prev = last - 1;
    const dtS = (this.tick[last] as number) - (this.tick[prev] as number);
    if (dtS <= 0) return 2;
    const dx = (this.px[last] as number) - (this.px[prev] as number);
    const dz = (this.pz[last] as number) - (this.pz[prev] as number);
    if (Math.hypot(dx, dz) > TELEPORT_M) return 2;
    const ahead = Math.min(rt - (this.tick[last] as number), MAX_EXTRAPOLATION_MS / tickMs);
    const k = ahead / dtS;
    out[0] = (out[0] as number) + dx * k;
    out[1] = (out[1] as number) + ((this.py[last] as number) - (this.py[prev] as number)) * k;
    out[2] = (out[2] as number) + dz * k;
    return ahead < rt - (this.tick[last] as number) ? 2 : 1;
  }

  private pose(i: number, out: Float32Array | number[]): void {
    out[0] = this.px[i] as number;
    out[1] = this.py[i] as number;
    out[2] = this.pz[i] as number;
    out[3] = this.yaw[i] as number;
    out[4] = this.pitch[i] as number;
  }
}
