import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import {
  Codec, FpakWriter, buildManifest, checkBudgets, checkConsistency, checkPlaceholders, globToRegExp, manifestDigest, parseLedger, publishPath,
  serializeManifest, writeStats, bundleBytes, categoryBreakdown, formatCategoryBreakdown, type Manifest, type Stats,
} from '../src/index.ts';
import { main } from '../src/cli.ts';
import { ExitCode } from '../src/errors.ts';
import { ledgerText } from './helpers.ts';

const tmp = mkdtempSync(join(tmpdir(), 'fl-manifest-test-'));
afterAll(() => rmSync(tmp, { recursive: true, force: true }));

const LEDGER = parseLedger(ledgerText([
  { asset_id: 'AST-OK', req_id: 'V-VEH-001', status: 'OK', raw_path: 'assets/incoming/V-VEH-001/a/' },
  { asset_id: 'AST-PEND', req_id: 'V-VEH-002', status: 'PENDING', raw_path: 'assets/incoming/V-VEH-002/b/' },
]));

/** A pack with a model (2 LODs + meta), written to `{root}/a/xx/yyyy.fpak`. */
function makeWorld(name: string, ids: { logical: string; asset: string; lodBytes: number }[]) {
  const root = join(tmp, name);
  const packs = join(root, 'packs');
  const work = join(root, 'work');
  const w = new FpakWriter();
  for (const { logical, asset, lodBytes } of ids) {
    w.add(`${logical}@lod0`, 'mesh', Codec.meshV1, new Uint8Array(lodBytes).fill(1));
    w.add(`${logical}@lod1`, 'mesh', Codec.meshV1, new Uint8Array(Math.floor(lodBytes / 3)).fill(2));
    w.add(`${logical}@meta`, 'meta', Codec.json, new TextEncoder().encode('{}'));
    const stats: Stats = {
      schema: 'frontline.asset-stats/1', asset_id: asset, logical_id: logical, source_bytes: 1, textures: [], warnings: [],
      lods: [], chains: [{ suffix: '', bbox_error: 0, lods: [{ lod: 0, tris: 100, vertices: 1, bytes: lodBytes, simplify_error: 0, simplify_error_m: 0, seam_lock: 'none', file: 'x' }, { lod: 1, tris: 30, vertices: 1, bytes: 1, simplify_error: 0, simplify_error_m: 0, seam_lock: 'none', file: 'y' }] }],
    };
    mkdirSync(join(work, asset), { recursive: true });
    writeStats(join(work, asset, 'stats.json'), stats);
  }
  const b = w.finish();
  const file = join(packs, publishPath(b.sha256).slice(1));
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, b.bytes);
  return { packs, work, pak: publishPath(b.sha256), file, bytes: b.bytes };
}

const BUNDLES = { map_carentan_medium: ['veh_*'], core: ['ui_*'] };
const BUDGETS = { core_bytes: 8388608, map_carentan_medium_bytes: 100_000 };

describe('manifest generation', () => {
  it('lists logical ids with lods, tris, placeholder flag and license_ref', async () => {
    const w = makeWorld('gen', [{ logical: 'veh_us_test', asset: 'AST-OK', lodBytes: 3000 }, { logical: 'veh_us_pend', asset: 'AST-PEND', lodBytes: 600 }]);
    const m = await buildManifest({ ...w, ledger: LEDGER, bundles: BUNDLES, budgets: BUDGETS, build: 't' });
    expect(Object.keys(m.assets)).toEqual(['veh_us_pend', 'veh_us_test']);
    const a = m.assets.veh_us_test!;
    expect(a.kind).toBe('model');
    expect(a.placeholder).toBe(false);
    expect(a.license_ref).toBe('LICENSES.md#AST-OK');
    expect(a.req_id).toBe('V-VEH-001');
    expect(a.lods!.map((l) => [l.lod, l.tris, l.pak])).toEqual([[0, 100, w.pak], [1, 30, w.pak]]);
    expect(m.assets.veh_us_pend!.placeholder).toBe(true);
    expect(await checkConsistency(m, w.packs)).toEqual([]);
    expect(checkPlaceholders(m, LEDGER)).toEqual([]);
  });

  it('blocks an INTERNAL pack even if its manifest entry is a placeholder', async () => {
    const w = makeWorld('internal', [{ logical: 'veh_us_test', asset: 'AST-OK', lodBytes: 3000 }]);
    const options = { ...w, bundles: BUNDLES, budgets: BUDGETS, overrides: { '*': { placeholder: true } } };
    const m = await buildManifest({ ...options, ledger: LEDGER });
    expect(m.assets.veh_us_test!.placeholder).toBe(true);
    const ledger = parseLedger(ledgerText([{ asset_id: 'AST-OK', status: 'INTERNAL' }]));
    expect(checkPlaceholders(m, ledger)).toEqual([expect.stringContaining('internal-only')]);
    await expect(buildManifest({ ...options, ledger })).rejects.toMatchObject({ code: ExitCode.NotOk });
  });

  it('is byte-identical for equal inputs apart from the build timestamp (acceptance 5)', async () => {
    const w = makeWorld('det', [{ logical: 'veh_us_test', asset: 'AST-OK', lodBytes: 3000 }]);
    const a = await buildManifest({ ...w, ledger: LEDGER, bundles: BUNDLES, budgets: BUDGETS, build: '1' });
    const b = await buildManifest({ ...w, ledger: LEDGER, bundles: BUNDLES, budgets: BUDGETS, build: '2' });
    expect(manifestDigest(a)).toBe(manifestDigest(b));
    expect(serializeManifest(a)).not.toBe(serializeManifest(b));
  });

  it('rejects an offset changed by +1 and names the entry (acceptance 3)', async () => {
    const w = makeWorld('cons', [{ logical: 'veh_us_test', asset: 'AST-OK', lodBytes: 3000 }]);
    const m = await buildManifest({ ...w, ledger: LEDGER, bundles: BUNDLES, budgets: BUDGETS });
    m.assets.veh_us_test!.lods![0]!.offset += 1;
    const p = await checkConsistency(m, w.packs);
    expect(p).toHaveLength(1);
    expect(p[0]).toContain('veh_us_test@lod0');
    expect(p[0]).toContain('offset');
  });

  it('rejects placeholder:false for a PENDING ledger row (acceptance 4)', async () => {
    const w = makeWorld('ph', [{ logical: 'veh_us_pend', asset: 'AST-PEND', lodBytes: 600 }]);
    const m = await buildManifest({ ...w, ledger: LEDGER, bundles: BUNDLES, budgets: BUDGETS, overrides: { 'veh_*': { placeholder: false } } });
    expect(m.assets.veh_us_pend!.placeholder).toBe(false);
    expect(checkPlaceholders(m, LEDGER)[0]).toMatch(/veh_us_pend.*PENDING/);
  });

  it('1000 entries build + check in under 5 s', async () => {
    const root = join(tmp, 'big');
    const w = new FpakWriter();
    for (let i = 0; i < 1000; i++) w.add(`prop_crate_${i}@lod0`, 'mesh', Codec.meshV1, new Uint8Array(64).fill(i & 255));
    const b = w.finish();
    const f = join(root, 'packs', publishPath(b.sha256).slice(1));
    mkdirSync(dirname(f), { recursive: true });
    writeFileSync(f, b.bytes);
    const t0 = performance.now();
    const m = await buildManifest({ packs: join(root, 'packs'), work: join(root, 'work'), ledger: LEDGER, bundles: {}, budgets: {} });
    expect(await checkConsistency(m, join(root, 'packs'))).toEqual([]);
    expect(Object.keys(m.assets)).toHaveLength(1000);
    expect(performance.now() - t0).toBeLessThan(5000);
  });
});

function synthetic(bytes: number, tier: string[] = ['medium']): Manifest {
  const ref = { pak: '/a/x.fpak', entry: 'veh_a@lod0', offset: 0, bytes, sha256: '', index_bytes: 0 };
  return {
    schema: 1, build: '', game_version: '', data_hash: '', bundles: { map_carentan_medium: ['veh_*'] }, budgets: {},
    assets: { veh_a: { kind: 'model', tier: tier as never, placeholder: true, lods: [{ lod: 0, suffix: '', ...ref }] } },
  };
}
const BUDGET = { map_carentan_medium_bytes: 1_000_000 };

describe('budget arithmetic', () => {
  it.each([
    [1_000_000, true], [1_009_999, true], [1_010_000, true], [1_010_001, false], [1_020_000, false], [900_000, true],
  ])('%i bytes against a 1,000,000 budget -> ok=%s (1 %% tolerance, acceptance 1)', (bytes, ok) => {
    const r = checkBudgets(synthetic(bytes), BUDGET);
    expect(r.ok).toBe(ok);
    if (!ok) expect(r.failures[0]).toMatch(/map_carentan_medium.*over/);
  });

  it('2 % over fails naming the bundle and the excess', () => {
    const r = checkBudgets(synthetic(1_020_000), BUDGET);
    expect(r.failures[0]).toContain('map_carentan_medium');
    expect(r.failures[0]).toContain('2.00 %');
  });

  it('0.9 % over passes', () => {
    expect(checkBudgets(synthetic(1_009_000), BUDGET).ok).toBe(true);
  });

  it.each([
    [100_000, 120_000, false, true], // exactly +20 %
    [100_000, 120_001, false, false],
    [100_000, 121_000, true, true], // approved label
    [100_000, 119_999, false, true],
    [100_000, 50_000, false, true], // shrinking
  ])('growth %i -> %i approved=%s -> ok=%s (acceptance 2)', (base, now, approved, ok) => {
    const r = checkBudgets(synthetic(now), BUDGET, synthetic(base), { approved });
    expect(r.ok).toBe(ok);
    if (!ok) expect(r.failures[0]).toContain('budget-approved');
  });

  it('core is skipped without a size report and measured with one (acceptance 7)', () => {
    const m = synthetic(10);
    m.bundles.core = ['ui_*'];
    const b = { core_bytes: 8388608, ...BUDGET };
    expect(checkBudgets(m, b).skipped).toEqual(['core']);
    const r = checkBudgets(m, b, undefined, { external: { core: 7_000_000 } });
    expect(r.skipped).toEqual([]);
    expect(r.usages.find((u) => u.bundle === 'core')!.bytes).toBe(7_000_000);
    expect(checkBudgets(m, b, undefined, { external: { core: 9_000_000 } }).ok).toBe(false);
  });

  it('a tiered bundle counts only assets shipped on that tier', () => {
    expect(bundleBytes(synthetic(500, ['high', 'ultra']), 'map_carentan_medium')).toBe(0);
    expect(bundleBytes(synthetic(500, ['medium']), 'map_carentan_medium')).toBe(500);
  });
});

describe('globs', () => {
  it.each([
    ['ui_*', 'ui_button', true], ['ui_*', 'xui_button', false], ['terrain_carentan/*', 'terrain_carentan/b_1_2', true],
    ['terrain_carentan/*', 'terrain_carentan/a/b', false], ['terrain_carentan/**', 'terrain_carentan/a/b', true], ['veh_us_m4*', 'veh_us_m4a1', true],
  ])('%s vs %s -> %s', (g, id, ok) => {
    expect(globToRegExp(g).test(id)).toBe(ok);
  });
});

describe('category breakdown & core-size.mjs compatibility', () => {
  it('breaks down bytes by category prefix including _fp and _tp weapon chains', () => {
    const ref = (entry: string, bytes: number) => ({ pak: '/a/x.fpak', entry, offset: 0, bytes, sha256: '', index_bytes: 20 });
    const m: Manifest = {
      schema: 1, build: '', game_version: '', data_hash: '', bundles: {}, budgets: {},
      assets: {
        wpn_us_garand: {
          kind: 'model', tier: ['medium'], placeholder: true,
          lods: [
            { lod: 0, suffix: '_fp', ...ref('wpn_us_garand@lod0_fp', 5000) },
            { lod: 0, suffix: '_tp', ...ref('wpn_us_garand@lod0_tp', 1200) },
          ],
        },
        snd_garand_fire: {
          kind: 'audio', tier: ['medium'], placeholder: true,
          audio: [{ codec: 'opus', ...ref('snd_garand_fire@opus', 800) }],
        },
      },
    };
    const rows = categoryBreakdown(m, 'medium');
    expect(rows).toEqual([
      { category: 'snd', assets: 1, entries: 1, bytes: 820, fpBytes: 0, tpBytes: 0 },
      { category: 'wpn', assets: 1, entries: 2, bytes: 6240, fpBytes: 5020, tpBytes: 1220 },
    ]);
    expect(formatCategoryBreakdown(rows)).toContain('wpn');
    expect(formatCategoryBreakdown(rows)).toContain('5020');
  });

  it('manifest-budget CLI reads brotliBytes produced by tools/ci/core-size.mjs', async () => {
    const root = join(tmp, 'core-size-cli');
    mkdirSync(join(root, 'assets/manifests'), { recursive: true });
    writeFileSync(join(root, 'LICENSES.md'), ledgerText([]));
    writeFileSync(join(root, 'assets/manifests/budgets.json'), JSON.stringify({ core_bytes: 8_388_608, map_carentan_medium_bytes: 157_286_400 }));
    writeFileSync(join(root, 'assets/manifests/manifest.json'), JSON.stringify({
      schema: 1, build: 't', game_version: '0.1.0', data_hash: '',
      assets: {}, bundles: { core: ['ui_*'], map_carentan_medium: ['veh_*'] },
      budgets: { core_bytes: 8_388_608, map_carentan_medium_bytes: 157_286_400 },
    }));
    writeFileSync(join(root, 'size-report.json'), JSON.stringify({ files: 10, rawBytes: 4_000_000, brotliBytes: 1_150_132, dataBytes: 9000, max: 8_388_608, pass: true }));
    const okLines: string[] = [];
    expect(await main(['manifest-budget', '--root', root, '--core-size-file', 'size-report.json'], (l) => okLines.push(l))).toBe(ExitCode.Ok);
    expect(okLines.join('\n')).toContain('1150132');
    expect(okLines.join('\n')).not.toContain('skipped');
    writeFileSync(join(root, 'size-report.json'), JSON.stringify({ files: 10, rawBytes: 20_000_000, brotliBytes: 9_000_000, dataBytes: 0, max: 8_388_608, pass: false }));
    const badLines: string[] = [];
    expect(await main(['manifest-budget', '--root', root, '--core-size-file', 'size-report.json'], (l) => badLines.push(l))).not.toBe(ExitCode.Ok);
    expect(badLines.join('\n')).toMatch(/FAIL core.*over/);
  });
});
