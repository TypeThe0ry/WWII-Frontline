/**
 * Local prediction: a thin wrapper over the sim-wasm `Predictor` (M0-014) that copies the shared-memory
 * views out immediately (they are only valid until the next wasm call) and records the per-command state
 * the reconciler needs.
 */
import {
  LAYOUT_EV_SHOT,
  LAYOUT_STATE_EVENTS,
  LAYOUT_STATE_GROUNDED,
  LAYOUT_STATE_LEN,
  LAYOUT_STATE_PITCH,
  LAYOUT_STATE_STANCE,
  LAYOUT_STATE_X,
  LAYOUT_STATE_Y,
  LAYOUT_STATE_YAW,
  LAYOUT_STATE_Z,
  type SimWasmModule,
  type WasmPredictor,
} from '../net/wasm';
import { AUTH_BYTES, WEAPON_BYTES, type StateRef } from './unacked-ring';
import type { ReplayPort } from './reconcile';

export interface PredictorOptions {
  matchSeed: bigint;
  mapId: number;
  playerId: number;
  /** Server `EntityId` of the player (shot RNG stream key); defaults to `playerId`. */
  entityId?: number;
  /** Weapon given to the predicted player; `null` = no weapon prediction. */
  weapon?: { id: string; reserve: number } | null;
}

export class LocalPredictor implements ReplayPort {
  readonly wasm: WasmPredictor;
  /** Copy of the wasm state array after the last step (`LAYOUT_STATE_*`). */
  readonly state = new Float32Array(LAYOUT_STATE_LEN);
  /** Steps executed (diagnostic). */
  steps = 0;
  /** Replay steps executed by reconciliations (diagnostic). */
  replaySteps = 0;
  /**
   * The server says the player is dead: it ignores the inputs, so nothing is predicted. `step` / `replay` leave the
   * state as it is (and report no events) until the next life begins.
   */
  frozen = false;
  private readonly weaponOpts: { id: string; reserve: number } | null;

  constructor(
    private readonly sim: SimWasmModule,
    opts: PredictorOptions,
  ) {
    this.wasm = new sim.Predictor(opts.matchSeed, opts.mapId, opts.playerId);
    this.wasm.setEntity(opts.entityId ?? opts.playerId);
    const w = opts.weapon === undefined ? { id: 'us_m1_garand', reserve: 80 } : opts.weapon;
    this.weaponOpts = w;
    if (w) this.wasm.equipWeapon(w.id, w.reserve);
    this.pull(this.wasm.state());
  }

  get x(): number {
    return this.state[LAYOUT_STATE_X] as number;
  }
  get y(): number {
    return this.state[LAYOUT_STATE_Y] as number;
  }
  get z(): number {
    return this.state[LAYOUT_STATE_Z] as number;
  }
  get yaw(): number {
    return this.state[LAYOUT_STATE_YAW] as number;
  }
  get pitch(): number {
    return this.state[LAYOUT_STATE_PITCH] as number;
  }
  get stance(): number {
    return this.state[LAYOUT_STATE_STANCE] as number;
  }
  get grounded(): boolean {
    return (this.state[LAYOUT_STATE_GROUNDED] as number) !== 0;
  }
  /** Bit set of what the last step did (`LAYOUT_EV_*`). */
  get events(): number {
    return this.state[LAYOUT_STATE_EVENTS] as number;
  }
  get shotLastStep(): boolean {
    return ((this.events as number) & LAYOUT_EV_SHOT) !== 0;
  }
  get tick(): number {
    return this.wasm.tick();
  }

  private pull(v: Float32Array): void {
    this.state.set(v);
  }

  loadCollision(bx: number, bz: number, data: Uint8Array): void {
    this.wasm.loadCollision(bx, bz, data);
  }

  /** Places the player (feet position, yaw) as a fresh grounded-to-be standing state. */
  spawnAt(x: number, y: number, z: number, yaw: number, tick = 0): void {
    const b = new Uint8Array(AUTH_BYTES);
    const dv = new DataView(b.buffer);
    dv.setFloat32(0, x, true);
    dv.setFloat32(4, y, true);
    dv.setFloat32(8, z, true);
    dv.setFloat32(24, yaw, true);
    this.wasm.reset(b, tick);
    this.pull(this.wasm.state());
  }

  /** One live prediction step (none while `frozen`). */
  step(cmd: Uint8Array): void {
    if (this.frozen) {
      this.state[LAYOUT_STATE_EVENTS] = 0;
      return;
    }
    this.pull(this.wasm.step(cmd));
    this.steps++;
  }

  /** Copies the state after the last step into `e`. */
  capture(e: StateRef): void {
    e.x = this.x;
    e.y = this.y;
    e.z = this.z;
    e.predTick = this.wasm.tick();
    e.auth.set(this.wasm.authState().subarray(0, AUTH_BYTES));
    const w = this.wasm.weaponState();
    e.weaponLen = w.length === WEAPON_BYTES ? WEAPON_BYTES : 0;
    if (e.weaponLen) e.weapon.set(w);
  }

  /** Rollback: resets to `auth` (+ weapon) at predictor tick `predTick`. */
  reset(auth: Uint8Array, predTick: number, weapon: Uint8Array | undefined): void {
    this.wasm.reset(auth, predTick, weapon && weapon.length === WEAPON_BYTES ? weapon : undefined);
    this.pull(this.wasm.state());
  }

  /** Replay step (identical to `step`, counted separately; callers suppress fire events). */
  replay(cmd: Uint8Array): void {
    if (this.frozen) return;
    this.pull(this.wasm.step(cmd));
    this.replaySteps++;
  }

  /**
   * A respawn is a teleport into a new life: rewrites `auth` to a fresh standing state at (x, y, z) keeping the yaw
   * it held (no velocity, stance or airborne time of the old life) and returns the starting weapon of that life, so
   * the rollback replays the unacknowledged commands from exactly what the server starts from.
   */
  freshLife(auth: Uint8Array, x: number, y: number, z: number): Uint8Array | undefined {
    const dv = new DataView(auth.buffer, auth.byteOffset, auth.byteLength);
    const yaw = dv.getFloat32(24, true);
    auth.fill(0);
    dv.setFloat32(0, x, true);
    dv.setFloat32(4, y, true);
    dv.setFloat32(8, z, true);
    dv.setFloat32(24, yaw, true);
    if (!this.weaponOpts) return undefined;
    this.wasm.equipWeapon(this.weaponOpts.id, this.weaponOpts.reserve);
    return this.wasm.weaponState().slice();
  }

  weaponDefId(): number {
    const w = this.wasm.weaponState();
    return w.length >= 2 ? (w[0] as number) | ((w[1] as number) << 8) : 0;
  }

  free(): void {
    this.wasm.free();
  }
}
