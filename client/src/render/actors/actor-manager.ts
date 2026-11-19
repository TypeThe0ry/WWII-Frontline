/**
 * Actor manager (M0-025): turns the game's read-only state into grey-box nodes.
 *  - remote entities (`state.entities`, already interpolated by the game) -> one `PlaceholderCharacter` each, created on
 *    `entity.spawned` (or when first seen in the frame data) and removed on `entity.removed` (or when absent),
 *  - the local player is never drawn as a capsule: the camera is put at the predicted pose and only the first-person weapon is drawn,
 *  - shot / impact feedback via `FxManager` (bus events `fire.local`, `fire.remote`, `impact`).
 * It only reads `GameState` and listens to the event bus (type imports only); it knows nothing about the net layer.
 */
import type { TargetCamera } from '@babylonjs/core/Cameras/targetCamera';
import type { Scene } from '@babylonjs/core/scene';
import type { EventBus, GameEvents } from '../../game/event-bus';
import type { GameState } from '../../game/state';
import { FirstPersonRig, FxManager } from '../fx';
import { FpWeaponPass } from '../fp/fp-pass';
import type { ActorData, WeaponActorDef } from './actor-data';
import { lodFor } from './anim-graph';
import { type Faction, PlaceholderRegistry } from './names';
import { CharacterSources, type CharacterPose, PlaceholderCharacter } from './placeholder-character';
import { WeaponSources, buildFirstPersonWeapon } from './placeholder-weapon';

export type ActorFeed = Pick<GameState, 'local' | 'entities'>;

export interface ActorManagerOptions {
  scene: Scene;
  bus: EventBus<GameEvents>;
  state: ActorFeed;
  data: ActorData;
  /** Scene camera that becomes the first-person camera; omit to leave the camera alone (tests, free camera). */
  camera?: TargetCamera;
  /** Weapon index (`WeaponState.def_id`) the local player holds; polled every frame (default: none). */
  localWeapon?: () => number;
  /** Is the local player aiming down sights (input state). */
  aiming?: () => boolean;
  now?: () => number;
  tracers?: boolean;
  /** GPU particle tier + shader language (M1-047b); omit for no particles. */
  particles?: { tier: 'high' | 'medium' | 'low'; wgsl: boolean };
  /** Weapon camera pass (default on when a first-person camera exists). */
  fpPass?: boolean;
  /** Third-person weapon scale relative to the first-person one. */
  thirdPersonScale?: number;
}

/** Character LOD distance thresholds (m): beyond the first the small markers go, beyond the second the head and weapon [tbd]. */
export const LOD_THRESHOLDS_M = [40, 120] as const;

/** Scale of the third-person weapon (a size smaller than the first-person one). */
export const THIRD_PERSON_WEAPON_SCALE = 0.9;

interface ActorSlot {
  actor: PlaceholderCharacter;
  lastX: number;
  lastZ: number;
  hasLast: boolean;
  faction: Faction;
  weapon: number;
  stamp: number;
}

export class ActorManager {
  readonly registry = new PlaceholderRegistry();
  readonly fx: FxManager;
  readonly rig: FirstPersonRig | null;
  /** Separate weapon camera / depth pass (M1-046b); null without a first-person camera or with `fpPass: false`. */
  readonly fpPass: FpWeaponPass | null;
  private readonly chars = new Map<Faction, CharacterSources>();
  private readonly wsrc = new Map<number, WeaponSources>();
  private readonly weaponsByIndex = new Map<number, WeaponActorDef>();
  private readonly actors = new Map<number, ActorSlot>();
  private readonly offs: (() => void)[] = [];
  private stamp = 0;
  private fpIndex = -1;
  private readonly pose: CharacterPose = { x: 0, y: 0, z: 0, yaw: 0, pitch: 0, stance: 0, anim: 0, flags: 0, health: 255 };
  private disposed = false;
  /** Entity count seen by the last `update` and the actor count right after it (diagnostics: they are always equal). */
  readonly lastUpdate = { entities: 0, actors: 0 };

  constructor(private readonly o: ActorManagerOptions) {
    const { scene, data } = o;
    for (const w of data.weapons) this.weaponsByIndex.set(w.index, w);
    for (const f of ['us', 'ger'] as const) this.chars.set(f, new CharacterSources(scene, f, data.capsule, this.registry));
    for (const w of data.weapons) this.wsrc.set(w.index, new WeaponSources(scene, w, this.registry));
    // the local weapon's requirement ids exist even before the first-person weapon does
    scene.setRenderingAutoClearDepthStencil(1, true, true, false);
    this.rig = o.camera ? new FirstPersonRig(o.camera, data, o.aiming ?? (() => false)) : null;
    this.fpPass = o.camera && o.fpPass !== false ? new FpWeaponPass(scene, o.camera, data.hipFovDeg * (Math.PI / 180)) : null;
    if (this.rig) this.rig.onAds = (state, t) => o.bus.emit('ads.state', { state, t });
    this.fx = new FxManager({
      scene,
      bus: o.bus,
      registry: this.registry,
      weapons: this.weaponsByIndex,
      localMuzzle: () => this.rig?.weapon?.muzzle ?? null,
      remoteMuzzle: (id) => this.actors.get(id)?.actor.weapon?.muzzle ?? null,
      now: o.now,
      tracers: o.tracers,
      particles: o.particles,
    });
    this.offs.push(
      o.bus.on('fire.local', (e) => this.rig?.queueShot(e.weapon)),
      // Creation needs the entity's weapon (faction): only possible once the id is in the frame data; otherwise `update` creates it.
      o.bus.on('entity.spawned', (e) => {
        const k = this.slotOf(e.id);
        if (k >= 0) this.ensure(e.id, o.state.entities.weapon[k] as number, o.state.entities.flags[k] as number);
      }),
      o.bus.on('entity.removed', (e) => this.remove(e.id)),
      o.bus.on('fire.remote', (e) => this.actorForEvent(e.shooter)?.animation.fire()),
      o.bus.on('death', (e) => this.actorForEvent(e.victim)?.animation.death(e.canBeRevived)),
      o.bus.on('revive', (e) => this.actors.get(e.target)?.actor.animation.revive()),
    );
  }

  get actorCount(): number {
    return this.actors.size;
  }
  actor(id: number): PlaceholderCharacter | undefined {
    return this.actors.get(id)?.actor;
  }
  allActors(): IterableIterator<ActorSlot> {
    return this.actors.values();
  }

  /** Index into the entity SoA of wire id `id` (-1 when absent). */
  private slotOf(id: number): number {
    const e = this.o.state.entities;
    for (let k = 0; k < e.count; k++) if (e.ids[k] === id) return k;
    return -1;
  }

  /** Snapshot record `flags` bit 7 = faction (0 US, 1 GER; PI-024 item 4). */
  private factionOf(flags: number): Faction {
    return (flags & 0x80) === 0 ? 'us' : 'ger';
  }

  private ensure(id: number, weapon: number, flags: number): ActorSlot {
    const faction = this.factionOf(flags);
    let s = this.actors.get(id);
    if (s && s.faction !== faction) {
      this.remove(id);
      s = undefined;
    }
    if (!s) {
      const actor = new PlaceholderCharacter(this.o.scene, id, this.chars.get(faction) as CharacterSources, this.registry);
      s = { actor, lastX: 0, lastZ: 0, hasLast: false, faction, weapon: -1, stamp: this.stamp };
      this.actors.set(id, s);
    }
    if (s.weapon !== weapon) {
      s.weapon = weapon;
      const src = this.wsrc.get(weapon);
      s.actor.setWeapon(src ? src.instantiate(`tp_${id}`, this.o.thirdPersonScale ?? THIRD_PERSON_WEAPON_SCALE) : null);
    }
    return s;
  }

  /** Events can arrive after a sample was published but before the first render update. */
  private actorForEvent(id: number): PlaceholderCharacter | undefined {
    const existing = this.actors.get(id)?.actor;
    if (existing) return existing;
    const k = this.slotOf(id);
    if (k < 0) return;
    const e = this.o.state.entities;
    return this.ensure(id, e.weapon[k] as number, e.flags[k] as number).actor;
  }

  private readonly stepPayload = { entity: 0, foot: 'left' as 'left' | 'right', pos: { x: 0, y: 0, z: 0 }, stance: 0 };

  private footstep(id: number, foot: 'left' | 'right', p: Readonly<CharacterPose>): void {
    const e = this.stepPayload;
    e.entity = id;
    e.foot = foot;
    e.pos.x = p.x;
    e.pos.y = p.y;
    e.pos.z = p.z;
    e.stance = p.stance;
    this.o.bus.emit('entity.footstep', e);
  }

  private remove(id: number): void {
    const s = this.actors.get(id);
    if (!s) return;
    s.actor.dispose();
    this.actors.delete(id);
  }

  /** Builds the first-person weapon of weapon index `index` (or removes it for an unknown index). */
  attachWeaponFP(index: number | string): void {
    if (!this.rig) return;
    const def = typeof index === 'string' ? this.o.data.weapons.find((w) => w.id === index) : this.weaponsByIndex.get(index);
    this.fpIndex = def?.index ?? -1;
    const w = def ? buildFirstPersonWeapon(this.o.scene, def, this.registry) : null;
    this.rig.setWeapon(w);
    if (w) this.fpPass?.adopt(w.root, w.meshes);
  }

  /** Once per render frame, after the game update and before the scene renders. */
  update(alpha: number, dtMs: number): void {
    void alpha; // the game's state is already interpolated for this frame
    const { state } = this.o;
    const lw = this.o.localWeapon?.();
    if (lw !== undefined && lw !== this.fpIndex && this.weaponsByIndex.has(lw)) this.attachWeaponFP(lw);
    this.rig?.update(state.local, dtMs);
    this.fpPass?.sync();

    const e = state.entities;
    const stamp = ++this.stamp;
    const cam = this.o.scene.activeCamera?.globalPosition;
    const p = this.pose;
    for (let k = 0; k < e.count; k++) {
      const id = e.ids[k] as number;
      const s = this.ensure(id, e.weapon[k] as number, e.flags[k] as number);
      s.stamp = stamp;
      p.x = e.pos[k * 3] as number;
      p.y = e.pos[k * 3 + 1] as number;
      p.z = e.pos[k * 3 + 2] as number;
      p.yaw = e.yaw[k] as number;
      p.pitch = e.pitch[k] as number;
      p.stance = e.stance[k] as number;
      p.health = e.health[k] as number;
      p.anim = e.anim[k] as number;
      p.flags = e.flags[k] as number;
      const distance = s.hasLast ? Math.hypot(p.x - s.lastX, p.z - s.lastZ) : 0;
      s.hasLast = true;
      s.lastX = p.x;
      s.lastZ = p.z;
      const foot = s.actor.update(p, dtMs, distance < 5 ? distance : 0);
      if (foot) this.footstep(id, foot, p);
      if (cam) s.actor.setLod(lodFor(Math.hypot(p.x - cam.x, p.y - cam.y, p.z - cam.z), s.actor.lod, LOD_THRESHOLDS_M));
    }
    for (const [id, s] of this.actors) if (s.stamp !== stamp) this.remove(id);
    this.lastUpdate.entities = e.count;
    this.lastUpdate.actors = this.actors.size;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const off of this.offs) off();
    for (const id of [...this.actors.keys()]) this.remove(id);
    this.fx.dispose();
    this.rig?.dispose();
    this.fpPass?.dispose();
    for (const c of this.chars.values()) c.dispose();
    for (const w of this.wsrc.values()) w.dispose();
  }
}
