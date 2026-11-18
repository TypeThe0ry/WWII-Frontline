/**
 * `?vehicle=demo` (M1-048b): one placeholder M4A1 driving a circle at 30 Hz authoritative updates, drawn with velocity-aided
 * extrapolation (`game/vehicle-interp.ts`), a turret that is told to swing every 4 s and fires a muzzle effect from
 * `socket_muzzle` (047b particles). No server needed: it is the vehicle counterpart of `?actors=demo`.
 */
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { extrapolate } from '../../game/vehicle-interp';
import type { RenderContext } from '../engine';
import type { FxManager } from '../fx';
import { PlaceholderRegistry } from '../actors/names';
import { TankModel, tankDefFromJson, type TankState } from './tank-model';

export interface VehicleDemoHandle {
  model: TankModel;
  state: TankState;
  /** Muzzle effects fired so far and the largest turret error (rad) between shown and authoritative yaw seen. */
  stats(): { shots: number; maxTurretErrRad: number };
}

const TICK_S = 1 / 30;
const RADIUS_M = 24;

export async function attachVehicleDemo(ctx: RenderContext, params: URLSearchParams, fx: FxManager | null, heightAt: (x: number, z: number) => number, registry: PlaceholderRegistry): Promise<VehicleDemoHandle> {
  const r = await fetch(`${import.meta.env.BASE_URL}data/vehicles/us_m4a1_75.json`);
  if (!r.ok) throw new Error(`us_m4a1_75.json: HTTP ${r.status} (run \`pnpm -C client sync-map\`)`);
  const def = tankDefFromJson((await r.json()) as Parameters<typeof tankDefFromJson>[0]);
  const model = new TankModel(ctx.scene, def, registry, 'demo');
  const c = (params.get('vehicle_pos') ?? '').split(',').map(Number);
  const cx = c.length === 2 && c.every(Number.isFinite) ? c[0]! : 128;
  const cz = c.length === 2 && c.every(Number.isFinite) ? c[1]! : 128;
  const speed = Math.min(def.maxSpeedMps, 5);
  const omega = speed / RADIUS_M; // yaw rate of a constant-speed circle (right turn)
  const state: TankState = { x: cx + RADIUS_M, y: 0, z: cz, yaw: Math.PI, speed, yawRate: omega, turretYaw: 0, gunPitchDeg: 0 };
  let sinceTick = 0;
  let t = 0;
  let shots = 0;
  let maxErr = 0;
  let nextShot = 4;
  const muzzle = new Vector3();
  let sample = { x: state.x, z: state.z, yaw: state.yaw };
  const advance = (dt: number): void => {
    t += dt;
    const a = t * omega; // angle on the circle, clockwise seen from above
    state.x = cx + RADIUS_M * Math.cos(a);
    state.z = cz - RADIUS_M * Math.sin(a);
    state.yaw = Math.PI + a; // heading of the circle, increasing = turning right (sim yaw)
    state.y = heightAt(state.x, state.z);
    state.gunPitchDeg = 8 * Math.sin(t);
    if (t >= nextShot) {
      nextShot += 4;
      state.turretYaw = ((shots % 2 === 0 ? 1 : -1) * 150 * Math.PI) / 180; // a long swing: the shown turret must not outrun the data rate
    }
    sample = { x: state.x, z: state.z, yaw: state.yaw };
  };
  ctx.onFrame((dtMs) => {
    const dt = dtMs / 1000;
    sinceTick += dt;
    while (sinceTick >= TICK_S) {
      sinceTick -= TICK_S;
      advance(TICK_S);
    }
    // velocity-aided extrapolation between ticks (the same helper the remote vehicles use)
    const ageMs = sinceTick * 1000;
    const vx = state.speed * Math.sin(sample.yaw);
    const vz = state.speed * Math.cos(sample.yaw);
    const shown: TankState = { ...state, x: extrapolate(sample.x, vx, ageMs), z: extrapolate(sample.z, vz, ageMs), yaw: sample.yaw + state.yawRate * (ageMs / 1000) };
    shown.y = heightAt(shown.x, shown.z);
    const before = model.shownTurretYaw;
    model.update(shown, dt);
    maxErr = Math.max(maxErr, Math.abs(model.shownTurretYaw - state.turretYaw));
    // fire when the swing has settled (shown == authoritative) after the order
    if (before !== state.turretYaw && model.shownTurretYaw === state.turretYaw) {
      shots++;
      if (fx) {
        model.socketWorld('socket_gun_muzzle', muzzle);
        fx.emitTankShot(muzzle.x, muzzle.y, muzzle.z);
      }
    }
  }, undefined, 'vehicle_update');
  return { model, state, stats: () => ({ shots, maxTurretErrRad: maxErr }) };
}
