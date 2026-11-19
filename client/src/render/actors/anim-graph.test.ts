import { describe, expect, it } from 'vitest';
import { AnimGraph, FootstepClock, locoState, lodFor, MOTION, RELOAD_BIT, stanceTransitionS } from './anim-graph';

const standing = { stance: 0, anim: MOTION.idle, flags: 0, health: 255 };

describe('stance transitions (GD 9.2)', () => {
  it('table values', () => {
    expect(stanceTransitionS('stand', 'crouch')).toBe(0.25);
    expect(stanceTransitionS('crouch', 'stand')).toBe(0.25);
    expect(stanceTransitionS('crouch', 'prone')).toBe(0.5);
    expect(stanceTransitionS('prone', 'crouch')).toBe(0.6);
    expect(stanceTransitionS('stand', 'prone', true)).toBe(0.7);
    expect(stanceTransitionS('stand', 'prone')).toBe(0.75);
    expect(stanceTransitionS('prone', 'stand')).toBe(0.9);
    expect(stanceTransitionS('stand', 'stand')).toBe(0);
  });

  it('samples the via-crouch key pose at 250 ms and completes at 750 ms even with irregular frames', () => {
    const g = new AnimGraph();
    g.update(0, standing, 0);
    const prone = { ...standing, stance: 2 };
    g.update(0.2, prone, 0);
    expect(g.stancePose).toBeCloseTo(0.8);
    expect(g.canFire).toBe(false);
    g.update(0.05, prone, 0);
    expect(g.stancePose).toBe(1);
    g.update(0.49, prone, 0);
    expect(g.stancePose).toBeCloseTo(1.98);
    expect(g.canFire).toBe(false);
    g.update(0.01, prone, 0);
    expect(g.stancePose).toBe(2);
    expect(g.canFire).toBe(true);
  });

  it('starts at the published stance and restarts an interrupted transition from the visible pose', () => {
    const g = new AnimGraph();
    g.update(0, { ...standing, stance: 1 }, 0);
    expect(g.stancePose).toBe(1);
    g.update(0.25, { ...standing, stance: 2 }, 0);
    expect(g.stancePose).toBe(1.5);
    g.update(0, standing, 0);
    expect(g.stancePose).toBe(1.5);
    g.update(0.9, standing, 0);
    expect(g.stancePose).toBe(0);
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

  it('cross-fades from the currently sampled weights when a locomotion change is interrupted', () => {
    const g = new AnimGraph();
    g.update(0.05, standing, 0.075); // 1.5 m/s with idle wire hint
    expect(g.state).toBe('walk');
    const partial = g.strideWeight;
    expect(partial).toBeGreaterThan(0);
    expect(partial).toBeLessThan(0.55);
    const sprint = { ...standing, anim: MOTION.sprint | RELOAD_BIT };
    g.update(0, sprint, 0);
    expect(g.state).toBe('sprint');
    expect(g.strideWeight).toBe(partial);
    g.update(0.2, sprint, 0);
    expect(g.strideWeight).toBe(1);
    expect(g.upper).toBe('reload');
    expect(g.update(0.2, { ...standing, anim: MOTION.airborne }, 2)).toBeUndefined();
    expect(g.state).toBe('airborne');
    expect(g.airWeight).toBe(1);
    expect(g.strideWeight).toBe(0);
  });
});

describe('upper body and life states', () => {
  it('holds reload from the snapshot bit and briefly layers confirmed fire during a stance transition', () => {
    const g = new AnimGraph();
    g.update(0, standing, 0);
    const reloading = { ...standing, stance: 1, anim: RELOAD_BIT };
    g.update(0.1, reloading, 0);
    expect(g.upper).toBe('reload');
    expect(g.canFire).toBe(false);
    g.fire();
    g.update(0.016, reloading, 0);
    expect(g.upper).toBe('fire');
    expect(g.recoil).toBeGreaterThan(0);
    g.update(0.15, reloading, 0);
    expect(g.upper).toBe('reload');
    expect(g.reloadWeight).toBe(1);
    g.update(0.075, { ...reloading, anim: 0 }, 0);
    expect(g.reloadWeight).toBe(0.5);
    g.update(0.075, { ...reloading, anim: 0 }, 0);
    expect(g.upper).toBe('aim');
  });

  it('reacts once per hurt edge and clears action layers when downed or dead', () => {
    const g = new AnimGraph();
    const hit = { ...standing, flags: 0x82 };
    g.update(0, hit, 0);
    expect(g.life).toBe('hit_react');
    g.update(0.2, hit, 0);
    expect(g.life).toBe('alive');
    g.update(0.01, standing, 0);
    g.update(0.01, hit, 0);
    expect(g.life).toBe('hit_react');
    g.death(true);
    g.fire();
    expect(g.update(0.02, { ...standing, anim: RELOAD_BIT }, 1)).toBeUndefined();
    expect(g.life).toBe('downed');
    expect(g.upper).toBe('disabled');
    expect(g.recoil).toBe(0);
    g.revive();
    g.update(0.02, standing, 0);
    expect(g.life).toBe('alive');
    g.death(false);
    g.update(0, standing, 0);
    expect(g.life).toBe('dead');
    g.update(0, { ...standing, health: 0 }, 0);
    expect(g.life).toBe('dead');
    g.update(0, standing, 0);
    expect(g.life).toBe('alive');
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
