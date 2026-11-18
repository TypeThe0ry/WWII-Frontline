import { afterEach, describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../../game/event-bus';
import { GameState } from '../../game/state';
import { ActorManager } from '../actors/actor-manager';
import { HAVE_WASM, makeScene, realData, type TestScene } from '../actors/test-support';
import { FP_LAYER } from './fp-pass';
import { WALL_MARGIN_M } from './fp-math';

const d = describe.skipIf(!HAVE_WASM);
let ts: TestScene | undefined;
let mgr: ActorManager | undefined;
afterEach(() => {
  mgr?.dispose();
  mgr = undefined;
  ts?.dispose();
  ts = undefined;
});

async function setup(aiming = () => false) {
  const { data } = await realData();
  ts = makeScene();
  const bus = new EventBus<GameEvents>();
  const state = new GameState();
  state.local.pos.set([100, 5, 200]);
  const us = data.weapons.find((w) => w.faction === 'us')!;
  mgr = new ActorManager({ scene: ts.scene, bus, state, data, camera: ts.camera, localWeapon: () => us.index, aiming });
  mgr.update(1, 16); // attaches the weapon
  return { bus, state, us, data, m: mgr };
}

d('FpWeaponPass (M1-046b)', () => {
  it('weapon meshes are on the weapon layer, the world camera excludes it, two active cameras', async () => {
    const { m } = await setup();
    const w = m.rig!.weapon!;
    for (const mesh of w.meshes) expect(mesh.layerMask & FP_LAYER, mesh.name).toBe(FP_LAYER);
    expect(ts!.camera.layerMask & FP_LAYER).toBe(0);
    expect(m.fpPass!.camera.layerMask).toBe(FP_LAYER);
    const ac = ts!.scene.activeCameras!;
    expect(ac.length).toBe(2);
    expect(ac[0]).toBe(ts!.camera);
    expect(ac[1]).toBe(m.fpPass!.camera);
  });

  it('weapon FOV is independent of the world FOV (40-110 deg) and the ADS zoom (fp_fov_independent)', async () => {
    const { m, data } = await setup();
    const fp = m.fpPass!;
    const hip = data.hipFovDeg * (Math.PI / 180);
    for (const deg of [40, 60, 90, 110]) {
      ts!.camera.fov = (deg * Math.PI) / 180;
      fp.sync();
      expect(fp.camera.fov).toBeCloseTo(hip, 9);
    }
    // the weapon camera follows the world camera pose
    ts!.camera.position.set(1, 2, 3);
    ts!.camera.rotation.set(0.1, 0.2, 0);
    fp.sync();
    expect(fp.camera.position.asArray()).toEqual([1, 2, 3]);
    expect(fp.camera.rotation.asArray()).toEqual([0.1, 0.2, 0]);
  });

  it('wall probe pulls the weapon back so the muzzle stays 5 cm short of the wall (0.1 - 2 m)', async () => {
    const { m, state } = await setup();
    const rig = m.rig!;
    for (const wall of [0.1, 0.25, 0.5, 0.8, 1.2, 2]) {
      rig.wallProbe = () => wall;
      m.update(1, 16);
      const sock = rig.weapon!.muzzle;
      sock.computeWorldMatrix(true);
      const mw = sock.getWorldMatrix().m;
      // yaw 0 / pitch 0: forward = +z
      const reach = (mw[14] as number) - (ts!.camera.position.z);
      expect(reach, `wall at ${wall}`).toBeLessThanOrEqual(Math.max(wall - WALL_MARGIN_M, 0) + 1e-3);
      if (wall >= 0.5) expect(reach).toBeGreaterThan(0.3);
    }
    void state;
  });

  it('ads.state goes hip -> entering -> ads -> leaving -> hip on the bus', async () => {
    let aim = false;
    const { bus, m } = await setup(() => aim);
    const seen: string[] = [];
    bus.on('ads.state', (e) => seen.push(e.state));
    aim = true;
    for (let i = 0; i < 100; i++) m.update(1, 16);
    aim = false;
    for (let i = 0; i < 100; i++) m.update(1, 16);
    expect(seen).toEqual(['entering', 'ads', 'leaving', 'hip']);
  });
});
