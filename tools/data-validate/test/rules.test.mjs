import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkUnits } from '../rules/units.mjs';
import { checkTunable, resolvePath } from '../rules/tunable.mjs';
import { checkRanges } from '../rules/ranges.mjs';
import { checkRefs } from '../rules/refs.mjs';

const msgs = (errs) => errs.map((e) => e.message);

test('units: accepts suffixed, multiplier and dimensionless fields', () => {
  const doc = {
    muzzle_velocity_mps: 800, mass_kg: 4, rof_rpm: 60, part_mult: { head: 2 }, armor_mm: { front: 50 },
    falloff: [{ dist_m: 0, mult: 1 }], mag_reserve: 3, gun_elevation_deg: [-10, 25], _meta: { x: 1 },
  };
  assert.deepEqual(checkUnits(doc), []);
});
test('units: rejects missing suffix', () => {
  assert.match(msgs(checkUnits({ damage_torso: 70 }))[0], /missing unit suffix on numeric field 'damage_torso'/);
});
test('units: rejects wrong unit suffix', () => {
  assert.match(msgs(checkUnits({ muzzle_velocity_kph: 900 }))[0], /unit suffix '_kph' not allowed/);
  assert.match(msgs(checkUnits({ range_ft: 10 }))[0], /unit suffix '_ft' not allowed/);
});

test('tunable: valid meta passes', () => {
  const doc = { a: { b_ms: 1 }, _meta: { status: 'initial', tunable: ['a.b_ms'] }, historical: { _source: 'x' } };
  assert.deepEqual(checkTunable(doc, 'weapon'), []);
});
test('tunable: dangling path, bad/missing status, missing meta, missing source', () => {
  assert.deepEqual(msgs(checkTunable({ _meta: { status: 'initial', tunable: ['nope'] } }, 'weapon')), ['tunable path not found: nope']);
  assert.match(msgs(checkTunable({ _meta: { tunable: [] } }, 'weapon'))[0], /missing: _meta.status/);
  assert.match(msgs(checkTunable({ _meta: { status: 'final', tunable: [] } }, 'weapon'))[0], /invalid _meta.status/);
  assert.deepEqual(msgs(checkTunable({}, 'weapon')), ['missing: _meta']);
  assert.deepEqual(checkTunable({}, 'spawns'), []); // map files may omit _meta
  assert.deepEqual(msgs(checkTunable({ _meta: { status: 'initial', tunable: [] }, historical: {} }, 'weapon')), ['missing: historical._source']);
});
test('tunable: resolvePath handles arrays', () => {
  assert.equal(resolvePath({ a: [{ b: 1 }] }, 'a.0.b').found, true);
  assert.equal(resolvePath({ a: [] }, 'a.0').found, false);
});

const weapon = (g) => ({ historical: { muzzle_velocity_mps: 800 }, gameplay: g });
test('ranges: cap_rpm', () => {
  assert.deepEqual(checkRanges(weapon({ fire: { mode: 'auto', cap_rpm: 600 } }), 'weapon'), []);
  assert.match(msgs(checkRanges(weapon({ fire: { mode: 'auto', cap_rpm: 2001 } }), 'weapon'))[0], /cap_rpm out of range/);
  assert.match(msgs(checkRanges(weapon({ fire: { mode: 'auto', cap_rpm: 0 } }), 'weapon'))[0], /cap_rpm out of range/);
});
test('ranges: muzzle velocity', () => {
  assert.deepEqual(checkRanges(weapon({ ballistics: { muzzle_velocity_mps: 1500 } }), 'weapon'), []);
  assert.match(msgs(checkRanges(weapon({ ballistics: { muzzle_velocity_mps: 1501 } }), 'weapon'))[0], /muzzle_velocity_mps out of range/);
  assert.match(msgs(checkRanges({ historical: { muzzle_velocity_mps: 49 } }, 'weapon'))[0], /out of range/);
});
test('ranges: falloff ordering and multiplier', () => {
  const ok = [{ dist_m: 0, mult: 1 }, { dist_m: 100, mult: 0.5 }];
  assert.deepEqual(checkRanges(weapon({ falloff: ok }), 'weapon'), []);
  assert.match(msgs(checkRanges(weapon({ falloff: [{ dist_m: 5, mult: 1 }, { dist_m: 5, mult: 1 }] }), 'weapon'))[0], /strictly increasing/);
  assert.match(msgs(checkRanges(weapon({ falloff: [{ dist_m: 0, mult: 0 }] }), 'weapon'))[0], /in \(0,1\]/);
  assert.match(msgs(checkRanges(weapon({ falloff: [{ dist_m: 0, mult: 1.01 }] }), 'weapon'))[0], /in \(0,1\]/);
});
test('ranges: fire mode vs fields', () => {
  assert.deepEqual(checkRanges(weapon({ fire: { mode: 'bolt', bolt_cycle_ms: 1500 } }), 'weapon'), []);
  assert.match(msgs(checkRanges(weapon({ fire: { mode: 'bolt' } }), 'weapon'))[0], /requires bolt_cycle_ms/);
  assert.match(msgs(checkRanges(weapon({ fire: { mode: 'semi' } }), 'weapon'))[0], /requires cap_rpm/);
});
test('ranges: role movement ordering', () => {
  const m = { prone_mps: 0.6, crouch_mps: 1.2, walk_mps: 1.8, run_mps: 4.2, sprint_mps: 6 };
  assert.deepEqual(checkRanges({ movement: m }, 'role'), []);
  assert.match(msgs(checkRanges({ movement: { ...m, walk_mps: 5 } }, 'role'))[0], /movement speeds must satisfy/);
});

const f = (kind, doc) => ({ kind, file: `${kind}/${doc.id}.json`, doc });
const base = () => [
  f('weapon', { id: 'us_w' }),
  f('role', { id: 'r', faction_overrides: { us: { loadout_ids: ['us_r_1'] } } }),
  f('loadout', { id: 'us_r_1', role: 'r', faction: 'us', primary: 'us_w', secondary: null, gadget_1: { id: 'us_w', count: 1 } }),
];
test('refs: consistent set passes', () => assert.deepEqual(checkRefs(base()), []));
test('refs: unknown weapon / role / loadout / duplicate / mismatch', () => {
  let s = base(); s[2].doc.primary = 'us_x';
  assert.deepEqual(msgs(checkRefs(s)), ['unknown weapon id: us_x']);
  s = base(); s[2].doc.role = 'zzz';
  assert.ok(msgs(checkRefs(s)).includes('unknown role id: zzz'));
  s = base(); s[1].doc.faction_overrides.us.loadout_ids = ['nope'];
  assert.deepEqual(msgs(checkRefs(s)), ['unknown loadout id: nope']);
  s = [...base(), f('weapon', { id: 'us_w' })];
  assert.deepEqual(msgs(checkRefs(s)), ['duplicate weapon id: us_w']);
  s = base(); s[2].doc.faction = 'ger';
  assert.match(msgs(checkRefs(s))[0], /is declared for role/);
});

// ---- hist-source-required (M0-031) ----
import { checkHistorical } from '../rules/historical.mjs';
const rec = (field, locator = 'par. 4') => ({ field, value: 1, unit: 'x', source_id: 'T', title: 't', locator, accessed: '2026-11-17' });
const hdoc = (status, hist, unverified) => ({ historical: hist, _meta: { status, tunable: [], unverified } });
test('hist-source-required: initial is not enforced', () => {
  assert.deepEqual(checkHistorical(hdoc('initial', { mass_kg: 4, _source: 'x' }), 'weapon'), []);
});
test('hist-source-required: reviewed with records and unverified passes', () => {
  const d = hdoc('reviewed', { mass_kg: 4, muzzle_velocity_mps: 800, _source: [rec('mass_kg')] }, ['historical.muzzle_velocity_mps']);
  assert.deepEqual(checkHistorical(d, 'weapon'), []);
});
test('hist-source-required: reviewed field without source is rejected', () => {
  const d = hdoc('reviewed', { mass_kg: 4, muzzle_velocity_mps: 800, _source: [rec('mass_kg')] }, []);
  assert.match(msgs(checkHistorical(d, 'weapon'))[0], /historical\.muzzle_velocity_mps has no _source record/);
});
test('hist-source-required: record without locator does not count; text _source rejected', () => {
  const d = hdoc('measured', { mass_kg: 4, _source: [rec('mass_kg', '')] }, []);
  const m = msgs(checkHistorical(d, 'weapon'));
  assert.ok(m.some((x) => /no locator/.test(x)) && m.some((x) => /has no _source record/.test(x)));
  assert.match(msgs(checkHistorical(hdoc('reviewed', { mass_kg: 4, _source: 'text' }, ['historical.mass_kg']), 'weapon'))[0], /structured _source array/);
});
test('hist-source-required: bogus unverified entry rejected; 未核实 value accepted', () => {
  assert.match(msgs(checkHistorical(hdoc('reviewed', { mass_kg: 4, _source: [rec('mass_kg')] }, ['historical.nope']), 'weapon'))[0], /does not name a historical field/);
  assert.deepEqual(checkHistorical(hdoc('reviewed', { mass_kg: '未核实', _source: [rec('x')] }, []), 'weapon'), []);
});
