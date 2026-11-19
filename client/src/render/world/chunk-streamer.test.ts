import { describe, expect, it, vi } from 'vitest';
import { ChunkStreamer, type ChunkCoord, type ChunkStreamerOptions, type PrefetchProgress } from './chunk-streamer';
import { CHUNK_M, R_NEAR, UPLOAD_BUDGET_BYTES } from './chunk-streamer-core';

const visible = () => true;
const still = { x: 0, z: 0 };
const position = (bx: number, bz: number) => ({ x: (bx + 0.5) * CHUNK_M, z: (bz + 0.5) * CHUNK_M });
const flush = async () => { await Promise.resolve(); await Promise.resolve(); };
const payload = (data: ChunkCoord, uploadBytes = 120_000) => ({ data, uploadBytes });

function fixture(overrides: Partial<ChunkStreamerOptions<ChunkCoord>> = {}) {
  let now = 0;
  const load = vi.fn(async (c: ChunkCoord) => payload(c));
  const upload = vi.fn<ChunkStreamerOptions<ChunkCoord>['upload']>(() => { now += 0.1; });
  const evict = vi.fn();
  const progress: PrefetchProgress[] = [];
  const streamer = new ChunkStreamer({ load, upload, evict, hasFallback: visible, now: () => now, onProgress: (e) => progress.push(e), ...overrides });
  const frame = (bx = 16, bz = 16, dt = 1 / 60) => streamer.update(position(bx, bz), still, visible, dt);
  const drain = async (count: number, bx?: number, bz?: number) => {
    for (let i = 0; i < count; i++) { frame(bx, bz); await flush(); }
  };
  return { streamer, load, upload, evict, progress, frame, drain, advanceClock: (ms: number) => { now += ms; } };
}

describe('world chunk streamer', () => {
  it('prefetches a remote 81-chunk spawn before local work and reports progress through upload', async () => {
    const f = fixture({ radius: 0, headingPrefetch: false });
    f.frame(1, 1);
    const complete = f.streamer.prefetchAround(position(24, 24));
    expect(f.progress).toEqual([{ type: 'stream.progress', total: 81, ready: 0, failed: 0 }]);
    expect(f.load.mock.calls).toHaveLength(6);
    expect(f.load.mock.calls[1]?.[0]).toMatchObject({ bx: 24, bz: 24 });
    await flush();
    expect(f.streamer.getState(24, 24)).toBe('uploading');
    await f.drain(20, 1, 1);
    await expect(complete).resolves.toMatchObject({ total: 81, ready: 81, failed: 0, cancelled: false });
    expect(f.progress.at(-1)?.ready).toBe(81);
    expect(f.progress.every((e, i) => i === 0 || e.ready > f.progress[i - 1]!.ready)).toBe(true);
    const calls = f.load.mock.calls.length;
    await expect(f.streamer.prefetchAround(position(24, 24))).resolves.toMatchObject({ ready: 81, elapsedMs: 0 });
    expect(f.load.mock.calls).toHaveLength(calls);
  });

  it('supersedes spawn requests, cancels stale I/O and discards its late completion', async () => {
    const signals: AbortSignal[] = [];
    let finish!: () => void;
    const f = fixture({ radius: 0, hasChunk: ({ bx }) => bx >= 20, load: (c, signal) => {
      signals.push(signal);
      if (c.bx !== 24) return Promise.resolve(payload(c));
      return new Promise((resolve) => { finish = () => resolve(payload(c)); });
    } });
    f.frame(1, 1);
    const first = f.streamer.prefetchAround(position(24, 24), { ring: 0 });
    const next = f.streamer.prefetchAround(position(30, 30), { ring: 0 });
    expect(signals[0]?.aborted).toBe(true);
    finish();
    await f.drain(3, 1, 1);
    await expect(first).resolves.toMatchObject({ cancelled: true });
    await expect(next).resolves.toMatchObject({ cancelled: false, ready: 1 });
    expect(f.streamer.getState(24, 24)).toBe('unloaded');
    expect(f.upload.mock.calls.map(([c]) => c.bx)).toEqual([30]);
    f.streamer.dispose();
    f.streamer.dispose();
    expect(f.evict).toHaveBeenCalledTimes(1);
  });

  it('limits procedural loading and evicts immediately outside retainRadius without pressure', async () => {
    const f = fixture({ radius: R_NEAR, retainRadius: R_NEAR });
    const east = { x: 10, z: 0 };
    f.streamer.update(position(16, 16), east, visible, 0);
    await flush();
    await f.drain(17);
    expect(f.load).toHaveBeenCalledTimes(81);
    expect(f.streamer.metrics.stream_resident).toBe(81);
    f.frame(17, 16);
    expect(f.evict).toHaveBeenCalledTimes(9);
    expect(f.streamer.getState(12, 16)).toBe('unloaded');
    expect(f.streamer.getState(13, 16)).toBe('resident');
    await f.drain(4, 17, 16);
    expect(f.streamer.metrics.stream_resident).toBe(81);
    expect(f.load).toHaveBeenCalledTimes(90);
  });

  it('keeps far visuals below the soft budget and evicts farthest first when over budget', async () => {
    const indices = new Set([4, 6, 7, 8]);
    const f = fixture({ hasChunk: ({ bx, bz }) => indices.has(bx) && bz === 4, softBudgetBytes: 370_000 });
    await f.drain(4, 4, 4);
    expect(f.streamer.metrics.stream_resident).toBe(4); // Over budget, all still within the retention ring.
    f.frame(14, 4);
    expect(f.evict.mock.calls.map(([c]) => c.bx)).toEqual([4]);
    expect(f.streamer.getState(8, 4)).toBe('resident'); // d=6 still protected.
    await f.drain(5, 14, 4);
    expect(f.load.mock.calls.filter(([c]) => c.bx === 4)).toHaveLength(1);

    const below = fixture({ hasChunk: ({ bx, bz }) => bx === 4 && bz === 4, softBudgetBytes: 200_000 });
    await below.drain(3, 4, 4);
    below.frame(14, 4);
    expect(below.evict).not.toHaveBeenCalled();
  });

  it('uploads FIFO by load completion, adapts to measured time, and caps each normal batch by bytes', async () => {
    const finish = new Map<number, () => void>();
    const uploaded: number[] = [];
    const f = fixture({ radius: 4, hasChunk: ({ bx, bz }) => bx >= 16 && bx <= 18 && bz === 16,
      load: (c) => new Promise((resolve) => { finish.set(c.bx, () => resolve(payload(c, 900_000))); }),
      upload: (c) => { uploaded.push(c.bx); f.advanceClock(1.6); },
    });
    f.frame();
    for (const bx of [18, 16, 17]) finish.get(bx)!();
    await flush();
    f.frame();
    expect(uploaded).toEqual([18]);
    expect(f.streamer.metrics).toMatchObject({ stream_upload_ms: 1.6, stream_upload_bytes: 900_000 });
    f.frame();
    expect(uploaded).toEqual([18, 16]);
    f.frame();
    expect(uploaded).toEqual([18, 16, 17]);

    const cheap = fixture({ radius: 2, load: async (c) => payload(c, 1_000_000), upload: () => undefined });
    for (let i = 0; i < 25; i++) {
      cheap.frame();
      expect(cheap.streamer.metrics.stream_upload_bytes).toBeLessThanOrEqual(UPLOAD_BUDGET_BYTES);
      await flush();
    }
    expect(cheap.streamer.metrics.stream_resident).toBe(25);
  });

  it.each(['404', 'timeout'])('limits %s attempts with backoff and uses fallback during failure', async (kind) => {
    let attempts = 0;
    const f = fixture({ radius: 0, loadTimeoutSeconds: 0.125, load: (_c, signal) => {
      attempts++;
      if (kind === '404') return Promise.reject(new Error('HTTP 404'));
      return new Promise((_resolve, reject) => signal.addEventListener('abort', () => reject(signal.reason), { once: true }));
    } });
    const complete = f.streamer.prefetchAround(position(16, 16), { ring: 0 });
    for (let i = 0; i < 4; i++) {
      expect(attempts).toBe(i + 1);
      f.frame(16, 16, 0.125);
      await flush();
      expect(f.streamer.getState(16, 16)).toBe('failed');
      expect(f.streamer.metrics.stream_holes).toBe(0);
      const delay = 0.25 * 2 ** i;
      f.frame(16, 16, delay - 0.125);
      expect(attempts).toBe(i + 1);
      f.frame(16, 16, 0.125);
    }
    await expect(complete).resolves.toMatchObject({ failed: 1, ready: 0, cancelled: false });
    expect(attempts).toBe(4);
  });

  it('keeps collision independent of visible state, bounds shared concurrency and gates until install', async () => {
    const install = vi.fn();
    const remove = vi.fn();
    let active = 0;
    let high = 0;
    const loads = async <T>(value: T) => {
      active++;
      high = Math.max(high, active);
      await Promise.resolve();
      active--;
      return value;
    };
    const f = fixture({ radius: 0, headingPrefetch: false, load: (c) => loads(payload(c)), collision: {
      load: () => loads(new Uint8Array([1])), install, remove,
    } });
    f.frame();
    await flush();
    expect(f.streamer.collisionReady(16, 16)).toBe(false);
    expect(f.streamer.collisionReady(18, 16)).toBe(false);
    await f.drain(7);
    expect(high).toBe(6);
    expect(install).toHaveBeenCalledTimes(25);
    expect(f.streamer.collisionReady(18, 16)).toBe(true);
    expect(f.streamer.getState(18, 16)).toBe('unloaded');
    expect(f.streamer.collisionReady(19, 16)).toBe(false);
    f.frame(20, 16);
    expect(f.streamer.collisionReady(16, 16)).toBe(false);
    expect(remove).toHaveBeenCalledTimes(20);
  });

  it('reports unbaked spawn chunks as failed without requesting assets outside the index', async () => {
    const f = fixture({ radius: 0, hasChunk: ({ bx, bz }) => bx === 0 && bz === 0 });
    const complete = f.streamer.prefetchAround(position(0, 0), { ring: 1 });
    expect(f.progress[0]).toMatchObject({ total: 4, ready: 0, failed: 3 });
    await f.drain(3, 0, 0);
    await expect(complete).resolves.toMatchObject({ total: 4, ready: 1, failed: 3 });
    expect(f.load).toHaveBeenCalledTimes(1);
  });

  it('reports uncovered visible chunks separately from missing detail', () => {
    const f = fixture({ radius: 1, hasChunk: () => false, hasFallback: ({ bx }) => bx !== 16 });
    f.streamer.update(position(16, 16), still, (_bx, bz) => bz === 16, 0);
    expect(f.streamer.metrics.stream_holes).toBe(1);
  });
});
