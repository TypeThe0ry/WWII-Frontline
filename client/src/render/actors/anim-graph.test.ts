import { describe, expect, it } from 'vitest';
import { AnimGraph, FootstepClock, locoState, lodFor, stanceTransitionS } from './anim-graph';

describe('stance transitions (GD 9.2)', () => {
  it('table values', () => {
    expect(stanceTransitionS('stand', 'crouch')).toBe(0.25);
    expect(stanceTransitionS('crouch', 'stand')).toBe(0.25);
    expect(stanceTransitionS('crouch', 'prone')).toBe(0.5);
    expect(stanceTransitionS('prone', 'crouch')).toBe(0.6);
    expect(stanceTransitionS('stand', 'prone', true)).toBe(0.7);
    expect(stanceTransitionS('prone', 'stand')).toBe(0.9);
    expect(stanceTransitionS('stand', 'stand')).toBe(0);
  });

  it('cannot fire while changing stance, can afterwards', () => {
    const g = new AnimGraph();
    g.update(0.01, 'crouch', 0, false);
    expect(g.canFire).toBe(false);
    for (let i = 0; i < 30; i++) g.update(0.01, 'crouch', 0, false);
    expect(g.canFire).toBe(true);
    expect(g.state).toBe('crouch_idle');
  });
});

describe('locomotion', () => {
  it('state by stance, speed and sprint', () => {
    expect(locoState('stand', 0, false)).toBe('idle');
    expect(locoState('stand', 1.5, false)).toBe('walk');
    expect(locoState('stand', 4, false)).toBe('run');
    expect(locoState('stand', 6, true)).toBe('sprint');
    expect(locoState('crouch', 1, false)).toBe('crouch_walk');
    expect(locoState('prone', 0.5, false)).toBe('prone_crawl');
    expect(locoState('prone', 0, false)).toBe('prone_idle');
  });

  it('cross-fades to a new state and completes', () => {
    const g = new AnimGraph();
    g.update(0.05, 'stand', 1.5, false);
    expect(g.state).toBe('walk');
    expect(g.prev).toBe('idle');
    expect(g.blend).toBeGreaterThan(0);
    expect(g.blend).toBeLessThan(1);
    for (let i = 0; i < 10; i++) g.update(0.05, 'stand', 1.5, false);
    expect(g.blend).toBe(1);
  });
});

describe('footsteps', () => {
  it('alternate left/right, two per stride, none when standing', () => {
    const c = new FootstepClock();
    const feet: string[] = [];
    for (let i = 0; i < 100; i++) {
      const f = c.advance(0.08); // 8 m total at 1.6 m stride = 5 strides = 10 steps
      if (f) feet.push(f);
    }
    expect(feet.length).toBe(10);
    expect(feet.slice(0, 4)).toEqual(['left', 'right', 'left', 'right']);
    expect(c.advance(0)).toBeUndefined();
  });
});

describe('lod', () => {
  const t = [30, 80, 150];
  it('picks by distance', () => {
    expect(lodFor(10, 0, t)).toBe(0);
    expect(lodFor(100, 0, t)).toBe(2);
    expect(lodFor(400, 0, t)).toBe(3);
  });
  it('hysteresis stops flicker at a boundary', () => {
    let lod = 0;
    const picks: number[] = [];
    for (const d of [29, 31, 33, 29, 27, 36, 28]) {
      lod = lodFor(d, lod, t);
      picks.push(lod);
    }
    expect(picks).toEqual([0, 0, 0, 0, 0, 1, 1]);
  });
});
