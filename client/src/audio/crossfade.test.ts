import { describe, it, expect } from 'vitest';
import { layerGains, shotDelaySeconds, pickVariant, applyAvailability, suppressFilter, footstepAudibleM, limitFootsteps, AUDIO_PARAMS } from './crossfade';

const near = (d: number) => Math.min(1, Math.max(0, 1 - (d - 30) / 60));

describe('layerGains (acceptance 1: pointwise vs ARCHITECTURE §12.2)', () => {
  const pts = [0, 30, 60, 100, 200, 250, 300, 600, 1000];
  it('near = clamp(1 − (d−30)/60) exactly', () => {
    for (const d of pts) expect(Math.abs(layerGains(d).near - near(d))).toBeLessThanOrEqual(1e-6);
    expect(layerGains(0).near).toBe(1);
    expect(layerGains(90).near).toBe(0);
  });
  it('mid: in 30–60, full to 200, out 200–300', () => {
    const exp: Record<number, number> = { 0: 0, 30: 0, 60: 1, 100: 1, 200: 1, 250: 0.5, 300: 0, 600: 0, 1000: 0 };
    for (const d of pts) expect(Math.abs(layerGains(d).mid - exp[d]!)).toBeLessThanOrEqual(1e-6);
    expect(layerGains(45).mid).toBeCloseTo(0.5);
  });
  it('far: in 200→600, peak at 600, then falls with distance', () => {
    const exp: Record<number, number> = { 0: 0, 30: 0, 60: 0, 100: 0, 200: 0, 250: 0.125, 300: 0.25, 600: 1, 1000: 0.6 };
    for (const d of pts) expect(Math.abs(layerGains(d).far - exp[d]!)).toBeLessThanOrEqual(1e-6);
    expect(layerGains(2000).far).toBeLessThan(layerGains(1000).far);
  });
  it('parameters come from /data (not hard-coded) and respond to overrides', () => {
    expect(AUDIO_PARAMS.speed_of_sound_mps).toBe(343);
    expect(layerGains(60, { ...AUDIO_PARAMS, layer_mid_in_end_m: 120 }).mid).toBeCloseTo(0.3333, 3);
  });
  it('negative distance behaves like 0', () => {
    expect(layerGains(-5)).toEqual(layerGains(0));
  });
});

describe('shotDelaySeconds (acceptance 2)', () => {
  it('0 up to 150 m, then d/343, capped at 3 s', () => {
    expect(shotDelaySeconds(0)).toBe(0);
    expect(shotDelaySeconds(150)).toBe(0);
    for (const d of [200, 343, 686, 1029]) expect(Math.abs(shotDelaySeconds(d) - d / 343)).toBeLessThanOrEqual(1 / 60);
    expect(shotDelaySeconds(343)).toBeCloseTo(1);
    expect(shotDelaySeconds(5000)).toBe(3);
  });
});

describe('variants and availability (acceptance 4: missing layer is silent, nothing substitutes)', () => {
  it('never repeats the last variant', () => {
    let seed = 7;
    const rnd = () => ((seed = (seed * 1103515245 + 12345) >>> 0) / 2 ** 32);
    let last: number | undefined;
    for (let i = 0; i < 500; i++) {
      const v = pickVariant(4, last, rnd);
      expect(v).not.toBe(last);
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThan(4);
      last = v;
    }
    expect(pickVariant(1, 0, rnd)).toBe(0);
  });
  it('missing layers get 0 gain', () => {
    expect(applyAvailability({ near: 1, mid: 0.5, far: 0.2 }, { near: true, mid: false, far: false })).toEqual({ near: 1, mid: 0, far: 0 });
  });
});

describe('suppression-lite (acceptance 6)', () => {
  it('intensity 255 closes the filter, and 400 ms later it is back at the base cutoff ±5%', () => {
    const hit = suppressFilter(255, 0);
    expect(hit.cutoffHz).toBeCloseTo(1200);
    expect(hit.gainDb).toBe(-9);
    const rec = suppressFilter(255, 400);
    expect(Math.abs(rec.cutoffHz - 20000) / 20000).toBeLessThanOrEqual(0.05);
    expect(rec.gainDb).toBeCloseTo(0);
    expect(suppressFilter(255, 200).cutoffHz).toBeGreaterThan(hit.cutoffHz);
  });
  it('re-triggerable immediately and scaled by intensity', () => {
    expect(suppressFilter(255, 0).cutoffHz).toBe(suppressFilter(255, 0).cutoffHz);
    expect(suppressFilter(0, 0).cutoffHz).toBeCloseTo(20000);
    expect(suppressFilter(128, 0).cutoffHz).toBeGreaterThan(suppressFilter(255, 0).cutoffHz);
  });
});

describe('footsteps (acceptance 5 logic)', () => {
  it('keeps the nearest 8 of 20 mover steps', () => {
    const steps = Array.from({ length: 20 }, (_, i) => ({ distM: (i * 7) % 50 + i }));
    const kept = limitFootsteps(steps, 8);
    expect(kept).toHaveLength(8);
    const maxKept = Math.max(...kept.map((s) => s.distM));
    expect(steps.filter((s) => !kept.includes(s)).every((s) => s.distM >= maxKept)).toBe(true);
  });
  it('audible distance by gait', () => {
    const p = { footstep_audible_walk_m: 20, footstep_audible_run_m: 35, footstep_audible_crouch_m: 8 };
    expect(footstepAudibleM('run', p)).toBe(35);
    expect(footstepAudibleM('crouch', p)).toBe(8);
    expect(footstepAudibleM('walk', p)).toBe(20);
  });
});
