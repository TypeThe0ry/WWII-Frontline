import { NullEngine } from '@babylonjs/core/Engines/nullEngine';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Scene } from '@babylonjs/core/scene';
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { PH_NAME_RE, PlaceholderRegistry } from '../actors/names';
import { TANK_REQ, TankModel, requiredSockets, tankDefFromJson, type TankState } from './tank-model';

const json = JSON.parse(readFileSync(fileURLToPath(new URL('../../../../data/vehicles/us_m4a1_75.json', import.meta.url)), 'utf8')) as Parameters<typeof tankDefFromJson>[0];
const def = tankDefFromJson(json);
const base: TankState = { x: 10, y: 2, z: 20, yaw: 0.5, speed: 5, yawRate: 0, turretYaw: 0, gunPitchDeg: 0 };

function make() {
  const scene = new Scene(new NullEngine());
  const reg = new PlaceholderRegistry();
  return { scene, reg, tank: new TankModel(scene, def, reg, 0) };
}

describe('TankModel (M1-048b)', () => {
  it('names every node and material PH_V-VEH-US-003... and counts the requirement in the registry', () => {
    const { scene, reg } = make();
    const names = [...scene.meshes.map((m) => m.name), ...scene.materials.map((m) => m.name), ...scene.transformNodes.map((n) => n.name)].filter((n) => n !== 'default material');
    for (const n of names) expect(n, n).toMatch(PH_NAME_RE);
    expect(names.some((n) => n.startsWith(`PH_${TANK_REQ}`))).toBe(true);
    expect(reg.report().find((e) => e.id === TANK_REQ)!.instances).toBe(5); // hull, 2 tracks, turret body, barrel
  });

  it('has every socket, uses the data-file names as primary and resolves the task-card alias', () => {
    const { tank } = make();
    for (const s of requiredSockets()) expect(tank.sockets.has(s), s).toBe(true);
    expect(requiredSockets().length).toBe(10);
    for (const s of json.gameplay.sockets) expect(tank.sockets.has(s), `data socket ${s}`).toBe(true);
    expect(tank.sockets.has('socket_muzzle')).toBe(false); // alias only, no node of its own
    const a = tank.socketWorld('socket_muzzle', new Vector3());
    const b = tank.socketWorld('socket_gun_muzzle', new Vector3());
    expect(Vector3.Distance(a, b)).toBe(0);
  });

  it('hull box is sized from historical.dimensions_m (error < 1 %)', () => {
    const { tank } = make();
    tank.root.computeWorldMatrix(true);
    const hull = tank.meshes.find((m) => m.name.endsWith('_hull'))!;
    hull.refreshBoundingInfo();
    const ext = hull.getBoundingInfo().boundingBox.extendSize;
    const tracks = tank.meshes.filter((m) => m.name.includes('track'))[0]!;
    tracks.refreshBoundingInfo();
    // the track box spans the full data length; hull + two tracks span the full data width
    expect(Math.abs(tracks.getBoundingInfo().boundingBox.extendSize.z * 2 - def.dims.length) / def.dims.length).toBeLessThan(0.01);
    expect(Math.abs(ext.x * 2 + 2 * 0.38 - def.dims.width) / def.dims.width).toBeLessThan(0.01);
  });

  it('turret never outruns the data traverse rate and ends within 1 deg of the authoritative yaw', () => {
    const { tank } = make();
    const target = (150 * Math.PI) / 180;
    let prev = 0;
    let t = 0;
    let maxStep = 0;
    for (let i = 0; i < 1000; i++) {
      tank.update({ ...base, turretYaw: target }, 1 / 60);
      maxStep = Math.max(maxStep, Math.abs(tank.shownTurretYaw - prev));
      prev = tank.shownTurretYaw;
      t += 1 / 60;
      if (t > 7) break;
    }
    expect(maxStep).toBeLessThanOrEqual(def.turretTraverseRadS / 60 + 1e-9);
    expect(tank.turretError(target)).toBeLessThan((1 * Math.PI) / 180);
  });

  it('clamps gun pitch to the data range and the muzzle socket follows turret yaw', () => {
    const { tank } = make();
    tank.update({ ...base, gunPitchDeg: 80 }, 1 / 60);
    expect(tank.gun.rotation.x).toBeCloseTo(-(25 * Math.PI) / 180, 6);
    tank.update({ ...base, gunPitchDeg: -80 }, 1 / 60);
    expect(tank.gun.rotation.x).toBeCloseTo((10 * Math.PI) / 180, 6);
    const a = tank.socketWorld('socket_gun_muzzle', new Vector3());
    for (let i = 0; i < 400; i++) tank.update({ ...base, turretYaw: Math.PI / 2 }, 1 / 60);
    const b = tank.socketWorld('socket_gun_muzzle', new Vector3());
    expect(Vector3.Distance(a, b)).toBeGreaterThan(1);
  });

  it('tracks scroll at their own speed: turning in place scrolls them in opposite directions, stopped = still', () => {
    const { tank } = make();
    tank.update({ ...base, speed: 0, yawRate: 0 }, 0.1);
    expect([tank.trackOffsetL, tank.trackOffsetR]).toEqual([0, 0]);
    tank.update({ ...base, speed: 0, yawRate: 1 }, 0.1);
    expect(tank.trackOffsetL).toBeGreaterThan(0);
    expect(tank.trackOffsetR).toBeLessThan(0); // right track runs backwards while turning right in place
  });
});
