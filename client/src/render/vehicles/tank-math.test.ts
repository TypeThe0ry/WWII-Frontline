import { describe, expect, it } from 'vitest';
import sherman from '../../../../data/vehicles/us_m4a1_75.json';
import { phName } from '../actors/names';
import { PH_NAME_RE } from '../actors/names';
import { angleDiff, clampGunPitch, followAngle, hullBox, TANK_SOCKETS, trackSpeeds, trackUvRate, wheelOmega } from './tank-math';

describe('tracks', () => {
  it('straight line: both tracks at hull speed (20 km/h within 5 %)', () => {
    const v = 20 / 3.6;
    const t = trackSpeeds(v, 0, 2.2);
    expect(t.left).toBeCloseTo(v);
    expect(t.right).toBeCloseTo(v);
    expect(Math.abs(trackUvRate(t.left, 0.5) * 0.5 - v) / v).toBeLessThan(0.05);
  });
  it('stopped = still; pivot turn = opposite tracks; right turn slows the right track', () => {
    expect(trackSpeeds(0, 0, 2.2)).toEqual({ left: 0, right: 0 });
    const p = trackSpeeds(0, 0.5, 2.2);
    expect(p.left).toBeCloseTo(-p.right);
    expect(p.left).toBeGreaterThan(0);
    const r = trackSpeeds(5, 0.2, 2.2);
    expect(r.right).toBeLessThan(r.left);
    expect(wheelOmega(5, 0.25)).toBe(20);
  });
});

describe('turret following', () => {
  it('angleDiff takes the short way across +-PI', () => {
    expect(angleDiff(3.0, -3.0)).toBeCloseTo(2 * Math.PI - 6.0);
    expect(angleDiff(0, 1)).toBe(1);
  });
  it('never rotates faster than the traverse rate, goes the short way, then locks on', () => {
    const rate = (24 * Math.PI) / 180;
    let a = 3.0;
    const target = -3.0; // short arc is +0.28 rad through PI
    let last = a;
    for (let i = 0; i < 120; i++) {
      a = followAngle(a, target, 1 / 60, rate);
      expect(Math.abs(angleDiff(last, a))).toBeLessThanOrEqual(rate / 60 + 1e-9);
      last = a;
    }
    expect(Math.abs(angleDiff(a, target))).toBeLessThan((1 * Math.PI) / 180); // within 1 deg (vehicle_turret_sync)
  });
  it('gun pitch clamps to the data range', () => {
    const range = sherman.gameplay.gun_elevation_deg as [number, number];
    expect(clampGunPitch(40, range)).toBe(25);
    expect(clampGunPitch(-20, range)).toBe(-10);
  });
});

describe('placeholder', () => {
  it('hull box matches the data within 1 %', () => {
    const d = sherman.historical.dimensions_m;
    const b = hullBox(d);
    expect(Math.abs(b.size[2] - d.length) / d.length).toBeLessThan(0.01);
    expect(Math.abs(b.size[0] - d.width) / d.width).toBeLessThan(0.01);
    expect(Math.abs(b.size[1] - d.height) / d.height).toBeLessThan(0.01);
  });
  it('placeholder node names and sockets', () => {
    expect(PH_NAME_RE.test(phName('V-VEH-US-003', 'sherman'))).toBe(true);
    expect(new Set(TANK_SOCKETS).size).toBe(TANK_SOCKETS.length);
    for (const s of ['socket_gun_muzzle', 'socket_turret', 'socket_seat_driver', 'socket_seat_gunner', 'socket_seat_commander', 'socket_hatch', 'socket_exhaust', 'socket_mg_coax']) {
      expect(TANK_SOCKETS).toContain(s);
    }
  });
});
