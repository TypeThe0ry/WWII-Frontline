// Native vs. wasm bit-for-bit comparison of the prediction step (M0-014; full drift tooling is M0-029).
// 1. cargo run --release -p frontline-sim-wasm --example trace [-- OUT_DIR [TICKS]]   (native PredictSim)
// 2. node crates/sim-wasm/tests/native-vs-wasm.mjs [OUT_DIR]
// Exit 0 when every tick's PlayerAuthState (39 B) + WeaponState (33 B) bytes are identical,
// 1 with the first diverging tick and field otherwise.
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const pkg = join(here, '../pkg');
const dir = resolve(process.argv[2] ?? join(here, '../../../target/wasm-trace'));
const wasm = await import(join(pkg, 'frontline_sim_wasm.js'));
wasm.initSync({ module: readFileSync(join(pkg, 'frontline_sim_wasm_bg.wasm')) });
wasm.init();

const meta = JSON.parse(readFileSync(join(dir, 'meta.json'), 'utf8'));
const cmds = readFileSync(join(dir, 'cmds.bin'));
const trace = readFileSync(join(dir, 'trace.bin'));
const n = meta.ticks;
if (BigInt(meta.dataHash) !== wasm.dataHash()) throw new Error('data hash differs from native');

const root = join(here, '../../..');
const p = new wasm.Predictor(BigInt(meta.matchSeed), 0, meta.player);
for (let bz = 0; bz < 4; bz++) for (let bx = 0; bx < 4; bx++) p.loadCollision(bx, bz, readFileSync(join(root, `data/maps/m0/chunks/c_${bx}_${bz}.bin`)));
const st = new DataView(new ArrayBuffer(80));
[...meta.spawn, 0, 0, 0, 0, 0, 0].forEach((v, i) => st.setFloat32(i * 4, v, true));
p.reset(new Uint8Array(st.buffer), 0);
p.equipWeapon('us_m1_garand', meta.reserve);

const AUTH = ['pos.x', 'pos.y', 'pos.z', 'vel.x', 'vel.y', 'vel.z', 'yaw', 'pitch', 'stance_blend'];
const WEAP = [
  [0, 2, 'def_id'], [2, 2, 'mag'], [4, 2, 'reserve'], [6, 1, 'fsm'], [7, 1, 'flags'], [8, 4, 'timer_units'],
  [12, 4, 'since_shot_units'], [16, 4, 'recoil_pitch_deg'], [20, 4, 'recoil_yaw_deg'], [24, 4, 'bloom_deg'], [28, 4, 'ads_units'], [32, 1, 'reserved'],
];
const field = (off) => {
  if (off < 36) return AUTH[off >> 2];
  if (off === 36) return 'stance';
  if (off === 37) return 'stance_timer';
  if (off === 38) return 'flags(grounded|blocked|locked)';
  if (off === 39) return 'stance_from';
  if (off < 44) return 'stamina^100';
  if (off === 44) return 'stamina_delay';
  if (off < 49) return 'lean';
  if (off < 80) return 'vault';
  const w = off - 80;
  const f = WEAP.find(([o, l]) => w >= o && w < o + l);
  return `weapon.${f ? f[2] : '?'}`;
};

let first = null;
let diffTicks = 0;
for (let t = 0; t < n; t++) {
  p.step(cmds.subarray(t * 18, t * 18 + 18));
  const got = Buffer.concat([p.authState(), p.weaponState()]);
  const want = trace.subarray(t * 113, t * 113 + 113);
  if (!got.equals(want)) {
    diffTicks++;
    if (!first) {
      const off = got.findIndex((b, i) => b !== want[i]);
      first = { tick: t + 1, offset: off, field: field(off), wasm: got.subarray(off, off + 4).toString('hex'), native: want.subarray(off, off + 4).toString('hex') };
    }
  }
}
if (first) {
  console.log(`MISMATCH: ${diffTicks}/${n} ticks differ; first at tick ${first.tick}, byte ${first.offset} (${first.field}): wasm=${first.wasm} native=${first.native}`);
  process.exit(1);
}
console.log(`native == wasm: ${n} ticks, PlayerAuthState(80 B) + WeaponState(33 B) bit-identical on every tick`);
const f = p.state();
console.log(`final: pos=(${f[0].toFixed(3)}, ${f[1].toFixed(3)}, ${f[2].toFixed(3)}) ammo=${f[10]}/${f[11]} tick=${p.tick()}`);
