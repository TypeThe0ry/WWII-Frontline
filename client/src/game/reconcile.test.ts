import { describe, expect, it } from 'vitest';
import { RECONCILE_THRESHOLD_M, Reconciler, RenderSmoother, SMOOTH_MS, SNAP_DISTANCE_M, type ReplayPort } from './reconcile';
import { UnackedRing, type StateRef } from './unacked-ring';

/** A prediction engine whose "physics" is `x += cmd[0] / 100` per command: exact, so errors can be constructed. */
class LinePort implements ReplayPort {
  x = 0;
  y = 0;
  z = 0;
  replays = 0;
  tick = 0;
  resets = 0;
  reset(auth: Uint8Array, predTick: number): void {
    const dv = new DataView(auth.buffer, auth.byteOffset, auth.byteLength);
    this.x = dv.getFloat32(0, true);
    this.y = dv.getFloat32(4, true);
    this.z = dv.getFloat32(8, true);
    this.tick = predTick;
    this.resets++;
  }
  replay(cmd: Uint8Array): void {
    this.x += (cmd[0] as number) / 100;
    this.replays++;
    this.tick++;
  }
  capture(e: StateRef): void {
    e.x = this.x;
    e.y = this.y;
    e.z = this.z;
    e.predTick = this.tick;
    new DataView(e.auth.buffer, e.auth.byteOffset, e.auth.byteLength).setFloat32(0, this.x, true);
    new DataView(e.auth.buffer, e.auth.byteOffset, e.auth.byteLength).setFloat32(4, this.y, true);
    new DataView(e.auth.buffer, e.auth.byteOffset, e.auth.byteLength).setFloat32(8, this.z, true);
  }
}

/** 10 commands of +0.5 m each (seq 1..10), predicted along. Returns the rig. */
const rig = (spawned = true) => {
  const port = new LinePort();
  const ring = new UnackedRing();
  ring.clear(0);
  port.capture(ring.base);
  const rc = new Reconciler(ring, port);
  rc.spawned = spawned;
  for (let s = 1; s <= 10; s++) {
    port.x += 0.5;
    const e = ring.push();
    e.seq = s;
    e.tick = s;
    e.cmd[0] = 50;
    port.capture(e);
  }
  port.replays = 0;
  return { port, ring, rc };
};

describe('reconcile_threshold (R5: 0.05 m / 1.0 m)', () => {
  it('constants', () => {
    expect(RECONCILE_THRESHOLD_M).toBe(0.05);
    expect(SNAP_DISTANCE_M).toBe(1.0);
    expect(SMOOTH_MS).toBe(100);
  });

  it('0.04 m off at the acknowledged command: kept, no rollback, no replay, corrections unchanged', () => {
    const { port, ring, rc } = rig();
    const ref = (ring.stateAt(4) as StateRef).x;
    const r = rc.onSelf(4, true, ref + 0.04, 0, 0);
    expect(r.kind).toBe('ok');
    expect(r.err0).toBeCloseTo(0.04, 6);
    expect(rc.stats.corrections).toBe(0);
    expect(port.replays).toBe(0);
    expect(port.resets).toBe(0);
    expect(ring.length).toBe(6); // 5..10 still unacknowledged
    expect(ring.base.seq).toBe(4);
    expect(rc.smoother.active).toBe(false);
  });

  it('exactly 0.05 m is still tolerated; just above rolls back', () => {
    const a = rig();
    expect(a.rc.onSelf(4, true, (a.ring.stateAt(4) as StateRef).x + 0.05, 0, 0).kind).toBe('ok');
    const b = rig();
    expect(b.rc.onSelf(4, true, (b.ring.stateAt(4) as StateRef).x + 0.0501, 0, 0).kind).toBe('corrected');
  });

  it('0.06 m off: rollback to the authoritative position and replay of all 6 unacknowledged commands', () => {
    const { port, ring, rc } = rig();
    const ref = (ring.stateAt(4) as StateRef).x;
    const before = port.x;
    const r = rc.onSelf(4, true, ref + 0.06, 0, 0);
    expect(r.kind).toBe('corrected');
    expect(rc.stats.corrections).toBe(1);
    expect(port.resets).toBe(1);
    expect(port.replays).toBe(6);
    expect(port.x).toBeCloseTo(before + 0.06, 5); // replay on top of the corrected position
    expect(r.err).toBeCloseTo(0.06, 5);
    expect(rc.stats.snaps).toBe(0);
    // the replayed states were written back into the ring
    expect((ring.newest() as StateRef).x).toBeCloseTo(port.x, 6);
  });

  it('0.5 m after the replay: visual smoothing, offset starts at the full error and is < 5 % after 100 ms', () => {
    const { port, ring, rc } = rig();
    const before = port.x;
    const ref = (ring.stateAt(4) as StateRef).x;
    const r = rc.onSelf(4, true, ref + 0.5, 0, 0);
    expect(r.kind).toBe('corrected');
    expect(r.err).toBeCloseTo(0.5, 5);
    // render position = sim position + offset = where the player was drawn before the correction
    expect(port.x + rc.smoother.x).toBeCloseTo(before, 5);
    expect(rc.stats.snaps).toBe(0);
    const start = Math.abs(rc.smoother.x);
    rc.smoother.advance(40);
    expect(Math.abs(rc.smoother.x) / start).toBeCloseTo(0.311, 2); // tau = 40 ms shaped decay
    rc.smoother.advance(60);
    expect(Math.abs(rc.smoother.x) / start).toBeLessThan(0.05);
    expect(rc.smoother.active).toBe(false);
  });

  it('1.2 m: snap (no smoothing), snaps +1', () => {
    const { port, ring, rc } = rig();
    const ref = (ring.stateAt(4) as StateRef).x;
    const r = rc.onSelf(4, true, ref + 1.2, 0, 0);
    expect(r.kind).toBe('snapped');
    expect(rc.stats.snaps).toBe(1);
    expect(rc.stats.corrections).toBe(1);
    expect(rc.smoother.magnitude).toBe(0);
    expect(rc.stats.lastErrM).toBeCloseTo(1.2, 5);
    expect(port.replays).toBe(6);
  });

  it('just below 1.0 m still smooths, 1.0 m snaps', () => {
    const a = rig();
    expect(a.rc.onSelf(4, true, (a.ring.stateAt(4) as StateRef).x + 0.99, 0, 0).kind).toBe('corrected');
    const b = rig();
    expect(b.rc.onSelf(4, true, (b.ring.stateAt(4) as StateRef).x + 1.0, 0, 0).kind).toBe('snapped');
  });

  it('error is 3D: a vertical 0.06 m difference also triggers', () => {
    const { ring, rc } = rig();
    expect(rc.onSelf(4, true, (ring.stateAt(4) as StateRef).x, 0.06, 0).kind).toBe('corrected');
  });

  it('the first placement is an initial sync, not a correction or a snap', () => {
    const { rc, ring } = rig(false);
    const r = rc.onSelf(0, true, 50, 0, 50); // ack 0 = the initial base
    expect(r.kind).toBe('initial');
    expect(rc.stats.initialSyncs).toBe(1);
    expect(rc.stats.corrections).toBe(0);
    expect(rc.stats.snaps).toBe(0);
    expect(rc.spawned).toBe(true);
    expect(ring.length).toBe(10); // nothing acknowledged yet: all replayed
    expect(rc.smoother.magnitude).toBe(0);
  });

  it('acks: no correction -> only discards; stale ack ignored; future ack is counted and ignored', () => {
    const { rc, ring } = rig();
    expect(rc.onSelf(6, false, 0, 0, 0).kind).toBe('none');
    expect(ring.length).toBe(4);
    expect(rc.onSelf(3, true, 99, 0, 99).kind).toBe('none'); // older than base: reordered snapshot, ignored
    expect(rc.stats.corrections).toBe(0);
    expect(rc.onSelf(60, true, 1, 0, 1).kind).toBe('unknown');
    expect(rc.stats.unknownAcks).toBe(1);
    expect(ring.length).toBe(4);
  });

  it('consecutive corrections accumulate the visible offset (restart of the decay, no jump)', () => {
    const { port, ring, rc } = rig();
    rc.onSelf(2, true, (ring.stateAt(2) as StateRef).x + 0.3, 0, 0);
    rc.smoother.advance(20);
    const visible = port.x + rc.smoother.x;
    rc.onSelf(4, true, (ring.stateAt(4) as StateRef).x + 0.3, 0, 0);
    expect(port.x + rc.smoother.x).toBeCloseTo(visible, 5); // render position is continuous across the second correction
  });
});

describe('RenderSmoother', () => {
  it('decays like exp(-t/40 ms) (shifted to reach 0 at 100 ms) and stays monotonic', () => {
    const s = new RenderSmoother();
    s.add(1, 0, 0);
    expect(s.x).toBeCloseTo(1, 9);
    const w: number[] = [];
    for (let i = 0; i < 8; i++) {
      s.advance(16.7);
      w.push(s.x);
    }
    for (let i = 1; i < w.length; i++) expect(w[i] as number).toBeLessThanOrEqual(w[i - 1] as number);
    expect(w[w.length - 1]).toBe(0);
    const t = new RenderSmoother();
    t.add(1, 0, 0);
    t.advance(40);
    const e = Math.exp(-1);
    const a = Math.exp(-2.5);
    expect(t.x).toBeCloseTo((e - a) / (1 - a), 9);
    // the plain exponential would still leave exp(-2.5) = 8.2 % at 100 ms: the shift removes it
    expect(a).toBeCloseTo(0.082, 3);
  });
});
