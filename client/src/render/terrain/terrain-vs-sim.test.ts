/**
 * M0-029 (handed over from M0-023/PI-014): the client's CPU terrain height (`TerrainData.heightAt`,
 * mirrors the vertex shader) must agree with the simulation's `height_at` (what the server and the
 * predictor collide with). The sim side is the real wasm module (`Predictor.heightAt` over the 16
 * collision chunks), the client side the real `.r16` height maps. Inside the 1 m near block both are
 * bilinear over the same samples, so they may differ only by f32 / quantisation rounding.
 *
 * Needs the git-ignored build products (skipped with the command to run when missing; set
 * FRONTLINE_REQUIRE_TERRAIN=1 to turn the skip into a failure, as CI does):
 *   PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0
 *   wasm-pack build crates/sim-wasm --target web --profile wasm-release
 */
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { TerrainData, type MapMeta } from './terrain-data';

const ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
const MAP_DIR = `${ROOT}data/maps/m0/`;
const PKG_DIR = `${ROOT}crates/sim-wasm/pkg/`;
const haveMap = existsSync(`${MAP_DIR}heightmap_1m.r16`) && existsSync(`${MAP_DIR}meta.json`) && existsSync(`${MAP_DIR}chunks/c_3_3.bin`);
const haveWasm = existsSync(`${PKG_DIR}frontline_sim_wasm_bg.wasm`);
const required = process.env.FRONTLINE_REQUIRE_TERRAIN === '1';
const HOW = 'PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0 ; wasm-pack build crates/sim-wasm --target web --profile wasm-release';

interface SimWasm {
  initSync(opts: { module: Uint8Array }): unknown;
  init(): void;
  Predictor: new (seed: bigint, map: number, player: number) => {
    loadCollision(bx: number, bz: number, bytes: Uint8Array): void;
    heightAt(x: number, z: number): number;
    free(): void;
  };
}

/** mulberry32: small seeded PRNG so the sample set is identical on every run. */
function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

if (!haveMap || !haveWasm) {
  it.skipIf(!required)('terrain-vs-sim inputs are present', () => {
    throw new Error(`missing terrain data or sim-wasm pkg; run: ${HOW}`);
  });
}

describe.skipIf(!haveMap || !haveWasm)(`client heightAt vs sim height_at (${HOW})`, () => {
  it('agree within 1e-4 m on a 0.37 m grid + 20000 random points + chunk borders', async () => {
    const wasm = (await import(/* @vite-ignore */ new URL('../../../../crates/sim-wasm/pkg/frontline_sim_wasm.js', import.meta.url).href)) as SimWasm;
    wasm.initSync({ module: new Uint8Array(readFileSync(`${PKG_DIR}frontline_sim_wasm_bg.wasm`)) });
    wasm.init();
    const sim = new wasm.Predictor(1n, 0, 0);
    for (let bz = 0; bz < 4; bz++) for (let bx = 0; bx < 4; bx++) sim.loadCollision(bx, bz, new Uint8Array(readFileSync(`${MAP_DIR}chunks/c_${bx}_${bz}.bin`)));
    const meta = JSON.parse(readFileSync(`${MAP_DIR}meta.json`, 'utf8')) as MapMeta;
    const terrain = new TerrainData(
      meta,
      new Uint8Array(readFileSync(`${MAP_DIR}heightmap_1m.r16`)),
      new Uint8Array(readFileSync(`${MAP_DIR}heightmap_far_8m.r16`)),
    );

    const size = meta.crs.size_m; // 256: the sim covers [0, size) in x and z
    const pts: Array<[number, number]> = [];
    for (let x = 0; x < size; x += 0.37) for (let z = 0; z < size; z += 0.37) pts.push([x, z]);
    const r = rng(0x29);
    for (let i = 0; i < 20000; i++) pts.push([r() * (size - 1e-3), r() * (size - 1e-3)]);
    for (const b of [0, 64, 128, 192]) {
      for (let u = 0; u < size; u += 0.5) pts.push([b, u], [u, b], [b - 1e-4, u], [u, b - 1e-4]);
    }
    let worst = 0;
    let at: [number, number] = [0, 0];
    let n = 0;
    for (const [x, z] of pts) {
      if (x < 0 || z < 0 || x >= size || z >= size) continue;
      const s = sim.heightAt(x, z);
      if (!Number.isFinite(s)) throw new Error(`sim has no height at ${x},${z}`);
      const e = Math.abs(terrain.heightAt(x, z) - s);
      if (e > worst) {
        worst = e;
        at = [x, z];
      }
      n++;
    }
    console.log(`client heightAt vs sim height_at: ${n} points, max |err| = ${worst.toExponential(2)} m at (${at[0].toFixed(3)}, ${at[1].toFixed(3)})`);
    expect(n).toBeGreaterThan(450_000);
    expect(worst).toBeLessThanOrEqual(1e-4);
    // Outside the loaded chunks the sim has no height (NaN); the client keeps its far-map surface.
    expect(Number.isNaN(sim.heightAt(-5, 100))).toBe(true);
    expect(Number.isNaN(sim.heightAt(100, 300))).toBe(true);
    sim.free();
  });
});
