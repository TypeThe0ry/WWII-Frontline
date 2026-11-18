import { describe, expect, it } from 'vitest';
import { cascadeAt, CascadeScheduler, characterCastsIn, csmConfig } from './csm-math';
import { checkProbeBudget, PROBE_BYTES, trilinear, type ProbeGrid } from './probes-math';
import { mixPresets, parsePreset } from './preset';

describe('csm', () => {
  it('WebGPU high is the 4-cascade spec; WebGL2 has 3', () => {
    const h = csmConfig('high', false);
    expect(h.splits).toEqual([12, 40, 120, 350]);
    expect(h.sizes).toEqual([2048, 2048, 2048, 1024]);
    expect(csmConfig('high', true).splits.length).toBe(3);
    expect(csmConfig('medium', false).pcf).toBe(3);
    expect(h.pcf).toBe(5);
  });

  it('cascade lookup and character rule', () => {
    const c = csmConfig('high', false);
    expect([cascadeAt(c, 5), cascadeAt(c, 12), cascadeAt(c, 13), cascadeAt(c, 349), cascadeAt(c, 351)]).toEqual([0, 0, 1, 3, -1]);
    expect([0, 1, 2, 3].map(characterCastsIn)).toEqual([true, true, true, false]);
  });

  it('static cascades 0/1 redraw <= 60 times in 120 still frames; others every frame', () => {
    const s = new CascadeScheduler(4);
    const out = [false, false, false, false];
    for (let f = 0; f < 120; f++) s.next(0, out);
    expect(s.redraws).toEqual([60, 60, 120, 120]);
  });

  it('a moving camera redraws every cascade every frame', () => {
    const s = new CascadeScheduler(4);
    const out = [false, false, false, false];
    for (let f = 0; f < 10; f++) s.next(0.6, out);
    expect(s.redraws).toEqual([10, 10, 10, 10]);
  });
});

describe('probes', () => {
  const g: ProbeGrid = { origin: [0, 0, 0], spacing: 10, dims: [4, 3, 4] };
  const idx = new Int32Array(8);
  const w = new Float32Array(8);

  it('weights sum to 1 and reproduce a probe exactly on its position', () => {
    trilinear(g, [10, 10, 20], idx, w);
    expect(Array.from(w).reduce((a, b) => a + b, 0)).toBeCloseTo(1);
    const hit = idx[Array.from(w).findIndex((x) => x > 0.999)];
    expect(hit).toBe(1 + 4 * (1 + 3 * 2));
  });

  it('midpoint weights 1/8 each; outside the grid clamps', () => {
    trilinear(g, [5, 5, 5], idx, w);
    for (const x of w) expect(x).toBeCloseTo(0.125);
    trilinear(g, [-100, -100, -100], idx, w);
    expect(w[0]).toBeCloseTo(1);
    trilinear(g, [1e6, 1e6, 1e6], idx, w);
    expect(Array.from(w).reduce((a, b) => a + b, 0)).toBeCloseTo(1);
    expect(Array.from(idx).every((i) => i >= 0 && i < 48)).toBe(true);
  });

  it('1500 probes (~81 KB) fit the 1 MB budget; an oversized grid throws', () => {
    expect(() => checkProbeBudget({ origin: [0, 0, 0], spacing: 20, dims: [25, 6, 10] })).not.toThrow();
    expect(PROBE_BYTES).toBe(54);
    expect(() => checkProbeBudget({ origin: [0, 0, 0], spacing: 1, dims: [100, 100, 100] })).toThrow(/budget|exceeds/);
  });
});

describe('presets', () => {
  const a = parsePreset({ id: 'dawn', sunDir: [0, 1, 0], sunColor: [1, 0.5, 0.2], sunIntensity: 2, iblA: 'dawn.ktx2', cloudSpeed: 1 });
  const b = parsePreset({ id: 'noon', sunDir: [1, 0, 0], sunColor: [1, 1, 1], sunIntensity: 4, iblA: 'noon.ktx2', cloudSpeed: 3 });

  it('mix interpolates and keeps the sun direction unit length', () => {
    const m = mixPresets(a, b, 0.5);
    expect(m.sunIntensity).toBe(3);
    expect(m.cloudSpeed).toBe(2);
    expect(Math.hypot(...m.sunDir)).toBeCloseTo(1);
    expect(m.iblA).toBe('dawn.ktx2');
    expect(m.iblB).toBe('noon.ktx2');
    expect(mixPresets(a, b, 9).blend).toBe(1);
  });

  it('parsePreset rejects malformed data', () => {
    expect(() => parsePreset({ id: 'x' })).toThrow(/sunDir/);
    expect(() => parsePreset({ id: '', sunDir: [0, 1, 0], sunColor: [1, 1, 1], sunIntensity: 1, iblA: 'a', cloudSpeed: 0 })).toThrow(/id/);
  });
});
