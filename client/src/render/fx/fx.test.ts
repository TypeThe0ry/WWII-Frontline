import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { afterEach, describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../../game/event-bus';
import { GameState } from '../../game/state';
import { ActorManager } from '../actors/actor-manager';
import { freshWorldMatrix } from '../actors/world';
import { HAVE_WASM, makeScene, realData, type TestScene } from '../actors/test-support';
import { TRACER_LENGTH_M } from './tracer';
import { DUST_LIFE_S, DUST_PER_IMPACT, MARK_LIFE_S } from './impact-dust';

const d = describe.skipIf(!HAVE_WASM);
let ts: TestScene | undefined;
let mgr: ActorManager | undefined;
afterEach(() => {
  mgr?.dispose();
  mgr = undefined;
  ts?.dispose();
  ts = undefined;
});

async function setup(opts: { aiming?: () => boolean } = {}) {
  const { data } = await realData();
  ts = makeScene();
  const bus = new EventBus<GameEvents>();
  const state = new GameState();
  state.local.pos.set([100, 5, 200]);
  const us = data.weapons.find((w) => w.faction === 'us')!;
  const ger = data.weapons.find((w) => w.faction === 'ger')!;
  let clock = 0;
  mgr = new ActorManager({ scene: ts.scene, bus, state, data, camera: ts.camera, localWeapon: () => us.index, aiming: opts.aiming, now: () => clock });
  const advance = (ms: number) => {
    clock += ms;
  };
  return { data, bus, state, us, ger, ts, mgr, advance, clock: () => clock };
}

const dir = { x: 0, y: 0, z: 1 };
const muzzle = { x: 100, y: 6.6, z: 200 };

d('muzzle flash', () => {
  it('30 consecutive local shots: fire event frame and flash-visible frame are equal (difference 0)', async () => {
    const { bus, us, mgr: m, advance } = await setup();
    for (let i = 0; i < 30; i++) {
      ts!.frame(() => {
        m.update(1, 16);
        advance(16);
        bus.emit('fire.local', { weapon: us.index, muzzlePos: muzzle, dir });
      });
      // a few quiet frames between shots, as in play
      ts!.frame(() => m.update(1, 16));
    }
    const fire = m.fx.fireEventFrames;
    const flash = m.fx.muzzleFlashFrames;
    expect(fire.length).toBe(30);
    expect(flash.length).toBe(30);
    for (let i = 0; i < 30; i++) expect(flash[i]! - fire[i]!, `shot ${i}`).toBe(0);
    expect(new Set(fire).size).toBe(30); // really 30 different frames
    expect(flash.every((f) => f >= 0)).toBe(true);
  });

  it('shows for fx.muzzle_flash_ms (data) at socket_muzzle and follows the weapon', async () => {
    const { bus, us, mgr: m, advance, state } = await setup();
    ts!.frame(() => m.update(1, 16));
    ts!.frame(() => {
      bus.emit('fire.local', { weapon: us.index, muzzlePos: muzzle, dir });
    });
    expect(m.fx.flashes.active).toBe(1);
    const flash = m.fx.flashes.visibleMeshes()[0]!;
    const sock = freshWorldMatrix(m.rig!.weapon!.muzzle).m;
    expect(flash.position.x).toBeCloseTo(sock[12]!, 5);
    expect(flash.position.y).toBeCloseTo(sock[13]!, 5);
    expect(flash.position.z).toBeCloseTo(sock[14]!, 5);
    expect(us.muzzleFlashMs).toBe(50);
    advance(us.muzzleFlashMs - 1);
    ts!.frame(() => m.update(1, 16));
    expect(m.fx.flashes.active).toBe(1);
    // the weapon moves (player walks): the flash follows the socket
    state.local.pos.set([103, 5, 200]);
    ts!.frame(() => m.update(1, 16));
    expect(flash.position.x).toBeGreaterThan(102);
    advance(2);
    ts!.frame(() => m.update(1, 16));
    expect(m.fx.flashes.active).toBe(0);
    expect(flash.isEnabled()).toBe(false);
  });

  it('remote shots flash at the shooter weapon socket (third-person model)', async () => {
    const { bus, ger, state, mgr: m } = await setup();
    state.entities.count = 1;
    state.entities.ids[0] = 21;
    state.entities.pos.set([110, 3, 210]);
    state.entities.weapon[0] = ger.index;
    state.entities.flags[0] = 0x80;
    state.entities.health[0] = 255;
    ts!.frame(() => m.update(1, 16));
    ts!.frame(() => bus.emit('fire.remote', { shooter: 21, weapon: ger.index, muzzlePos: { x: 0, y: 0, z: 0 }, dir, distance: 12 }));
    const f = m.fx.flashes.visibleMeshes()[0]!;
    const sock = freshWorldMatrix(m.actor(21)!.weapon!.muzzle).m;
    expect(f.position.x).toBeCloseTo(sock[12]!, 5);
    expect(f.position.z).toBeCloseTo(sock[14]!, 5);
    expect(Math.hypot(f.position.x - 110, f.position.z - 210)).toBeLessThan(3); // near the shooter, not at the event's (0,0,0)
    expect(m.fx.fireEventFrames.length).toBe(1);
    expect(m.fx.fireEventFrames[0]).toBe(m.fx.muzzleFlashFrames[0]);
  });
});

d('tracer and impact', () => {
  it('tracer flies along the shot direction and is gone after its range; an impact shortens it', async () => {
    const { bus, us, mgr: m, advance } = await setup();
    const o = { ...dir };
    ts!.frame(() => {
      m.update(1, 16);
      advance(16);
      bus.emit('fire.local', { weapon: us.index, muzzlePos: muzzle, dir: o });
    });
    expect(m.fx.tracers.active).toBe(1);
    // impact 30 m ahead of the muzzle on the line
    const sock = freshWorldMatrix(m.rig!.weapon!.muzzle).m;
    ts!.frame(() => {
      advance(16);
      bus.emit('impact', { pos: { x: sock[12]!, y: sock[13]!, z: sock[14]! + 30 }, normal: 0, material: 0 });
    });
    expect(m.fx.dust.activeDust).toBe(DUST_PER_IMPACT);
    expect(m.fx.dust.activeMarks).toBe(1);
    // 30 m at <= 600 m/s: gone within about 0.1 s of simulated time
    let frames = 0;
    while (m.fx.tracers.active > 0 && frames < 120) {
      ts!.frame(() => {
        advance(16);
        m.update(1, 16);
      });
      frames++;
    }
    expect(m.fx.tracers.active).toBe(0);
    expect(frames).toBeLessThan(15);
    expect(frames).toBeGreaterThan(0);
    expect(TRACER_LENGTH_M).toBeGreaterThan(0);
  });

  it('without an impact the tracer lives until the visual range cap', async () => {
    const { bus, us, mgr: m, advance } = await setup();
    ts!.frame(() => {
      m.update(1, 16);
      advance(16);
      bus.emit('fire.local', { weapon: us.index, muzzlePos: muzzle, dir });
    });
    for (let i = 0; i < 20; i++) ts!.frame(() => (advance(16), m.update(1, 16)));
    expect(m.fx.tracers.active).toBe(1);
    for (let i = 0; i < 200; i++) ts!.frame(() => (advance(16), m.update(1, 16)));
    expect(m.fx.tracers.active).toBe(0);
  });

  it('tracers can be turned off', async () => {
    const { bus, us, mgr: m } = await setup();
    m.fx.tracersOn = false;
    ts!.frame(() => bus.emit('fire.local', { weapon: us.index, muzzlePos: muzzle, dir }));
    expect(m.fx.tracers.active).toBe(0);
    expect(m.fx.flashes.active).toBe(1);
  });

  it('dust cubes and the ground mark expire (DUST_LIFE_S / MARK_LIFE_S)', async () => {
    const { bus, mgr: m, advance } = await setup();
    ts!.frame(() => bus.emit('impact', { pos: { x: 1, y: 2, z: 3 }, normal: 0, material: 0 }));
    expect(m.fx.dust.activeDust).toBe(DUST_PER_IMPACT);
    for (let t = 0; t < DUST_LIFE_S * 1000 + 100; t += 16) ts!.frame(() => (advance(16), m.update(1, 16)));
    expect(m.fx.dust.activeDust).toBe(0);
    expect(m.fx.dust.activeMarks).toBe(1);
    for (let t = 0; t < MARK_LIFE_S * 1000 + 100; t += 16) ts!.frame(() => (advance(16), m.update(1, 16)));
    expect(m.fx.dust.activeMarks).toBe(0);
  });
});

d('first-person camera and weapon', () => {
  it('camera sits at feet + eye height and looks along yaw / pitch (forward = (sin y cos p, sin p, cos y cos p))', async () => {
    const { state, mgr: m, data } = await setup();
    state.local.yaw = 0.7;
    state.local.pitch = 0.3;
    m.update(1, 16);
    const cam = ts!.camera;
    expect(cam.position.x).toBeCloseTo(100, 6);
    expect(cam.position.y).toBeCloseTo(5 + data.capsule.eyeStand, 6);
    expect(cam.position.z).toBeCloseTo(200, 6);
    const f = cam.getDirection(Vector3.Forward());
    expect(f.x).toBeCloseTo(Math.sin(0.7) * Math.cos(0.3), 5);
    expect(f.y).toBeCloseTo(Math.sin(0.3), 5);
    expect(f.z).toBeCloseTo(Math.cos(0.7) * Math.cos(0.3), 5);
  });

  it('eye height eases to the crouch eye height', async () => {
    const { state, mgr: m, data } = await setup();
    state.local.stance = 1;
    m.update(1, data.stanceTransitionMs * 2);
    expect(ts!.camera.position.y).toBeCloseTo(5 + data.capsule.eyeCrouch, 6);
  });

  it('ADS blends the weapon to the view centre and narrows the FOV over ads_ms (data)', async () => {
    let aim = false;
    const { state, mgr: m, data, us } = await setup({ aiming: () => aim });
    m.update(1, 16);
    const w = m.rig!.weapon!;
    const hipFov = ts!.camera.fov;
    expect(hipFov).toBeCloseTo((data.hipFovDeg * Math.PI) / 180, 6);
    const hipX = w.root.position.x - state.local.pos[0]!;
    aim = true;
    m.update(1, us.adsMs / 2);
    expect(ts!.camera.fov).toBeLessThan(hipFov);
    expect(m.rig!.ads).toBeGreaterThan(0.4);
    expect(m.rig!.ads).toBeLessThan(0.6);
    m.update(1, us.adsMs);
    expect(m.rig!.ads).toBe(1);
    expect(ts!.camera.fov).toBeCloseTo(hipFov * us.view.adsFovMult, 6);
    const adsX = w.root.position.x - state.local.pos[0]!;
    expect(Math.abs(adsX)).toBeLessThan(Math.abs(hipX));
    expect(adsX).toBeCloseTo(us.view.adsOffset[0], 6); // yaw 0: right = +x
    aim = false;
    m.update(1, us.adsMs * 2);
    expect(ts!.camera.fov).toBeCloseTo(hipFov, 6);
  });

  it('predicted recoil kicks the weapon up and back', async () => {
    const { state, mgr: m } = await setup();
    m.update(1, 16);
    const w = m.rig!.weapon!;
    const z0 = w.root.position.z;
    const rx0 = w.root.rotation.x;
    state.local.recoilPitch = 2;
    m.update(1, 16);
    expect(w.root.position.z).toBeLessThan(z0);
    expect(w.root.rotation.x).toBeLessThan(rx0); // rotation.x negative = muzzle up
  });

  it('weapon change swaps the first-person model (Garand -> Kar98k) and keeps the names', async () => {
    const { mgr: m, ger } = await setup();
    m.update(1, 16);
    expect(m.rig!.weapon!.root.name).toBe('PH_V-WPN-US-001__fp');
    m.attachWeaponFP(ger.index);
    expect(m.rig!.weapon!.root.name).toBe('PH_V-WPN-GE-001__fp');
    expect(ts!.scene.getTransformNodeByName('PH_V-WPN-US-001__fp')).toBeNull();
  });
});

d('GPU particles (M1-047b)', () => {
  async function withParticles(tier: 'high' | 'low') {
    const { data } = await realData();
    ts = makeScene();
    const bus = new EventBus<GameEvents>();
    const state = new GameState();
    state.local.pos.set([100, 5, 200]);
    const us = data.weapons.find((w) => w.faction === 'us')!;
    mgr = new ActorManager({ scene: ts.scene, bus, state, data, camera: ts.camera, localWeapon: () => us.index, now: () => 0, particles: { tier, wgsl: false } });
    return { bus, us, m: mgr };
  }

  it('fire.local -> muzzle flash + shell; impact -> dust; explosion -> blast + smoke, scaled by radius', async () => {
    const { bus, us, m } = await withParticles('high');
    const pool = m.fx.particlePool!;
    expect(pool.active).toBe(0);
    ts!.frame(() => bus.emit('fire.local', { weapon: us.index, muzzlePos: muzzle, dir }));
    const afterFire = pool.active;
    expect(afterFire).toBe(2); // 1 flash + 1 shell casing
    ts!.frame(() => bus.emit('impact', { pos: { x: 105, y: 5, z: 210 }, normal: 0, material: 0 }));
    expect(pool.active).toBeGreaterThan(afterFire);
    const before = pool.active;
    const cp = ts!.camera.globalPosition; // near the camera: far explosions merge into one particle
    ts!.frame(() => bus.emit('explosion', { pos: { x: cp.x, y: cp.y, z: cp.z + 10 }, radius: 8, own: false }));
    expect(pool.active - before).toBe(9); // 3 blast + 6 smoke
    expect(m.fx.particles!.drawn).toBe(pool.active);
  });

  it('Low tier ejects no shell casings and caps the pool at 300', async () => {
    const { bus, us, m } = await withParticles('low');
    ts!.frame(() => bus.emit('fire.local', { weapon: us.index, muzzlePos: muzzle, dir }));
    expect(m.fx.particlePool!.active).toBe(1); // flash only
    expect(m.fx.particlePool!.capacity).toBe(300);
  });

  it('without the particles option nothing is created', async () => {
    const { mgr: m } = await setup();
    expect(m.fx.particlePool).toBeNull();
    expect(m.fx.particles).toBeNull();
  });
});
