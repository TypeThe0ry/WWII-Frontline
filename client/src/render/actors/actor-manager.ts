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
import type { ActorData, WeaponActorDef } from './actor-data';
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
  /** Third-person weapon scale relative to the first-person one. */
  thirdPersonScale?: number;
}

/** Scale of the third-person weapon (a size smaller than the first-person one). */
export const THIRD_PERSON_WEAPON_SCALE = 0.9;

interface ActorSlot {
  actor: PlaceholderCharacter;
  faction: Faction;
  weapon: number;
  stamp: number;
}

export class ActorManager {
  readonly registry = new PlaceholderRegistry();
  readonly fx: FxManager;
  readonly rig: FirstPersonRig | null;
  private readonly chars = new Map<Faction, CharacterSources>();
  private readonly wsrc = new Map<number, WeaponSources>();
  private readonly weaponsByIndex = new Map<number, WeaponActorDef>();
  private readonly actors = new Map<number, ActorSlot>();
  private readonly offs: (() => void)[] = [];
  private stamp = 0;
  private fpIndex = -1;
  private readonly pose: CharacterPose = { x: 0, y: 0, z: 0, yaw: 0, pitch: 0, stance: 0, health: 255 };
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
    this.fx = new FxManager({
      scene,
      bus: o.bus,
      registry: this.registry,
      weapons: this.weaponsByIndex,
      localMuzzle: () => this.rig?.weapon?.muzzle ?? null,
      remoteMuzzle: (id) => this.actors.get(id)?.actor.weapon?.muzzle ?? null,
      now: o.now,
      tracers: o.tracers,
    });
    this.offs.push(
      // Creation needs the entity's weapon (faction): only possible once the id is in the frame data; otherwise `update` creates it.
      o.bus.on('entity.spawned', (e) => {
        const k = this.slotOf(e.id);
        if (k >= 0) this.ensure(e.id, o.state.entities.weapon[k] as number, o.state.entities.flags[k] as number);
      }),
      o.bus.on('entity.removed', (e) => this.remove(e.id)),
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
      s = { actor, faction, weapon: -1, stamp: this.stamp };
      this.actors.set(id, s);
    }
    if (s.weapon !== weapon) {
      s.weapon = weapon;
      const src = this.wsrc.get(weapon);
      s.actor.setWeapon(src ? src.instantiate(`tp_${id}`, this.o.thirdPersonScale ?? THIRD_PERSON_WEAPON_SCALE) : null);
    }
    return s;
  }

  private remove(id: number): void {
    const s = this.actors.get(id);
    if (!s) return;
    s.actor.dispose();
    this.actors.delete(id);
  }

  /** Builds the first-person weapon of weapon index `index` (or removes it for an unknown index). */
  attachWeaponFP(index: number | string): void {
    const def = typeof index === 'string' ? this.o.data.weapons.find((w) => w.id === index) : this.weaponsByIndex.get(index);
    this.fpIndex = def?.index ?? -1;
    this.rig?.setWeapon(def ? buildFirstPersonWeapon(this.o.scene, def, this.registry) : null);
  }

  /** Once per render frame, after the game update and before the scene renders. */
  update(alpha: number, dtMs: number): void {
    void alpha; // the game's state is already interpolated for this frame
    const { state, data } = this.o;
    const lw = this.o.localWeapon?.();
    if (lw !== undefined && lw !== this.fpIndex && this.weaponsByIndex.has(lw)) this.attachWeaponFP(lw);
    this.rig?.update(state.local, dtMs);

    const e = state.entities;
    const stamp = ++this.stamp;
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
      s.actor.update(p, dtMs, data.stanceTransitionMs);
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
    for (const c of this.chars.values()) c.dispose();
    for (const w of this.wsrc.values()) w.dispose();
  }
}
