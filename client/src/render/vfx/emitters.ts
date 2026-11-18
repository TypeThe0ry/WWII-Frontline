/**
 * Emitter presets (M1-047b): how many particles of which kind an effect event spawns, how long they live and how far
 * they scatter. The numbers are placeholders [tbd] until the effect strengths move to `/data` (hash-bound, PI-099);
 * `emitEffect` only calls `pool.emit`, so it never allocates.
 */
import { ParticlePool, PRIORITY_FAR, PRIORITY_NEAR, PRIORITY_SELF, mergeFar, vfxPriority, type VfxKind } from './particle-pool';

export type EffectName = 'muzzle' | 'impact_dust' | 'explosion' | 'tank_shot' | 'tracer' | 'shell';

interface Preset {
  kind: VfxKind;
  count: number;
  life: number;
  spread: number;
  /** Extra smoke puffs emitted with the main kind. */
  smoke?: { count: number; life: number };
}

export const PRESETS: Record<EffectName, Preset> = {
  muzzle: { kind: 'muzzle_flash', count: 1, life: 0.06, spread: 0 },
  tracer: { kind: 'tracer', count: 1, life: 0.12, spread: 0 },
  impact_dust: { kind: 'dust', count: 4, life: 0.5, spread: 0.4 },
  explosion: { kind: 'explosion', count: 3, life: 0.9, spread: 1.2, smoke: { count: 6, life: 3 } },
  shell: { kind: 'shell', count: 1, life: 1.2, spread: 0.05 },
  tank_shot: { kind: 'muzzle_flash', count: 2, life: 0.12, spread: 0.3, smoke: { count: 3, life: 1.5 } },
};

/** Deterministic [-1, 1) from an integer seed (no Math.random: effects must not break replays). */
const jitter = (seed: number): number => (((Math.imul(seed | 0, 0x9e3779b1) >>> 8) & 0xffff) / 32768) - 1;

/**
 * Emits `name` at (x, y, z). `own` = the local player's effect (never evicted by others'); `distM` = distance to the
 * camera (> 100 m merges to one far particle, priority `PRIORITY_FAR`). Returns the number of particles stored.
 */
export function emitEffect(pool: ParticlePool, name: EffectName, x: number, y: number, z: number, own: boolean, distM: number, seed: number, opts: { life?: number; scale?: number } = {}): number {
  const base = PRESETS[name];
  const sc = opts.scale ?? 1;
  const p = { ...base, life: opts.life ?? base.life, spread: base.spread * sc };
  const prio = vfxPriority(own, distM);
  const merged = mergeFar(distM);
  const n = merged ? 1 : p.count;
  let stored = 0;
  for (let i = 0; i < n; i++) {
    const s = seed * 31 + i;
    if (pool.emit({ kind: p.kind, x: x + jitter(s) * p.spread, y: y + Math.abs(jitter(s + 7)) * p.spread, z: z + jitter(s + 13) * p.spread, life: p.life, priority: prio, seed: s })) stored++;
  }
  if (p.smoke && !merged) {
    for (let i = 0; i < p.smoke.count; i++) {
      const s = seed * 17 + i;
      if (pool.emit({ kind: 'smoke', x: x + jitter(s) * p.spread, y: y + Math.abs(jitter(s + 3)) * p.spread, z: z + jitter(s + 5) * p.spread, life: p.smoke.life, priority: Math.min(prio, PRIORITY_NEAR), seed: s })) stored++;
    }
  }
  return stored;
}

export { PRIORITY_FAR, PRIORITY_NEAR, PRIORITY_SELF };
