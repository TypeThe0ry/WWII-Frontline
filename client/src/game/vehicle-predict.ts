/**
 * Prediction of the vehicle the local player drives (M1-041b): wraps the sim-wasm vehicle API, replays the
 * unacknowledged commands after an authoritative snapshot and decides how the visible error is corrected.
 *
 * Reconcile thresholds (ARCHITECTURE §6.x): an error below `SOFT_M` is ignored (the prediction keeps its own
 * pose), between `SOFT_M` and `SNAP_M` the visual offset decays after rollback,
 * and at or above `SNAP_M` the view snaps.
 */
import type { WasmPredictor } from '../net/wasm';
import { RING_CAPACITY } from './unacked-ring';

export const VEHICLE_SOFT_M = 0.05;
export const VEHICLE_SNAP_M = 1.0;
export const VEHICLE_BLEND_S = 0.25;

/** `pos[3] vel[3] quat[4] yawRate forwardSpeed`. */
export const VEHICLE_STATE_LEN = 12;

export type VehicleCorrection = 'none' | 'blend' | 'snap';

interface VehicleCommand {
  tick: number;
  cmd: Uint8Array;
}

export class VehiclePredictor {
  /** Predicted state after the last step (copy of the wasm array). */
  readonly state = new Float32Array(VEHICLE_STATE_LEN);
  /** Visual offset (m) still to be blended out, added to the predicted position by `view`. */
  readonly offset = { x: 0, y: 0, z: 0 };
  active = false;
  private defId = '';
  private lastStepTick = 0;
  private readonly positions = new Map<number, [number, number, number]>();

  constructor(private readonly wasm: WasmPredictor) {}

  /** Enter from the first authoritative sample. `kin` = 11 floats (see `WasmPredictor.resetVehicle`). */
  enter(defId: string, tick: number, kin: Float32Array, cmds: readonly VehicleCommand[] = []): void {
    this.defId = defId;
    this.reset(tick, kin);
    this.active = true;
    this.offset.x = this.offset.y = this.offset.z = 0;
    for (const { tick, cmd } of cmds) this.step(tick, cmd);
  }

  leave(): void {
    if (!this.active) return;
    this.wasm.clearVehicle();
    this.active = false;
    this.positions.clear();
    this.offset.x = this.offset.y = this.offset.z = 0;
  }

  step(tick: number, cmd: Uint8Array): boolean {
    if (!this.active || ((tick - this.lastStepTick) | 0) <= 0) return false;
    const s = this.wasm.stepVehicle(cmd);
    if (!s) return false;
    this.state.set(s);
    this.rememberPosition(tick);
    return true;
  }

  private reset(tick: number, kin: Float32Array): void {
    this.wasm.resetVehicle(this.defId, kin);
    this.state.set(kin);
    this.state[11] = 0;
    this.positions.clear();
    this.rememberPosition(tick);
  }

  private rememberPosition(tick: number): void {
    this.lastStepTick = tick;
    this.positions.set(tick, [this.state[0] as number, this.state[1] as number, this.state[2] as number]);
    if (this.positions.size > RING_CAPACITY) {
      const oldest = this.positions.keys().next().value;
      if (oldest !== undefined) this.positions.delete(oldest);
    }
  }

  /**
   * Compares with the prediction at the sample's server tick before touching the sim. On rollback,
   * `cmds` contains only commands the server has not acknowledged whose target tick follows the sample.
   */
  reconcile(tick: number, kin: Float32Array, cmds: readonly VehicleCommand[]): VehicleCorrection {
    if (!this.active) return 'none';
    const p = this.positions.get(tick);
    // When prediction has no matching tick (e.g. after a clock resync), re-anchor directly.
    const err = p
      ? Math.hypot(p[0] - (kin[0] as number), p[1] - (kin[1] as number), p[2] - (kin[2] as number))
      : Infinity;
    if (err < VEHICLE_SOFT_M) return 'none';
    const px = (this.state[0] as number) + this.offset.x;
    const py = (this.state[1] as number) + this.offset.y;
    const pz = (this.state[2] as number) + this.offset.z;
    this.reset(tick, kin);
    for (const { tick, cmd } of cmds) this.step(tick, cmd);
    if (err >= VEHICLE_SNAP_M) {
      this.offset.x = this.offset.y = this.offset.z = 0;
      return 'snap';
    }
    // Show the old pose and blend out the difference.
    this.offset.x = px - (this.state[0] as number);
    this.offset.y = py - (this.state[1] as number);
    this.offset.z = pz - (this.state[2] as number);
    return 'blend';
  }

  /** Decays the visual offset for a render-frame interval of `dt` seconds. */
  update(dt: number): void {
    const k = Math.max(0, 1 - dt / VEHICLE_BLEND_S);
    this.offset.x *= k;
    this.offset.y *= k;
    this.offset.z *= k;
  }

  /** Visual position: predicted pose plus the remaining blend offset. */
  view(out: { x: number; y: number; z: number }): void {
    out.x = (this.state[0] as number) + this.offset.x;
    out.y = (this.state[1] as number) + this.offset.y;
    out.z = (this.state[2] as number) + this.offset.z;
  }
}
