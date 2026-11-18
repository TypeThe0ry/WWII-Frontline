import { describe, expect, it } from 'vitest';
import { chunkOf, headingAhead, MAX_CONCURRENT_FETCH, priority, R_NEAR, ringDist, StreamerCore, UPLOAD_BUDGET_BYTES } from './chunk-streamer-core';

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
  it('requested -> fetching -> uploading -> resident', () => {
    const s = new StreamerCore();
    const q = s.wanted(32, 32, 0, 0, always);
    const f = s.nextFetches(q, 0);
    expect(f.length).toBe(MAX_CONCURRENT_FETCH);
    expect(s.state(f[0]!.bx, f[0]!.bz)).toBe('fetching');
    s.fetched(f[0]!.bx, f[0]!.bz, 40_000);
    expect(s.state(f[0]!.bx, f[0]!.bz)).toBe('uploading');
    s.uploaded(f[0]!.bx, f[0]!.bz);
    expect(s.state(f[0]!.bx, f[0]!.bz)).toBe('resident');
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

describe('uploads and eviction', () => {
  it('upload batch respects the 2 MB / 1.5 ms budget but always makes progress', () => {
    const s = new StreamerCore();
    const q = s.wanted(1000, 1000, 0, 0, always).slice(0, 6);
    s.nextFetches(q, 0);
    for (const c of q) s.fetched(c.bx, c.bz, 900_000);
    const batch = s.nextUploads();
    expect(batch.length).toBeGreaterThanOrEqual(1);
    expect(batch.length).toBeLessThan(6);
    expect(batch.length * 900_000).toBeLessThanOrEqual(Math.max(UPLOAD_BUDGET_BYTES, 900_000));
    const huge = new StreamerCore();
    huge.nextFetches([{ bx: 1, bz: 1 }], 0);
    huge.fetched(1, 1, 5 * 1048576);
    expect(huge.nextUploads().length).toBe(1); // one oversized chunk still goes through
  });

  it('holes count is 0 once R_near is resident and rises when the player moves on; eviction frees far chunks', () => {
    const s = new StreamerCore();
    const px = 16 * 64 + 10;
    const pz = 16 * 64 + 10;
    const need = s.wanted(px, pz, 0, 0, always).filter((c) => ringDist(c.bx, c.bz, 16, 16) <= R_NEAR);
    expect(s.countHoles(px, pz)).toBe(81);
    for (const c of need) {
      s.nextFetches([c], 0);
      s.fetched(c.bx, c.bz, 1000);
      s.uploaded(c.bx, c.bz);
    }
    expect(s.countHoles(px, pz)).toBe(0);
    // Walk 10 chunks east: the old column is beyond R_near + 2 and gets evicted.
    const evicted = s.evict(px + 10 * 64, pz);
    expect(evicted.length).toBeGreaterThan(0);
    expect(s.countHoles(px + 10 * 64, pz)).toBeGreaterThan(0);
  });

  it('spawn target multiplies the priority of blocks within R_near of it by 3', () => {
    const plain = new StreamerCore();
    const boosted = new StreamerCore();
    boosted.spawnTarget = { bx: 20, bz: 20 };
    const score = (s: StreamerCore) => s.wanted(16 * 64, 16 * 64, 0, 0, always).find((c) => c.bx === 20 && c.bz === 20)!.score;
    expect(score(boosted) / score(plain)).toBeCloseTo(3);
    const far = (s: StreamerCore) => s.wanted(16 * 64, 16 * 64, 0, 0, always).find((c) => c.bx === 12 && c.bz === 12)!.score;
    expect(far(boosted)).toBe(far(plain)); // 8 chunks from the target: no boost
  });

  it('collision readiness gates prediction', () => {
    const s = new StreamerCore();
    expect(s.collisionReady(3, 3)).toBe(false);
    s.nextFetches([{ bx: 3, bz: 3 }], 0);
    expect(s.collisionReady(3, 3)).toBe(false);
    s.fetched(3, 3, 10);
    s.uploaded(3, 3);
    expect(s.collisionReady(3, 3)).toBe(true);
    expect(chunkOf(2047)).toBe(31);
    expect(chunkOf(5000)).toBe(31);
    expect(chunkOf(-5)).toBe(0);
  });
});
