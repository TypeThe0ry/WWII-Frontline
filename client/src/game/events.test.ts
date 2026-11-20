import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from './event-bus';
import { EV_DAMAGE_DIR, EV_EXPLOSION, EV_HIT_CONFIRM, EV_IMPACT, EV_MARKER_EXPIRED, EV_MARKER_UPSERT, EV_SHOT, EV_SUPPRESS, EVENT_POS_STEP_M, EventDecoder } from './events';
import { GameLoop, type ClockPort } from './game-loop';
import { ScriptInput, HAVE_WASM, loadWasm } from './test-harness';
import { LAYOUT_EVENT_RECORD_BYTES } from '../net/wasm';

const rec = (id: number, type: number, payload: number[]): number[] => {
  const r = new Array<number>(LAYOUT_EVENT_RECORD_BYTES).fill(0);
  r[0] = id & 255;
  r[1] = id >> 8;
  r[2] = type;
  payload.forEach((v, i) => (r[3 + i] = v));
  return r;
};
const u16 = (v: number): number[] => [v & 255, v >> 8];

describe('EventDecoder', () => {
  const rig = () => {
    const bus = new EventBus<GameEvents>();
    const got: string[] = [];
    const data: Record<string, unknown> = {};
    for (const k of ['fire.remote', 'impact', 'hit.confirm', 'damage.taken'] as const) {
      bus.on(k, (p) => {
        got.push(k);
        data[k] = JSON.parse(JSON.stringify(p));
      });
    }
    const d = new EventDecoder(bus, () => ({ x: 0, y: 0, z: 0 }));
    return { d, got, data };
  };

  it('stores, refreshes, expires and de-duplicates marker lifecycle events', () => {
    const { d } = rig();
    const apply = (event: number[], tick: number, nowMs: number) => d.decode(Uint8Array.from(event), tick, nowMs);
    const upsert = (seq: number, kind: number, ttl: number) => rec(seq, EV_MARKER_UPSERT,
      [...u16(0x1234), kind, ...u16(4095), ...u16(2), ttl, 9, 1, ...u16(65535)]);
    apply(upsert(0xfffd, 1, 6), 100, 1000);
    expect(d.markers.lifecycle.get(0x1234)).toEqual({ id: 0x1234, kind: 1, pos: [4095, 2], ttl: 6, owner: 9, expiresAtMs: 7000 });
    apply(upsert(0xfffd, 1, 6), 101, 1200);
    expect(d.markers.lifecycle.get(0x1234)?.expiresAtMs).toBe(7000);
    apply(upsert(0xffff, 7, 5), 102, 1400);
    expect(d.markers.lifecycle.get(0x1234)).toMatchObject({ target: 65535, expiresAtMs: 6400 });
    apply(rec(0, EV_MARKER_EXPIRED, u16(0x1234)), 104, 1500);
    apply(upsert(0xfffe, 1, 6), 103, 1600);
    expect(d.markers.lifecycle.size).toBe(0);
    apply(upsert(1, 0x88, 1), 106, 1800);
    expect(d.markers.lifecycle.get(0x1234)?.kind).toBe(0x88);
    d.markers.expire(2800);
    expect(d.markers.lifecycle.size).toBe(0);
  });

  it('keeps command orders until revoked and handles scope changes across delayed delivery', () => {
    const { d } = rig();
    const upsert = (seq: number, tick: number, nowMs: number) => d.decode(Uint8Array.from(
      rec(seq, EV_MARKER_UPSERT, [...u16(7), 0x8f, ...u16(3), ...u16(2), 255, 4]),
    ), tick, nowMs);
    const expire = (seq: number, tick: number, nowMs: number) => d.decode(Uint8Array.from(
      rec(seq, EV_MARKER_EXPIRED, u16(7)),
    ), tick, nowMs);
    upsert(1, 100, 1000);
    d.markers.expire(300_000);
    expect(d.markers.lifecycle.has(7)).toBe(true);
    expire(3, 10_000, 300_010); // Lost scope.
    d.markers.expire(310_000);
    upsert(2, 9_999, 310_000); // Delayed old scope packet must stay revoked.
    expect(d.markers.lifecycle.has(7)).toBe(false);
    upsert(5, 12_000, 400_000); // Scope regained using the same marker ID.
    expire(4, 11_999, 400_010); // Delayed revocation from the previous scope.
    expect(d.markers.lifecycle.has(7)).toBe(true);
    d.reset();
    expect(d.markers.lifecycle.size).toBe(0);
  });

  it('replaces the C3 projection, ignoring stale frames across tick wrap', () => {
    const { d } = rig();
    const markers = [{ kind: 8, pos: [0, 4095], ttl: 1 }] as const;
    d.markers.applyLegacy({ tick: 65535, markers }, 0);
    d.markers.applyLegacy({ tick: 0, markers: [] }, 100);
    d.markers.applyLegacy({ tick: 65535, markers }, 200);
    expect(d.markers.legacyProjection).toEqual([]);
    d.markers.applyLegacy({ tick: 1, markers }, 200);
    expect(d.markers.legacyProjection).toMatchObject(markers);
    d.markers.expire(1200);
    expect(d.markers.legacyProjection).toEqual([]);
    d.reset();
    d.markers.applyLegacy({ tick: 65535, markers }, 1300);
    expect(d.markers.legacyProjection).toMatchObject(markers);
  });

  it('decodes the four event types', () => {
    const { d, got, data } = rig();
    const shot = rec(1, EV_SHOT, [7, 2, ...u16(1600), ...u16(160), ...u16(3200), ...u16(256), ...u16(511)]);
    const impact = rec(2, EV_IMPACT, [...u16(100), ...u16(200), ...u16(300), ...u16(77), 5]);
    const hit = rec(3, EV_HIT_CONFIRM, [9, 1, 255]);
    const dmg = rec(4, EV_DAMAGE_DIR, [64, 128]);
    d.decode(Uint8Array.from([...shot, ...impact, ...hit, ...dmg]));
    expect(got).toEqual(['fire.remote', 'impact', 'hit.confirm', 'damage.taken']);
    const s = data['fire.remote'] as { shooter: number; weapon: number; muzzlePos: { x: number; y: number; z: number }; dir: { x: number; y: number; z: number }; distance: number };
    expect([s.shooter, s.weapon]).toEqual([7, 2]);
    expect(s.muzzlePos.x).toBeCloseTo(1600 * EVENT_POS_STEP_M, 6);
    expect(Math.hypot(s.dir.x, s.dir.y, s.dir.z)).toBeCloseTo(1, 5);
    expect(s.dir.y).toBeCloseTo(1, 3); // pitch code 511 = +90 deg
    expect(s.distance).toBeCloseTo(Math.hypot(s.muzzlePos.x, s.muzzlePos.y, s.muzzlePos.z), 4);
    const i = data['impact'] as { pos: { x: number }; material: number; normal: number };
    expect([i.pos.x, i.material, i.normal]).toEqual([100 * EVENT_POS_STEP_M, 5, 77]);
    expect(data['hit.confirm']).toEqual({ target: 9, zone: 1, damage: 1, killed: false });
    const dm = data['damage.taken'] as { angle: number; amount: number };
    expect(dm.angle).toBeCloseTo(Math.PI / 2, 6);
    expect(dm.amount).toBeCloseTo(128 / 255, 6);
  });

  it('decodes Explosion (type 5) and Suppress (type 6)', () => {
    const bus = new EventBus<GameEvents>();
    const got: Array<[string, unknown]> = [];
    for (const k of ['explosion', 'suppress'] as const) bus.on(k, (p) => got.push([k, JSON.parse(JSON.stringify(p))]));
    const d = new EventDecoder(bus, () => ({ x: 0, y: 0, z: 0 }));
    d.decode(Uint8Array.from([...rec(1, EV_EXPLOSION, [...u16(640), ...u16(64), ...u16(1280), 0, 40]), ...rec(2, EV_SUPPRESS, [128, 255])]));
    expect(got[0]).toEqual(['explosion', { pos: { x: 640 * EVENT_POS_STEP_M, y: 64 * EVENT_POS_STEP_M, z: 1280 * EVENT_POS_STEP_M }, radius: 5, own: false }]);
    const sp = got[1]?.[1] as { angle: number; intensity: number };
    expect(got[1]?.[0]).toBe('suppress');
    expect(sp.angle).toBeCloseTo(Math.PI, 6);
    expect(sp.intensity).toBe(1);
  });

  it('redundant copies (the server resends for up to 3 packets) are emitted once; unknown types are ignored', () => {
    const { d, got } = rig();
    const hit = rec(300, EV_HIT_CONFIRM, [1, 0, 10]);
    d.decode(Uint8Array.from(hit));
    d.decode(Uint8Array.from([...rec(299, EV_DAMAGE_DIR, [0, 1]), ...hit]));
    d.decode(Uint8Array.from(hit));
    expect(got).toEqual(['hit.confirm', 'damage.taken']);
    expect(d.duplicates).toBe(2);
    d.decode(Uint8Array.from(rec(301, 31, [])));
    expect(got).toHaveLength(2);
    // the de-dup window is bounded
    for (let i = 0; i < EventDecoder.WINDOW + 5; i++) d.decode(Uint8Array.from(rec(1000 + i, EV_HIT_CONFIRM, [0, 0, 0])));
    d.decode(Uint8Array.from(hit)); // 300 fell out of the window: emitted again
    expect(got.filter((g) => g === 'hit.confirm').length).toBe(1 + EventDecoder.WINDOW + 5 + 1);
  });

  it('a trailing partial record is ignored', () => {
    const { d, got } = rig();
    d.decode(Uint8Array.from(rec(1, EV_HIT_CONFIRM, [1, 1, 1]).slice(0, 10)));
    expect(got).toEqual([]);
  });
});

const GOLDEN = fileURLToPath(new URL('../../../crates/sim-wasm/tests/golden/', import.meta.url));

describe.skipIf(!HAVE_WASM)('GameLoop.onSnapshot with the real wasm SnapshotDecoder (golden packets from M0-014)', () => {
  it('spawns / removes entities, emits events, records the self block, survives garbage', async () => {
    const wasm = await loadWasm();
    const bus = new EventBus<GameEvents>();
    const log: string[] = [];
    for (const k of ['entity.spawned', 'entity.removed', 'fire.remote', 'snapshot.applied', 'hit.confirm', 'impact', 'damage.taken'] as const) bus.on(k, (p) => log.push(`${k}:${JSON.stringify(p)}`));
    let t = 0;
    const clock: ClockPort = { inputTargetTick: () => 90000, estimatedServerTick: () => 90000, now: () => 90000 + t / 33.3, applySlack: () => undefined, rtt: () => 50 };
    const g = new GameLoop({ wasm, decoder: new wasm.SnapshotDecoder(), bus, net: { clock, sendInputBundle: () => undefined }, input: new ScriptInput(), matchSeed: 1n, mapId: 0, playerId: 9, now: () => t, log: () => undefined });
    g.onSnapshot(new Uint8Array(readFileSync(`${GOLDEN}snapshot_0.bin`)));
    expect(log.filter((l) => l.startsWith('entity.spawned')).map((l) => (JSON.parse(l.slice(l.indexOf(':') + 1)) as { id: number }).id)).toEqual([0, 3, 6]);
    expect(log.some((l) => l.startsWith('snapshot.applied:{"tick":90000,"entityCount":3'))).toBe(true);
    expect(log.some((l) => l.startsWith('fire.remote'))).toBe(true);
    expect(g.state.net.ammoAuth.value).toBe(0); // ack 777 is not in our ring: the self block is not applied
    expect(g.reconciler.stats.unknownAcks).toBe(1);
    g.onSnapshot(new Uint8Array(readFileSync(`${GOLDEN}snapshot_1.bin`)));
    expect(log.some((l) => l.startsWith('entity.removed:{"id":6,"reason":"removed"}'))).toBe(true);
    expect(g.store.count).toBe(2);
    // render frame: 2 remote entities in the SoA view
    t = 100;
    g.update(0, 0.5, 16);
    expect(g.state.entities.count).toBe(2);
    // garbage: counted, no throw, state intact
    g.onSnapshot(Uint8Array.from([0xff, 0xff, 0xff, 0xff, 0xff]));
    g.onSnapshot(new Uint8Array(0));
    expect(g.stats.decodeErrors).toBe(2);
    expect(g.store.count).toBe(2);
    g.dispose();
  });
});
