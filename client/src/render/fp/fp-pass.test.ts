import '@babylonjs/core/Meshes/thinInstanceMesh';
import { Matrix } from '@babylonjs/core/Maths/math.vector';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../../game/event-bus';
import { GameState } from '../../game/state';
import { ActorManager } from '../actors/actor-manager';
import { freshWorldMatrix } from '../actors/world';
import { HAVE_WASM, makeScene, realData, type TestScene } from '../actors/test-support';
import { FP_LAYER } from './fp-pass';
import { WALL_MARGIN_M } from './fp-math';
import { createFpWallProbe } from './wall-probe';

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

  it('shows fire.local kick and muzzle flash in the same engine frame with no sim recoil', async () => {
    const { bus, m, state, us } = await setup();
    const weapon = m.rig!.weapon!;
    const before = weapon.root.position.z;
    let fireFrame = -1;
    let weaponRendered = false;
    const scene = ts!.scene;
    const rendered = scene.onBeforeCameraRenderObservable.add((camera) => {
      if (camera !== m.fpPass!.camera) return;
      weaponRendered = true;
      expect(ts!.engine.frameId).toBe(fireFrame);
      expect(weapon.root.position.z).toBeLessThan(before - 0.01);
      expect(m.fx.muzzleFlashFrames.at(-1)).toBe(fireFrame);
    });
    ts!.frame(() => {
      fireFrame = ts!.engine.frameId;
      bus.emit('fire.local', { weapon: us.index, muzzlePos: { x: 100, y: 6.6, z: 200 }, dir: { x: 0, y: 0, z: 1 } });
      m.update(1, 0);
    });
    expect(weaponRendered).toBe(true);
    expect(state.local.recoilPitch).toBe(0);
    scene.onBeforeCameraRenderObservable.remove(rendered);
    ts!.frame(() => m.update(1, 1000));
    expect(weapon.root.position.z).toBeCloseTo(before, 8);
  });

  it.each([30, 60, 144])('reload pose follows tactical and empty progress from game data at %i FPS', async (fps) => {
    const { m, us } = await setup();
    const path = fileURLToPath(new URL(`../../../../data/weapons/${us.id}.json`, import.meta.url));
    const item = JSON.parse(readFileSync(path, 'utf8')) as {
      gameplay: { reload: { tactical_ms: number; empty_ms: number } };
    };
    const rig = m.rig!;
    const baseline = rig.weapon!.root.position.y;
    let progress: number | undefined;
    rig.localReloadProgress = () => progress;
    const frameMs = 1000 / fps;
    for (const duration of Object.values(item.gameplay.reload)) {
      let frame = 0;
      let lowered = false;
      do {
        frame++;
        progress = frame * frameMs < duration ? (frame * frameMs) / duration : undefined;
        m.update(1, frameMs);
        lowered ||= rig.weapon!.root.position.y < baseline - 0.1;
      } while (progress !== undefined);
      expect(lowered).toBe(true);
      expect(frame * frameMs - duration).toBeLessThanOrEqual(frameMs);
      expect(rig.weapon!.root.position.y).toBeCloseTo(baseline, 8);
    }
  });

  it('keeps all weapon mesh bounds short of a real head-on wall', async () => {
    const { m } = await setup();
    const { camera, scene } = ts!;
    const wall = MeshBuilder.CreateBox('wall', { width: 4, height: 4, depth: 0.2 }, scene);
    wall.position.copyFrom(camera.position);
    wall.position.z += 0.2;
    wall.computeWorldMatrix(true);
    wall.metadata = { blocks: true };
    m.rig!.wallProbe = createFpWallProbe(scene);
    m.update(1, 16);
    expect(m.rig!.wallDist).toBeCloseTo(0.1, 4);
    for (const mesh of m.rig!.weapon!.meshes) {
      freshWorldMatrix(mesh);
      expect(mesh.getBoundingInfo().boundingBox.maximumWorld.z).toBeLessThan(camera.position.z + 0.1);
    }
  });
});

describe('createFpWallProbe', () => {
  it('hits streamed thin-instance boxes and ignores gaps and unrelated meshes', () => {
    ts = makeScene();
    const { scene } = ts;
    const probe = createFpWallProbe(scene, (mesh) => mesh.name.startsWith('hedge_'));
    const wall = MeshBuilder.CreateBox('wall', { width: 10, height: 2, depth: 1 }, scene);
    wall.position.z = 4;
    wall.computeWorldMatrix(true);
    wall.metadata = { blocks: true };
    const hedge = MeshBuilder.CreateBox('hedge_chunk', { size: 1 }, scene);
    hedge.thinInstanceSetBuffer('matrix', Float32Array.from([...Matrix.Translation(-2, 0, 2).m, ...Matrix.Translation(2, 0, 2).m]), 16, true);
    hedge.thinInstanceRefreshBoundingInfo(true);
    hedge.isPickable = false;
    MeshBuilder.CreateBox('unrelated', { size: 0.4 }, scene).position.set(0, 0, 1);
    expect(probe(2, 0, 0, 0, 0, 1, 2)).toBeCloseTo(1.5);
    expect(probe(0, 0, 0, 0, 0, 1, 2)).toBeNull();
    expect(probe(0, 0, 0, 0, 0, 1, 5)).toBeCloseTo(3.5);
    expect(hedge.isPickable).toBe(false);
    hedge.dispose();
    expect(probe(2, 0, 0, 0, 0, 1, 5)).toBeCloseTo(3.5);
  });
});
