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
const FILES = ['meta.json', 'spawns.json', 'heightmap_1m.r16', 'heightmap_far_8m.r16']; // spawns.json: vegetation keep-out discs (M0-024)

if (FILES.some((f) => !existsSync(join(src, f)))) {
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
mkdirSync(dst, { recursive: true });
for (const f of FILES) copyFileSync(join(src, f), join(dst, f));
console.log(`[sync-map] ${FILES.length} files -> ${dst}`);
