/**
 * Emitter presets (M1-047b): how many particles of which kind an effect event spawns, how long they live and how far
 * they scatter. The numbers are placeholders [tbd] until the effect strengths move to `/data` (hash-bound, PI-099);
 * `emitEffect` forwards scalars to the pool so the burst itself does not allocate.
 */
import { ParticlePool, PRIORITY_FAR, PRIORITY_NEAR, PRIORITY_SELF, dustPreset, mergeFar, vfxPriority, type DustPreset, type VfxKind } from './particle-pool';

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

// Provisional C2 material classes until collision material IDs have a data-backed client mapping.
export type ImpactPreset = DustPreset | 'stone';

const IMPACT_PRESETS: Record<ImpactPreset, Preset> = {
  earth: PRESETS.impact_dust,
  sand: { kind: 'dust', count: 5, life: 0.55, spread: 0.48 },
  wood: { kind: 'dust', count: 3, life: 0.3, spread: 0.28 },
  brick: { kind: 'dust', count: 4, life: 0.4, spread: 0.34 },
  metal: { kind: 'dust', count: 2, life: 0.18, spread: 0.16 },
  stone: { kind: 'dust', count: 3, life: 0.35, spread: 0.24 },
};

export interface EffectOptions {
  life?: number;
  scale?: number;
  material?: number;
  impactPreset?: ImpactPreset;
}

/** Deterministic [-1, 1) from an integer seed (no Math.random: effects must not break replays). */
const jitter = (seed: number): number => (((Math.imul(seed | 0, 0x9e3779b1) >>> 8) & 0xffff) / 32768) - 1;

/**
 * Emits `name` at (x, y, z). `own` = the local player's effect (never evicted by others'); `distM` = distance to the
 * camera (> 100 m merges to one far particle, priority `PRIORITY_FAR`). Returns the number of particles stored.
 */
export function emitEffect(pool: ParticlePool, name: EffectName, x: number, y: number, z: number, own: boolean, distM: number, seed: number, opts?: EffectOptions): number {
  const merged = mergeFar(distM);
  if (pool.tier === 'low' && (name === 'shell' || (name === 'impact_dust' && !merged))) return 0;
  const preset = name === 'impact_dust' ? IMPACT_PRESETS[opts?.impactPreset ?? dustPreset(opts?.material ?? 0)] : PRESETS[name];
  const prio = vfxPriority(own && name !== 'shell', distM);
  const kind = merged && name === 'impact_dust' ? 'muzzle_flash' : preset.kind;
  const life = opts?.life ?? (merged && name === 'impact_dust' ? PRESETS.muzzle.life : preset.life);
  const spread = merged ? 0 : preset.spread * (opts?.scale ?? 1);
  const count = merged ? 1 : preset.count;
  let stored = 0;
  for (let i = 0; i < count; i++) {
    const s = seed * 31 + i;
    if (pool.emitParticle(kind, x + jitter(s) * spread, y + Math.abs(jitter(s + 7)) * spread, z + jitter(s + 13) * spread, life, prio)) stored++;
  }
  if (preset.smoke && !merged) {
    for (let i = 0; i < preset.smoke.count; i++) {
      const s = seed * 17 + i;
      if (pool.emitParticle('smoke', x + jitter(s) * spread, y + Math.abs(jitter(s + 3)) * spread, z + jitter(s + 5) * spread, preset.smoke.life, Math.min(prio, PRIORITY_NEAR))) stored++;
    }
  }
  return stored;
}

export { PRIORITY_FAR, PRIORITY_NEAR, PRIORITY_SELF };
