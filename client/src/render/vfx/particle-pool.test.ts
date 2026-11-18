import { describe, expect, it } from 'vitest';
import {
  DecalRing, dustPreset, ParticlePool, PARTICLE_CAP, PARTICLE_SIZE_M, PRIORITY_FAR, PRIORITY_NEAR, PRIORITY_SELF, TracerCadence, vfxPriority, type VfxEmitRequest,
} from './particle-pool';

const req = (priority: number, life = 10, kind: VfxEmitRequest['kind'] = 'dust'): VfxEmitRequest => ({ kind, x: 0, y: 0, z: 0, life, priority, seed: 1 });

describe('ParticlePool', () => {
  it('capacity follows the tier', () => {
    expect([new ParticlePool('high').capacity, new ParticlePool('medium').capacity, new ParticlePool('low').capacity]).toEqual([1600, 800, 300]);
  });

  it('vfx_priority_drop: 3000 requests/s never exceed the cap; own muzzle flashes are never dropped', () => {
    const p = new ParticlePool('medium');
    let selfEmitted = 0;
    // 5 simulated seconds at 3000 req/s, 60 fps; 1 in 20 requests is the player's own flash.
    let n = 0;
    for (let frame = 0; frame < 300; frame++) {
      for (let k = 0; k < 50; k++) {
        const own = n % 20 === 0;
        const pr = own ? vfxPriority(true, 0) : vfxPriority(false, 20 + (n % 150));
        if (p.emit(req(pr, 1.5, own ? 'muzzle_flash' : 'dust'))) if (own) selfEmitted++;
        n++;
        expect(p.active).toBeLessThanOrEqual(p.capacity);
      }
      p.update(1 / 60);
    }
    expect(p.dropped).toBeGreaterThan(0);
    expect(p.droppedSelf).toBe(0);
    expect(selfEmitted).toBe(Math.ceil(n / 20));
  });

  it('a full pool evicts the lowest priority; a lower-priority newcomer is dropped', () => {
    const p = new ParticlePool('low');
    for (let i = 0; i < 300; i++) p.emit(req(PRIORITY_NEAR));
    expect(p.active).toBe(300);
    expect(p.emit(req(PRIORITY_FAR))).toBe(false);
    expect(p.emit(req(PRIORITY_NEAR))).toBe(false); // equal priority is not better
    expect(p.emit(req(PRIORITY_SELF))).toBe(true);
    expect(p.evicted).toBe(1);
    expect(p.active).toBe(300);
  });

  it('particles expire and free their slots', () => {
    const p = new ParticlePool('low');
    p.emit(req(PRIORITY_NEAR, 0.1));
    p.update(0.05);
    expect(p.active).toBe(1);
    p.update(0.06);
    expect(p.active).toBe(0);
    expect(PARTICLE_CAP.low).toBe(300);
  });
});

describe('DecalRing', () => {
  it('500 placements keep <= 128 active and recycle the oldest', () => {
    const d = new DecalRing();
    for (let i = 0; i < 500; i++) d.place(i, 0, 0, i % 4);
    expect(d.active).toBe(128);
    expect(d.total).toBe(500);
    // Slot 0 was overwritten by placement 384 (500 - 128 + 12 ...): all held x values are the newest 128.
    const xs = Array.from({ length: 128 }, (_, i) => d.pos[i * 3] as number);
    expect(Math.min(...xs)).toBe(500 - 128);
  });
});

describe('cadence and mapping', () => {
  it('every 5th round is a tracer', () => {
    const t = new TracerCadence(5);
    const hits = Array.from({ length: 20 }, () => t.next());
    expect(hits.filter(Boolean).length).toBe(4);
    expect(hits[4]).toBe(true);
    expect(hits[3]).toBe(false);
  });
  it('priority and material preset', () => {
    expect(vfxPriority(true, 500)).toBe(PRIORITY_SELF);
    expect(vfxPriority(false, 50)).toBe(PRIORITY_NEAR);
    expect(vfxPriority(false, 150)).toBe(PRIORITY_FAR);
    expect(dustPreset(2)).toBe('wood');
    expect(dustPreset(99)).toBe('earth');
  });
});

describe('writeInstances (M1-047b)', () => {
  it('writes size/kind/age and position of the live particles only', () => {
    const p = new ParticlePool('low');
    p.emit({ kind: 'smoke', x: 1, y: 2, z: 3, life: 2, priority: 128, seed: 0 });
    p.emit({ kind: 'dust', x: 4, y: 5, z: 6, life: 1, priority: 128, seed: 0 });
    p.update(0.5);
    const out = new Float32Array(16 * 8);
    const n = p.writeInstances(out);
    expect(n).toBe(2);
    expect(out[0]).toBeCloseTo(PARTICLE_SIZE_M[2]!, 6);
    expect(Array.from(out.subarray(1, 4))).toEqual([2, 0.25, 0]);
    expect(Array.from(out.subarray(12, 16))).toEqual([1, 2, 3, 1]);
    expect(out[16 + 1]).toBe(4);
    expect(out[16 + 2]).toBe(0.5);
    p.update(0.6); // dust (life 1) expires
    expect(p.writeInstances(out)).toBe(1);
  });
});
