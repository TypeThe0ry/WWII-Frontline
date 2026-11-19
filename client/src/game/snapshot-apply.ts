/**
 * Snapshot ingestion: wasm decode -> header/self summary + remote entities into the interpolation store.
 * Reconciliation of the `self` block is done by the caller (`GameLoop`), because it needs the input ring.
 */
import {
  LAYOUT_SELF_AMMO_IN_MAG,
  LAYOUT_SELF_AMMO_RESERVE,
  LAYOUT_SELF_CORR_X,
  LAYOUT_SELF_CORR_Y,
  LAYOUT_SELF_CORR_Z,
  LAYOUT_SELF_HAS_CORR,
  LAYOUT_SELF_HEALTH,
  LAYOUT_SELF_PRESENT,
  LAYOUT_SELF_SEAT,
  LAYOUT_SELF_STAMINA,
  LAYOUT_SELF_VEHICLE_SLOT,
  type WasmSnapshotDecoder,
} from '../net/wasm';
import type { EventBus, GameEvents } from './event-bus';
import type { EntityStore } from './interpolation';
import { RemoteVehicleStore, VEHICLE_ID_BASE } from './vehicle-store';

export interface SnapshotSummary {
  tick: number;
  ackInputSeq: number;
  inputSlack: number;
  hasSelf: boolean;
  hasCorr: boolean;
  corrX: number;
  corrY: number;
  corrZ: number;
  ammo: number;
  reserve: number;
  health: number;
  stamina: number;
  vehicleSlot: number;
  seat: number;
  entityCount: number;
}

export class SnapshotApplier {
  readonly summary: SnapshotSummary = {
    tick: 0,
    ackInputSeq: 0,
    inputSlack: 0,
    hasSelf: false,
    hasCorr: false,
    corrX: 0,
    corrY: 0,
    corrZ: 0,
    ammo: 0,
    reserve: 0,
    health: 0,
    stamina: 0,
    vehicleSlot: 0,
    seat: 0,
    entityCount: 0,
  };
  private readonly spawned: GameEvents['entity.spawned'] = { id: 0, kind: 'player', pos: { x: 0, y: 0, z: 0 } };
  private readonly removedEv: GameEvents['entity.removed'] = { id: 0, reason: 'removed' };
  /** Entity ids that are not remote players (the local player's own record, when the server sends it). */
  ignoreId = -1;
  /** Remote vehicles (wire ids >= 128) of the last decodes. */
  readonly vehicles = new RemoteVehicleStore();

  constructor(
    readonly decoder: WasmSnapshotDecoder,
    private readonly store: EntityStore,
    private readonly bus: EventBus<GameEvents>,
  ) {}

  /** Decodes `bytes` (throws on a malformed packet) and fills `summary`. Column views stay valid until the next decode. */
  decode(bytes: Uint8Array): SnapshotSummary {
    const d = this.decoder;
    const s = this.summary;
    s.tick = d.decode(bytes);
    s.ackInputSeq = d.ackInputSeq();
    s.inputSlack = d.inputSlack();
    const self = d.selfState();
    s.hasSelf = (self[LAYOUT_SELF_PRESENT] as number) !== 0;
    s.hasCorr = (self[LAYOUT_SELF_HAS_CORR] as number) !== 0;
    s.corrX = self[LAYOUT_SELF_CORR_X] as number;
    s.corrY = self[LAYOUT_SELF_CORR_Y] as number;
    s.corrZ = self[LAYOUT_SELF_CORR_Z] as number;
    s.ammo = self[LAYOUT_SELF_AMMO_IN_MAG] as number;
    s.reserve = self[LAYOUT_SELF_AMMO_RESERVE] as number;
    s.health = self[LAYOUT_SELF_HEALTH] as number;
    s.stamina = self[LAYOUT_SELF_STAMINA] as number;
    s.vehicleSlot = self[LAYOUT_SELF_VEHICLE_SLOT] as number;
    s.seat = self[LAYOUT_SELF_SEAT] as number;
    s.entityCount = d.count();
    return s;
  }

  /** Pushes the entity records of the last decode into the store and handles removals. */
  applyEntities(nowMs: number): void {
    const d = this.decoder;
    const tick = this.summary.tick;
    const n = this.summary.entityCount;
    if (n > 0) {
      const ids = d.ids();
      const pos = d.pos();
      const yaw = d.yaw();
      const pitch = d.pitch();
      const anim = d.anim();
      const stance = d.stance();
      const weapon = d.weapon();
      const flags = d.flags();
      const health = d.health();
      for (let i = 0; i < n; i++) {
        const id = ids[i] as number;
        if (id === this.ignoreId || id >= VEHICLE_ID_BASE) continue;
        const x = pos[i * 3] as number;
        const y = pos[i * 3 + 1] as number;
        const z = pos[i * 3 + 2] as number;
        const isNew = this.store.push(id, tick, x, y, z, yaw[i] as number, pitch[i] as number, nowMs);
        this.store.setAttrs(id, anim[i] as number, stance[i] as number, weapon[i] as number, flags[i] as number, health[i] as number);
        if (isNew) {
          const e = this.spawned;
          e.id = id;
          e.pos.x = x;
          e.pos.y = y;
          e.pos.z = z;
          this.bus.emit('entity.spawned', e);
        }
      }
    }
    const vids = d.vehicleIds();
    if (vids.length > 0) {
      for (const id of this.vehicles.push(tick, nowMs, vids, d.vehicleState(), d.vehicleMeta())) {
        const e = this.spawned;
        e.id = id;
        e.kind = 'vehicle';
        e.pos.x = e.pos.y = e.pos.z = 0;
        this.bus.emit('entity.spawned', e);
        e.kind = 'player';
      }
    }
    const removed = d.removed();
    for (let i = 0; i < removed.length; i++) {
      const id = removed[i] as number;
      if (id >= VEHICLE_ID_BASE ? this.vehicles.remove(id) : this.store.remove(id)) {
        this.removedEv.id = id;
        this.removedEv.reason = 'removed';
        this.bus.emit('entity.removed', this.removedEv);
      }
    }
  }

  /** Emits `entity.removed{timeout}` for entities that went silent. */
  expire(nowMs: number): void {
    this.store.expire(nowMs, (id) => {
      this.removedEv.id = id;
      this.removedEv.reason = 'timeout';
      this.bus.emit('entity.removed', this.removedEv);
    });
  }
}
