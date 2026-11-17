import { describe, expect, it } from 'vitest';
import { ENTITY_TIMEOUT_MS, EntityStore, MAX_EXTRAPOLATION_MS, RenderTimeline, RING, TELEPORT_M, shortestArc } from './interpolation';
import { mulberry32 } from './test-harness';

const TM = 1000 / 30;
const out = (): number[] => [0, 0, 0, 0, 0];

describe('EntityStore sampling', () => {
  it('linear position, shortest-arc yaw', () => {
    const s = new EntityStore();
    s.push(5, 10, 0, 0, 0, 6.0, 0, 0);
    s.push(5, 12, 4, 2, 8, 0.2, 0.4, 0);
    const o = out();
    expect(s.sample(5, 11, TM, o)).toBe(0);
    expect(o[0]).toBeCloseTo(2, 6);
    expect(o[1]).toBeCloseTo(1, 6);
    expect(o[2]).toBeCloseTo(4, 6);
    expect(o[4]).toBeCloseTo(0.2, 6);
    // 6.0 -> 0.2 goes through 2*pi (short way, +0.483), not backwards through 3.14
    const mid = o[3] as number;
    expect(shortestArc(6.0, 0.2)).toBeCloseTo(0.2 + 2 * Math.PI - 6.0, 6);
    expect(mid).toBeCloseTo(6.0 + (0.2 + 2 * Math.PI - 6.0) / 2, 5);
  });

  it('out-of-order and duplicate samples are placed by tick; the ring keeps the newest RING samples', () => {
    const s = new EntityStore();
    for (const t of [10, 14, 12, 12, 16]) s.push(1, t, t, 0, 0, 0, 0, 0);
    expect(s.samples(1)).toBe(4);
    const o = out();
    s.sample(1, 13, TM, o);
    expect(o[0]).toBeCloseTo(13, 6);
    for (let t = 20; t < 20 + RING * 3; t++) s.push(1, t, t, 0, 0, 0, 0, 0);
    expect(s.samples(1)).toBe(RING);
    s.push(1, 5, 5, 0, 0, 0, 0, 0); // older than the whole buffer
    expect(s.droppedStale).toBe(1);
  });

  it('extrapolates with the last velocity for at most 150 ms, then holds', () => {
    const s = new EntityStore();
    s.push(1, 0, 0, 0, 0, 0, 0, 0);
    s.push(1, 3, 3, 0, 0, 0, 0, 0); // 1 m per tick
    const o = out();
    expect(s.sample(1, 5, TM, o)).toBe(1);
    expect(o[0]).toBeCloseTo(5, 6);
    const limit = MAX_EXTRAPOLATION_MS / TM; // 4.5 ticks
    expect(s.sample(1, 3 + limit + 3, TM, o)).toBe(2);
    expect(o[0]).toBeCloseTo(3 + limit, 6);
    expect(s.sample(1, 1000, TM, o)).toBe(2);
    expect(o[0]).toBeCloseTo(3 + limit, 6);
  });

  it('single sample holds; before the first sample shows the first pose; teleports do not interpolate', () => {
    const s = new EntityStore();
    s.push(1, 10, 7, 1, 7, 0, 0, 0);
    const o = out();
    expect(s.sample(1, 20, TM, o)).toBe(2);
    expect(o[0]).toBe(7);
    expect(s.sample(1, 5, TM, o)).toBe(3);
    expect(o[0]).toBe(7);
    s.push(1, 12, 7 + TELEPORT_M + 5, 1, 7, 0, 0, 0);
    s.sample(1, 11, TM, o);
    expect(o[0]).toBe(7); // held at the old pose until the new sample's tick
    s.sample(1, 12.5, TM, o);
    expect(o[0]).toBeCloseTo(7 + TELEPORT_M + 5, 5);
  });

  it('time-outs: > 1.5 s without an update removes the entity; remove() works and reports', () => {
    const s = new EntityStore();
    expect(s.push(2, 1, 0, 0, 0, 0, 0, 1000)).toBe(true);
    expect(s.push(2, 2, 0, 0, 0, 0, 0, 1100)).toBe(false);
    expect(s.push(3, 2, 0, 0, 0, 0, 0, 2000)).toBe(true);
    const gone: number[] = [];
    s.expire(1100 + ENTITY_TIMEOUT_MS, (id) => gone.push(id));
    expect(gone).toEqual([]);
    s.expire(1100 + ENTITY_TIMEOUT_MS + 1, (id) => gone.push(id));
    expect(gone).toEqual([2]);
    expect(s.count).toBe(1);
    expect(s.remove(3)).toBe(true);
    expect(s.remove(3)).toBe(false);
    expect(s.count).toBe(0);
  });
});

describe('interpolation quality: 10 entities, 20 Hz snapshots, jitter, 2 % loss, 60 fps for 60 s', () => {
  it('P99 of |dpos - v*dt| / (v*dt) <= 5 % and zero backward steps', () => {
    const rnd = mulberry32(42);
    const N = 10;
    const vel = Array.from({ length: N }, (_, i) => 1.5 + i * 0.7); // m/s along +x
    const store = new EntityStore();
    const tl = new RenderTimeline(TM);
    // snapshot schedule: server ticks 0,1,_,3,4,_ ... (20 Hz); network delay 50 ms + U(0, 20 ms) jitter; 2 % lost
    const arrivals: { at: number; tick: number }[] = [];
    for (let tick = 0; tick < 30 * 62; tick++) {
      if (tick % 3 === 2) continue;
      if (rnd() < 0.02) continue;
      arrivals.push({ at: tick * TM + 50 + rnd() * 20, tick });
    }
    arrivals.sort((a, b) => a.at - b.at);
    let ai = 0;
    const FRAME = 1000 / 60;
    const prev = new Float64Array(N).fill(NaN);
    const errs: number[] = [];
    let backward = 0;
    let noiseTick = 0;
    const o = out();
    let t = 0;
    for (let f = 0; f < 3600; f++) {
      t += FRAME;
      while (ai < arrivals.length && (arrivals[ai] as { at: number }).at <= t) {
        const { tick } = arrivals[ai] as { tick: number };
        for (let i = 0; i < N; i++) store.push(i, tick, (vel[i] as number) * ((tick * TM) / 1000), 0, 0, 0, 0, t);
        ai++;
      }
      // clock estimate: exact, with a +-0.3 tick step noise once per second (what Pong re-selection does)
      if (f % 60 === 0) noiseTick = (rnd() - 0.5) * 0.6;
      tl.update(FRAME, t / TM + noiseTick - 3);
      if (f < 120) continue; // warm-up: buffers fill, render clock settles
      for (let i = 0; i < N; i++) {
        store.sample(i, tl.tick, TM, o);
        const x = o[0] as number;
        const p = prev[i] as number;
        if (!Number.isNaN(p)) {
          if (x < p - 1e-6) backward++;
          const expected = (vel[i] as number) * (FRAME / 1000);
          errs.push(Math.abs(x - p - expected) / expected);
        }
        prev[i] = x;
      }
    }
    errs.sort((a, b) => a - b);
    const p99 = errs[Math.floor(errs.length * 0.99)] as number;
    console.log(`[interp] samples ${errs.length}, P50 ${(errs[errs.length >> 1] as number * 100).toFixed(2)} % P99 ${(p99 * 100).toFixed(2)} % max ${(errs[errs.length - 1] as number * 100).toFixed(2)} %, backward steps ${backward}, render-clock resyncs ${tl.resyncs}`);
    expect(backward).toBe(0);
    expect(p99).toBeLessThanOrEqual(0.05);
  });
});

describe('RenderTimeline', () => {
  it('advances with real time, slews (not jumps) towards the target, resyncs when far off', () => {
    const tl = new RenderTimeline(TM);
    expect(tl.update(16, NaN)).toBeNaN();
    tl.update(16, 100);
    expect(tl.tick).toBe(100);
    // target 0.5 tick ahead: a small, bounded correction per frame
    const t0 = tl.tick;
    tl.update(TM, t0 + 1 + 0.5);
    expect(tl.tick - t0).toBeGreaterThan(1);
    expect(tl.tick - t0).toBeLessThanOrEqual(1 + 0.03 + 1e-9);
    tl.update(16, 500);
    expect(tl.tick).toBe(500);
    expect(tl.resyncs).toBe(1);
  });
});
