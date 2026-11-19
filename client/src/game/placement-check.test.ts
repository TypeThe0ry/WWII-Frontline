import { readdirSync } from 'node:fs';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { EventBus } from './event-bus';
import { GameLoop } from './game-loop';
import { PLACE_ERROR_KEY, PLACEMENT_IDLE, placementNext, type PlaceError, type PlacementKind } from './placement';
import type { PlacementFacts, PlacementPoint, PlacementWasm } from './placement-check';
import { FakeSnapshotDecoder, HAVE_WASM, loadWasm, ScriptInput } from './test-harness';
import en from '../ui/i18n/en.json';
import zh from '../ui/i18n/zh-Hans.json';

if (!HAVE_WASM && process.env.FRONTLINE_REQUIRE_TERRAIN === '1') {
  throw new Error('placement test requires wasm-pack build crates/sim-wasm --target web --profile wasm-release');
}

const point = [24, 5, 24] as const;
const offset = (dx: number): PlacementPoint => [point[0] + dx, point[1], point[2]];
const water = readdirSync(new URL('../../../data/materials/', import.meta.url).pathname).sort().indexOf('water.json');
const garrison: Extract<PlacementFacts, { kind: 'garrison' }> = {
  kind: 'garrison', alive: true, officer: true, team: 'us',
  hq: [], garrisons: [], objectives: [], enemies: [], available_supplies: 50,
};

/** Synthetic FCOL chunk, flat at y=5 unless the x-gradient is a 45-degree slope. */
function chunk(terrain: 'flat' | 'slope' | 'water'): Uint8Array {
  const samples = 65;
  const header = 16;
  const materialOffset = header + samples * samples * 2;
  const b = new Uint8Array(materialOffset + 64 * 64);
  b.set([70, 67, 79, 76, 1]); // FCOL v1
  const dv = new DataView(b.buffer);
  dv.setUint16(6, samples, true);
  dv.setFloat32(8, 5, true);
  dv.setFloat32(12, terrain === 'slope' ? 69 : 5, true);
  if (terrain === 'slope') {
    for (let z = 0; z < samples; z++) {
      for (let x = 0; x < samples; x++) dv.setUint16(header + (z * samples + x) * 2, Math.round(x * 65535 / 64), true);
    }
  }
  if (terrain === 'water') b.fill(water, materialOffset);
  return b;
}

const enter = (kind: PlacementKind) => placementNext(PLACEMENT_IDLE, { t: 'enter', kind });
const commit = (state: ReturnType<typeof enter>) => placementNext(state, { t: 'commit' });

describe.skipIf(!HAVE_WASM)('placement checks through the real sim-wasm and GameLoop', () => {
  let game: GameLoop;
  let wasm: PlacementWasm;
  beforeAll(async () => {
    const module = await loadWasm();
    wasm = module as typeof module & PlacementWasm;
    const zero = () => 0;
    const noop = () => undefined;
    game = new GameLoop({
      wasm: module, decoder: new FakeSnapshotDecoder(), bus: new EventBus(), input: new ScriptInput(),
      matchSeed: 1n, mapId: 0, playerId: 3, weapon: null,
      net: { clock: { now: zero, estimatedServerTick: zero, inputTargetTick: zero, rtt: zero, applySlack: noop }, sendInputBundle: noop },
      spawn: { x: point[0], y: point[1], z: point[2], yaw: 0 },
    });
  });
  afterAll(() => game?.predictor.wasm.free());

  const cases: readonly [PlaceError, Partial<typeof garrison>, ('flat' | 'slope' | 'water')?][] = [
    ['EnemyNear', { enemies: [offset(19)] }],
    ['Slope', {}, 'slope'],
    ['Water', {}, 'water'],
    ['TooCloseOwnGarrison', { garrisons: [['us', offset(149)]] }],
    ['NoSupplies', { available_supplies: 49 }],
    ['TeamCap', { garrisons: Array.from({ length: 6 }, (_, i) => ['us', offset(151 + i * 151)]) }],
    ['TooCloseObjective', { objectives: [{ id: 1, pos: offset(119), radius_m: 20, owner: null }] }],
    ['TooCloseHq', { hq: [offset(199)] }],
  ];
  it.each(cases)('%s reaches the placement state as a localized reason and blocks commit', (code, patch, terrain = 'flat') => {
    game.loadCollision([{ bx: 0, bz: 0, data: chunk(terrain) }]);
    const checked = game.checkPlacement(enter('garrison'), point, { ...garrison, ...patch });
    const reasonKey = PLACE_ERROR_KEY[code];
    expect(checked).toMatchObject({ mode: 'invalid', reasonKey, result: { legal: false, reasonKey } });
    expect(reasonKey in en && reasonKey in zh).toBe(true);
    expect(commit(checked)).toBe(checked);
  });

  it('reports sim cost and remaining slots, without changing prediction state', () => {
    game.loadCollision([{ bx: 0, bz: 0, data: chunk('flat') }]);
    const preview = enter('garrison');
    expect(commit(preview)).toBe(preview);
    const hash = game.predictor.wasm.stateHash();
    expect(game.checkPlacement(preview, point, garrison).result).toEqual({ legal: true, supplyCost: 50, remaining: 6 });
    const checked = game.checkPlacement(preview, point, {
      ...garrison, objectives: [{ id: 1, pos: offset(160), radius_m: 20, owner: 'ger' }], available_supplies: 100,
    });
    expect(checked.result).toEqual({ legal: true, supplyCost: 100, remaining: 6 });
    expect(commit(checked).mode).toBe('committing');
    expect(game.predictor.wasm.stateHash()).toBe(hash);
  });

  it('clears previously legal previews when point or matching world facts are missing', () => {
    game.loadCollision([{ bx: 0, bz: 0, data: chunk('flat') }]);
    const checked = game.checkPlacement(enter('garrison'), point, garrison);
    for (const s of [game.checkPlacement(checked, null, garrison), game.checkPlacement(checked, point, null)]) {
      expect(s).toMatchObject({ mode: 'invalid', result: { legal: false, reasonKey: 'ui.officer.err.previewUnavailable' } });
      expect(commit(s)).toBe(s);
    }
  });

  it('checks Supply Crates at predicted feet and applies the sim enemy-objective exclusion', () => {
    const facts = { kind: 'supply_crate', alive: true, support: true, carrying: true, team: 'us', objectives: [] } as const;
    const preview = enter('supply_crate');
    expect(game.checkPlacement(preview, null, facts).result).toEqual({ legal: true });
    const blocked = game.checkPlacement(preview, offset(1000), {
      ...facts, objectives: [{ id: 1, pos: offset(79), radius_m: 20, owner: 'ger' }],
    });
    expect(blocked.reasonKey).toBe(PLACE_ERROR_KEY.TooCloseObjective);
    expect(game.checkPlacement(preview, null, { ...facts, carrying: false }).reasonKey).toBe('ui.officer.err.notCarrying');
  });

  it('checks Ammo Crate cooldown, stack cap and per-life limit at predicted feet', () => {
    const facts = { kind: 'ammo_crate', alive: true, support: true, tick: 1000, used: 1, last_placed_tick: 100, crates: [] } as const;
    const preview = enter('ammo_crate');
    expect(game.checkPlacement(preview, null, facts).result).toEqual({ legal: true, remaining: 1 });
    expect(game.checkPlacement(preview, null, { ...facts, tick: 999 }).reasonKey).toBe('ui.officer.err.ammoCooldown');
    expect(game.checkPlacement(preview, offset(1000), { ...facts, crates: [point, point, point] }).reasonKey).toBe('ui.officer.err.ammoStacked');
    expect(game.checkPlacement(preview, null, { ...facts, used: 2 }).reasonKey).toBe('ui.officer.err.ammoLimit');
  });

  it('rejects malformed WASM arguments', () => {
    const p = game.predictor.wasm;
    expect(() => wasm.placement_check(p, 'garrison', Float32Array.of(24, NaN, 24), JSON.stringify(garrison))).toThrow(/finite/);
    expect(() => wasm.placement_check(p, 'garrison', Float32Array.from(point), '{')).toThrow();
    expect(() => wasm.placement_check(p, 'ammo_crate', Float32Array.from(point), JSON.stringify(garrison))).toThrow(/kind/);
  });
});
