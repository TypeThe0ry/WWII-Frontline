import { describe, expect, it } from 'vitest';
import { chunkOf, headingAhead, MAX_CONCURRENT_FETCH, priority, R_MID, R_NEAR, ringDist, StreamerCore } from './chunk-streamer-core';

const always = () => true;

describe('priority', () => {
  it('formula and ordering', () => {
    expect(priority({ d: 0, inFrustum: false, headingAhead: false, spawnTarget: false })).toBe(1);
    expect(priority({ d: 1, inFrustum: true, headingAhead: true, spawnTarget: true })).toBeCloseTo((1 / 2) * 1.5 * 1.3 * 3);
    expect(priority({ d: 2, inFrustum: false, headingAhead: false, spawnTarget: false })).toBeLessThan(priority({ d: 1, inFrustum: false, headingAhead: false, spawnTarget: false }));
  });
  it('heading cone is +-45 degrees', () => {
    expect(headingAhead(0, 0, 1, 0, 3, 0)).toBe(true); // straight ahead (+x)
    expect(headingAhead(0, 0, 1, 0, 3, 4)).toBe(false); // ~53 deg off
    expect(headingAhead(0, 0, 1, 0, 0, 3)).toBe(false);
    expect(headingAhead(0, 0, 0.1, 0, 3, 0)).toBe(false); // standing still: no heading
  });
});

describe('state machine', () => {
  it('unloaded -> fetching -> uploading -> resident -> unloaded', () => {
    const s = new StreamerCore();
    const q = s.wanted(32, 32, 0, 0, always);
    const f = s.nextFetches(q, 0);
    expect(f.length).toBe(MAX_CONCURRENT_FETCH);
    expect(s.state(f[0]!.bx, f[0]!.bz)).toBe('fetching');
    s.fetched(f[0]!.bx, f[0]!.bz);
    expect(s.state(f[0]!.bx, f[0]!.bz)).toBe('uploading');
    s.uploaded(f[0]!.bx, f[0]!.bz);
    expect(s.state(f[0]!.bx, f[0]!.bz)).toBe('resident');
    s.unload(f[0]!.bx, f[0]!.bz);
    expect(s.state(f[0]!.bx, f[0]!.bz)).toBe('unloaded');
  });

  it('fetch concurrency never exceeds 6', () => {
    const s = new StreamerCore();
    const q = s.wanted(500, 500, 0, 0, always);
    s.nextFetches(q, 0);
    expect(s.nextFetches(q, 0).length).toBe(0);
    expect(s.fetching).toBe(MAX_CONCURRENT_FETCH);
  });

  it('stream_failure_inject: backoff doubles, <=3 retries, then dead without throwing', () => {
    const s = new StreamerCore();
    const q = [{ bx: 5, bz: 5 }];
    let now = 0;
    const retryTimes: number[] = [];
    for (let attempt = 0; attempt < 6; attempt++) {
      const f = s.nextFetches(q, now);
      if (f.length === 0) {
        now += 0.05;
        continue;
      }
      retryTimes.push(now);
      s.failed(5, 5, now);
      now += 0.01;
    }
    // Run the clock forward to let every allowed retry fire.
    for (let i = 0; i < 400; i++) {
      now += 0.05;
      if (s.nextFetches(q, now).length) {
        retryTimes.push(now);
        s.failed(5, 5, now);
      }
    }
    expect(retryTimes.length).toBe(4); // first try + 3 retries
    expect(s.dead(5, 5)).toBe(true);
    expect(s.fetching).toBe(0);
    const gaps = retryTimes.slice(1).map((t, i) => t - (retryTimes[i] as number));
    expect(gaps[1]!).toBeGreaterThan(gaps[0]!); // exponential growth
    expect(gaps[2]!).toBeGreaterThan(gaps[1]!);
  });
});

describe('wanted chunks', () => {
  it('adds one heading ring in the cone, within map bounds', () => {
    const s = new StreamerCore();
    const wanted = s.wanted(16 * 64 + 32, 16 * 64 + 32, 8, 0, always);
    expect(wanted.some((c) => c.bx === 16 + R_MID + 1 && c.bz === 16)).toBe(true);
    expect(wanted.some((c) => c.bx === 16 - R_MID - 1 && c.bz === 16)).toBe(false);
    expect(chunkOf(2047)).toBe(31);
    expect(chunkOf(5000)).toBe(31);
    expect(chunkOf(-5)).toBe(0);
  });

  it('puts every spawn chunk ahead of local chunks, even outside R_MID', () => {
    const s = new StreamerCore();
    s.spawnTarget = { bx: 24, bz: 24 };
    const wanted = s.wanted(4 * 64, 4 * 64, 0, 0, always);
    const count = (2 * R_NEAR + 1) ** 2;
    expect(wanted[0]).toMatchObject({ bx: 24, bz: 24, spawn: true, score: 4.5 });
    expect(wanted.slice(0, count).every((c) => c.spawn && ringDist(c.bx, c.bz, 24, 24) <= R_NEAR)).toBe(true);
    expect(wanted[count]).toMatchObject({ bx: 4, bz: 4, spawn: false });
  });
});
