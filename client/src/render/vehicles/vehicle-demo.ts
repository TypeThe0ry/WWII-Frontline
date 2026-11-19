/**
 * `?vehicle=demo` (M1-048b): one placeholder M4A1 driving a circle at 30 Hz authoritative updates, drawn with velocity-aided
 * extrapolation (`game/vehicle-interp.ts`), a turret that is told to swing every 4 s and fires a muzzle effect from
 * `socket_gun_muzzle` (047b particles). No server needed: it is the vehicle counterpart of `?actors=demo`.
 */
import { extrapolate } from '../../game/vehicle-interp';
import type { RenderContext } from '../engine';
import type { FxManager } from '../fx';
import { PlaceholderRegistry } from '../actors/names';
import { tankDefFromJson, type TankModel, type TankState } from './tank-model';
import { angleDiff, followAngle } from './tank-math';
import { VehicleRenderer, type VehicleView } from './vehicle-renderer';

export interface VehicleDemoHandle {
  model: TankModel;
  state: TankState;
  dispose(): void;
  /** Muzzle effects fired so far and the largest turret error (rad) between shown and authoritative yaw seen. */
  stats(): { shots: number; maxTurretErrRad: number };
}

const TICK_S = 1 / 30;
const RADIUS_M = 24;

export async function attachVehicleDemo(ctx: Pick<RenderContext, 'scene' | 'onFrame'>, params: URLSearchParams, fx: Pick<FxManager, 'emitTankShot'> | null, heightAt: (x: number, z: number) => number, registry: PlaceholderRegistry): Promise<VehicleDemoHandle> {
  const r = await fetch(`${import.meta.env.BASE_URL}data/vehicles/us_m4a1_75.json`);
  if (!r.ok) throw new Error(`us_m4a1_75.json: HTTP ${r.status} (run \`pnpm -C client sync-map\`)`);
  const def = tankDefFromJson((await r.json()) as Parameters<typeof tankDefFromJson>[0]);
  const renderer = new VehicleRenderer(ctx.scene, def, registry, fx);
  const c = (params.get('vehicle_pos') ?? '').split(',').map(Number);
  const cx = c.length === 2 && c.every(Number.isFinite) ? c[0]! : 128;
  const cz = c.length === 2 && c.every(Number.isFinite) ? c[1]! : 128;
  const speed = Math.min(def.maxSpeedMps, 5);
  const omega = speed / RADIUS_M; // yaw rate of a constant-speed circle (right turn)
  const state: TankState = { x: cx + RADIUS_M, y: heightAt(cx + RADIUS_M, cz), z: cz, yaw: Math.PI, speed, yawRate: omega, turretYaw: 0, gunPitchDeg: 0 };
  const shown: VehicleView = { id: 0, ...state };
  const views = [shown];
  renderer.sync(views, 0);
  const model = renderer.getModel(shown.id)!;
  let sinceTick = 0;
  let t = 0;
  let shots = 0;
  let maxErr = 0;
  let nextShot = 4;
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
  };
  const off = ctx.onFrame((dtMs) => {
    const dt = dtMs / 1000;
    sinceTick += dt;
    while (sinceTick >= TICK_S) {
      sinceTick -= TICK_S;
      advance(TICK_S);
    }
    // velocity-aided extrapolation between ticks (the same helper the remote vehicles use)
    const ageMs = sinceTick * 1000;
    const vx = state.speed * Math.sin(state.yaw);
    const vz = state.speed * Math.cos(state.yaw);
    shown.x = extrapolate(state.x, vx, ageMs);
    shown.z = extrapolate(state.z, vz, ageMs);
    shown.y = heightAt(shown.x, shown.z);
    shown.yaw = state.yaw + state.yawRate * (ageMs / 1000);
    shown.gunPitchDeg = state.gunPitchDeg;
    const before = shown.turretYaw;
    shown.turretYaw = followAngle(before, state.turretYaw, dt, def.turretTraverseRadS);
    maxErr = Math.max(maxErr, Math.abs(angleDiff(shown.turretYaw, state.turretYaw)));
    // fire when the swing has settled (shown == authoritative) after the order
    if (before !== state.turretYaw && shown.turretYaw === state.turretYaw) {
      shots++;
      renderer.queueShot(shown.id);
    }
    renderer.sync(views, dt);
  }, undefined, 'vehicle_update');
  const dispose = (): void => {
    off();
    renderer.dispose();
  };
  ctx.scene.onDisposeObservable.addOnce(dispose);
  return { model, state, dispose, stats: () => ({ shots, maxTurretErrRad: maxErr }) };
}
