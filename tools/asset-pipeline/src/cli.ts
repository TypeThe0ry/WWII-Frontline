import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { ExitCode, GateError } from './errors.ts';
import { ingest } from './ingest.ts';
import { checkSourceIndex } from './source-index.ts';
import { loadLedger, isBlank } from './ledger.ts';
import type { Axis } from './math.ts';
import { assertArchives, repoRootOf } from './gate.ts';
import { normalize, type OriginMode } from './normalize.ts';
import { loadAssetClass } from './asset-class.ts';
import { processMesh } from './process.ts';
import { pack } from './pack.ts';
import { packAudio, writeProbe, type AudioItem } from './audio/index.ts';
import { publishPath } from './fpak/writer.ts';
import { loadLedger as loadLedgerForAudio } from './ledger.ts';
import { assertNoStandalone } from './fpak/standalone.ts';
import { readStats, sizeRatio, statsPath } from './stats.ts';
import { buildReport, formatReport, parseShoppingList } from './report.ts';
import { buildManifest, readManifest, writeManifest, type Overrides } from './manifest.ts';
import { checkBudgets, formatUsages, loadBudgets } from './budget.ts';
import { checkConsistency, checkPlaceholders } from './consistency.ts';

const USAGE = `asset-pipeline <command> [options]

  ingest            gate + copy assets/incoming/** to work/{asset_id}/raw/ (exit 2 unregistered, 3 not OK, 4 D-LIC, 5 archive missing)
  normalize         unit/axes/origin/triangulate/merge one ingested glTF -> work/{asset_id}/normalized.gltf
  process            LOD chains, KTX2 textures, mesh compression, collision -> work/{asset_id}/lodN.glb, stats.json (M1-002)
  pack              work/ intermediates -> assets/processed/a/xx/yyyy.fpak + packs.json (M1-003)
  audio-pack        audio items JSON -> loudnorm + Opus/AAC -> .fpak (@opus/@aac entries), work/audio-report.json, probe.json (M1-004)
  assert-no-standalone  fail when the output directory holds .glb/.ktx2/.ogg/... files
  check-size-ratio  LOD0 bytes / source bytes from stats.json (fails above --max, default 0.25)
  report            per requirement ID real/placeholder/missing/internal-only (ASSET_GAPS 3.4)
  ledger-check      LICENSES.md 4.4 validation: counts per status, missing archive files
  manifest          .fpak files + stats + ledger -> assets/manifests/manifest.json (+ missing-report.json) (M1-005)
  manifest-budget   per-bundle size budgets (1 % over fails, > 20 % growth needs budget-approved), pack consistency, placeholder rule
  gen-requirements  build assets/requirements.json from docs/ASSET_SHOPPING_LIST.md

common:  --ledger LICENSES.md   --work work   --root <repo root, default cwd>
ingest:  [--incoming assets/incoming | --metadata-only INDEX.json]
normalize: --asset AST-... --id <logical_id> [--input rel/in/raw] [--unit-scale m] [--up Y|Z] [--forward +Z|-Y|...] [--origin bottom-center|center|none]
process: --asset AST-... --id <logical_id> --class asset-class.json [--no-collision]
pack:    [--assets AST-a,AST-b] [--out assets/processed] [--max-bytes 16777216]
audio-pack: --items items.json [--out assets/processed]   items: [{logical_id, src, cls, loop?, loop_start_ms?, loop_end_ms?, asset_id?}], src relative to --root
assert-no-standalone: [--out assets/processed]
check-size-ratio: --asset AST-... [--max 0.25] [--suffix _fp]
report:  [--requirements assets/requirements.json] [--manifest draft.json] [--placeholders data/placeholders] [--metadata-only INDEX.json] [--milestone M1] [--out report.json] [--gaps] [--strict]
manifest: [--packs assets/processed] [--out assets/manifests/manifest.json] [--bundles assets/manifests/bundles.json] [--budgets assets/manifests/budgets.json] [--overrides f.json] [--game-version 0.1.0] [--data-hash h] [--requirements assets/requirements.json]
manifest-budget: [--manifest assets/manifests/manifest.json] [--budgets ...] [--base base-manifest.json] [--packs assets/processed] [--core-size-file client/dist/size-report.json] [--approved | env BUDGET_APPROVED=1]
gen-requirements: [--shopping-list docs/ASSET_SHOPPING_LIST.md] [--out assets/requirements.json]
`;

export async function main(argv: string[], out: (s: string) => void = console.log): Promise<number> {
  const [cmd, ...rest] = argv;
  if (!cmd || cmd === '-h' || cmd === '--help') {
    out(USAGE);
    return cmd ? ExitCode.Ok : ExitCode.Usage;
  }
  const { values: v } = parseArgs({
    args: rest,
    options: {
      root: { type: 'string' },
      ledger: { type: 'string' },
      work: { type: 'string' },
      incoming: { type: 'string' },
      asset: { type: 'string' },
      id: { type: 'string' },
      input: { type: 'string' },
      'unit-scale': { type: 'string' },
      up: { type: 'string' },
      forward: { type: 'string' },
      origin: { type: 'string' },
      requirements: { type: 'string' },
      'metadata-only': { type: 'string' },
      manifest: { type: 'string' },
      placeholders: { type: 'string' },
      milestone: { type: 'string' },
      out: { type: 'string' },
      'shopping-list': { type: 'string' },
      assets: { type: 'string' },
      items: { type: 'string' },
      'max-bytes': { type: 'string' },
      class: { type: 'string' },
      max: { type: 'string' },
      suffix: { type: 'string' },
      'no-collision': { type: 'boolean' },
      gaps: { type: 'boolean' },
      strict: { type: 'boolean' },
      packs: { type: 'string' },
      bundles: { type: 'string' },
      budgets: { type: 'string' },
      overrides: { type: 'string' },
      'game-version': { type: 'string' },
      'data-hash': { type: 'string' },
      base: { type: 'string' },
      'core-size-file': { type: 'string' },
      approved: { type: 'boolean' },
    },
  });
  const root = resolve(v.root ?? process.cwd());
  const ledger = resolve(root, v.ledger ?? 'LICENSES.md');
  const work = resolve(root, v.work ?? 'work');
  try {
    switch (cmd) {
      case 'ingest': {
        if (v['metadata-only']) {
          if (v.incoming) throw new GateError(ExitCode.Usage, 'use either --metadata-only or --incoming');
          const index = checkSourceIndex(resolve(root, v['metadata-only']), loadLedger(ledger), root);
          out(`ingest (metadata-only): ${index.files} file record(s) passed the ledger gate; no files copied`);
          return ExitCode.Ok;
        }
        const r = ingest({ incoming: resolve(root, v.incoming ?? 'assets/incoming'), ledger, work, repoRoot: root });
        out(`ingest: ${r.files.length} file(s) from ${r.assets.length} asset(s) copied to ${work}`);
        for (const f of r.files) out(`  ${f.assetId}  ${f.sha256}  ${f.logicalPath}`);
        return ExitCode.Ok;
      }
      case 'normalize': {
        if (!v.asset || !v.id) throw new GateError(ExitCode.Usage, 'normalize needs --asset and --id');
        const r = await normalize({
          work,
          ledger,
          assetId: v.asset,
          logicalId: v.id,
          input: v.input,
          unitScale: v['unit-scale'] !== undefined ? Number(v['unit-scale']) : undefined,
          up: v.up as 'Y' | 'Z' | undefined,
          forward: v.forward as Axis | undefined,
          origin: v.origin as OriginMode | undefined,
        });
        out(`normalize: ${r.output} (${r.mode}, ${r.triangles} tris, bbox ${JSON.stringify(r.bbox)})`);
        for (const w of r.warnings) out(`  warning: ${w}`);
        return ExitCode.Ok;
      }
      case 'process': {
        if (!v.asset || !v.id) throw new GateError(ExitCode.Usage, 'process needs --asset and --id');
        const cls = v.class ? loadAssetClass(resolve(root, v.class)) : {};
        const r = await processMesh({ work, ledger, assetId: v.asset, logicalId: v.id, cls, noCollision: v['no-collision'] });
        for (const c of r.stats.chains) {
          out(`process: chain "${c.suffix || 'base'}": ` + c.lods.map((l) => `LOD${l.lod} ${l.tris} tris ${l.bytes} B`).join(', '));
        }
        if (r.stats.collision) out(`  collision ${r.stats.collision.mode}: ${r.stats.collision.tris} tris, ${r.stats.collision.hulls} piece(s)`);
        for (const t of r.textures) out(`  texture ${t.name}: ${t.slot}/${t.format} ${t.width}x${t.height} ${t.levels} mips ${t.bytes} B`);
        for (const w of r.stats.warnings) out(`  warning: ${w}`);
        return ExitCode.Ok;
      }
      case 'pack': {
        const r = await pack({
          work, ledger, out: resolve(root, v.out ?? 'assets/processed'),
          ...(v.assets ? { assets: v.assets.split(',') } : {}),
          ...(v['max-bytes'] ? { maxBytes: Number(v['max-bytes']) } : {}),
        });
        for (const p of r.packs) out(`pack: ${p.path} ${p.bytes} B, ${p.entries.length} entries (${p.group})`);
        out(`pack: ${r.packs.length} pack(s); index ${r.indexFile}`);
        return ExitCode.Ok;
      }
      case 'audio-pack': {
        if (!v.items) throw new GateError(ExitCode.Usage, 'audio-pack needs --items');
        const items = (JSON.parse(readFileSync(resolve(root, v.items), 'utf8')) as AudioItem[]).map((i) => ({ ...i, src: resolve(root, i.src) }));
        const outDir = resolve(root, v.out ?? 'assets/processed');
        const res = await packAudio(items, { ledger: loadLedgerForAudio(ledger), reportPath: join(work, 'audio-report.json') });
        const dest = join(outDir, publishPath(res.build.sha256).slice(1));
        mkdirSync(dirname(dest), { recursive: true });
        writeFileSync(dest, res.build.bytes);
        await writeProbe(join(outDir, 'probe.json'));
        out(`audio-pack: ${publishPath(res.build.sha256)} ${res.build.bytes.length} B, ${res.report.items.length} item(s) (opus ${res.report.total_opus_bytes} B, aac ${res.report.total_aac_bytes} B)`);
        for (const r of res.report.items) out(`  ${r.logical_id}: ${r.integrated_lufs.toFixed(1)} LUFS (target ${r.target_lufs}), ${r.true_peak_dbtp.toFixed(1)} dBTP, ${r.duration_s} s${r.preload ? ' preload' : ''}`);
        return ExitCode.Ok;
      }
      case 'assert-no-standalone': {
        const dir = resolve(root, v.out ?? 'assets/processed');
        if (existsSync(dir)) assertNoStandalone(dir);
        out(`assert-no-standalone: ${dir} is clean`);
        return ExitCode.Ok;
      }
      case 'check-size-ratio': {
        if (!v.asset) throw new GateError(ExitCode.Usage, 'check-size-ratio needs --asset');
        const s = readStats(statsPath(work, v.asset));
        const ratio = sizeRatio(s, v.suffix as '' | '_fp' | '_tp' | undefined);
        const max = v.max !== undefined ? Number(v.max) : 0.25;
        out(`check-size-ratio: ${v.asset} LOD0 ${(ratio * 100).toFixed(1)} % of source (limit ${(max * 100).toFixed(0)} %)`);
        return ratio <= max ? ExitCode.Ok : ExitCode.NotOk;
      }
      case 'report': {
        const reqPath = resolve(root, v.requirements ?? 'assets/requirements.json');
        const rep = buildReport({
          requirements: reqPath,
          ledger,
          work,
          manifest: v.manifest ? resolve(root, v.manifest) : undefined,
          placeholders: v.placeholders ? resolve(root, v.placeholders) : undefined,
          milestone: v.milestone,
          sourceIndex: v['metadata-only'] ? resolve(root, v['metadata-only']) : undefined,
        });
        if (v.out) {
          mkdirSync(dirname(resolve(root, v.out)), { recursive: true });
          writeFileSync(resolve(root, v.out), JSON.stringify(rep, null, 2) + '\n');
        }
        out(formatReport(rep, { onlyGaps: v.gaps }));
        return v.strict && rep.violations.length > 0 ? ExitCode.NotOk : ExitCode.Ok;
      }
      case 'ledger-check': {
        const l = loadLedger(ledger);
        const counts: Record<string, number> = { OK: 0, PENDING: 0, FLAGGED: 0, REJECTED: 0, INTERNAL: 0 };
        for (const r of l.rows) counts[r.status] = (counts[r.status] ?? 0) + 1;
        out(`ledger-check: ${l.rows.length} row(s): OK ${counts.OK}, PENDING ${counts.PENDING}, FLAGGED ${counts.FLAGGED}, REJECTED ${counts.REJECTED}, INTERNAL ${counts.INTERNAL}`);
        let bad = 0;
        for (const r of l.rows) {
          try {
            assertArchives(r, repoRootOf(ledger));
          } catch (e) {
            bad++;
            out(`  ${(e as Error).message}`);
          }
          if (r.status === 'OK' && isBlank(r.license_name_version)) out(`  warning: ${r.asset_id}: OK row without license_name_version`);
        }
        return bad > 0 ? ExitCode.ArchiveMissing : ExitCode.Ok;
      }
      case 'manifest': {
        const dir = resolve(root, 'assets/manifests');
        const bundlesFile = resolve(root, v.bundles ?? join(dir, 'bundles.json'));
        const m = await buildManifest({
          packs: resolve(root, v.packs ?? 'assets/processed'), work, ledger,
          bundles: existsSync(bundlesFile) ? (JSON.parse(readFileSync(bundlesFile, 'utf8')) as Record<string, string[]>) : {},
          budgets: loadBudgets(resolve(root, v.budgets ?? join(dir, 'budgets.json'))),
          ...(v['game-version'] ? { gameVersion: v['game-version'] } : {}),
          ...(v['data-hash'] ? { dataHash: v['data-hash'] } : {}),
          ...(v.overrides ? { overrides: JSON.parse(readFileSync(resolve(root, v.overrides), 'utf8')) as Overrides } : {}),
        });
        const dst = resolve(root, v.out ?? join(dir, 'manifest.json'));
        writeManifest(dst, m);
        const reqPath = resolve(root, v.requirements ?? 'assets/requirements.json');
        if (existsSync(reqPath)) {
          const rep = buildReport({ requirements: reqPath, ledger, work, manifest: dst });
          writeFileSync(join(dirname(dst), 'missing-report.json'), JSON.stringify(rep, null, 2) + '\n');
        }
        out(`manifest: ${Object.keys(m.assets).length} asset(s) -> ${dst}`);
        return ExitCode.Ok;
      }
      case 'manifest-budget': {
        const dir = resolve(root, 'assets/manifests');
        const m = readManifest(resolve(root, v.manifest ?? join(dir, 'manifest.json')));
        const base = v.base && existsSync(resolve(root, v.base)) ? readManifest(resolve(root, v.base)) : undefined;
        const external: Record<string, number> = {};
        if (v['core-size-file'] && existsSync(resolve(root, v['core-size-file']))) {
          const j = JSON.parse(readFileSync(resolve(root, v['core-size-file']), 'utf8')) as { core_bytes?: number; br_bytes?: number; total_br_bytes?: number };
          const n = j.core_bytes ?? j.br_bytes ?? j.total_br_bytes;
          if (typeof n === 'number') external.core = n;
        }
        const approved = v.approved === true || process.env.BUDGET_APPROVED === '1';
        const r = checkBudgets(m, loadBudgets(resolve(root, v.budgets ?? join(dir, 'budgets.json'))), base, { external, approved });
        out(formatUsages(r));
        const problems = [...r.failures, ...checkPlaceholders(m, ledger)];
        const packs = resolve(root, v.packs ?? 'assets/processed');
        if (Object.keys(m.assets).length > 0) problems.push(...(await checkConsistency(m, packs)));
        for (const p of problems) out(`FAIL ${p}`);
        if (r.skipped.length > 0) out(`note: skipped ${r.skipped.join(', ')} (no size measurement)`);
        return problems.length === 0 ? ExitCode.Ok : ExitCode.Usage;
      }
      case 'gen-requirements': {
        const src = resolve(root, v['shopping-list'] ?? 'docs/ASSET_SHOPPING_LIST.md');
        if (!existsSync(src)) throw new GateError(ExitCode.Usage, `${src}: not found`);
        const f = parseShoppingList(readFileSync(src, 'utf8'), v['shopping-list'] ?? 'docs/ASSET_SHOPPING_LIST.md');
        const dst = resolve(root, v.out ?? 'assets/requirements.json');
        mkdirSync(dirname(dst), { recursive: true });
        writeFileSync(dst, JSON.stringify(f, null, 2) + '\n');
        out(`gen-requirements: ${f.requirements.length} requirement id(s) -> ${dst}`);
        return ExitCode.Ok;
      }
      default:
        out(USAGE);
        return ExitCode.Usage;
    }
  } catch (e) {
    if (e instanceof GateError) {
      console.error(`asset-pipeline: ${e.message}`);
      return e.code;
    }
    console.error(`asset-pipeline: ${(e as Error).stack ?? e}`);
    return ExitCode.Usage;
  }
}
