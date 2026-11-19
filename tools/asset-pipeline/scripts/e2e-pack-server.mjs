#!/usr/bin/env node
/**
 * Builds temporary end-to-end `.fpak` + `manifest.json` + `probe.json` fixtures via the real asset pipeline and serves
 * them over HTTP with both 206 Range (`/range/...`) and 200 full-file fallback (`/norange/...`) plus request counters.
 * Used by `client/e2e/asset-loader.spec.ts` to test M1-001..M1-006 in real Chromium without committing binary packs.
 */
import { Buffer } from 'node:buffer';
import { createServer } from 'node:http';
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import process from 'node:process';
import { Document, NodeIO } from '@gltf-transform/core';
import {
  Codec,
  FpakWriter,
  assertNoStandalone,
  assertOpaque,
  buildManifest,
  checkBudgets,
  checkConsistency,
  checkPlaceholders,
  encodeKtx2,
  encodeMeshEntry,
  encodePng,
  encodeTextureEntry,
  ingest,
  normalize,
  pack,
  packAudio,
  parseLedger,
  processMesh,
  publishPath,
  textureEntryFromKtx2,
  writeManifest,
  writeProbe,
  writeWav,
} from '../dist/index.js';
import { ffmpeg } from '../dist/audio/ffmpeg.js';

const workRoot = resolve(process.argv[2] ?? '/tmp/fl-e2e-packs');
mkdirSync(workRoot, { recursive: true });

function makeLedger(rows) {
  const hdr =
    '| asset_id | req_id | name | type | source_channel | author_vendor | url | license_name_version | license_text_path | chk_engine | chk_web | chk_commercial | chk_attribution | chk_seats | order_id | purchase_date | purchaser | confirmation_path | raw_path | processed_package | allowed_processing | status | notes | credit_text |\n' +
    '|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n';
  const lines = rows.map(
    (r) =>
      `| ${r.asset_id} | ${r.req_id} | ${r.name} | ${r.type} | Poly Haven | Synth | https://example.invalid | CC0 1.0 | licenses/texts/cc0.txt | Y | Y | Y | NONE | Y | FREE-2026-11-19 | 2026-11-19 | e2e | — | ${r.raw_path} | — | ${r.allowed} | OK | e2e fixture | — |`,
  );
  return `# LICENSES\n\n## 3. 台账\n\n${hdr}${lines.join('\n')}\n`;
}

function makeSphere(lon, lat) {
  const pos = [];
  const norm = [];
  const uv = [];
  for (let j = 0; j <= lat; j++) {
    const v = j / lat;
    const phi = v * Math.PI;
    for (let i = 0; i <= lon; i++) {
      const u = i / lon;
      const th = u * Math.PI * 2;
      const x = Math.sin(phi) * Math.cos(th);
      const y = Math.cos(phi);
      const z = Math.sin(phi) * Math.sin(th);
      pos.push(x, y, z);
      norm.push(x, y, z);
      uv.push(u, v);
    }
  }
  const idx = [];
  const stride = lon + 1;
  for (let j = 0; j < lat; j++) {
    for (let i = 0; i < lon; i++) {
      const a = j * stride + i;
      const b = a + 1;
      const c = a + stride;
      const d = c + 1;
      if (j > 0) idx.push(a, c, b);
      if (j < lat - 1) idx.push(b, c, d);
    }
  }
  return { pos: Float32Array.from(pos), norm: Float32Array.from(norm), uv: Float32Array.from(uv), idx: Uint32Array.from(idx) };
}

function makeRaster(size, fn) {
  const data = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const [r, g, b, a] = fn(x / size, y / size, x, y);
      const o = (y * size + x) * 4;
      data[o] = r;
      data[o + 1] = g;
      data[o + 2] = b;
      data[o + 3] = a;
    }
  }
  return { width: size, height: size, data };
}

async function pngToJpeg(pngBytes) {
  const { stdout } = await ffmpeg(
    ['-f', 'image2pipe', '-vcodec', 'png', '-i', 'pipe:0', '-f', 'image2pipe', '-vcodec', 'mjpeg', '-q:v', '4', 'pipe:1'],
    { input: pngBytes },
  );
  return new Uint8Array(stdout.buffer, stdout.byteOffset, stdout.byteLength);
}

async function writeTexturedGltf(outPath, sphere, { jpegAlbedo = false } = {}) {
  const doc = new Document();
  const buf = doc.createBuffer();
  const pos = doc.createAccessor().setType('VEC3').setArray(sphere.pos).setBuffer(buf);
  const norm = doc.createAccessor().setType('VEC3').setArray(sphere.norm).setBuffer(buf);
  const uv = doc.createAccessor().setType('VEC2').setArray(sphere.uv).setBuffer(buf);
  const idx = doc.createAccessor().setType('SCALAR').setArray(sphere.idx).setBuffer(buf);
  const albedoBytes = encodePng(makeRaster(64, (u, v) => [Math.round(u * 255), Math.round(v * 255), 140, 255]));
  const normalBytes = encodePng(makeRaster(64, (u, v) => [Math.round(128 + 30 * Math.sin(u * 12)), Math.round(128 + 30 * Math.cos(v * 12)), 250, 255]));
  const aoBytes = encodePng(makeRaster(64, (u) => { const g = Math.round(160 + 80 * u); return [g, g, g, 255]; }));
  const mrBytes = encodePng(makeRaster(64, (_, v) => [0, Math.round(120 + 100 * v), 40, 255]));
  const mat = doc.createMaterial('demo_mat').setRoughnessFactor(0.8).setMetallicFactor(0.2);
  const tAlb = doc
    .createTexture('albedo')
    .setImage(jpegAlbedo ? await pngToJpeg(albedoBytes) : albedoBytes)
    .setMimeType(jpegAlbedo ? 'image/jpeg' : 'image/png');
  const tNor = doc.createTexture('normal').setImage(normalBytes).setMimeType('image/png');
  const tAo = doc.createTexture('ao').setImage(aoBytes).setMimeType('image/png');
  const tMr = doc.createTexture('mr').setImage(mrBytes).setMimeType('image/png');
  mat.setBaseColorTexture(tAlb).setNormalTexture(tNor).setOcclusionTexture(tAo).setMetallicRoughnessTexture(tMr);
  const prim = doc.createPrimitive().setAttribute('POSITION', pos).setAttribute('NORMAL', norm).setAttribute('TEXCOORD_0', uv).setIndices(idx).setMaterial(mat);
  const mesh = doc.createMesh('demo_mesh').addPrimitive(prim);
  const node = doc.createNode('demo_node').setMesh(mesh);
  doc.createScene('scene').addChild(node);
  mkdirSync(dirname(outPath), { recursive: true });
  await new NodeIO().write(outPath, doc);
}

async function buildDemoSuite(dir) {
  mkdirSync(join(dir, 'licenses/texts'), { recursive: true });
  writeFileSync(join(dir, 'licenses/texts/cc0.txt'), 'CC0 1.0 Universal');
  const ledgerPath = join(dir, 'LICENSES.md');
  writeFileSync(
    ledgerPath,
    makeLedger([
      { asset_id: 'AST-V-PROP-DEMO-01', req_id: 'V-PRP-001', name: 'Demo Prop JPEG', type: 'model', raw_path: 'assets/incoming/V-PRP-001/PolyHaven-FREE/', allowed: 'CONVERT,LOD,TEXCOMP' },
      { asset_id: 'AST-V-WPN-DEMO-01', req_id: 'V-WPN-US-001', name: 'Demo Rifle FP/TP', type: 'model', raw_path: 'assets/incoming/V-WPN-US-001/PolyHaven-FREE/', allowed: 'CONVERT,LOD,TEXCOMP' },
      { asset_id: 'AST-A-WPN-DEMO-01', req_id: 'A-WPN-002', name: 'Demo Shot Audio', type: 'sfx', raw_path: 'assets/incoming/A-WPN-002/Free-FREE/', allowed: 'TRANSCODE,LOUDNORM' },
    ]),
  );
  const incoming = join(dir, 'assets/incoming');
  const work = join(dir, 'work');
  const processed = join(dir, 'assets/processed');
  await writeTexturedGltf(join(incoming, 'V-PRP-001/PolyHaven-FREE/prop.gltf'), makeSphere(32, 32), { jpegAlbedo: true });
  await writeTexturedGltf(join(incoming, 'V-WPN-US-001/PolyHaven-FREE/rifle.gltf'), makeSphere(36, 36), { jpegAlbedo: false });
  const sr = 48000;
  const pcm = new Float32Array(sr);
  for (let i = 0; i < sr; i++) pcm[i] = 0.4 * Math.sin((2 * Math.PI * 440 * i) / sr);
  const wav = writeWav({
    sampleRate: sr,
    samples: pcm,
    extraChunks: [{ id: 'iXML', data: Buffer.from('<BWFXML><NOTE>SECRET_VENDOR</NOTE></BWFXML>') }],
  });
  const wavPath = join(incoming, 'A-WPN-002/Free-FREE/shot.wav');
  mkdirSync(dirname(wavPath), { recursive: true });
  writeFileSync(wavPath, wav);

  ingest({ incoming, ledger: ledgerPath, work, repoRoot: dir });
  await normalize({ work, ledger: ledgerPath, assetId: 'AST-V-PROP-DEMO-01', logicalId: 'prop_demo_crate' });
  await processMesh({
    work,
    ledger: ledgerPath,
    assetId: 'AST-V-PROP-DEMO-01',
    logicalId: 'prop_demo_crate',
    cls: { class: 'prop', texture_profile: 'bg', collision: { mode: 'trimesh', max_tris: 200 } },
  });
  await normalize({ work, ledger: ledgerPath, assetId: 'AST-V-WPN-DEMO-01', logicalId: 'wpn_demo_rifle' });
  await processMesh({
    work,
    ledger: ledgerPath,
    assetId: 'AST-V-WPN-DEMO-01',
    logicalId: 'wpn_demo_rifle',
    cls: { class: 'weapon', texture_profile: 'hero', variants: [{ suffix: '_fp' }, { suffix: '_tp', lod_ratios: [1, 0.25] }] },
  });
  await pack({ work, ledger: ledgerPath, out: processed });

  const audioRes = await packAudio(
    [{ logical_id: 'snd_demo_shot', src: join(work, 'AST-A-WPN-DEMO-01/raw/shot.wav'), cls: 'gun_near', asset_id: 'AST-A-WPN-DEMO-01' }],
    { ledger: parseLedger(readFileSync(ledgerPath, 'utf8')), reportPath: join(work, 'audio-report.json') },
  );
  const audioPak = join(processed, publishPath(audioRes.build.sha256).slice(1));
  mkdirSync(dirname(audioPak), { recursive: true });
  writeFileSync(audioPak, audioRes.build.bytes);
  await writeProbe(join(processed, 'probe.json'));

  assertNoStandalone(processed);
  const manifest = await buildManifest({
    packs: processed,
    work,
    ledger: ledgerPath,
    bundles: { map_carentan_medium: ['prop_*', 'wpn_*', 'snd_*'] },
    budgets: { core_bytes: 8_388_608, map_carentan_medium_bytes: 157_286_400 },
    gameVersion: '0.1.0-e2e',
  });
  writeManifest(join(processed, 'manifest.json'), manifest);
  const consistency = await checkConsistency(manifest, processed);
  const placeholders = checkPlaceholders(manifest, ledgerPath);
  const budget = checkBudgets(manifest, manifest.budgets);
  if (consistency.length || placeholders.length || !budget.ok) {
    throw new Error(`demo suite gate failed: ${JSON.stringify({ consistency, placeholders, failures: budget.failures })}`);
  }
  // Verify opacity on one real mesh & texture entry from the packed demo.
  const lod0Ref = manifest.assets.prop_demo_crate.lods[0];
  const texRef = manifest.assets.prop_demo_crate.textures[0];
  const pakBuf = readFileSync(join(processed, lod0Ref.pak.slice(1)));
  const meshOp = await assertOpaque(pakBuf.subarray(lod0Ref.offset, lod0Ref.offset + lod0Ref.bytes), 'prop_demo_crate@lod0');
  const texOp = await assertOpaque(pakBuf.subarray(texRef.offset, texRef.offset + texRef.bytes), texRef.entry);
  if (meshOp.some((r) => r.recognized) || texOp.some((r) => r.recognized)) {
    throw new Error(`opaque check failed: ${JSON.stringify({ meshOp, texOp })}`);
  }

  return {
    root: processed,
    manifestPath: '/manifest.json',
    probePath: '/probe.json',
    assets: Object.keys(manifest.assets),
    textures: manifest.assets.prop_demo_crate.textures.map((t) => t.entry),
  };
}

/** Generate a real meshopt-compressed mesh layout v1 payload of ~targetBytes using deterministic pseudo-random vertex positions. */
async function makeBenchMeshTemplate(vertexCount) {
  const pos = new Float32Array(vertexCount * 3);
  const uv = new Uint16Array(vertexCount * 2);
  let seed = 0x12345678;
  const next = () => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return seed / 4294967296;
  };
  for (let i = 0; i < vertexCount; i++) {
    pos[i * 3] = (next() - 0.5) * 100;
    pos[i * 3 + 1] = (next() - 0.5) * 100;
    pos[i * 3 + 2] = (next() - 0.5) * 100;
    uv[i * 2] = Math.floor(next() * 65535);
    uv[i * 2 + 1] = Math.floor(next() * 65535);
  }
  const triCount = Math.floor(vertexCount / 3);
  const idx = new Uint32Array(triCount * 3);
  for (let i = 0; i < idx.length; i++) idx[i] = i;
  return encodeMeshEntry({
    bbox: { min: [-50, -50, -50], max: [50, 50, 50] },
    materials: [{ baseColor: [0.6, 0.5, 0.4, 1], metallic: 0.1, roughness: 0.8, emissive: [0, 0, 0], textures: ['', '', '', ''] }],
    primitives: [
      {
        materialIndex: 0,
        matrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
        vertexCount,
        indexSize: 4,
        indices: idx,
        attributes: [
          { semantic: 'POSITION', componentType: 5126, elements: 3, normalized: false, data: new Uint8Array(pos.buffer) },
          { semantic: 'TEXCOORD_0', componentType: 5123, elements: 2, normalized: true, data: new Uint8Array(uv.buffer) },
        ],
      },
    ],
  });
}

async function buildBench100MbSuite(dir) {
  const processed = join(dir, 'processed');
  mkdirSync(processed, { recursive: true });
  // Build one ~1.6 MiB real meshopt-encoded mesh template and one 512x512 UASTC KTX2 texture template.
  const meshTmpl = await makeBenchMeshTemplate(115_000);
  const texKtx2 = await encodeKtx2(
    makeRaster(512, (u, v, x, y) => [((x * 13) ^ (y * 7)) & 255, Math.round(u * 255), Math.round(v * 255), 255]),
    { format: 'UASTC', srgb: true, channels: 3 },
  );
  const texTmpl = encodeTextureEntry(textureEntryFromKtx2(texKtx2));

  const meshCount = 64;
  const texCount = 8;
  const maxPakBytes = 16 * 1024 * 1024;
  let writer = new FpakWriter();
  let totalPayloadBytes = 0;
  let packCount = 0;

  const flush = () => {
    if (writer.count === 0) return;
    const built = writer.finish();
    const out = join(processed, publishPath(built.sha256).slice(1));
    mkdirSync(dirname(out), { recursive: true });
    writeFileSync(out, built.bytes);
    packCount++;
    writer = new FpakWriter();
  };

  const meshIds = [];
  for (let i = 0; i < meshCount; i++) {
    const copy = meshTmpl.slice();
    // Perturb bbox.max[2] float (offset 28..32 in mesh layout v1 header) so every entry has a unique SHA-256 cache key.
    new DataView(copy.buffer).setFloat32(28, 50 + i * 0.01, true);
    if (writer.count > 0 && writer.size + copy.byteLength > maxPakBytes) flush();
    const id = `prop_bench_${String(i).padStart(2, '0')}`;
    writer.add(`${id}@lod0`, 'mesh', Codec.meshV1, copy);
    meshIds.push(id);
    totalPayloadBytes += copy.byteLength;
  }
  const texIds = [];
  for (let i = 0; i < texCount; i++) {
    const copy = texTmpl.slice();
    // Keep KTX2 texture valid; vary unused reserved byte or duplicate into separate logical id.
    if (writer.count > 0 && writer.size + copy.byteLength > maxPakBytes) flush();
    const id = `tex_bench_${String(i).padStart(2, '0')}@tex_albedo`;
    writer.add(id, 'texture', Codec.textureV1, copy);
    texIds.push(id);
    totalPayloadBytes += copy.byteLength;
  }
  flush();

  const ledgerPath = join(dir, 'LICENSES.md');
  writeFileSync(ledgerPath, makeLedger([]));
  const manifest = await buildManifest({
    packs: processed,
    work: join(dir, 'work'),
    ledger: ledgerPath,
    bundles: { map_carentan_medium: ['prop_*', 'tex_*'] },
    budgets: { core_bytes: 8_388_608, map_carentan_medium_bytes: 157_286_400 },
    gameVersion: '0.1.0-bench',
  });
  // Ensure every texture entry has a unique sha256 cache key in manifest so OPFS stores each separately.
  for (let i = 0; i < texIds.length; i++) {
    const base = `tex_bench_${String(i).padStart(2, '0')}`;
    const ref = manifest.assets[base].textures[0];
    ref.sha256 = `${ref.sha256}_${i}`;
  }
  writeManifest(join(processed, 'manifest.json'), manifest);
  return { root: processed, totalPayloadBytes, packCount, meshIds, texIds, meshEntryBytes: meshTmpl.byteLength, texEntryBytes: texTmpl.byteLength };
}

async function buildUpload200Suite(dir) {
  const processed = join(dir, 'processed');
  mkdirSync(processed, { recursive: true });
  const tmpl = await makeBenchMeshTemplate(1500);
  const w = new FpakWriter();
  const meshIds = [];
  for (let i = 0; i < 200; i++) {
    const copy = tmpl.slice();
    new DataView(copy.buffer).setFloat32(28, 50 + i * 0.001, true);
    const id = `prop_small_${String(i).padStart(3, '0')}`;
    w.add(`${id}@lod0`, 'mesh', Codec.meshV1, copy);
    meshIds.push(id);
  }
  const built = w.finish();
  const out = join(processed, publishPath(built.sha256).slice(1));
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, built.bytes);
  const ledgerPath = join(dir, 'LICENSES.md');
  writeFileSync(ledgerPath, makeLedger([]));
  const manifest = await buildManifest({
    packs: processed,
    work: join(dir, 'work'),
    ledger: ledgerPath,
    bundles: { map_carentan_medium: ['prop_*'] },
    budgets: { map_carentan_medium_bytes: 157_286_400 },
    gameVersion: '0.1.0-upload200',
  });
  writeManifest(join(processed, 'manifest.json'), manifest);
  return { root: processed, count: 200, entryBytes: tmpl.byteLength, totalBytes: tmpl.byteLength * 200, meshIds };
}

const demo = await buildDemoSuite(join(workRoot, 'demo'));
const bench100mb = await buildBench100MbSuite(join(workRoot, 'bench100mb'));
const upload200 = await buildUpload200Suite(join(workRoot, 'upload200'));
const suites = { demo: demo.root, bench100mb: bench100mb.root, upload200: upload200.root };

let stats = { total: 0, range206: 0, full200: 0, bytesSent: 0, bySuite: { demo: 0, bench100mb: 0, upload200: 0 } };

const server = createServer((req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Private-Network', 'true');
  res.setHeader('Access-Control-Allow-Headers', 'Range, Content-Type');
  res.setHeader('Access-Control-Expose-Headers', 'Content-Range, Content-Length, Accept-Ranges');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }
  const url = (req.url ?? '/').split('?')[0];
  if (url === '/stats') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(stats));
    return;
  }
  if (url === '/stats/reset' && req.method === 'POST') {
    stats = { total: 0, range206: 0, full200: 0, bytesSent: 0, bySuite: { demo: 0, bench100mb: 0, upload200: 0 } };
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ ok: true }));
    return;
  }
  const m = /^\/(range|norange)\/(demo|bench100mb|upload200)\/(.+)$/.exec(url);
  if (!m) {
    res.writeHead(404);
    res.end('not found');
    return;
  }
  const [, mode, suite, rel] = m;
  const root = suites[suite];
  const file = resolve(root, rel);
  if (!file.startsWith(root)) {
    res.writeHead(403);
    res.end('forbidden');
    return;
  }
  let st;
  try {
    st = statSync(file);
  } catch {
    res.writeHead(404);
    res.end('missing');
    return;
  }
  if (!st.isFile()) {
    res.writeHead(404);
    res.end('not a file');
    return;
  }
  const buf = readFileSync(file);
  stats.total++;
  stats.bySuite[suite]++;
  const range = req.headers.range;
  const ct = file.endsWith('.json') ? 'application/json' : 'application/octet-stream';
  if (mode === 'range' && range) {
    const rm = /^bytes=(-)?(\d+)(?:-(\d+))?$/.exec(range);
    if (!rm) {
      res.writeHead(416);
      res.end();
      return;
    }
    const start = rm[1] ? Math.max(0, buf.length - Number(rm[2])) : Number(rm[2]);
    const end = rm[1] ? buf.length - 1 : rm[3] !== undefined ? Math.min(buf.length - 1, Number(rm[3])) : buf.length - 1;
    if (start > end || start >= buf.length) {
      res.writeHead(416);
      res.end();
      return;
    }
    const slice = buf.subarray(start, end + 1);
    stats.range206++;
    stats.bytesSent += slice.length;
    res.writeHead(206, {
      'Content-Type': ct,
      'Accept-Ranges': 'bytes',
      'Content-Range': `bytes ${start}-${end}/${buf.length}`,
      'Content-Length': String(slice.length),
    });
    res.end(slice);
    return;
  }
  stats.full200++;
  stats.bytesSent += buf.length;
  res.writeHead(200, { 'Content-Type': ct, 'Content-Length': String(buf.length) });
  res.end(buf);
});

server.listen(0, '127.0.0.1', () => {
  const addr = server.address();
  process.stdout.write(`E2E_PACK_SERVER_READY ${JSON.stringify({ port: addr.port, demo, bench100mb, upload200 })}\n`);
});

const stop = () => server.close(() => process.exit(0));
process.on('SIGINT', stop);
process.on('SIGTERM', stop);
