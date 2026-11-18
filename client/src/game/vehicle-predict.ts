/**
 * Prediction of the vehicle the local player drives (M1-041b): wraps the sim-wasm vehicle API, replays the
 * unacknowledged commands after an authoritative snapshot and decides how the visible error is corrected.
 *
 * Reconcile thresholds (ARCHITECTURE §6.x): an error below `SOFT_M` is ignored (the prediction keeps its own
 * pose), between `SOFT_M` and `SNAP_M` the visual pose is blended toward the new prediction over
 * `BLEND_S` seconds, above `SNAP_M` the view snaps.
 */
import type { WasmPredictor } from '../net/wasm';

export const VEHICLE_SOFT_M = 0.05;
export const VEHICLE_SNAP_M = 1.0;
export const VEHICLE_BLEND_S = 0.25;

/** `pos[3] vel[3] quat[4] yawRate forwardSpeed`. */
export const VEHICLE_STATE_LEN = 12;

export type VehicleCorrection = 'none' | 'blend' | 'snap';

export class VehiclePredictor {
  /** Predicted state after the last step (copy of the wasm array). */
  readonly state = new Float32Array(VEHICLE_STATE_LEN);
  /** Visual offset (m) still to be blended out, added to the predicted position by `view`. */
  readonly offset = { x: 0, y: 0, z: 0 };
  active = false;
  private defId = '';

  constructor(private readonly wasm: WasmPredictor) {}

  /** Enter the driver seat / first authoritative sample. `kin` = 11 floats (see `WasmPredictor.resetVehicle`). */
  enter(defId: string, kin: Float32Array): void {
    this.defId = defId;
    this.wasm.resetVehicle(defId, kin);
    this.active = true;
    this.offset.x = this.offset.y = this.offset.z = 0;
    this.state.fill(0);
    this.state.set(kin.subarray(0, 11));
  }

  leave(): void {
    if (!this.active) return;
    this.wasm.clearVehicle();
    this.active = false;
    this.offset.x = this.offset.y = this.offset.z = 0;
  }

  step(cmd: Uint8Array): boolean {
    if (!this.active) return false;
    const s = this.wasm.stepVehicle(cmd);
    if (!s) return false;
    this.state.set(s);
    return true;
  }

  /**
   * Rolls back to the authoritative kinematics `kin` and replays `cmds` (the commands the server has not
   * acknowledged yet). Returns how the error against the pose predicted before the rollback is corrected.
   */
  reconcile(kin: Float32Array, cmds: readonly Uint8Array[]): VehicleCorrection {
    if (!this.active) return 'none';
    const px = this.state[0] as number;
    const py = this.state[1] as number;
    const pz = this.state[2] as number;
    this.wasm.resetVehicle(this.defId, kin);
    for (const c of cmds) {
      const s = this.wasm.stepVehicle(c);
      if (s) this.state.set(s);
    }
    if (cmds.length === 0) this.state.set(kin.subarray(0, 11));
    const ex = px - (this.state[0] as number);
    const ey = py - (this.state[1] as number);
    const ez = pz - (this.state[2] as number);
    const err = Math.hypot(ex, ey, ez);
    if (err < VEHICLE_SOFT_M) {
      this.offset.x = this.offset.y = this.offset.z = 0;
      return 'none';
    }
    if (err >= VEHICLE_SNAP_M) {
      this.offset.x = this.offset.y = this.offset.z = 0;
      return 'snap';
    }
    // Show the old pose and blend out the difference.
    this.offset.x = ex;
    this.offset.y = ey;
    this.offset.z = ez;
    return 'blend';
  }

  /** Decays the visual offset by `dt` seconds (linear over `VEHICLE_BLEND_S`). */
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
