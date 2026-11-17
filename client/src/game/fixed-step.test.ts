import { describe, expect, it } from 'vitest';
import { FixedStepper, STEP_MS } from './fixed-step';

describe('FixedStepper', () => {
  it('advance(100 ms) -> 3 steps', () => {
    const s = new FixedStepper();
    const r = s.advance(100);
    expect(r.steps).toBe(3);
    expect(r.alpha).toBeGreaterThanOrEqual(0);
    expect(r.alpha).toBeLessThan(1);
  });

  it('advance(1000 ms) -> 5 steps and stepper_clamped +1', () => {
    const s = new FixedStepper();
    const r = s.advance(1000);
    expect(r.steps).toBe(5);
    expect(s.stepperClamped).toBe(1);
    expect(r.alpha).toBeGreaterThanOrEqual(0);
    expect(r.alpha).toBeLessThan(1);
    // surplus was discarded: next small frame does not replay the backlog
    expect(s.advance(1).steps).toBe(0);
  });

  it('does not clamp at exactly max steps worth of time', () => {
    const s = new FixedStepper();
    expect(s.advance(5 * STEP_MS).steps).toBe(5);
    expect(s.stepperClamped).toBe(0);
  });

  it('ignores non-finite and non-positive dt', () => {
    const s = new FixedStepper();
    expect(s.advance(NaN).steps).toBe(0);
    expect(s.advance(-5).steps).toBe(0);
    expect(s.advance(Infinity).steps).toBe(0);
  });

  it('1 h of random real_dt stays within 1 step of floor(total / step)', () => {
    let seed = 12345;
    const rnd = () => {
      seed = (seed * 1664525 + 1013904223) >>> 0;
      return seed / 2 ** 32;
    };
    const s = new FixedStepper();
    let total = 0;
    let steps = 0;
    while (total < 3_600_000) {
      const dt = 1 + rnd() * 60; // 1..61 ms, never exceeds 5 steps (166 ms)
      total += dt;
      const r = s.advance(dt);
      steps += r.steps;
      expect(r.alpha).toBeGreaterThanOrEqual(0);
      expect(r.alpha).toBeLessThan(1);
    }
    expect(s.stepperClamped).toBe(0);
    expect(Math.abs(steps - Math.floor(total / STEP_MS))).toBeLessThanOrEqual(1);
  });
});
