import { Quaternion, Vector3 } from '@babylonjs/core/Maths/math.vector';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PlaceholderRegistry } from '../actors/names';
import { makeScene } from '../actors/test-support';
import { angleDiff } from './tank-math';
import { TANK_REQ, TRACK_REPEAT_M, tankDefFromJson } from './tank-model';
import { attachVehicleDemo } from './vehicle-demo';
import { VehicleRenderer, type VehicleView } from './vehicle-renderer';

const data = readFileSync(fileURLToPath(new URL('../../../../data/vehicles/us_m4a1_75.json', import.meta.url)), 'utf8');
const def = tankDefFromJson(JSON.parse(data) as Parameters<typeof tankDefFromJson>[0]);
const view: VehicleView = { id: 42, x: 5, y: 0, z: 30, yaw: 0, speed: 0, yawRate: 0, turretYaw: 0, gunPitchDeg: 0 };
let testScene: ReturnType<typeof makeScene> | undefined;
let renderer: VehicleRenderer | undefined;

afterEach(() => {
  renderer?.dispose();
  renderer = undefined;
  testScene?.dispose();
  testScene = undefined;
  vi.unstubAllGlobals();
});

function setup() {
  testScene = makeScene();
  const registry = new PlaceholderRegistry();
  const emitTankShot = vi.fn<(x: number, y: number, z: number, own: boolean) => void>();
  renderer = new VehicleRenderer(testScene.scene, def, registry, { emitTankShot });
  return { ...testScene, registry, r: renderer, emitTankShot };
}

describe('VehicleRenderer', () => {
  it('allocates by ID, exposes both muzzle names, and removes despawned sockets and resources', () => {
    const { r, registry, scene } = setup();
    r.sync([view, { ...view, id: 77 }], 0);
    const model = r.getModel(view.id);
    expect(model).toBeDefined();
    expect(r.getSocket(view.id, 'socket_muzzle')).toBe(r.getSocket(view.id, 'socket_gun_muzzle'));
    expect(r.getSocket(view.id, 'missing')).toBeNull();
    expect(registry.report().find((entry) => entry.id === TANK_REQ)?.instances).toBe(10);
    expect(scene.textures.length).toBe(4);
    r.sync([view], 0);
    expect(r.getModel(view.id)).toBe(model);
    expect(r.getSocket(77, 'socket_gun_muzzle')).toBeNull();
    expect(scene.textures.length).toBe(2);
    expect(scene.materials.filter((m) => m.name.includes('__track_')).length).toBe(2);
    r.dispose();
    expect(registry.report().find((entry) => entry.id === TANK_REQ)?.instances).toBe(0);
  });

  it('renders 1,000 sampled turret angles across the wrap with <1 degree error and scrolls tracks at 20 km/h', () => {
    const { r } = setup();
    const dt = 1 / 60;
    const speed = 20 / 3.6;
    let worstYawError = 0;
    for (let i = 0; i < 1000; i++) {
      const yaw = angleDiff(0, 3 + i * dt * def.turretTraverseRadS);
      r.sync([{ ...view, turretYaw: yaw, speed }], dt);
      const model = r.getModel(view.id)!;
      worstYawError = Math.max(worstYawError, model.turretError(yaw));
      if (i === 0) {
        const offsets = model.meshes.filter((mesh) => mesh.name.includes('_track_')).map((mesh) => {
          const [texture] = mesh.material!.getActiveTextures();
          if (!texture || !('vOffset' in texture)) throw new Error('missing track texture');
          return (texture.vOffset as number) * TRACK_REPEAT_M / dt;
        });
        expect(offsets).toHaveLength(2);
        for (const measured of offsets) expect(Math.abs(measured - speed)).toBeLessThanOrEqual(speed * 0.05);
      }
    }
    expect(worstYawError * 180 / Math.PI).toBeLessThan(1);
  });

  it('emits queued muzzle FX before render with the updated hull quaternion, turret and gun pose', () => {
    const { r, frame, scene, engine, emitTankShot } = setup();
    r.sync([view], 0);
    const oldMuzzle = r.getSocket(view.id, 'socket_gun_muzzle')!.getTranslation();
    const hullQuat = Quaternion.RotationYawPitchRoll(0.7, 0.15, -0.2);
    emitTankShot.mockImplementation(() => expect(engine.frameId).toBe(shotFrame));
    let shotFrame = -1;
    r.queueShot(77);
    r.queueShot(view.id, true);
    expect(emitTankShot).not.toHaveBeenCalled();
    let didRender = false;
    scene.onBeforeRenderObservable.addOnce(() => {
      didRender = true;
      expect(emitTankShot).toHaveBeenCalledTimes(1);
    });
    frame(() => {
      shotFrame = engine.frameId;
      r.sync([{ ...view, x: view.x + 10, hullQuat, turretYaw: Math.PI / 2, gunPitchDeg: 20 }], 1 / 60);
    });
    expect(didRender).toBe(true);
    const model = r.getModel(view.id)!;
    const current = model.socketWorld('socket_muzzle', Vector3.Zero());
    const origin = new Vector3(view.x + 10, view.y, view.z);
    const gunRotation = Quaternion.RotationYawPitchRoll(0, -20 * Math.PI / 180, 0);
    const turretRotation = Quaternion.RotationYawPitchRoll(Math.PI / 2, 0, 0);
    const expected = model.sockets.get('socket_gun_muzzle')!.position
      .applyRotationQuaternion(gunRotation).add(model.gun.position)
      .applyRotationQuaternion(turretRotation).add(model.turret.position)
      .applyRotationQuaternion(hullQuat).add(origin);
    expect(Vector3.Distance(current, expected)).toBeLessThan(1e-5);
    expect(Vector3.Distance(current, oldMuzzle)).toBeGreaterThan(5);
    expect(emitTankShot).toHaveBeenCalledWith(current.x, current.y, current.z, true);
    r.sync([view], 1 / 60);
    expect(emitTankShot).toHaveBeenCalledTimes(1);
  });

  it('runs the local demo through the renderer and removes its frame callback on dispose', async () => {
    testScene = makeScene();
    vi.stubGlobal('fetch', vi.fn(async () => new Response(data)));
    let update: ((dtMs: number) => void) | undefined;
    const emitTankShot = vi.fn();
    const demo = await attachVehicleDemo({
      scene: testScene.scene,
      onFrame: (cb) => {
        update = cb;
        return () => { update = undefined; };
      },
    }, new URLSearchParams(), { emitTankShot }, () => 2, new PlaceholderRegistry());
    const before = demo.model.root.position.clone();
    for (let i = 0; i < 1000; i++) update!(1000 / 60);
    expect(demo.model.root.position.y).toBe(2);
    expect(Vector3.Distance(demo.model.root.position, before)).toBeGreaterThan(1);
    expect(demo.stats().shots).toBeGreaterThan(0);
    expect(emitTankShot).toHaveBeenCalledTimes(demo.stats().shots);
    demo.dispose();
    expect(update).toBeUndefined();
    expect(demo.model.root.isDisposed()).toBe(true);
  });
});
