#!/usr/bin/env node
// Copies the generated map binaries (git-ignored) into client/public/maps/<id>/ so Vite serves them at /maps/<id>/.
// Regenerates data/maps/<id> with the map pipeline first when the height maps are missing (M0-009, DEVELOPING.md).
// Usage: node scripts/sync-map.mjs [mapId=m0]
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '..', '..');
const mapId = process.argv[2] ?? 'm0';
const src = join(repo, 'data', 'maps', mapId);
const dst = join(here, '..', 'public', 'maps', mapId);

/** Files the client fetches (M0-023 terrain; M0-022 appends the collision chunks it needs). */
const FILES = ['meta.json', 'heightmap_1m.r16', 'heightmap_far_8m.r16']; // spawns.json: vegetation keep-out discs (M0-024)
// M0-022: the whole M0 map (4 x 4 blocks of 64 m) is loaded into the prediction collision world at start-up (ARCHITECTURE 9.5).
if (mapId === 'm0') FILES.push('spawns.json');
if (mapId === 'm0') for (let bz = 0; bz < 4; bz++) for (let bx = 0; bx < 4; bx++) FILES.push(`chunks/c_${bx}_${bz}.bin`);
// M1-051b/052b: the Carentan world layers drawn by the client (hedgerows, buildings); no m0 chunk files for this map.
if (mapId !== 'm0') for (const f of ['hedgerows.geojson', 'buildings.geojson', 'roads.geojson', 'water.geojson', 'forests.geojson']) if (existsSync(join(src, f))) FILES.push(f);

if (mapId === 'm0' && FILES.some((f) => !existsSync(join(src, f)))) {
  console.log(`[sync-map] ${src} incomplete -> running the map pipeline (seed 1)`);
  const r = spawnSync('python3', ['-m', 'map_pipeline', mapId, '--seed', '1', '--out', src], {
    cwd: repo,
    stdio: 'inherit',
    env: { ...process.env, PYTHONPATH: join(repo, 'tools', 'map-pipeline') },
  });
  if (r.status !== 0) {
    console.error('[sync-map] map pipeline failed (needs python3 + numpy; see DEVELOPING.md)');
    process.exit(1);
  }
}
mkdirSync(join(dst, 'chunks'), { recursive: true });
for (const f of FILES) copyFileSync(join(src, f), join(dst, f));
if (mapId !== 'm0' && existsSync(join(src, 'm1', 'spawns.json'))) copyFileSync(join(src, 'm1', 'spawns.json'), join(dst, 'spawns.json')); // vegetation keep-out discs
console.log(`[sync-map] ${FILES.length} files -> ${dst}`);

// M0-022: the sim-wasm package (git-ignored build output of `wasm-pack build crates/sim-wasm --target web --profile wasm-release`)
// is served at /wasm/ so the client can load it at run time (client/src/game/net-game.ts). Missing package: warn, do not fail.
const pkg = join(repo, 'crates', 'sim-wasm', 'pkg');
const wasmDst = join(here, '..', 'public', 'wasm');
const WASM_FILES = ['frontline_sim_wasm.js', 'frontline_sim_wasm_bg.wasm'];
if (WASM_FILES.every((f) => existsSync(join(pkg, f)))) {
  mkdirSync(wasmDst, { recursive: true });
  for (const f of WASM_FILES) copyFileSync(join(pkg, f), join(wasmDst, f));
  console.log(`[sync-map] sim-wasm package -> ${wasmDst}`);
} else {
  console.warn('[sync-map] crates/sim-wasm/pkg missing: networked play (?server=) needs `wasm-pack build crates/sim-wasm --target web --profile wasm-release`');
}

// M0-025: grey-box weapon sizes and first-person view placement (data/placeholders/weapon_models.json) -> /data/placeholders/.
{
  const f = join(repo, 'data', 'placeholders', 'weapon_models.json');
  const d = join(here, '..', 'public', 'data', 'placeholders');
  if (existsSync(f)) {
    mkdirSync(d, { recursive: true });
    copyFileSync(f, join(d, 'weapon_models.json'));
    console.log(`[sync-map] weapon_models.json -> ${d}`);
  } else console.warn('[sync-map] data/placeholders/weapon_models.json missing: the actors cannot be built');
}
