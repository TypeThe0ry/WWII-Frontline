/**
 * Shot / impact feedback driver. Bus events are queued by the handlers and consumed in `scene.onBeforeRenderObservable`,
 * i.e. in the very render pass that follows the event: the muzzle flash of a shot is visible in the frame in which the
 * `fire.*` event was emitted. `fireEventFrames` / `muzzleFlashFrames` record the engine frame id at emit time and at
 * the moment the flash was made visible (published as `window.__perf.fireEventFrame` / `muzzleFlashVisibleFrame`).
 */
import type { Observer } from '@babylonjs/core/Misc/observable';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import type { Scene } from '@babylonjs/core/scene';
import type { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import type { EventBus, GameEvents } from '../../game/event-bus';
import type { WeaponActorDef } from '../actors/actor-data';
import type { PlaceholderRegistry } from '../actors/names';
import { freshWorldMatrix } from '../actors/world';
import { ImpactDustPool } from './impact-dust';
import { MuzzleFlashPool } from './muzzle-flash';
import { TracerPool } from './tracer';
import { ImpactDecals } from '../vfx/decals';
import { emitEffect, type EffectName, type EffectOptions, type ImpactPreset } from '../vfx/emitters';
import { ParticlePool, TracerCadence, type VfxTier } from '../vfx/particle-pool';
import { ParticleRenderer } from '../vfx/particle-render';

export interface FxOptions {
  scene: Scene;
  bus: EventBus<GameEvents>;
  registry: PlaceholderRegistry;
  weapons: ReadonlyMap<number, WeaponActorDef>;
  /** Muzzle socket of the local first-person weapon, or of the remote shooter's third-person weapon. */
  localMuzzle: () => TransformNode | null;
  remoteMuzzle: (shooterId: number) => TransformNode | null;
  /** Monotonic ms clock (default `performance.now`; tests inject a fake). */
  now?: () => number;
  tracers?: boolean;
  /** GPU particles (M1-047b): pool tier and shader language; omit to leave particles off (tests, no-GPU). */
  particles?: { tier: VfxTier; wgsl: boolean };
}

const KIND_FLASH_LOCAL = 0;
const KIND_FLASH_REMOTE = 1;
const KIND_IMPACT = 2;
const KIND_EXPLOSION = 3;
const KIND_TANK = 4;
const DEFAULT_FLASH_MS = 50;
const LOG_CAP = 512;
/** `damage_radius_m` of the reference grenade (data/explosives/us_mk2.json): the explosion preset is sized for it. */
const EXPLOSION_REF_RADIUS_M = 8;

interface Pending {
  kind: number;
  weapon: number;
  shooter: number;
  x: number;
  y: number;
  z: number;
  dx: number;
  dy: number;
  dz: number;
  frame: number;
  radius: number;
  own: boolean;
  normal: number;
  material: number;
}

export class FxManager {
  readonly flashes: MuzzleFlashPool;
  readonly tracers: TracerPool;
  readonly dust: ImpactDustPool;
  /** GPU particle pool / renderer (null when `particles` was not requested). */
  readonly particlePool: ParticlePool | null;
  readonly particles: ParticleRenderer | null;
  private impactDecals: ImpactDecals | null = null;
  /** Engine frame ids at which each `fire.*` event was emitted / its flash became visible (same order). */
  readonly fireEventFrames: number[] = [];
  readonly muzzleFlashFrames: number[] = [];
  tracersOn: boolean;

  private readonly pending: Pending[] = [];
  private pendingCount = 0;
  private readonly effectOptions: EffectOptions = {};
  private readonly tracerCadence = new Map<number, TracerCadence>();
  private readonly offs: (() => void)[] = [];
  private readonly obs: Observer<Scene>;
  private lastNow: number;
  private readonly now: () => number;

  constructor(private readonly o: FxOptions) {
    this.now = o.now ?? (() => performance.now());
    this.lastNow = this.now();
    this.tracersOn = o.tracers ?? true;
    this.flashes = new MuzzleFlashPool(o.scene, o.registry);
    this.tracers = new TracerPool(o.scene, o.registry);
    this.dust = new ImpactDustPool(o.scene, o.registry);
    this.particlePool = o.particles ? new ParticlePool(o.particles.tier) : null;
    this.particles = o.particles && this.particlePool ? new ParticleRenderer(o.scene, this.particlePool, o.particles.wgsl) : null;
    const engine = o.scene.getEngine();
    this.offs.push(
      o.bus.on('fire.local', (e) => {
        const p = this.slot();
        p.kind = KIND_FLASH_LOCAL;
        p.weapon = e.weapon;
        p.shooter = -1;
        p.x = e.muzzlePos.x;
        p.y = e.muzzlePos.y;
        p.z = e.muzzlePos.z;
        p.dx = e.dir.x;
        p.dy = e.dir.y;
        p.dz = e.dir.z;
        p.frame = engine.frameId;
      }),
      o.bus.on('fire.remote', (e) => {
        const p = this.slot();
        p.kind = KIND_FLASH_REMOTE;
        p.weapon = e.weapon;
        p.shooter = e.shooter;
        p.x = e.muzzlePos.x;
        p.y = e.muzzlePos.y;
        p.z = e.muzzlePos.z;
        p.dx = e.dir.x;
        p.dy = e.dir.y;
        p.dz = e.dir.z;
        p.frame = engine.frameId;
      }),
      o.bus.on('impact', (e) => {
        const p = this.slot();
        p.kind = KIND_IMPACT;
        p.x = e.pos.x;
        p.y = e.pos.y;
        p.z = e.pos.z;
        p.normal = e.normal;
        p.material = e.material;
        p.frame = engine.frameId;
      }),
      o.bus.on('explosion', (e) => {
        const p = this.slot();
        p.kind = KIND_EXPLOSION;
        p.x = e.pos.x;
        p.y = e.pos.y;
        p.z = e.pos.z;
        p.radius = e.radius;
        p.own = e.own;
        p.frame = engine.frameId;
      }),
      o.bus.on('entity.removed', (e) => this.tracerCadence.delete(e.id)),
    );
    this.obs = o.scene.onBeforeRenderObservable.add(() => this.beforeRender());
  }

  private slot(): Pending {
    let p = this.pending[this.pendingCount];
    if (!p) {
      p = { kind: 0, weapon: 0, shooter: 0, x: 0, y: 0, z: 0, dx: 0, dy: 0, dz: 1, frame: 0, radius: 0, own: false, normal: 0, material: 0 };
      this.pending[this.pendingCount] = p;
    }
    this.pendingCount++;
    return p;
  }

  /** Install static mesh receivers and the CPU height sampler for GPU-displaced terrain. Replaces old placements. */
  setImpactSurfaces(meshes: readonly AbstractMesh[], heightAt?: (x: number, z: number) => number, getSurfaceAt?: (x: number, z: number) => number): void {
    this.impactDecals ??= new ImpactDecals(this.o.scene);
    this.impactDecals.setSurfaces(meshes, heightAt, getSurfaceAt);
  }

  get decals(): ImpactDecals | null {
    return this.impactDecals;
  }

  private traceThisRound(shooter: number): boolean {
    if (!this.tracersOn) return false;
    if (!this.particles) return true; // M0 debug feedback
    let cadence = this.tracerCadence.get(shooter);
    if (!cadence) {
      cadence = new TracerCadence();
      this.tracerCadence.set(shooter, cadence);
    }
    return cadence.next();
  }

  private beforeRender(): void {
    const now = this.now();
    const dtS = Math.max(0, Math.min(0.25, (now - this.lastNow) / 1000));
    this.lastNow = now;
    const frame = this.o.scene.getEngine().frameId;
    const cam = this.o.scene.activeCamera;
    cam?.getViewMatrix();
    const eye = cam?.globalPosition;
    this.particlePool?.update(dtS);
    for (let i = 0; i < this.pendingCount; i++) {
      const p = this.pending[i] as Pending;
      if (p.kind === KIND_IMPACT) {
        if (!this.particles) this.dust.spawn(p.x, p.y, p.z, this.decals === null);
        const preset = eye && this.decals?.impact(p.x, p.y, p.z, p.normal, p.material, eye);
        this.tracers.impactAt(p.x, p.y, p.z);
        this.emitParticles('impact_dust', p.x, p.y, p.z, false, p.frame, undefined, undefined, p.material, preset);
        continue;
      }
      if (p.kind === KIND_EXPLOSION) {
        this.emitParticles('explosion', p.x, p.y, p.z, p.own, p.frame, undefined, Math.max(0.25, p.radius / EXPLOSION_REF_RADIUS_M));
        continue;
      }
      if (p.kind === KIND_TANK) {
        this.emitParticles('tank_shot', p.x, p.y, p.z, p.own, p.frame);
        continue;
      }
      const def = this.o.weapons.get(p.weapon);
      const socket = p.kind === KIND_FLASH_LOCAL ? this.o.localMuzzle() : this.o.remoteMuzzle(p.shooter);
      this.fireEventFrames.push(p.frame);
      if (this.flashes.spawn(now, def?.muzzleFlashMs ?? DEFAULT_FLASH_MS, socket, p.x, p.y, p.z, frame)) this.muzzleFlashFrames.push(frame);
      else this.muzzleFlashFrames.push(-1);
      const matrix = socket ? freshWorldMatrix(socket).m : null;
      const ox = matrix?.[12] ?? p.x;
      const oy = matrix?.[13] ?? p.y;
      const oz = matrix?.[14] ?? p.z;
      const own = p.kind === KIND_FLASH_LOCAL;
      this.emitParticles('muzzle', ox, oy, oz, own, p.frame, (def?.muzzleFlashMs ?? DEFAULT_FLASH_MS) / 1000);
      if (own) this.emitParticles('shell', ox, oy, oz, true, p.frame);
      if (this.traceThisRound(p.shooter)) {
        this.tracers.spawn(ox, oy, oz, p.dx, p.dy, p.dz, def?.muzzleVelocityMps ?? 600, this.particles !== null);
      }
    }
    this.pendingCount = 0;
    for (const log of [this.fireEventFrames, this.muzzleFlashFrames]) if (log.length > LOG_CAP) log.splice(0, log.length - LOG_CAP);
    this.flashes.update(now);
    this.tracers.update(dtS);
    this.dust.update(dtS);
    if (cam) this.particles?.upload(cam);
    if (eye) this.decals?.update(eye);
  }

  /** Tank main-gun muzzle blast + smoke at (x, y, z) (M1-048b); priority follows the camera distance. */
  emitTankShot(x: number, y: number, z: number, own = false): void {
    if (!this.particlePool) return;
    const p = this.slot();
    p.kind = KIND_TANK;
    p.x = x;
    p.y = y;
    p.z = z;
    p.own = own;
    p.frame = this.o.scene.getEngine().frameId;
  }

  private emitParticles(name: EffectName, x: number, y: number, z: number, own: boolean, seed: number, life?: number, scale?: number, material?: number, impactPreset?: ImpactPreset): void {
    const pool = this.particlePool;
    if (!pool) return;
    const cam = this.o.scene.activeCamera;
    const pos = cam?.globalPosition;
    const dist = pos ? Math.hypot(x - pos.x, y - pos.y, z - pos.z) : 0;
    const opts = this.effectOptions;
    opts.life = life;
    opts.scale = scale;
    opts.material = material;
    opts.impactPreset = impactPreset;
    emitEffect(pool, name, x, y, z, own, dist, seed, opts);
  }

  dispose(): void {
    for (const off of this.offs) off();
    this.o.scene.onBeforeRenderObservable.remove(this.obs);
    this.flashes.dispose();
    this.tracers.dispose();
    this.dust.dispose();
    this.particles?.dispose();
    this.decals?.dispose();
  }
}

export { MuzzleFlashPool } from './muzzle-flash';
export { TracerPool } from './tracer';
export { ImpactDustPool } from './impact-dust';
export { emitEffect } from '../vfx/emitters';
export { FirstPersonRig } from './first-person-camera';
