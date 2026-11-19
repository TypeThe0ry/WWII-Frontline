// Run after generating the web bindings; optionally pass the bindings directory as argv[2].
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const pkg = process.argv[2] ? pathToFileURL(`${resolve(process.argv[2])}/`) : new URL('../pkg/', import.meta.url);
const wasm = await import(new URL('frontline_sim_wasm.js', pkg));
wasm.initSync({ module: readFileSync(new URL('frontline_sim_wasm_bg.wasm', pkg)) });
wasm.init();

const predictor = new wasm.Predictor(1n, 0, 0);
try {
  // Synthetic flat FCOL v1 chunk so outside-zone garrison placement can also succeed.
  const terrain = new Uint8Array(16 + 65 * 65 * 2 + 64 * 64);
  terrain.set([70, 67, 79, 76, 1]);
  const header = new DataView(terrain.buffer);
  header.setUint16(6, 65, true);
  header.setFloat32(8, 5, true);
  header.setFloat32(12, 5, true);
  predictor.loadCollision(0, 0, terrain);

  const check = (facts, x) => wasm.placement_check(predictor, facts.kind, Float32Array.of(x, 5, 24), JSON.stringify(facts));
  const cases = [
    { kind: 'garrison', alive: true, officer: true, team: 'us', hq: [], garrisons: [], objectives: [], enemies: [], available_supplies: 50 },
    { kind: 'supply_crate', alive: true, support: true, carrying: true, team: 'us', objectives: [] },
    { kind: 'ammo_crate', alive: true, support: true, tick: 60, used: 0, last_placed_tick: null, crates: [] },
  ];
  const zones = [[[16, 16], [32, 16], [32, 32], [16, 32]]];
  predictor.setNoBuildZones(JSON.stringify(zones));
  assert.throws(() => predictor.setNoBuildZones('[[[0,0],[1,1],[2,2]]]'), /polygon at index 0/);
  for (const facts of cases) {
    for (const x of [24, 32]) {
      assert.deepEqual(check(facts, x), { legal: false, reason: 'InNoBuildZone' }, facts.kind);
    }
    assert.equal(check(facts, 48).legal, true, facts.kind);
  }

  predictor.setNoBuildZones('[]');
  for (const facts of cases) {
    assert.equal(check(facts, 24).legal, true, facts.kind);
  }
  console.log('ok no-build zones in garrison, supply crate, and ammo crate previews');
} finally {
  predictor.free();
}
