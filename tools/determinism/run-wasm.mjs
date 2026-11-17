// wasm half of the M0-029 check. Needs the native outputs (crates/sim-wasm/examples/determinism.rs)
// and a built pkg (`wasm-pack build crates/sim-wasm --target web --profile wasm-release`).
// Usage: node tools/determinism/run-wasm.mjs [DIR=target/determinism]
// For each seed: two independent Predictor instances (the second after unrelated allocations and an
// unrelated session), per-tick stateHash compared; positions + hashes written next to the native ones.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '../..');
const pkg = join(root, 'crates/sim-wasm/pkg');
const dir = resolve(process.argv[2] ?? join(root, 'target/determinism'));
const wasm = await import(join(pkg, 'frontline_sim_wasm.js'));
wasm.initSync({ module: readFileSync(join(pkg, 'frontline_sim_wasm_bg.wasm')) });
wasm.init();

const meta = JSON.parse(readFileSync(join(dir, 'native_meta.json'), 'utf8'));
if (BigInt(meta.data_hash) !== wasm.dataHash()) throw new Error('DATA_HASH differs between native and wasm builds');
const chunks = [];
for (let bz = 0; bz < 4; bz++) for (let bx = 0; bx < 4; bx++) chunks.push([bx, bz, readFileSync(join(root, `data/maps/m0/chunks/c_${bx}_${bz}.bin`))]);

function readInputs(seed) {
  const b = readFileSync(join(dir, `inputs_seed${seed}.bin`));
  const n = b.readUInt32LE(0);
  if (b.length !== 4 + 18 * n) throw new Error(`inputs_seed${seed}.bin: bad length`);
  return { n, bytes: b };
}

function newPredictor(seed, spawn) {
  const p = new wasm.Predictor(BigInt(seed), 0, meta.player);
  for (const [bx, bz, c] of chunks) p.loadCollision(bx, bz, c);
  const st = new DataView(new ArrayBuffer(80));
  [...spawn, 0, 0, 0, 0, 0, 0].forEach((v, i) => st.setFloat32(i * 4, v, true));
  p.reset(new Uint8Array(st.buffer), 0);
  p.equipWeapon('us_m1_garand', meta.reserve);
  return p;
}

function run(seed, spawn, inp) {
  const p = newPredictor(seed, spawn);
  const hashes = new BigUint64Array(inp.n);
  const pos = new Float32Array(3 * inp.n);
  const t0 = performance.now();
  for (let t = 0; t < inp.n; t++) {
    const s = p.step(inp.bytes.subarray(4 + 18 * t, 4 + 18 * (t + 1)));
    pos[3 * t] = s[0];
    pos[3 * t + 1] = s[1];
    pos[3 * t + 2] = s[2];
    hashes[t] = p.stateHash();
  }
  const elapsed = (performance.now() - t0) / 1000;
  p.free();
  return { hashes, pos, elapsed };
}

function firstDiff(a, b) {
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return i + 1;
  return null;
}

const timing = {};
for (const seed of [1, 2, 3]) {
  const nat = JSON.parse(readFileSync(join(dir, `native_seed${seed}.json`), 'utf8'));
  const inp = readInputs(seed);
  const r1 = run(seed, nat.spawn, inp);
  // Different allocation history for the second instance: junk buffers, an unrelated session.
  const junk = Array.from({ length: 200 }, (_, i) => new Uint8Array(1000 + i * 37).fill(i));
  const other = newPredictor(seed + 100, nat.spawn);
  for (let t = 0; t < 50; t++) other.step(inp.bytes.subarray(4 + 18 * t, 4 + 18 * (t + 1)));
  const r2 = run(seed, nat.spawn, inp);
  other.free();
  junk.length = 0;
  const d = firstDiff(r1.hashes, r2.hashes);
  const equal = r1.hashes.reduce((c, h, i) => c + (h === r2.hashes[i] ? 1 : 0), 0);
  writeFileSync(join(dir, `wasm_pos_seed${seed}.f32`), Buffer.from(r1.pos.buffer));
  writeFileSync(join(dir, `wasm_hash_seed${seed}.bin`), Buffer.from(r1.hashes.buffer));
  writeFileSync(join(dir, `wasm_seed${seed}.json`), JSON.stringify({ seed, ticks: inp.n, wasm_x2: { equal: d === null, equal_ticks: equal, first_diff_tick: d }, final_hash: `0x${r1.hashes[inp.n - 1].toString(16).padStart(16, '0')}` }) + '\n');
  timing[`seed${seed}`] = { native_s: nat.elapsed_s, wasm_s: [r1.elapsed, r2.elapsed] };
  console.log(`seed ${seed}: wasm x2 ${equal}/${inp.n} ticks equal (${r1.elapsed.toFixed(2)}s / ${r2.elapsed.toFixed(2)}s)`);
}
writeFileSync(join(dir, 'timing.json'), JSON.stringify(timing, null, 2) + '\n');
