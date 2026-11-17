// Node smoke test for the sim-wasm package (M0-014 acceptance 2, 5, 6).
// Usage: node crates/sim-wasm/tests/node-smoke.mjs   (after `wasm-pack build ... --target web`)
// Terrain: needs data/maps/m0/chunks/c_*.bin (regenerate per DEVELOPING.md); set
// FRONTLINE_REQUIRE_TERRAIN=1 to fail instead of skipping the movement part when it is missing.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '../../..');
const pkgDir = join(here, '../pkg');
if (!existsSync(join(pkgDir, 'frontline_sim_wasm.js'))) {
  console.error('pkg/ missing: run `wasm-pack build crates/sim-wasm --target web --profile wasm-release` first');
  process.exit(2);
}
const wasm = await import(join(pkgDir, 'frontline_sim_wasm.js'));
const wasmBytes = readFileSync(join(pkgDir, 'frontline_sim_wasm_bg.wasm'));
const exports = wasm.initSync({ module: wasmBytes });
wasm.init();

const golden = (n) => join(here, 'golden', n);
const hex = (n) => Buffer.from(readFileSync(join(root, 'crates/protocol/tests/golden', n), 'utf8').trim(), 'hex');
let passed = 0;
const ok = (name) => {
  passed++;
  console.log(`ok  ${name}`);
};

// --- version / data hash agree with the native build (meta.json comes from the native example).
{
  assert.equal(wasm.protocolVersion(), 3);
  // The data hash changes with every /data edit, so compare against the native build's own value
  // (`cargo run --example trace` writes it) and fall back to the committed file only without a trace.
  const traceMeta = join(root, 'target/wasm-trace/meta.json');
  const native = JSON.parse(readFileSync(existsSync(traceMeta) ? traceMeta : golden('native.json'), 'utf8'));
  assert.equal(wasm.protocolVersion(), native.protocolVersion);
  assert.equal(wasm.dataHash(), BigInt(native.dataHash));
  ok(`protocolVersion=${wasm.protocolVersion()} dataHash=0x${wasm.dataHash().toString(16)} equal native`);
}

// --- data queries.
{
  const w = JSON.parse(Buffer.from(wasm.weaponDef('us_m1_garand')).toString());
  assert.equal(w.id, 'us_m1_garand');
  assert.ok(w.gameplay && !('_meta' in w));
  const r = JSON.parse(Buffer.from(wasm.roleDef('rifleman')).toString());
  assert.ok(r.capsule.radius_m > 0 && r.capsule.height_stand_m > r.capsule.height_crouch_m);
  assert.throws(() => wasm.weaponDef('nope'), /unknown/);
  assert.ok(wasm.weaponIndex('us_m1_garand') >= 0);
  ok('weaponDef / roleDef / weaponIndex');
}

// --- layout constants equal the TS table (client/src/net/wasm/layout.ts is generated from this JSON).
{
  const expect = JSON.parse(readFileSync(golden('layout.json'), 'utf8'));
  assert.deepEqual(JSON.parse(wasm.layoutJson()), expect);
  ok('layoutJson equals tests/golden/layout.json');
}

// --- messages: bytes equal the native protocol golden fixtures.
{
  assert.deepEqual([...wasm.encodePing(1234)], [...hex('ping.hex')]);
  // Parameters of protocol/tests/limits.rs `sample_messages` (Hello is bit packed: no byte offsets).
  const out = wasm.encodeHello(3, 0x0123456789abcdefn, new Uint8Array(40).fill(7), 42, 3);
  assert.deepEqual([...out], [...hex('hello.hex')]);
  const pong = wasm.decodeServerC0(hex('pong.hex'));
  assert.equal(pong.kind, 5);
  assert.deepEqual([pong.clientTime, pong.serverTick, pong.serverTimeMs], [1234, 90001, 0x2dc6e1]);
  const wel = wasm.decodeServerC0(hex('welcome.hex'));
  assert.deepEqual([wel.kind, wel.playerId, wel.serverTick, wel.tickHz], [2, 3, 90000, 30]);
  assert.equal(wasm.decodeServerC0(hex('reject.hex')).kind, 3);
  assert.equal(wasm.decodeServerC0(hex('disconnect.hex')).kind, 6);
  assert.deepEqual([...wasm.encodeDisconnect(1)].length > 0, true);
  const c = (i) => wasm.packInputCmd(i, 100 + i, 10, -20, 1234, -500, 1, 0, 0, 0);
  assert.equal(c(0).length, 18);
  const bundle = wasm.encodeInputBundle(Uint8Array.from([...c(0), ...c(1), ...c(2)]), 2, 7);
  assert.equal(wasm.inputBundleLen(bundle), 3);
  assert.deepEqual([...wasm.encodeInputCmd(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)], [...wasm.packInputCmd(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)]);
  ok('C0 / C1 messages match protocol golden bytes');
}

// --- snapshot decoder vs. natively decoded golden (key frame, then delta on the same decoder).
{
  const expected = JSON.parse(readFileSync(golden('snapshot_expected.json'), 'utf8'));
  const dec = new wasm.SnapshotDecoder();
  const arr = (a) => [...a];
  const f32 = (a) => [...a].map((v) => Math.fround(v));
  expected.forEach((e, i) => {
    const bytes = readFileSync(golden(`snapshot_${i}.bin`));
    assert.equal(dec.decode(bytes), e.tick);
    assert.equal(dec.tick(), e.tick);
    assert.equal(dec.ackInputSeq(), e.ackInputSeq);
    assert.equal(dec.inputSlack(), e.inputSlack);
    assert.equal(dec.count(), e.ids.length);
    const s = dec.selfState();
    assert.equal(s[0] === 1, e.selfPresent);
    if (e.selfPresent) {
      assert.deepEqual([s[5], s[6], s[7], s[8], s[9], s[10], s[11], s[12]], [e.ammoInMag, e.ammoReserve, e.health, e.stamina, e.suppression, e.stance, e.deployState, e.cooldownBits]);
      assert.deepEqual(f32([s[2], s[3], s[4]]), f32(e.corr));
    }
    assert.deepEqual(arr(dec.ids()), e.ids);
    assert.deepEqual(f32(dec.pos()), f32(e.pos));
    assert.deepEqual(f32(dec.yaw()), f32(e.yaw));
    assert.deepEqual(f32(dec.pitch()), f32(e.pitch));
    assert.deepEqual(arr(dec.anim()), e.anim);
    assert.deepEqual(arr(dec.stance()), e.entStance);
    assert.deepEqual(arr(dec.weapon()), e.weapon);
    assert.deepEqual(arr(dec.flags()), e.flags);
    assert.deepEqual(arr(dec.health()), e.entHealth);
    assert.deepEqual(arr(dec.removed()), e.removed);
    assert.deepEqual(arr(dec.events()), e.events);
  });
  ok('SnapshotDecoder: key + delta golden fields equal native decode');

  // acceptance 5: 1000 decodes do not grow linear memory by more than 64 KB.
  const key = readFileSync(golden('snapshot_0.bin'));
  const before = exports.memory.buffer.byteLength;
  for (let i = 0; i < 1000; i++) {
    dec.reset();
    dec.decode(key);
    dec.pos();
    dec.ids();
  }
  const growth = exports.memory.buffer.byteLength - before;
  assert.ok(growth <= 64 * 1024, `memory grew ${growth} B`);
  ok(`1000 decodes: wasm memory growth ${growth} B (<= 65536)`);
}

// --- acceptance 6: invalid input throws JS exceptions (no abort).
{
  const dec = new wasm.SnapshotDecoder();
  const key = readFileSync(golden('snapshot_0.bin'));
  assert.throws(() => dec.decode(key.subarray(0, 30)), /snapshot decode failed/); // truncated snapshot
  assert.throws(() => dec.decode(new Uint8Array(0)));
  assert.throws(() => dec.decode(Uint8Array.from([0xff, 0xff, 0xff, 0xff, 0xff])));
  const p = new wasm.Predictor(1n, 0, 3);
  assert.throws(() => p.step(new Uint8Array(17)), /18 bytes/); // wrong-length cmd
  assert.throws(() => p.step(new Uint8Array(19)), /18 bytes/);
  assert.throws(() => p.reset(new Uint8Array(5), 0), /invalid authoritative state/);
  assert.throws(() => p.loadCollision(0, 0, new Uint8Array(10)), /bad collision chunk/);
  assert.throws(() => wasm.decodeServerC0(hex('pong.hex').subarray(0, 3)), /bad server C0/);
  assert.throws(() => wasm.packInputCmd(0, 0, -128, 0, 0, 0, 0, 0, 0, 0), /./);
  assert.throws(() => new wasm.Predictor(1n, 0, 200), /out of range/);
  // still usable after the errors
  p.step(wasm.packInputCmd(0, 1, 0, 0, 0, 0, 0, 0, 0, 0));
  p.free();
  ok('invalid inputs throw JS exceptions (truncated snapshot, bad cmd length, bad state/chunk, ...)');
}

// --- predictor on the real terrain: load 16 chunks, reset, 120 steps move the player forward.
{
  const dir = join(root, 'data/maps/m0/chunks');
  const have = existsSync(join(dir, 'c_0_0.bin'));
  if (!have) {
    const msg = 'terrain binaries missing: PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0';
    if (process.env.FRONTLINE_REQUIRE_TERRAIN) {
      console.error(`FAIL ${msg}`);
      process.exit(1);
    }
    console.log(`SKIP movement part: ${msg}`);
  } else {
    const p = new wasm.Predictor(0x5eed0014n, 0, 3);
    for (let bz = 0; bz < 4; bz++) for (let bx = 0; bx < 4; bx++) p.loadCollision(bx, bz, readFileSync(join(dir, `c_${bx}_${bz}.bin`)));
    const st = new DataView(new ArrayBuffer(80));
    [128, 2.4, 56, 0, 0, 0, 0, 0, 0].forEach((v, i) => st.setFloat32(i * 4, v, true));
    p.reset(new Uint8Array(st.buffer), 0);
    p.equipWeapon('us_m1_garand', 400);
    const z0 = 56;
    let s;
    for (let i = 0; i < 120; i++) s = p.step(wasm.packInputCmd(i, i + 1, 0, 127, 0, 0, 0, 0, 0, 0));
    assert.ok(s[2] > z0 + 5, `player advanced in +z: z=${s[2]}`);
    assert.equal(s[9], 1, 'grounded');
    assert.equal(s[16], 0, 'not blocked');
    assert.equal(p.tick(), 120);
    assert.equal(p.weaponState().length, 33);
    assert.equal(p.authState().length, 80);
    assert.ok(typeof p.stateHash() === 'bigint');
    assert.equal(p.unloadCollision(0, 0), true);
    p.free();
    ok(`Predictor: 16 chunks, 120 steps, z ${z0} -> ${s[2].toFixed(2)}`);
  }
}

console.log(`\nnode-smoke: ${passed} groups passed`);
