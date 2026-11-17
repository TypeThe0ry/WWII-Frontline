import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../../game/event-bus';
import { GameState } from '../../game/state';
import { ActorManager } from './actor-manager';
import { weaponLayout } from './placeholder-weapon';
import { PH_NAME_RE, PlaceholderRegistry, REQ } from './names';
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
  it('weapons have socket_muzzle / socket_ejection / socket_magazine, characters socket_hand_r/_l and socket_head', async () => {
    const { mgr: m, state } = await setup(2);
    m.update(1, 16);
    expect(state.entities.count).toBe(2);
    const wnames = (root: { getChildTransformNodes(d: boolean): { name: string }[] }) => root.getChildTransformNodes(true).map((n) => n.name);
    for (const s of m.allActors()) {
      const a = s.actor;
      const sockets = a.root.getChildTransformNodes(false).map((n) => n.name);
      for (const n of ['socket_hand_r', 'socket_hand_l', 'socket_head']) expect(sockets, `${a.id}`).toContain(n);
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
  it('body height follows data and eases between stand and crouch over stance_transition_ms', async () => {
    const { mgr: m, state, data } = await setup(1);
    const c = data.capsule;
    m.update(1, 16);
    const a = m.actor(10)!;
    expect(a.height).toBeCloseTo(c.heightStand, 6);
    state.entities.stance[0] = 1;
    m.update(1, data.stanceTransitionMs / 2);
    expect(a.height).toBeLessThan(c.heightStand);
    expect(a.height).toBeGreaterThan(c.heightCrouch);
    m.update(1, data.stanceTransitionMs);
    expect(a.height).toBeCloseTo(c.heightCrouch, 6);
    state.entities.stance[0] = 0;
    m.update(1, data.stanceTransitionMs * 2);
    expect(a.height).toBeCloseTo(c.heightStand, 6);
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
    bus.emit('entity.removed', { id: 10, reason: 'removed' });
    expect(m.actorCount).toBe(0);
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
