// Compare the full sim World across native and WebAssembly, not just the web Predictor.
// rustup target add wasm32-wasip1
// node tools/determinism/run-c1-wasi.mjs (from repo root)
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { WASI } from 'node:wasi';
import assert from 'node:assert/strict';

const wasmPath = 'target/wasm32-wasip1/release/examples/c1_trace.wasm';
if (process.argv[2] === '--guest') {
  const wasi = new WASI({ version: 'preview1', args: [], env: {} });
  const mod = await WebAssembly.compile(readFileSync(wasmPath));
  wasi.start(await WebAssembly.instantiate(mod, wasi.getImportObject()));
} else {
  function run(bin, args) {
    const r = spawnSync(bin, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] });
    if (r.error) throw r.error;
    assert.equal(r.status, 0, `${bin} ${args.join(' ')}`);
    return r.stdout.trim().split('\n');
  }
  const opts = ['--locked', '--release', '-p', 'frontline-sim', '--example', 'c1_trace'];
  const native = run('cargo', ['run', ...opts]);
  run('cargo', ['build', ...opts, '--target', 'wasm32-wasip1']);
  const wasm = run(process.execPath, [process.argv[1], '--guest']);
  assert.equal(native.length, 481);
  assert.equal(wasm.length, native.length);
  for (let i = 0; i < native.length; i++) {
    assert.equal(wasm[i], native[i], `native vs WASI mismatch at line ${i + 1}`);
  }
  console.log(`C1 native vs WASI: ${native.length - 1} ticks identical; ${native.at(-1)}`);
}
