import { describe, it, expect } from 'vitest';
import { bootNext, mapNext, canEnterDeploy, backoffMs, withRetry } from './boot-fsm';
import { probeCapabilities, recommendTier, DEGRADED_FEATURES } from './capabilities';
import { createThrottle } from './shell-progress';
import en from '../../ui/i18n/en.json';

describe('boot fsm', () => {
  it('transitions; unsupported and ready are terminal', () => {
    expect(bootNext('probing', { t: 'probed', backend: 'webgl2' })).toBe('shell_loading');
    expect(bootNext('probing', { t: 'probed', backend: 'none' })).toBe('unsupported');
    expect(bootNext('shell_loading', { t: 'shell_ok' })).toBe('ready');
    expect(bootNext('shell_loading', { t: 'shell_fail' })).toBe('failed');
    expect(bootNext('failed', { t: 'retry' })).toBe('shell_loading');
    expect(bootNext('unsupported', { t: 'retry' })).toBe('unsupported');
    expect(bootNext('ready', { t: 'shell_fail' })).toBe('ready');
  });
  it('map phases advance, cache hit skips download, deploy gated on warm-up', () => {
    let p = mapNext('idle');
    const seen = [p];
    while (p !== 'done') seen.push((p = mapNext(p)));
    expect(seen).toEqual(['check_cache', 'downloading', 'compiling', 'warmup', 'done']);
    expect(mapNext('check_cache', true)).toBe('compiling');
    expect(canEnterDeploy('warmup')).toBe(false);
    expect(canEnterDeploy('done')).toBe(true);
  });
});

describe('retry backoff', () => {
  it('sequence is 1/2/4/8 s then capped', () => {
    expect([0, 1, 2, 3, 4, 9].map(backoffMs)).toEqual([1000, 2000, 4000, 8000, 8000, 8000]);
  });
  it('withRetry sleeps exactly the sequence and returns on success', async () => {
    const sleeps: number[] = [];
    let n = 0;
    const r = await withRetry(async () => { if (++n < 5) throw new Error('x'); return 'ok'; }, { maxAttempts: 6, sleep: async (ms) => void sleeps.push(ms) });
    expect(r).toBe('ok');
    expect(sleeps).toEqual([1000, 2000, 4000, 8000]);
  });
  it('withRetry rethrows after the last attempt', async () => {
    await expect(withRetry(async () => { throw new Error('boom'); }, { maxAttempts: 2, sleep: async () => {} })).rejects.toThrow('boom');
  });
});

describe('capabilities', () => {
  it('webgpu adapter → webgpu, name and storage read', async () => {
    const c = await probeCapabilities({ gpu: { requestAdapter: async () => ({ info: { description: 'RTX' } }) }, hardwareConcurrency: 16, deviceMemory: 8, storage: { estimate: async () => ({ quota: 10 * 1048576 }) } }, () => true);
    expect(c).toMatchObject({ backend: 'webgpu', gpuName: 'RTX', cores: 16, storageQuotaMb: 10, recommendedTier: 'high' });
  });
  it('no adapter or throwing adapter falls back to webgl2, then none', async () => {
    expect((await probeCapabilities({ gpu: { requestAdapter: async () => null } }, () => true)).backend).toBe('webgl2');
    expect((await probeCapabilities({ gpu: { requestAdapter: async () => { throw new Error('x'); } } }, () => true)).backend).toBe('webgl2');
    const none = await probeCapabilities({}, () => false);
    expect(none).toMatchObject({ backend: 'none', recommendedTier: 'low' });
  });
  it('tier rules', () => {
    expect(recommendTier('webgl2', 16, 8)).toBe('medium');
    expect(recommendTier('webgl2', 4, 4)).toBe('low');
    expect(recommendTier('webgpu', 4, 4)).toBe('medium');
    expect(recommendTier('webgpu', 2, 2)).toBe('low');
  });
  it('degraded list has ≥ 3 features, all with en keys (i18n to be added with the screens)', () => {
    expect(DEGRADED_FEATURES.length).toBeGreaterThanOrEqual(3);
    for (const k of DEGRADED_FEATURES) expect(k in en).toBe(true);
  });
});

describe('throttle', () => {
  it('10 Hz: at most one emit per 100 ms, final always passes', () => {
    let t = 0;
    const out: number[] = [];
    const th = createThrottle((v) => out.push(v), 100, () => t);
    for (let i = 0; i <= 50; i++) { t = i * 10; th.push(i / 100); }
    th.push(1, true);
    expect(out.length).toBe(7); // t=0,100,…,500 (6 emits) + the final value
    expect(out[out.length - 1]).toBe(1);
  });
});
