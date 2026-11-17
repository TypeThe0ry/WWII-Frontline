/**
 * Shot / impact feedback driver. Bus events are queued by the handlers and consumed in `scene.onBeforeRenderObservable`,
 * i.e. in the very render pass that follows the event: the muzzle flash of a shot is visible in the frame in which the
 * `fire.*` event was emitted. `fireEventFrames` / `muzzleFlashFrames` record the engine frame id at emit time and at
 * the moment the flash was made visible (published as `window.__perf.fireEventFrame` / `muzzleFlashVisibleFrame`).
 */
import type { Observer } from '@babylonjs/core/Misc/observable';
import type { Scene } from '@babylonjs/core/scene';
import type { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import type { EventBus, GameEvents } from '../../game/event-bus';
import type { WeaponActorDef } from '../actors/actor-data';
import type { PlaceholderRegistry } from '../actors/names';
import { freshWorldMatrix } from '../actors/world';
import { ImpactDustPool } from './impact-dust';
import { MuzzleFlashPool } from './muzzle-flash';
import { TracerPool } from './tracer';

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
}

const KIND_FLASH_LOCAL = 0;
const KIND_FLASH_REMOTE = 1;
const KIND_IMPACT = 2;
const DEFAULT_FLASH_MS = 50;
const LOG_CAP = 512;

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
}

export class FxManager {
  readonly flashes: MuzzleFlashPool;
  readonly tracers: TracerPool;
  readonly dust: ImpactDustPool;
  /** Engine frame ids at which each `fire.*` event was emitted / its flash became visible (same order). */
  readonly fireEventFrames: number[] = [];
  readonly muzzleFlashFrames: number[] = [];
  tracersOn: boolean;

  private readonly pending: Pending[] = [];
  private pendingCount = 0;
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
    const engine = o.scene.getEngine();
    const slot = (): Pending => {
      let p = this.pending[this.pendingCount];
      if (!p) {
        p = { kind: 0, weapon: 0, shooter: 0, x: 0, y: 0, z: 0, dx: 0, dy: 0, dz: 1, frame: 0 };
        this.pending[this.pendingCount] = p;
      }
      this.pendingCount++;
      return p;
    };
    this.offs.push(
      o.bus.on('fire.local', (e) => {
        const p = slot();
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
        const p = slot();
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
        const p = slot();
        p.kind = KIND_IMPACT;
        p.x = e.pos.x;
        p.y = e.pos.y;
        p.z = e.pos.z;
        p.frame = engine.frameId;
      }),
    );
    this.obs = o.scene.onBeforeRenderObservable.add(() => this.beforeRender());
  }

  private beforeRender(): void {
    const now = this.now();
    const dtS = Math.max(0, Math.min(0.25, (now - this.lastNow) / 1000));
    this.lastNow = now;
    const frame = this.o.scene.getEngine().frameId;
    for (let i = 0; i < this.pendingCount; i++) {
      const p = this.pending[i] as Pending;
      if (p.kind === KIND_IMPACT) {
        this.dust.spawn(p.x, p.y, p.z);
        this.tracers.impactAt(p.x, p.y, p.z);
        continue;
      }
      const def = this.o.weapons.get(p.weapon);
      const socket = p.kind === KIND_FLASH_LOCAL ? this.o.localMuzzle() : this.o.remoteMuzzle(p.shooter);
      this.fireEventFrames.push(p.frame);
      if (this.flashes.spawn(now, def?.muzzleFlashMs ?? DEFAULT_FLASH_MS, socket, p.x, p.y, p.z, frame)) this.muzzleFlashFrames.push(frame);
      else this.muzzleFlashFrames.push(-1);
      if (this.tracersOn) {
        let ox = p.x;
        let oy = p.y;
        let oz = p.z;
        if (socket) {
          const m = freshWorldMatrix(socket).m;
          ox = m[12] as number;
          oy = m[13] as number;
          oz = m[14] as number;
        }
        this.tracers.spawn(ox, oy, oz, p.dx, p.dy, p.dz, def?.muzzleVelocityMps ?? 600);
      }
    }
    this.pendingCount = 0;
    for (const log of [this.fireEventFrames, this.muzzleFlashFrames]) if (log.length > LOG_CAP) log.splice(0, log.length - LOG_CAP);
    this.flashes.update(now);
    this.tracers.update(dtS);
    this.dust.update(dtS);
  }

  dispose(): void {
    for (const off of this.offs) off();
    this.o.scene.onBeforeRenderObservable.remove(this.obs);
    this.flashes.dispose();
    this.tracers.dispose();
    this.dust.dispose();
  }
}

export { MuzzleFlashPool } from './muzzle-flash';
export { TracerPool } from './tracer';
export { ImpactDustPool } from './impact-dust';
export { FirstPersonRig } from './first-person-camera';
