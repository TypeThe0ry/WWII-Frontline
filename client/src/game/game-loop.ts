/**
 * Game loop glue (ARCHITECTURE 6.3 / 6.4): input -> `InputCmd` -> prediction -> unacknowledged ring ->
 * `InputBundle`; snapshots -> reconciliation + interpolation; local fire presentation.
 *
 * The composition root drives it from the `FixedStepper`: `update(steps, alpha, dtMs)` once per render
 * frame, `onSnapshot(bytes)` for every C2 packet. It depends on the network only through `NetPort`
 * (the real `Session` satisfies it) and never imports render / ui / audio.
 */
import type { InputSource } from '../input/sampler';
import { LAYOUT_STATE_AMMO_IN_MAG, LAYOUT_STATE_AMMO_RESERVE, LAYOUT_STATE_RECOIL_PITCH, LAYOUT_STATE_RECOIL_YAW, LAYOUT_STATE_VX, LAYOUT_STATE_VY, LAYOUT_STATE_VZ, type SimWasmModule, type WasmPredictor, type WasmSnapshotDecoder } from '../net/wasm';
import type { EventBus, GameEvents, Vec3 } from './event-bus';
import { EventDecoder } from './events';
import { EntityStore, INTERP_DELAY_TICKS, MAX_ENTITIES, RenderTimeline } from './interpolation';
import type { PlacementState } from './placement';
import { refreshPlacement, type PlacementFacts, type PlacementPoint, type PlacementWasm } from './placement-check';
import { LocalPredictor, type PredictorOptions } from './predictor';
import { Reconciler } from './reconcile';
import { SnapshotApplier, type SnapshotSummary } from './snapshot-apply';
import { GameState } from './state';
import { CMD_BYTES, seqDiff, UnackedRing, type UnackedEntry } from './unacked-ring';
import { VehiclePredictor } from './vehicle-predict';

export const MAX_BUNDLE_CMDS = 3;
const TICK_MS = 1000 / 30;
// The M1 hull is the Sherman. Vehicle snapshot rows do not identify the definition.
const M1_VEHICLE_DEF = 'us_m4a1_75';
type CollisionChunk = { bx: number; bz: number; data: Uint8Array };

/** The part of `ClockSync` the game uses. */
export interface ClockPort {
  /** Tick to stamp on a command generated now (`estimated_server_tick + input_lead`). */
  inputTargetTick(): number;
  estimatedServerTick(): number;
  /** Continuous estimate of the current server tick. */
  now(): number;
  applySlack(slack: number): void;
  rtt(): number;
}

/** The part of `Session` the game uses. */
export interface NetPort {
  readonly clock: ClockPort;
  sendInputBundle(bytes: Uint8Array): void;
}

export interface GameLoopOptions extends PredictorOptions {
  wasm: SimWasmModule;
  decoder: WasmSnapshotDecoder;
  bus: EventBus<GameEvents>;
  net: NetPort;
  input: InputSource;
  /** Monotonic ms clock (default `performance.now`). */
  now?: () => number;
  /** Where the predictor starts before the server has told us (default: map centre). */
  spawn?: { x: number; y: number; z: number; yaw: number };
  /** Wire entity id of the local player inside snapshot records (skipped by the interpolator). */
  selfWireId?: number;
  log?: (msg: string) => void;
}

/**
 * Decides how many commands to generate in a step so that the input lead follows the clock estimate
 * while ticks stay consecutive: 1 normally, 2 when more than one tick behind the wanted lead, 0 when
 * more than one tick ahead (the local simulation waits one step). At most one adjustment per 500 ms.
 */
export class TickPlanner {
  nextTick = NaN;
  extras = 0;
  skips = 0;
  resyncs = 0;
  private lastAdjustMs = -Infinity;
  static readonly HYSTERESIS_TICKS = 2;
  static readonly RESYNC_TICKS = 8;
  static readonly COOLDOWN_MS = 500;

  /** Number of commands to emit now (0..2); `nextTick` is the tick of the first one. */
  plan(target: number, nowMs: number): 0 | 1 | 2 {
    if (!Number.isFinite(this.nextTick)) {
      this.nextTick = target;
      return 1;
    }
    const diff = target - this.nextTick;
    if (Math.abs(diff) > TickPlanner.RESYNC_TICKS) {
      this.nextTick = target;
      this.resyncs++;
      return 1;
    }
    if (nowMs - this.lastAdjustMs >= TickPlanner.COOLDOWN_MS) {
      if (diff >= TickPlanner.HYSTERESIS_TICKS) {
        this.lastAdjustMs = nowMs;
        this.extras++;
        return 2;
      }
      if (diff <= -TickPlanner.HYSTERESIS_TICKS) {
        this.lastAdjustMs = nowMs;
        this.skips++;
        return 0;
      }
    }
    return 1;
  }
}

export interface GameStats {
  steps: number;
  bundles: number;
  bundleBytes: number;
  bundleBytesMax: number;
  /** Bundle bytes by commands per bundle (index 1..3). */
  bytesByN: [number, number, number, number];
  countByN: [number, number, number, number];
  snapshots: number;
  staleSnapshots: number;
  decodeErrors: number;
  fireLocal: number;
}

export class GameLoop {
  readonly state = new GameState();
  readonly ring = new UnackedRing();
  readonly predictor: LocalPredictor;
  readonly reconciler: Reconciler;
  readonly store = new EntityStore();
  readonly planner = new TickPlanner();
  readonly timeline = new RenderTimeline(TICK_MS);
  readonly applier: SnapshotApplier;
  readonly events: EventDecoder;
  readonly stats: GameStats = { steps: 0, bundles: 0, bundleBytes: 0, bundleBytesMax: 0, bytesByN: [0, 0, 0, 0], countByN: [0, 0, 0, 0], snapshots: 0, staleSnapshots: 0, decodeErrors: 0, fireLocal: 0 };

  private readonly wasm: SimWasmModule;
  private readonly createVehicleWasm: () => WasmPredictor;
  private readonly collisionChunks: CollisionChunk[] = [];
  private readonly vehicleKin = new Float32Array(11);
  private vehicleWasm?: WasmPredictor;
  private vehicle?: VehiclePredictor;
  private vehicleSlot = 0;
  private vehicleSeat = 0;
  private readonly bus: EventBus<GameEvents>;
  private readonly net: NetPort;
  private readonly input: InputSource;
  private readonly now: () => number;
  private readonly log: (m: string) => void;
  private nextSeq = 1;
  private lastSnapshotTick = -1;
  private ackSnapshotTick = 0;
  private hash = 0;
  private prevX = 0;
  private prevY = 0;
  private prevZ = 0;
  private readonly cmdBuf = new Uint8Array(CMD_BYTES * MAX_BUNDLE_CMDS);
  private readonly pose = new Float32Array(5);
  private readonly selfVec: Vec3 = { x: 0, y: 0, z: 0 };
  private readonly muzzle: Vec3 = { x: 0, y: 0, z: 0 };
  private readonly dir: Vec3 = { x: 0, y: 1, z: 0 };
  private readonly fireEv: GameEvents['fire.local'] = { weapon: 0, muzzlePos: this.muzzle, dir: this.dir };
  private readonly appliedEv: GameEvents['snapshot.applied'] = { tick: 0, entityCount: 0 };
  private eyeStand = 1.6;
  private eyeCrouch = 1.1;
  private lastExpireMs = 0;
  private kbpsWindowStart = 0;
  private kbpsBytes = 0;
  private overflowLogged = 0;

  constructor(opts: GameLoopOptions) {
    this.wasm = opts.wasm;
    // Vehicle stepping advances the WASM predictor's tick, so it cannot share the infantry instance.
    this.createVehicleWasm = () => new opts.wasm.Predictor(opts.matchSeed, opts.mapId, opts.playerId);
    this.bus = opts.bus;
    this.net = opts.net;
    this.input = opts.input;
    this.now = opts.now ?? (() => performance.now());
    this.log = opts.log ?? ((m) => console.warn(`[game] ${m}`));
    this.predictor = new LocalPredictor(opts.wasm, opts);
    this.reconciler = new Reconciler(this.ring, this.predictor);
    this.applier = new SnapshotApplier(opts.decoder, this.store, opts.bus);
    this.applier.ignoreId = opts.selfWireId ?? opts.playerId;
    this.events = new EventDecoder(opts.bus, () => this.selfVec);
    this.ring.onOverflow = (e) => {
      // Rate limited: one line per overflow burst, not per command.
      if (this.stats.steps - this.overflowLogged > this.ring.capacity) this.log(`unacked ring overflow: dropping seq ${e.seq} (no ack for ${this.ring.capacity} commands)`);
      this.overflowLogged = this.stats.steps;
    };
    try {
      const role = JSON.parse(new TextDecoder().decode(opts.wasm.roleDef('rifleman'))) as { movement?: { eye_height_stand_m?: number; eye_height_crouch_m?: number } };
      this.eyeStand = role.movement?.eye_height_stand_m ?? this.eyeStand;
      this.eyeCrouch = role.movement?.eye_height_crouch_m ?? this.eyeCrouch;
    } catch {
      /* keep defaults */
    }
    const sp = opts.spawn ?? { x: 128, y: 4, z: 128, yaw: 0 };
    this.predictor.spawnAt(sp.x, sp.y, sp.z, sp.yaw);
    this.ring.clear(0);
    this.predictor.capture(this.ring.base);
    this.ring.base.seq = 0;
    this.ring.base.tick = 0;
    this.prevX = this.predictor.x;
    this.prevY = this.predictor.y;
    this.prevZ = this.predictor.z;
    this.fillLocal(1);
  }

  /** Loads the collision chunks (`c_<bx>_<bz>.bin`, M0 = all 16). Call before the first `update`. */
  loadCollision(chunks: Iterable<CollisionChunk>): void {
    for (const c of chunks) {
      this.predictor.loadCollision(c.bx, c.bz, c.data);
      if (this.vehicleWasm) this.vehicleWasm.loadCollision(c.bx, c.bz, c.data);
      else this.collisionChunks.push(c);
    }
    // Settle onto the terrain (the spawn guess is airborne); not simulated through the ring.
    this.predictor.capture(this.ring.base);
  }

  /** Preview at the aimed ground point (Garrison) or predicted feet (crate actions). */
  checkPlacement(state: PlacementState, aimedPoint: PlacementPoint | null, facts: PlacementFacts | null): PlacementState {
    const point = state.kind === 'garrison' ? aimedPoint : [this.predictor.x, this.predictor.y, this.predictor.z] as const;
    return refreshPlacement(this.wasm as SimWasmModule & Partial<PlacementWasm>, this.predictor.wasm, state, point, facts);
  }

  get corrections(): number {
    return this.reconciler.stats.corrections;
  }
  get snaps(): number {
    return this.reconciler.stats.snaps;
  }
  get lastErrM(): number {
    return this.reconciler.stats.lastErrM;
  }

  /** The local hull prediction is available only while driving, after its first authoritative row. */
  get vehiclePredictor(): VehiclePredictor | undefined {
    return this.vehicleSeat === 0 && this.vehicle?.active ? this.vehicle : undefined;
  }

  /** Called once per render frame with the steps the fixed stepper granted. */
  update(steps: number, alpha: number, dtMs: number): void {
    const t = this.now();
    for (let i = 0; i < steps; i++) this.stepOnce(t);
    this.renderFrame(alpha, dtMs, t);
  }

  /** The tab is hidden or the window lost focus: every key goes up so the character stops. */
  suspendInput(): void {
    this.input.releaseAll();
  }

  private stepOnce(nowMs: number): void {
    const clock = this.net.clock;
    const count = this.planner.plan(clock.inputTargetTick(), nowMs);
    for (let k = 0; k < count; k++) this.emitCmd(clock);
    if (count > 0) this.sendBundle();
  }

  private emitCmd(clock: ClockPort): void {
    const tick = this.planner.nextTick;
    const seq = this.nextSeq;
    const viewTick = tick - (clock.estimatedServerTick() - INTERP_DELAY_TICKS);
    const f = this.input.build(seq, tick, viewTick);
    const cmd = this.wasm.packInputCmd(f.seq, f.tick, f.moveX, f.moveZ, f.yaw, f.pitch, f.buttons, f.weaponSlot, f.aux, f.viewTick);
    this.prevX = this.predictor.x;
    this.prevY = this.predictor.y;
    this.prevZ = this.predictor.z;
    this.predictor.step(cmd);
    this.vehiclePredictor?.step(tick, cmd);
    const e = this.ring.push();
    e.seq = seq;
    e.tick = tick;
    e.cmd.set(cmd);
    this.predictor.capture(e);
    this.nextSeq = (seq + 1) & 0xffff;
    this.planner.nextTick = tick + 1;
    this.stats.steps++;
    if (this.predictor.shotLastStep) this.emitFireLocal();
  }

  private emitFireLocal(): void {
    const p = this.predictor;
    const eye = p.stance === 1 ? this.eyeCrouch : this.eyeStand;
    this.muzzle.x = p.x;
    this.muzzle.y = p.y + eye;
    this.muzzle.z = p.z;
    const cp = Math.cos(p.pitch);
    this.dir.x = Math.sin(p.yaw) * cp;
    this.dir.y = Math.sin(p.pitch);
    this.dir.z = Math.cos(p.yaw) * cp;
    this.fireEv.weapon = p.weaponDefId();
    this.stats.fireLocal++;
    this.bus.emit('fire.local', this.fireEv);
  }

  /** Newest <= 3 unacknowledged commands with consecutive ticks -> one `InputBundle` on C1. */
  private sendBundle(): void {
    const n = this.ring.length;
    if (n === 0) return;
    let cnt = 1;
    while (cnt < MAX_BUNDLE_CMDS && n - 1 - cnt >= 0 && this.ring.at(n - 1 - cnt).tick === this.ring.at(n - cnt).tick - 1) cnt++;
    for (let i = 0; i < cnt; i++) this.cmdBuf.set(this.ring.at(n - cnt + i).cmd, i * CMD_BYTES);
    const bytes = this.wasm.encodeInputBundle(this.cmdBuf.subarray(0, cnt * CMD_BYTES), this.ackSnapshotTick & 0xffff, this.hash);
    this.net.sendInputBundle(bytes);
    const s = this.stats;
    s.bundles++;
    s.bundleBytes += bytes.length;
    if (bytes.length > s.bundleBytesMax) s.bundleBytesMax = bytes.length;
    s.bytesByN[cnt] = (s.bytesByN[cnt] as number) + bytes.length;
    s.countByN[cnt] = (s.countByN[cnt] as number) + 1;
    this.kbpsBytes += bytes.length;
  }

  /** One C2 packet (raw bytes, valid only for the duration of the call). */
  onSnapshot(bytes: Uint8Array): void {
    const nowMs = this.now();
    let s;
    try {
      s = this.applier.decode(bytes);
    } catch (e) {
      this.stats.decodeErrors++;
      this.log(`snapshot decode failed: ${e instanceof Error ? e.message : String(e)}`);
      return;
    }
    this.stats.snapshots++;
    const newer = this.lastSnapshotTick < 0 || ((s.tick - this.lastSnapshotTick) | 0) > 0;
    if (newer) {
      this.net.clock.applySlack(s.inputSlack);
      this.lastSnapshotTick = s.tick;
      this.applySelf(s);
    } else {
      this.stats.staleSnapshots++;
    }
    this.applier.applyEntities(nowMs);
    if (newer && s.hasSelf) this.applyVehicle(s);
    this.events.decode(this.applier.decoder.events());
    const ev = this.appliedEv;
    ev.tick = s.tick;
    ev.entityCount = this.store.count;
    this.bus.emit('snapshot.applied', ev);
  }

  private applyVehicle(s: SnapshotSummary): void {
    const vehicles = this.applier.vehicles;
    const slot = s.health > 0 ? s.vehicleSlot : 0;
    if (slot !== this.vehicleSlot || s.seat !== this.vehicleSeat || !vehicles.has(slot)) this.vehicle?.leave();
    this.vehicleSlot = slot;
    this.vehicleSeat = s.seat;
    if (slot === 0 || !vehicles.copyKinematics(slot, s.tick, this.vehicleKin)) return;
    if (!this.vehicle) {
      const wasm = this.createVehicleWasm();
      for (const { bx, bz, data } of this.collisionChunks) wasm.loadCollision(bx, bz, data);
      this.collisionChunks.length = 0;
      this.vehicleWasm = wasm;
      this.vehicle = new VehiclePredictor(wasm);
    }
    const cmds: UnackedEntry[] = [];
    if (s.seat === 0) {
      for (let i = 0; i < this.ring.length; i++) {
        const e = this.ring.at(i);
        if (seqDiff(e.seq, s.ackInputSeq) > 0 && ((e.tick - s.tick) | 0) > 0) cmds.push(e);
      }
    }
    if (!this.vehicle.active) this.vehicle.enter(M1_VEHICLE_DEF, s.tick, this.vehicleKin, cmds);
    else if (s.seat === 0) this.vehicle.reconcile(s.tick, this.vehicleKin, cmds);
  }

  private applySelf(s: ReturnType<SnapshotApplier['decode']>): void {
    if (!s.hasSelf) return;
    const rc = this.reconciler;
    // Dead players' inputs are ignored by the server: stop predicting (frozen), and a jump while alive again is a
    // respawn (PI-028; same rules as crates/headless-client). Must be set before the rollback replays anything.
    const alive = s.health > 0;
    this.predictor.frozen = !alive;
    rc.frozen = !alive;
    const r = rc.onSelf(s.ackInputSeq, s.hasCorr, s.corrX, s.corrY, s.corrZ);
    if (r.kind === 'unknown') return; // ack from the future: keep the previous hash / ack tick
    // pred_hash covers the position right after the acknowledged command: the server's own (an accepted
    // correction, so it stops resending it) or ours (predicted, or corrected = the server's).
    if (r.kind === 'ok') this.hash = this.wasm.predHash(s.corrX, s.corrY, s.corrZ);
    else this.hash = this.wasm.predHash(this.ring.base.x, this.ring.base.y, this.ring.base.z);
    this.ackSnapshotTick = s.tick;
    if (r.kind === 'corrected' || r.kind === 'snapped' || r.kind === 'initial') {
      const n = this.ring.length;
      if (n >= 2) {
        const p = this.ring.at(n - 2);
        this.prevX = p.x;
        this.prevY = p.y;
        this.prevZ = p.z;
      } else {
        this.prevX = this.ring.base.x;
        this.prevY = this.ring.base.y;
        this.prevZ = this.ring.base.z;
      }
      if (r.kind === 'snapped' || r.kind === 'initial') {
        this.prevX = this.predictor.x;
        this.prevY = this.predictor.y;
        this.prevZ = this.predictor.z;
      }
    }
    const net = this.state.net;
    net.corrections.value = rc.stats.corrections;
    net.snaps.value = rc.stats.snaps;
    net.lastErrM.value = rc.stats.lastErrM;
    net.ammoAuth.value = s.ammo;
    net.reserveAuth.value = s.reserve;
    net.health.value = s.health;
    net.stamina.value = s.stamina;
    net.snapshotTick.value = s.tick;
    net.rttMs.value = this.net.clock.rtt();
  }

  private fillLocal(alpha: number): void {
    const l = this.state.local;
    const p = this.predictor;
    const sm = this.reconciler.smoother;
    const a = alpha < 0 ? 0 : alpha > 1 ? 1 : alpha;
    l.pos[0] = this.prevX + (p.x - this.prevX) * a + sm.x;
    l.pos[1] = this.prevY + (p.y - this.prevY) * a + sm.y;
    l.pos[2] = this.prevZ + (p.z - this.prevZ) * a + sm.z;
    l.simPos[0] = p.x;
    l.simPos[1] = p.y;
    l.simPos[2] = p.z;
    const st = p.state;
    l.vel[0] = st[LAYOUT_STATE_VX] as number;
    l.vel[1] = st[LAYOUT_STATE_VY] as number;
    l.vel[2] = st[LAYOUT_STATE_VZ] as number;
    l.yaw = p.yaw;
    l.pitch = p.pitch;
    l.stance = p.stance;
    l.grounded = p.grounded;
    l.ammo = st[LAYOUT_STATE_AMMO_IN_MAG] as number;
    l.reserve = st[LAYOUT_STATE_AMMO_RESERVE] as number;
    l.recoilPitch = st[LAYOUT_STATE_RECOIL_PITCH] as number;
    l.recoilYaw = st[LAYOUT_STATE_RECOIL_YAW] as number;
    this.selfVec.x = l.pos[0] as number;
    this.selfVec.y = l.pos[1] as number;
    this.selfVec.z = l.pos[2] as number;
  }

  private renderFrame(alpha: number, dtMs: number, nowMs: number): void {
    this.reconciler.smoother.advance(dtMs);
    this.vehicle?.update(dtMs / 1000);
    this.fillLocal(alpha);
    // remote entities
    const clock = this.net.clock;
    const target = clock.now() - INTERP_DELAY_TICKS;
    const rt = this.timeline.update(dtMs, target);
    const ev = this.state.entities;
    let k = 0;
    if (Number.isFinite(rt)) {
      const alive = this.store.alive;
      const pose = this.pose;
      for (let id = 0; id < MAX_ENTITIES; id++) {
        if (alive[id] === 0) continue;
        this.store.sample(id, rt, TICK_MS, pose);
        ev.ids[k] = id;
        ev.pos[k * 3] = pose[0] as number;
        ev.pos[k * 3 + 1] = pose[1] as number;
        ev.pos[k * 3 + 2] = pose[2] as number;
        ev.yaw[k] = pose[3] as number;
        ev.pitch[k] = pose[4] as number;
        ev.anim[k] = this.store.anim[id] as number;
        ev.stance[k] = this.store.stance[id] as number;
        ev.weapon[k] = this.store.weapon[id] as number;
        ev.flags[k] = this.store.flags[id] as number;
        ev.health[k] = this.store.health[id] as number;
        k++;
      }
    }
    ev.count = k;
    if (nowMs - this.lastExpireMs >= 250) {
      this.lastExpireMs = nowMs;
      this.applier.expire(nowMs);
    }
    if (nowMs - this.kbpsWindowStart >= 1000) {
      const dt = (nowMs - this.kbpsWindowStart) / 1000;
      if (this.kbpsWindowStart > 0) this.state.net.upKbps.value = (this.kbpsBytes * 8) / 1000 / dt;
      this.kbpsWindowStart = nowMs;
      this.kbpsBytes = 0;
    }
  }

  dispose(): void {
    this.vehicleWasm?.free();
    this.predictor.free();
  }
}
