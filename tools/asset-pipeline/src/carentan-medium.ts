/** Build the Carentan slice as a single Medium bundle of content-addressed FPAK objects. */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative, sep } from 'node:path';
import { buildCarentanPlaceholders } from './carentan-placeholders.ts';
import { packCarentanSources } from './carentan-sources.ts';
import { BufferSource, FpakReader, FpakWriter, assertNoStandalone, publishPath } from './fpak/index.ts';
import { loadLedger } from './ledger.ts';
import { assetRefs, buildManifest, manifestDigest, matchesAny, readManifest, writeManifest, type Manifest } from './manifest.ts';
import { buildReport } from './report.ts';
import { loadBudgets } from './budget.ts';

const MEDIUM = 'map_carentan_medium';
const TERRAIN = 'terrain_carentan_*';

export async function buildCarentanMedium(root: string, incoming: string, out: string): Promise<Manifest> {
  // Refuse to mix a build with stale packs or overwrite an existing directory.
  mkdirSync(out);
  const work = mkdtempSync(join(tmpdir(), 'carentan-medium-'));
  try {
    const terrainFile = join(work, 'terrain.json');
    const bake = spawnSync('python3', ['-c', `
from pathlib import Path
import sys
from map_pipeline import chunker, pack_chunks
chunks, output, manifest = map(Path, sys.argv[1:])
_, results, entries = chunker.bake(Path('data/maps/carentan'), chunks, 4)
if chunker.report(results, entries)['chunks_over_limit']:
    raise ValueError('terrain chunks exceed per-chunk budget')
pack_chunks.pack_chunks(chunks, output, manifest)
`, join(work, 'chunks'), out, terrainFile], {
      cwd: root, env: { ...process.env, PYTHONPATH: join(root, 'tools/map-pipeline') }, stdio: 'inherit',
    });
    if (bake.error) throw bake.error;
    if (bake.status !== 0) throw new Error(`Carentan terrain bake failed (${bake.status ?? bake.signal})`);
    const terrain = readManifest(terrainFile);
    const manifestDir = join(root, 'assets/manifests');
    if (manifestDigest(terrain) !== manifestDigest(readManifest(join(manifestDir, 'terrain_carentan.json')))) {
      throw new Error('terrain bake differs from the tracked Carentan slice manifest');
    }
    const ledger = loadLedger(join(root, 'LICENSES.md'));
    await packCarentanSources(root, incoming, work, out, ledger);
    const placeholders = await buildCarentanPlaceholders(out);
    const bundleConfig = JSON.parse(readFileSync(join(manifestDir, 'bundles.json'), 'utf8')) as Manifest['bundles'];
    const manifest = await buildManifest({
      packs: out, work, ledger,
      bundles: { [MEDIUM]: [...bundleConfig[MEDIUM]!, TERRAIN] },
      budgets: loadBudgets(join(manifestDir, 'budgets.json')),
      build: terrain.build, gameVersion: terrain.game_version, dataHash: terrain.data_hash,
      overrides: { '*': { tier: ['medium'] }, ...placeholders },
    });
    const assembledTerrain = Object.fromEntries(Object.entries(manifest.assets).filter(([id]) => matchesAny(id, [TERRAIN])));
    if (manifestDigest({ ...terrain, assets: assembledTerrain }) !== manifestDigest(terrain)) {
      throw new Error('packed terrain entries differ from the terrain bake manifest');
    }
    await verifyCarentanMedium(manifest, out);
    const manifestPath = join(out, 'manifest.json');
    writeManifest(manifestPath, manifest);
    const report = buildReport({ requirements: join(root, 'assets/requirements.json'), ledger, work: join(work, 'manifest-only'), manifest: manifestPath });
    if (report.violations.length) throw new Error(report.violations.join('\n'));
    writeFileSync(join(out, 'missing-report.json'), JSON.stringify(report, null, 2) + '\n');
    return manifest;
  } catch (error) {
    rmSync(out, { recursive: true, force: true });
    throw error;
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

/** Verify exact payloads and canonical bytes, including alignment/framing outside the manifest size count. */
export async function verifyCarentanMedium(manifest: Manifest, packs: string) {
  assertNoStandalone(packs);
  const byPack = new Map<string, Map<string, ReturnType<typeof assetRefs>[number]>>();
  const ids = new Set<string>();
  for (const [id, asset] of Object.entries(manifest.assets)) {
    if (!asset.tier.includes('medium') || !matchesAny(id, manifest.bundles[MEDIUM] ?? [])) {
      throw new Error(`${id}: outside the Carentan Medium bundle`);
    }
    for (const ref of assetRefs(asset)) {
      if (ids.has(ref.entry) || !/^\/a\/[0-9a-f]{2}\/[0-9a-f]{14}\.fpak$/.test(ref.pak)) {
        throw new Error(`${ref.entry}: duplicate entry or invalid pack path`);
      }
      ids.add(ref.entry);
      const refs = byPack.get(ref.pak) ?? new Map();
      refs.set(ref.entry, ref);
      byPack.set(ref.pak, refs);
    }
  }
  for (const file of readdirSync(packs, { recursive: true, withFileTypes: true })) {
    if (!file.isFile() || !file.name.endsWith('.fpak')) continue;
    const path = '/' + relative(packs, join(file.parentPath, file.name)).split(sep).join('/');
    if (!byPack.has(path)) throw new Error(`${path}: pack is absent from the Medium manifest`);
  }
  let bytes = 0;
  for (const [path, refs] of byPack) {
    const original = readFileSync(join(packs, path));
    bytes += original.length;
    const reader = await FpakReader.open(new BufferSource(original));
    const writer = new FpakWriter();
    if (reader.entries().length !== refs.size) throw new Error(`${path}: entry count differs from manifest`);
    for (const entry of reader.entries()) {
      const ref = refs.get(entry.logical_id);
      const payload = await reader.read(entry.logical_id);
      if (!ref || ref.offset !== Number(entry.offset) || ref.bytes !== payload.length
        || ref.sha256 !== createHash('sha256').update(payload).digest('hex')) {
        throw new Error(`${path}: ${entry.logical_id} differs from manifest`);
      }
      writer.add(entry.logical_id, entry.kind, entry.codec, payload);
    }
    const rebuilt = writer.finish();
    if (path !== publishPath(rebuilt.sha256) || !original.equals(rebuilt.bytes)) {
      throw new Error(`${path}: FPAK round-trip did not reproduce the file`);
    }
  }
  if (bytes === 0 || bytes > 150_000_000) throw new Error(`Carentan Medium: ${bytes} physical B (limit 150,000,000)`);
  return { packs: byPack.size, entries: ids.size, bytes };
}
