import { describe, it, expect } from 'vitest';
import * as H from './hud-math';

describe('ammo', () => {
  it('tier thresholds (GD §6.10) and exact-number rule', () => {
    expect([210, 160, 105, 60, 20].map((c) => H.ammoTier(c, 210))).toEqual(['full', 'high', 'mid', 'low', 'empty']);
    expect(H.ammoTier(0, 0)).toBe('empty');
    expect(H.ammoTier(52.5, 210)).toBe('low');
    expect(H.ammoWarn('low') && H.ammoWarn('empty') && !H.ammoWarn('mid')).toBe(true);
    expect(H.showExactAmmo(false, false)).toBe(false);
    expect(H.showExactAmmo(true, false)).toBe(true);
    expect(H.magEmpty(0)).toBe(true);
  });
});

describe('compass', () => {
  it('labels in both languages and wrap-around', () => {
    expect(H.compassLabel(0, false)).toBe('N');
    expect(H.compassLabel(359, false)).toBe('N');
    expect(H.compassLabel(47, true)).toBe('东北');
    expect(H.compassLabel(-90, false)).toBe('W');
  });
  it('heading from yaw: 0 = north, +π/2 = east', () => {
    expect(H.headingDeg(0)).toBe(0);
    expect(H.headingDeg(Math.PI / 2)).toBeCloseTo(90);
    expect(H.headingDeg(-Math.PI / 2)).toBeCloseTo(270);
  });
  it('tape offset and marker wrap', () => {
    expect(H.compassOffsetPx(90, 4)).toBe(-360);
    expect(H.markerOffsetPx(350, 10, 4)).toBe(-80);
    expect(H.markerOffsetPx(10, 350, 4)).toBe(80);
    expect(H.markerOffsetPx(180, 0, 2)).toBe(360);
  });
});

describe('kill feed queue', () => {
  it('max 3 visible, 4th waits, 5 s expiry promotes', () => {
    const q = new H.KillFeedQueue<number>();
    for (let i = 0; i < 4; i++) q.push(i, 0);
    expect(q.visible.length).toBe(3);
    expect(q.pending).toBe(1);
    expect(q.update(4999)).toBe(false);
    expect(q.update(5000)).toBe(true);
    expect(q.visible.map((v) => v.item)).toEqual([3]);
    expect(q.pending).toBe(0);
    expect(q.visible[0]!.shownAt).toBe(5000);
  });
});

describe('damage wedge', () => {
  it('relative angle follows camera yaw', () => {
    expect(H.wedgeAngle(0, 0, 0, 10, 0)).toBeCloseTo(0);
    expect(H.wedgeAngle(0, 0, 10, 0, 0)).toBeCloseTo(Math.PI / 2);
    expect(H.wedgeAngle(0, 0, 10, 0, Math.PI / 2)).toBeCloseTo(0);
    expect(H.wedgeAngle(0, 0, 0, -10, 0)).toBeCloseTo(Math.PI);
  });
  it('fade over 600 ms, strength mapped and clamped', () => {
    expect(H.wedgeOpacity(0, 1)).toBe(1);
    expect(H.wedgeOpacity(300, 1)).toBeCloseTo(0.5);
    expect(H.wedgeOpacity(600, 1)).toBe(0);
    expect(H.wedgeStrength(5)).toBe(0.25);
    expect(H.wedgeStrength(200)).toBe(1);
  });
});

describe('capture ring', () => {
  it('state machine inputs', () => {
    expect(H.captureState(false, 1, 0, 'neutral', 0)).toBe('hidden');
    expect(H.captureState(true, 2, 1, 'neutral', 0)).toBe('contested');
    expect(H.captureState(true, 2, 0, 'friend', 1)).toBe('captured');
    expect(H.captureState(true, 2, 0, 'neutral', 0.4)).toBe('visible');
  });
  it('segments sum to 1', () => {
    for (const p of [-1, -0.3, 0, 0.6, 1, 2]) {
      const s = H.ringSegments(p);
      expect(s.friend + s.neutral + s.enemy).toBeCloseTo(1);
    }
    expect(H.ringSegments(-0.5)).toEqual({ friend: 0, neutral: 0.5, enemy: 0.5 });
  });
});

describe('bands and FX', () => {
  it('health band and stamina warn', () => {
    expect([1, 0.7, 0.4, 0.1].map(H.healthBand)).toEqual([0, 1, 2, 3]);
    expect(H.staminaWarn(0.2)).toBe(true);
    expect(H.staminaWarn(0.21)).toBe(false);
  });
  it('FX priority downed > bleeding > suppressed, with hysteresis', () => {
    expect(H.fxState('clear', { downed: true, hpFrac: 1, suppression: 1 })).toBe('downed');
    expect(H.fxState('clear', { downed: false, hpFrac: 0.2, suppression: 1 })).toBe('bleeding');
    expect(H.fxState('clear', { downed: false, hpFrac: 1, suppression: 0.1 })).toBe('clear');
    expect(H.fxState('clear', { downed: false, hpFrac: 1, suppression: 0.2 })).toBe('suppressed');
    expect(H.fxState('suppressed', { downed: false, hpFrac: 1, suppression: 0.1 })).toBe('suppressed');
    expect(H.fxState('suppressed', { downed: false, hpFrac: 1, suppression: 0.04 })).toBe('clear');
  });
  it('vignette; reduce-motion quantises', () => {
    expect(H.fxVignette('downed', 0, 0, false)).toBe(1);
    expect(H.fxVignette('clear', 0, 1, false)).toBe(0);
    expect(H.fxVignette('suppressed', 0.5, 1, true)).toBe(0.5);
    expect(H.fxVignette('suppressed', 0.43, 1, false)).toBeCloseTo(0.344);
  });
  it('team data staleness hides exact distance after 5 s', () => {
    expect(H.showExactDistance(0, 5000)).toBe(true);
    expect(H.showExactDistance(0, 5001)).toBe(false);
  });
});
