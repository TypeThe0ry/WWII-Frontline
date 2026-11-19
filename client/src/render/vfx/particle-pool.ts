/**
 * Particle pool, decal ring and tracer cadence (M1-047). Everything is preallocated typed arrays:
 * `emit` never allocates; when the pool is full the lowest-priority particle is evicted, and a
 * request that is itself the lowest priority is dropped (counted in `dropped`).
 */
export type VfxKind = 'muzzle_flash' | 'tracer' | 'smoke' | 'explosion' | 'dust' | 'shell';

export const KIND_ID: Record<VfxKind, number> = { muzzle_flash: 0, tracer: 1, smoke: 2, explosion: 3, dust: 4, shell: 5 };

/** Billboard edge (m) per kind id (muzzle_flash, tracer, smoke, explosion, dust, shell) [tbd]. */
export const PARTICLE_SIZE_M: readonly number[] = [0.35, 0.12, 1.2, 2.5, 0.5, 0.06];

/** Active-particle caps per quality tier (ARCHITECTURE 13.5.3). */
export const PARTICLE_CAP = { high: 1600, medium: 800, low: 300 } as const;
export type VfxTier = keyof typeof PARTICLE_CAP;

/** The local player's own effects are never evicted by anyone else's. */
export const PRIORITY_SELF = 255;
export const PRIORITY_NEAR = 128;
export const PRIORITY_FAR = 32;

export interface VfxEmitRequest {
  kind: VfxKind;
  x: number;
  y: number;
  z: number;
  /** Seconds the particle lives. */
  life: number;
  priority: number;
  seed: number;
}

export class ParticlePool {
  readonly capacity: number;
  private readonly kind: Uint8Array;
  private readonly prio: Uint8Array;
  private readonly age: Float32Array;
  private readonly life: Float32Array;
  readonly pos: Float32Array;
  private readonly alive: Uint8Array;
  active = 0;
  dropped = 0;
  /** Dropped requests whose priority was PRIORITY_SELF (must stay 0 under pressure). */
  droppedSelf = 0;
  evicted = 0;

  constructor(readonly tier: VfxTier) {
    const n = PARTICLE_CAP[tier];
    this.capacity = n;
    this.kind = new Uint8Array(n);
    this.prio = new Uint8Array(n);
    this.age = new Float32Array(n);
    this.life = new Float32Array(n);
    this.pos = new Float32Array(n * 3);
    this.alive = new Uint8Array(n);
  }

  /** Returns true when the particle was stored. */
  emit(r: VfxEmitRequest): boolean {
    return this.emitParticle(r.kind, r.x, r.y, r.z, r.life, r.priority);
  }

  emitParticle(kind: VfxKind, x: number, y: number, z: number, life: number, priority: number): boolean {
    let slot = -1;
    if (this.active < this.capacity) {
      for (let i = 0; i < this.capacity; i++) {
        if (this.alive[i] === 0) {
          slot = i;
          break;
        }
      }
    } else {
      // Evict the lowest priority, oldest first; refuse if the newcomer is not strictly better.
      let best = -1;
      let bp = 256;
      let ba = -1;
      for (let i = 0; i < this.capacity; i++) {
        const p = this.prio[i] as number;
        const a = this.age[i] as number;
        if (p < bp || (p === bp && a > ba)) {
          best = i;
          bp = p;
          ba = a;
        }
      }
      if (bp >= priority) {
        this.dropped++;
        if (priority === PRIORITY_SELF) this.droppedSelf++;
        return false;
      }
      slot = best;
      this.evicted++;
      this.dropped++;
      this.active--;
    }
    this.alive[slot] = 1;
    this.kind[slot] = KIND_ID[kind];
    this.prio[slot] = priority;
    this.age[slot] = 0;
    this.life[slot] = life;
    this.pos[slot * 3] = x;
    this.pos[slot * 3 + 1] = y;
    this.pos[slot * 3 + 2] = z;
    this.active++;
    return true;
  }

  update(dt: number): void {
    for (let i = 0; i < this.capacity; i++) {
      if (this.alive[i] === 0) continue;
      const a = (this.age[i] as number) + dt;
      if (a >= (this.life[i] as number)) {
        this.alive[i] = 0;
        this.prio[i] = 0;
        this.active--;
      } else this.age[i] = a;
    }
  }

  /**
   * Writes the live particles as thin-instance matrices (16 floats each, no allocation) for `ParticleRenderer`:
   * world0 = (size m, kind id, age / life, 0), world3 = (position, 1); the other rows stay zero (billboarding is done
   * in the vertex shader). Returns the instance count.
   */
  writeInstances(out: Float32Array): number {
    let n = 0;
    for (let i = 0; i < this.capacity; i++) {
      if (this.alive[i] === 0) continue;
      const o = n * 16;
      const k = this.kind[i] as number;
      out[o] = PARTICLE_SIZE_M[k] as number;
      out[o + 1] = k;
      out[o + 2] = (this.age[i] as number) / (this.life[i] as number);
      out[o + 3] = 0;
      out[o + 12] = this.pos[i * 3] as number;
      out[o + 13] = this.pos[i * 3 + 1] as number;
      out[o + 14] = this.pos[i * 3 + 2] as number;
      out[o + 15] = 1;
      n++;
    }
    return n;
  }

  stats(): { active: number; dropped: number } {
    return { active: this.active, dropped: this.dropped };
  }
}

/** Bullet-hole decals: a ring of the newest `cap` placements; the oldest is recycled. */
export class DecalRing {
  static readonly CAP = 128;
  readonly pos = new Float32Array(DecalRing.CAP * 3);
  readonly material = new Uint8Array(DecalRing.CAP);
  active = 0;
  private next = 0;
  /** Placements ever made (so tests can see recycling). */
  total = 0;

  clear(): void {
    this.active = this.total = this.next = 0;
  }

  place(x: number, y: number, z: number, material: number): number {
    const i = this.next;
    this.pos[i * 3] = x;
    this.pos[i * 3 + 1] = y;
    this.pos[i * 3 + 2] = z;
    this.material[i] = material;
    this.next = (this.next + 1) % DecalRing.CAP;
    if (this.active < DecalRing.CAP) this.active++;
    this.total++;
    return i;
  }
}

/** Every `n`-th round is a tracer (default 5 [tbd]); `n` comes from the weapon data. */
export class TracerCadence {
  private count = 0;
  constructor(readonly n = 5) {}
  next(): boolean {
    this.count++;
    if (this.count >= this.n) {
      this.count = 0;
      return true;
    }
    return false;
  }
}

/** Beyond this distance muzzle flashes / impacts merge into one small flash. */
export const FAR_MERGE_M = 100;
export const mergeFar = (distM: number): boolean => distM > FAR_MERGE_M;

/** Priority of an effect: own > near > far. */
export const vfxPriority = (own: boolean, distM: number): number => (own ? PRIORITY_SELF : mergeFar(distM) ? PRIORITY_FAR : PRIORITY_NEAR);

/** Impact material id -> dust preset. */
export type DustPreset = 'earth' | 'sand' | 'wood' | 'brick' | 'metal';
const DUST: readonly DustPreset[] = ['earth', 'sand', 'wood', 'brick', 'metal'];
export const dustPreset = (material: number): DustPreset => DUST[material] ?? 'earth';
