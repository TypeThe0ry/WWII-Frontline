/**
 * Reconciliation (ARCHITECTURE 6.4, R5). The server only sends a position (`pos_corr`) when the client's
 * `pred_hash` disagreed; that position is the authoritative one right after the command `ack_input_seq`.
 *
 *   err0 = |predicted position after ack - authoritative|
 *   err0 <= 0.05 m : keep the prediction (no rollback, no replay)
 *   otherwise      : roll back to the authoritative position, replay every still-unacknowledged command,
 *                    err = |old predicted position now - new one|
 *                    err <  1.0 m : visual smoothing (render offset, 100 ms, tau 40 ms)
 *                    err >= 1.0 m : snap
 *
 * Smoothing only moves the *render* position and camera; the simulation state is never touched.
 */
import type { StateRef, UnackedRing } from './unacked-ring';

export const RECONCILE_THRESHOLD_M = 0.05;
export const SNAP_DISTANCE_M = 1.0;
export const SMOOTH_MS = 100;
export const SMOOTH_TAU_MS = 40;

/** What the reconciler needs from the prediction engine. */
export interface ReplayPort {
  readonly x: number;
  readonly y: number;
  readonly z: number;
  reset(auth: Uint8Array, predTick: number, weapon: Uint8Array | undefined): void;
  /** Re-simulates one command (events must be suppressed by the caller's wiring). */
  replay(cmd: Uint8Array): void;
  /** Stores the state after the last step in `e`. */
  capture(e: StateRef): void;
  /** Rewrites `auth` (in place) to a fresh life at the given position; returns that life's starting weapon state. */
  freshLife(auth: Uint8Array, x: number, y: number, z: number): Uint8Array | undefined;
}

/**
 * Render-space correction offset. After a rollback `offset = old_pos - new_pos` is added and decays to
 * exactly zero over `SMOOTH_MS`: `w(t) = (e^(-t/tau) - e^(-T/tau)) / (1 - e^(-T/tau))` (exponential with
 * tau = 40 ms, shifted so that it reaches 0 at T = 100 ms; the pure exponential would still leave 8.2 %).
 */
export class RenderSmoother {
  private ox = 0;
  private oy = 0;
  private oz = 0;
  private t = SMOOTH_MS;

  constructor(
    readonly durationMs = SMOOTH_MS,
    readonly tauMs = SMOOTH_TAU_MS,
  ) {}

  private weight(t: number): number {
    if (t >= this.durationMs) return 0;
    const a = Math.exp(-this.durationMs / this.tauMs);
    return (Math.exp(-t / this.tauMs) - a) / (1 - a);
  }

  get x(): number {
    return this.ox * this.weight(this.t);
  }
  get y(): number {
    return this.oy * this.weight(this.t);
  }
  get z(): number {
    return this.oz * this.weight(this.t);
  }
  get magnitude(): number {
    return Math.hypot(this.x, this.y, this.z);
  }
  get active(): boolean {
    return this.t < this.durationMs;
  }

  /** Adds a correction on top of whatever is still decaying; restarts the decay. */
  add(dx: number, dy: number, dz: number): void {
    const w = this.weight(this.t);
    this.ox = this.ox * w + dx;
    this.oy = this.oy * w + dy;
    this.oz = this.oz * w + dz;
    this.t = 0;
  }

  advance(dtMs: number): void {
    if (this.t < this.durationMs && dtMs > 0) this.t += dtMs;
  }

  clear(): void {
    this.ox = this.oy = this.oz = 0;
    this.t = this.durationMs;
  }
}

export type ReconcileKind =
  /** no correction in this snapshot */
  | 'none'
  /** correction within tolerance: prediction kept */
  | 'ok'
  /** rolled back, replayed, smoothed */
  | 'corrected'
  /** rolled back, replayed, error >= 1 m: no smoothing */
  | 'snapped'
  /** first placement of the player (spawn): not counted as a correction */
  | 'initial'
  /** the acknowledged seq is unknown to the ring */
  | 'unknown';

export interface ReconcileResult {
  kind: ReconcileKind;
  /** |prediction at ack - authoritative| (m); NaN when there was nothing to compare. */
  err0: number;
  /** |old - new predicted position now| after a rollback (m), else 0. */
  err: number;
}

export interface ReconcileStats {
  /** Rollbacks (including snaps; excluding the initial placement). */
  corrections: number;
  snaps: number;
  /** `err` of the latest rollback. */
  lastErrM: number;
  maxErrM: number;
  /** Corrections within tolerance (prediction kept). */
  toleranceKept: number;
  initialSyncs: number;
  unknownAcks: number;
  replayedCommands: number;
}

export class Reconciler {
  readonly stats: ReconcileStats = { corrections: 0, snaps: 0, lastErrM: 0, maxErrM: 0, toleranceKept: 0, initialSyncs: 0, unknownAcks: 0, replayedCommands: 0 };
  readonly smoother = new RenderSmoother();
  /** `false` until the server has told us where the player is. */
  spawned = false;
  /** The player is dead: no new life can start (a jump is not a respawn) and the replay does not simulate. */
  frozen = false;
  private readonly result: ReconcileResult = { kind: 'none', err0: NaN, err: 0 };

  constructor(
    private readonly ring: UnackedRing,
    private readonly port: ReplayPort,
    readonly thresholdM = RECONCILE_THRESHOLD_M,
    readonly snapM = SNAP_DISTANCE_M,
  ) {}

  /**
   * Processes the `self` block of one snapshot: drops acknowledged commands and, when the server sent a
   * position, compares and (if needed) rolls back. `corr` is `[x, y, z]` metres or `null`.
   * The returned object is reused between calls.
   */
  onSelf(ackSeq: number, hasCorr: boolean, cx: number, cy: number, cz: number): ReconcileResult {
    const r = this.result;
    r.kind = 'none';
    r.err0 = NaN;
    r.err = 0;
    const cls = this.ring.classify(ackSeq);
    if (cls === 'future') {
      this.stats.unknownAcks++;
      r.kind = 'unknown';
      return r;
    }
    if (cls === 'stale') return r; // older than what we already know (reordered snapshot)

    // `ref` lives in the ring (base or an entry), so copy what is needed before discarding.
    const ref = this.ring.stateAt(ackSeq) as StateRef;
    const refX = ref.x;
    const refY = ref.y;
    const refZ = ref.z;
    this.ring.discardUpTo(ackSeq); // `base` is now the state at ackSeq
    if (!hasCorr) return r;

    const err0 = Math.hypot(refX - cx, refY - cy, refZ - cz);
    r.err0 = err0;
    if (err0 <= this.thresholdM) {
      r.kind = 'ok';
      this.stats.toleranceKept++;
      return r;
    }
    return this.rollback(cx, cy, cz, r, err0);
  }

  /** Rolls back to the authoritative position at `base` and replays the unacknowledged commands. */
  private rollback(cx: number, cy: number, cz: number, r: ReconcileResult, err0: number): ReconcileResult {
    const port = this.port;
    const base = this.ring.base;
    const oldX = port.x;
    const oldY = port.y;
    const oldZ = port.z;
    // Authoritative position, predicted everything else (velocity, stance, weapon) at that tick.
    const dv = new DataView(base.auth.buffer, base.auth.byteOffset, base.auth.byteLength);
    dv.setFloat32(0, cx, true);
    dv.setFloat32(4, cy, true);
    dv.setFloat32(8, cz, true);
    base.x = dv.getFloat32(0, true);
    base.y = dv.getFloat32(4, true);
    base.z = dv.getFloat32(8, true);
    let weapon = base.weaponLen ? base.weapon : undefined;
    if (this.spawned && !this.frozen && err0 >= this.snapM) {
      // A teleport while alive is a respawn: the server starts the new life from a fresh state (headless-client
      // does the same), so the old life's velocity / stance / weapon must not leak into the replay.
      const w = port.freshLife(base.auth, base.x, base.y, base.z);
      if (w && w.length === base.weapon.length) {
        base.weapon.set(w);
        base.weaponLen = w.length;
        weapon = base.weapon;
      }
    }
    port.reset(base.auth, base.predTick, weapon);
    const n = this.ring.length;
    for (let i = 0; i < n; i++) {
      const e = this.ring.at(i);
      port.replay(e.cmd);
      port.capture(e);
    }
    this.stats.replayedCommands += n;
    const err = Math.hypot(oldX - port.x, oldY - port.y, oldZ - port.z);
    r.err = err;
    if (!this.spawned) {
      this.spawned = true;
      this.stats.initialSyncs++;
      this.smoother.clear();
      r.kind = 'initial';
      return r;
    }
    this.stats.corrections++;
    this.stats.lastErrM = err;
    if (err > this.stats.maxErrM) this.stats.maxErrM = err;
    if (err < this.snapM) {
      this.smoother.add(oldX - port.x, oldY - port.y, oldZ - port.z);
      r.kind = 'corrected';
    } else {
      this.stats.snaps++;
      this.smoother.clear();
      r.kind = 'snapped';
    }
    return r;
  }

  /** Position at the acknowledged command (for the next `pred_hash`). */
  get baseSeq(): number {
    return this.ring.base.seq;
  }
}
