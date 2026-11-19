/**
 * Remote vehicles from the snapshot (M1-041c): a small per-id ring of samples interpolated 3 ticks behind
 * (`RenderTimeline`), with velocity-aided extrapolation for at most 150 ms when the ring runs dry.
 * Wire ids 128..255 are vehicles (`VEHICLE_ID_BASE`); players never use them.
 */
import { MAX_EXTRAPOLATION_MS } from './interpolation';

export const VEHICLE_ID_BASE = 128;
/** Floats per vehicle in `WasmSnapshotDecoder.vehicleState()` (mirror of crates/sim-wasm snapshot.rs `VEHICLE_STRIDE`). */
export const VEHICLE_STRIDE = 12;
/** Bytes per vehicle in `vehicleMeta()` (`seats, damage_flags, health_q, flags`). */
export const VEHICLE_META_BYTES = 4;

const RING = 6;
const TELEPORT_M = 25;

export interface VehiclePose {
  x: number;
  y: number;
  z: number;
  /** Unit quaternion x y z w. */
  qx: number;
  qy: number;
  qz: number;
  qw: number;
  /** Turret yaw relative to the hull, radians [0, 2pi). */
  turretYaw: number;
  /** Gun pitch, radians. */
  gunPitch: number;
}

interface Sample {
  tick: number;
  s: Float32Array; // VEHICLE_STRIDE floats
}

class Track {
  readonly samples: Sample[] = [];
  seats = 0;
  damage = 0;
  health = 0;
  flags = 0;
  lastMs = 0;
}

export class RemoteVehicleStore {
  private readonly tracks = new Map<number, Track>();

  get count(): number {
    return this.tracks.size;
  }

  has(id: number): boolean {
    return this.tracks.has(id);
  }

  ids(): number[] {
    return [...this.tracks.keys()];
  }

  /** Ingests the vehicle arrays of one decoded snapshot. Returns the ids that appeared for the first time. */
  push(tick: number, nowMs: number, ids: Uint8Array, state: Float32Array, meta: Uint8Array): number[] {
    const fresh: number[] = [];
    for (let i = 0; i < ids.length; i++) {
      const id = ids[i] as number;
      let t = this.tracks.get(id);
      if (!t) {
        t = new Track();
        this.tracks.set(id, t);
        fresh.push(id);
      }
      const last = t.samples[t.samples.length - 1];
      if (last && tick <= last.tick) continue; // stale or duplicate
      const s = new Float32Array(VEHICLE_STRIDE);
      s.set(state.subarray(i * VEHICLE_STRIDE, (i + 1) * VEHICLE_STRIDE));
      t.samples.push({ tick, s });
      if (t.samples.length > RING) t.samples.shift();
      t.seats = meta[i * VEHICLE_META_BYTES] as number;
      t.damage = meta[i * VEHICLE_META_BYTES + 1] as number;
      t.health = meta[i * VEHICLE_META_BYTES + 2] as number;
      t.flags = meta[i * VEHICLE_META_BYTES + 3] as number;
      t.lastMs = nowMs;
    }
    return fresh;
  }

  remove(id: number): boolean {
    return this.tracks.delete(id);
  }

  meta(id: number): { seats: number; damage: number; health: number; flags: number } | undefined {
    const t = this.tracks.get(id);
    return t && { seats: t.seats, damage: t.damage, health: t.health, flags: t.flags };
  }

  /** Pose at continuous server tick `rt`; false when the id is unknown. */
  sample(id: number, rt: number, tickMs: number, out: VehiclePose): boolean {
    const t = this.tracks.get(id);
    if (!t || t.samples.length === 0) return false;
    const a = t.samples;
    let j = 0;
    while (j < a.length && (a[j] as Sample).tick <= rt) j++;
    if (j === 0) {
      write(out, (a[0] as Sample).s);
      return true;
    }
    if (j < a.length) {
      const s0 = a[j - 1] as Sample;
      const s1 = a[j] as Sample;
      if (Math.hypot((s1.s[0] as number) - (s0.s[0] as number), (s1.s[2] as number) - (s0.s[2] as number)) > TELEPORT_M) {
        write(out, s0.s);
        return true;
      }
      lerp(out, s0.s, s1.s, (rt - s0.tick) / (s1.tick - s0.tick));
      return true;
    }
    // Ring ran dry: extrapolate with the sampled velocity for at most MAX_EXTRAPOLATION_MS.
    const last = a[a.length - 1] as Sample;
    write(out, last.s);
    const ageMs = Math.min((rt - last.tick) * tickMs, MAX_EXTRAPOLATION_MS);
    const k = ageMs / 1000;
    out.x += (last.s[3] as number) * k;
    out.y += (last.s[4] as number) * k;
    out.z += (last.s[5] as number) * k;
    return true;
  }

  /** Drops vehicles silent for more than `timeoutMs`; calls `cb(id)` for each. */
  expire(nowMs: number, timeoutMs: number, cb: (id: number) => void): void {
    for (const [id, t] of this.tracks) {
      if (nowMs - t.lastMs > timeoutMs) {
        this.tracks.delete(id);
        cb(id);
      }
    }
  }
}

function write(out: VehiclePose, s: Float32Array): void {
  out.x = s[0] as number;
  out.y = s[1] as number;
  out.z = s[2] as number;
  out.qx = s[6] as number;
  out.qy = s[7] as number;
  out.qz = s[8] as number;
  out.qw = s[9] as number;
  out.turretYaw = s[10] as number;
  out.gunPitch = s[11] as number;
}

const TAU = Math.PI * 2;
function lerpAngle(a: number, b: number, t: number): number {
  let d = (b - a) % TAU;
  if (d > Math.PI) d -= TAU;
  else if (d < -Math.PI) d += TAU;
  return a + d * t;
}

function lerp(out: VehiclePose, s0: Float32Array, s1: Float32Array, t: number): void {
  const m = (i: number): number => (s0[i] as number) + ((s1[i] as number) - (s0[i] as number)) * t;
  out.x = m(0);
  out.y = m(1);
  out.z = m(2);
  // Shortest-path normalised lerp of the hull quaternion.
  const dot = (s0[6] as number) * (s1[6] as number) + (s0[7] as number) * (s1[7] as number) + (s0[8] as number) * (s1[8] as number) + (s0[9] as number) * (s1[9] as number);
  const sg = dot < 0 ? -1 : 1;
  const q = (i: number): number => (s0[i] as number) + (sg * (s1[i] as number) - (s0[i] as number)) * t;
  const qx = q(6);
  const qy = q(7);
  const qz = q(8);
  const qw = q(9);
  const n = Math.hypot(qx, qy, qz, qw) || 1;
  out.qx = qx / n;
  out.qy = qy / n;
  out.qz = qz / n;
  out.qw = qw / n;
  out.turretYaw = lerpAngle(s0[10] as number, s1[10] as number, t);
  out.gunPitch = m(11);
}
