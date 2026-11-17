// Predictor.step timing in Node (M0-014 acceptance 4): 10 000 timed steps incl. the cmd copy
// and the returned Float32Array view. Prints median / p95 / mean in microseconds.
// Usage: node crates/sim-wasm/tests/bench.mjs [steps=10000]
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const pkg = join(here, '../pkg');
const wasm = await import(join(pkg, 'frontline_sim_wasm.js'));
wasm.initSync({ module: readFileSync(join(pkg, 'frontline_sim_wasm_bg.wasm')) });
wasm.init();
const root = join(here, '../../..');
const N = Number(process.argv[2] ?? 10000);

const p = new wasm.Predictor(1n, 0, 3);
for (let bz = 0; bz < 4; bz++) for (let bx = 0; bx < 4; bx++) p.loadCollision(bx, bz, readFileSync(join(root, `data/maps/m0/chunks/c_${bx}_${bz}.bin`)));
const st = new DataView(new ArrayBuffer(80));
[128, 2.4, 56].forEach((v, i) => st.setFloat32(i * 4, v, true));
const spawn = new Uint8Array(st.buffer);
p.reset(spawn, 0);
p.equipWeapon('us_m1_garand', 60000);

// Walking + turning + firing so that KCC, stance and weapon code all run.
const cmds = Array.from({ length: 256 }, (_, i) =>
  wasm.packInputCmd(i, i + 1, (i % 9) * 10 - 40, 127, (i * 997) & 0xffff, 0, (i % 7 === 0 ? 1 : 0) | (i % 50 === 0 ? 8 : 0), 0, 0, 0),
);
for (let i = 0; i < 2000; i++) p.step(cmds[i & 255]); // warm-up
const t = new Float64Array(N);
for (let i = 0; i < N; i++) {
  if ((i & 1023) === 0) p.reset(spawn, 0);
  const c = cmds[i & 255];
  const a = process.hrtime.bigint();
  p.step(c);
  t[i] = Number(process.hrtime.bigint() - a) / 1000;
}
const s = Array.from(t).sort((x, y) => x - y);
const med = s[N >> 1];
const p95 = s[Math.floor(N * 0.95)];
const mean = s.reduce((a, b) => a + b, 0) / N;
console.log(JSON.stringify({ steps: N, median_us: +med.toFixed(2), p95_us: +p95.toFixed(2), mean_us: +mean.toFixed(2), budget_us: 50 }));
if (med > 50) {
  console.error(`FAIL median ${med.toFixed(1)} us > 50 us`);
  process.exit(1);
}
