import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../../game/event-bus';
import { GameState } from '../../game/state';
import { ActorManager } from './actor-manager';
import { weaponLayout } from './placeholder-weapon';
import { PH_NAME_RE, PlaceholderRegistry, REQ, SOCKET, phName } from './names';
import { JOINT_PARENT, type HumanoidJoint } from './skeleton';
import { MOTION, RELOAD_BIT } from './anim-graph';
import { freshWorldMatrix } from './world';
import { HAVE_WASM, makeScene, realData, type TestScene } from './test-support';

const d = describe.skipIf(!HAVE_WASM);
let ts: TestScene | undefined;
let mgr: ActorManager | undefined;
afterEach(() => {
  mgr?.dispose();
  mgr = undefined;
  ts?.dispose();
  ts = undefined;
});

async function setup(n = 2, patchRole?: Parameters<typeof realData>[0]) {
  const { data } = await realData(patchRole);
  ts = makeScene();
  const bus = new EventBus<GameEvents>();
  const state = new GameState();
  state.entities.count = n;
  const us = data.weapons.find((w) => w.faction === 'us')!;
  const ger = data.weapons.find((w) => w.faction === 'ger')!;
  for (let k = 0; k < n; k++) {
    state.entities.ids[k] = 10 + k;
    state.entities.pos[k * 3] = k * 3;
    state.entities.weapon[k] = k % 2 === 0 ? us.index : ger.index;
    state.entities.health[k] = 255;
    state.entities.flags[k] = k % 2 === 0 ? 0 : 0x80;
  }
  mgr = new ActorManager({ scene: ts.scene, bus, state, data, camera: ts.camera, localWeapon: () => us.index, aiming: () => false });
  return { data, bus, state, us, ger, ts, mgr };
}

d('placeholder naming', () => {
  it('every placeholder mesh and material name matches PH_V-XXX-... and nothing else starts with PH_V', async () => {
    const { state, mgr: m } = await setup(4);
    m.update(1, 16);
    const names = [...ts!.scene.meshes.map((x) => x.name), ...ts!.scene.materials.map((x) => x.name)];
    expect(names.length).toBeGreaterThan(30);
    for (const n of names) {
      if (n.startsWith('PH_V-')) expect(n, n).toMatch(PH_NAME_RE);
      // not a placeholder: must not use the PH_ prefix for something that is not a requirement-bound placeholder
      else expect(n.startsWith('PH_V'), n).toBe(false);
    }
    expect(state.entities.count).toBe(4);
  });

  it('window.__placeholders lists at least the 6 requirement ids with placeholder:true', async () => {
    const { mgr: m } = await setup(4);
    m.update(1, 16);
    const rep = m.registry.report();
    const ids = rep.map((r) => r.id);
    for (const id of Object.values(REQ)) expect(ids).toContain(id);
    expect(ids.length).toBeGreaterThanOrEqual(6);
    expect(rep.every((r) => r.placeholder === true)).toBe(true);
    const target: { __placeholders?: unknown } = {};
    m.registry.install(target);
    expect(Array.isArray(target.__placeholders)).toBe(true);
    // live instance counts: 2 US-faction and 2 GER-faction actors -> character + third-person weapon + first-person weapon
    const get = (id: string) => rep.find((r) => r.id === id)!.instances;
    expect(get(REQ.CHR_US)).toBe(2);
    expect(get(REQ.CHR_GE)).toBe(2);
    expect(get(REQ.WPN_US)).toBe(2 + 1); // 2 third-person + the local first-person Garand
    expect(get(REQ.WPN_GE)).toBe(2);
  });

  it('PlaceholderRegistry counts down on dispose', () => {
    const r = new PlaceholderRegistry();
    r.add('V-X-001', 2);
    r.add('V-X-001', -1);
    expect(r.report()).toEqual([{ id: 'V-X-001', instances: 1, placeholder: true }]);
    r.add('V-X-001', -5);
    expect(r.report()[0]!.instances).toBe(0);
  });
});

d('sockets', () => {
  it('parents Humanoid joints and namespaced sockets, retaining the weapon-local muzzle', async () => {
    const { mgr: m, state } = await setup(2);
    m.update(1, 16);
    expect(state.entities.count).toBe(2);
    const wnames = (root: { getChildTransformNodes(d: boolean): { name: string }[] }) => root.getChildTransformNodes(true).map((n) => n.name);
    for (const s of m.allActors()) {
      const a = s.actor;
      const { joints, sockets } = a.skeleton;
      const req = s.faction === 'us' ? REQ.CHR_US : REQ.CHR_GE;
      for (const joint of Object.keys(JOINT_PARENT) as HumanoidJoint[]) {
        const parent = JOINT_PARENT[joint];
        expect(joints[joint].name).toBe(phName(req, `${a.id}_${joint}`));
        expect(joints[joint].parent).toBe(parent ? joints[parent] : a.root);
      }
      expect(sockets[SOCKET.head].parent).toBe(joints.Head);
      expect(sockets[SOCKET.handR].parent).toBe(joints.RightHand);
      expect(sockets[SOCKET.handL].parent).toBe(joints.LeftHand);
      expect(sockets[SOCKET.back].parent).toBe(joints.Chest);
      for (const key of Object.keys(sockets) as (keyof typeof sockets)[]) {
        expect(sockets[key].name).toBe(phName(req, `${a.id}_${key}`));
        expect(sockets[key].name).toMatch(PH_NAME_RE);
      }
      expect(a.weapon).not.toBeNull();
      expect(wnames(a.weapon!.root)).toEqual(expect.arrayContaining(['socket_muzzle', 'socket_ejection', 'socket_magazine']));
      expect(a.weapon!.root.parent).toBe(a.socketHandR);
      expect(a.weapon!.root.name).toMatch(/^PH_V-WPN-(US|GE)-001__tp_/);
      expect(a.root.name).toMatch(/^PH_V-CHR-(US|GE)-001__1[01]$/);
    }
    const fp = m.rig!.weapon!;
    expect(wnames(fp.root)).toEqual(expect.arrayContaining(['socket_muzzle', 'socket_ejection', 'socket_magazine']));
    expect(fp.root.name).toBe('PH_V-WPN-US-001__fp');
  });

  it('weapon layout: muzzle at the front of the barrel, sizes from data', async () => {
    const { data } = await realData();
    for (const w of data.weapons) {
      const l = weaponLayout(w.dims);
      const rear = -0.45 * w.dims.length;
      expect(l.muzzleZ).toBeCloseTo(rear + w.dims.length, 9);
      expect(l.sockets.muzzle[2]).toBe(l.muzzleZ);
      expect(l.receiver.size[2] + l.barrel.length).toBeCloseTo(w.dims.length, 9);
      expect(l.sockets.ejection[0]).toBeGreaterThan(0); // right side
      expect(l.sockets.magazine[1]).toBeLessThan(0); // below the receiver
    }
  });
});

d('stance, orientation and death', () => {
  it('uses data heights and the GD 9.2 time for stand → crouch → prone', async () => {
    const { mgr: m, state, data } = await setup(1);
    const c = data.capsule;
    m.update(1, 16);
    const a = m.actor(10)!;
    expect(a.height).toBeCloseTo(c.heightStand, 6);
    state.entities.stance[0] = 1;
    m.update(1, 125);
    expect(a.height).toBeCloseTo((c.heightStand + c.heightCrouch) / 2, 6);
    m.update(1, 125);
    expect(a.height).toBeCloseTo(c.heightCrouch, 6);
    state.entities.stance[0] = 2;
    m.update(1, 499);
    const prone = Math.min(c.heightCrouch, c.radius * 2.2);
    expect(a.height).toBeGreaterThan(prone);
    m.update(1, 1);
    expect(a.height).toBeCloseTo(prone, 6);
  });

  it('position / yaw / pitch follow the entity state; dead entities lie down', async () => {
    const { mgr: m, state } = await setup(1);
    state.entities.pos.set([5, 2, 7]);
    state.entities.yaw[0] = 1.25;
    state.entities.pitch[0] = 0.4;
    m.update(1, 16);
    const a = m.actor(10)!;
    expect(a.root.position.x).toBe(5);
    expect(a.root.position.y).toBe(2);
    expect(a.root.position.z).toBe(7);
    expect(a.root.rotation.y).toBeCloseTo(1.25, 6);
    expect(a.aim.rotation.x).toBeCloseTo(-0.4, 6);
    expect(a.dead).toBe(false);
    state.entities.health[0] = 0;
    m.update(1, 16);
    expect(a.dead).toBe(true);
    expect(a.root.rotation.z).toBeCloseTo(Math.PI / 2, 6);
  });

  it('actors are created for new ids, removed when they disappear, and not drawn for the local player', async () => {
    const { mgr: m, state, bus } = await setup(2);
    m.update(1, 16);
    expect(m.actorCount).toBe(2);
    expect(m.lastUpdate).toEqual({ entities: 2, actors: 2 });
    state.entities.count = 1; // id 11 gone from the frame data
    m.update(1, 16);
    expect(m.actorCount).toBe(1);
    const a = m.actor(10)!;
    const muzzle = a.weapon!.muzzle;
    bus.emit('entity.removed', { id: 10, reason: 'removed' });
    expect(m.actorCount).toBe(0);
    expect(a.skeleton.joints.Hips.isDisposed()).toBe(true);
    expect(a.socketHandL.isDisposed()).toBe(true);
    expect(muzzle.isDisposed()).toBe(true);
    expect(m.registry.report().find((r) => r.id === REQ.CHR_US)!.instances).toBe(0);
    // no character node named after a local id: the local player has no capsule
    expect(ts!.scene.meshes.filter((x) => /^PH_V-CHR-(US|GE)-001__\d+_body$/.test(x.name)).length).toBe(0);
  });
});

d('data driven sizes', () => {
  it('capsule height / radius follow data/roles/rifleman.json (patched role -> different body)', async () => {
    const a = await setup(1);
    a.mgr.update(1, 16);
    const body = ts!.scene.getMeshByName('PH_V-CHR-US-001__src_body')!;
    const h0 = body.getBoundingInfo().boundingBox.extendSize.y * 2;
    expect(h0).toBeCloseTo(a.data.capsule.heightStand, 3);
    const w0 = body.getBoundingInfo().boundingBox.extendSize.x * 2;
    const r0 = a.data.capsule.radius;
    mgr!.dispose();
    mgr = undefined;
    ts!.dispose();
    ts = undefined;
    const b = await setup(1, (r) => {
      r.capsule.height_stand_m = 2.2;
      r.capsule.radius_m = 0.5;
    });
    b.mgr.update(1, 16);
    const body2 = ts!.scene.getMeshByName('PH_V-CHR-US-001__src_body')!;
    expect(body2.getBoundingInfo().boundingBox.extendSize.y * 2).toBeCloseTo(2.2, 3);
    expect((body2.getBoundingInfo().boundingBox.extendSize.x * 2) / w0).toBeCloseTo(0.5 / r0, 3);
  });

  it('no capsule literals in the actors source (grep acceptance)', () => {
    const dir = fileURLToPath(new URL('.', import.meta.url));
    for (const f of readdirSync(dir)) {
      if (!f.endsWith('.ts') || f.endsWith('.test.ts') || f === 'test-support.ts') continue;
      const src = readFileSync(`${dir}${f}`, 'utf8');
      expect(src, f).not.toMatch(/\b1\.8\b/);
      expect(src, f).not.toMatch(/\b0\.35\b/);
    }
  });
});

d('M1-045b: footsteps, prone, LOD, recoil layer', () => {
  it('entity.footstep: ~10 alternating steps over 8 m of walking, none while prone, teleports ignored', async () => {
    const { bus, state, mgr: m } = await setup(1);
    const steps: { foot: string; stance: number }[] = [];
    bus.on('entity.footstep', (e) => steps.push({ foot: e.foot, stance: e.stance }));
    m.update(1, 16); // first frame only records the position
    for (let i = 1; i <= 80; i++) {
      state.entities.pos[0] = i * 0.1;
      m.update(1, 16);
    }
    expect(steps.length).toBeGreaterThanOrEqual(9);
    expect(steps.length).toBeLessThanOrEqual(11);
    steps.forEach((s, i) => expect(s.foot).toBe(i % 2 === 0 ? 'left' : 'right'));
    steps.length = 0;
    state.entities.stance[0] = 2;
    for (let i = 1; i <= 40; i++) {
      state.entities.pos[0] = 8 + i * 0.1;
      m.update(1, 16);
    }
    expect(steps.length).toBe(0); // prone: no footsteps
    state.entities.stance[0] = 0;
    state.entities.pos[0] = 500; // teleport
    m.update(1, 16);
    expect(steps.length).toBe(0);
  });

  it('prone lowers the body below crouch and the GD 9.2 transition time applies (stand->prone is not instant)', async () => {
    const { state, mgr: m, data } = await setup(1);
    m.update(1, 16);
    const a = m.actor(10)!;
    expect(a.height).toBeCloseTo(data.capsule.heightStand, 6);
    state.entities.stance[0] = 2;
    m.update(1, 100); // 100 ms into a 0.75 s transition: still tall
    expect(a.height).toBeGreaterThan(data.capsule.heightCrouch);
    for (let i = 0; i < 200; i++) m.update(1, 16); // 3.2 s: settled
    expect(a.height).toBeLessThan(data.capsule.heightCrouch);
    expect(a.height).toBeGreaterThan(0.2);
    state.entities.stance[0] = 0;
    for (let i = 0; i < 200; i++) m.update(1, 16);
    expect(a.height).toBeCloseTo(data.capsule.heightStand, 6);
  });

  it('LOD by camera distance with hysteresis: markers go beyond 40 m, head and weapon beyond 120 m', async () => {
    const { state, mgr: m, ts: scene } = await setup(1);
    const cam = scene.scene.activeCamera!;
    const place = (dist: number) => {
      state.entities.pos[0] = cam.globalPosition.x + dist;
      state.entities.pos[1] = cam.globalPosition.y;
      state.entities.pos[2] = cam.globalPosition.z;
      m.update(1, 16);
    };
    place(10);
    const a = m.actor(10)!;
    expect(a.lod).toBe(0);
    place(60);
    expect(a.lod).toBe(1);
    place(42); // inside the 5 m hysteresis band of the 40 m threshold: stays
    expect(a.lod).toBe(1);
    place(200);
    expect(a.lod).toBe(2);
    place(20);
    expect(a.lod).toBe(0);
  });

  it('applies locomotion to the lower pivots and reload to the attached weapon without changing the feed', async () => {
    const { state, mgr: m } = await setup(2);
    m.update(1, 16);
    const a = m.actor(10)!;
    const other = m.actor(11)!;
    const z = freshWorldMatrix(a.weapon!.muzzle).m[14];
    state.entities.anim[0] = MOTION.walk | RELOAD_BIT;
    state.entities.pos[0] = 0.3;
    m.update(1, 100);
    expect(a.animation.state).toBe('walk');
    expect(a.animation.upper).toBe('reload');
    expect(a.skeleton.joints.RightUpperLeg.rotation.x).toBeCloseTo(-a.skeleton.joints.LeftUpperLeg.rotation.x);
    expect(Math.abs(a.skeleton.joints.RightUpperLeg.rotation.x)).toBeGreaterThan(0.01);
    expect(freshWorldMatrix(a.weapon!.muzzle).m[14]).not.toBeCloseTo(z!);
    expect(other.animation.state).toBe('idle');
    expect(other.animation.upper).toBe('aim');
    expect(state.entities.anim[0]).toBe(MOTION.walk | RELOAD_BIT);
    expect(state.entities.pos[0]).toBeCloseTo(0.3);
  });

  it('fire.remote kicks the shooter aim pivot up and it decays', async () => {
    const { bus, state, mgr: m } = await setup(1);
    m.update(1, 16);
    const a = m.actor(10)!;
    state.entities.pitch[0] = 0;
    bus.emit('fire.remote', { shooter: 10, weapon: 0, muzzlePos: { x: 0, y: 1, z: 0 }, dir: { x: 0, y: 0, z: 1 }, distance: 5 });
    m.update(1, 16);
    expect(a.kick).toBeGreaterThan(0.05);
    expect(a.aim.rotation.x).toBeLessThan(-0.04);
    for (let i = 0; i < 60; i++) m.update(1, 16);
    expect(a.kick).toBe(0);
  });
});
