import { describe, expect, it } from 'vitest';
import { PARTICLE_CAP, ParticlePool } from './particle-pool';
import { PRESETS, emitEffect } from './emitters';

describe('emitters', () => {
  it('spawns the preset count plus smoke, deterministically', () => {
    const a = new ParticlePool('high');
    const b = new ParticlePool('high');
    expect(emitEffect(a, 'explosion', 10, 2, 5, true, 30, 42)).toBe(PRESETS.explosion.count + PRESETS.explosion.smoke!.count);
    emitEffect(b, 'explosion', 10, 2, 5, true, 30, 42);
    expect(Array.from(a.pos.subarray(0, 27))).toEqual(Array.from(b.pos.subarray(0, 27)));
  });
  it('merges far effects into one particle and drops the smoke', () => {
    const p = new ParticlePool('high');
    expect(emitEffect(p, 'explosion', 0, 0, 0, false, 250, 1)).toBe(1);
  });
  it('never drops the local player under pressure', () => {
    const p = new ParticlePool('low');
    for (let i = 0; i < PARTICLE_CAP.low * 4; i++) emitEffect(p, 'impact_dust', 0, 0, 0, false, 130, i);
    for (let i = 0; i < 50; i++) emitEffect(p, 'muzzle', 0, 0, 0, true, 0, i);
    expect(p.droppedSelf).toBe(0);
    expect(p.active).toBe(PARTICLE_CAP.low);
    expect(p.dropped).toBeGreaterThan(0);
    expect(p.evicted).toBe(50);
  });
});
