import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { ExitCode, GateError } from './errors.ts';
import { ingest } from './ingest.ts';
import { loadLedger, isBlank } from './ledger.ts';
import type { Axis } from './math.ts';
import { assertArchives, repoRootOf } from './gate.ts';
import { normalize, type OriginMode } from './normalize.ts';
import { loadAssetClass } from './asset-class.ts';
import { processMesh } from './process.ts';
import { pack } from './pack.ts';
import { assertNoStandalone } from './fpak/standalone.ts';
import { readStats, sizeRatio, statsPath } from './stats.ts';
import { buildReport, formatReport, parseShoppingList } from './report.ts';

const USAGE = `asset-pipeline <command> [options]

  ingest            gate + copy assets/incoming/** to work/{asset_id}/raw/ (exit 2 unregistered, 3 not OK, 4 D-LIC, 5 archive missing)
  normalize         unit/axes/origin/triangulate/merge one ingested glTF -> work/{asset_id}/normalized.gltf
  process            LOD chains, KTX2 textures, mesh compression, collision -> work/{asset_id}/lodN.glb, stats.json (M1-002)
  pack              work/ intermediates -> assets/processed/a/xx/yyyy.fpak + packs.json (M1-003)
  assert-no-standalone  fail when the output directory holds .glb/.ktx2/.ogg/... files
  check-size-ratio  LOD0 bytes / source bytes from stats.json (fails above --max, default 0.25)
  report            per requirement ID real/placeholder/missing (ASSET_GAPS 3.4)
  ledger-check      LICENSES.md 4.4 validation: counts per status, missing archive files
  gen-requirements  build assets/requirements.json from docs/ASSET_SHOPPING_LIST.md

common:  --ledger LICENSES.md   --work work   --root <repo root, default cwd>
ingest:  --incoming assets/incoming
normalize: --asset AST-... --id <logical_id> [--input rel/in/raw] [--unit-scale m] [--up Y|Z] [--forward +Z|-Y|...] [--origin bottom-center|center|none]
process: --asset AST-... --id <logical_id> --class asset-class.json [--no-collision]
pack:    [--assets AST-a,AST-b] [--out assets/processed] [--max-bytes 16777216]
assert-no-standalone: [--out assets/processed]
check-size-ratio: --asset AST-... [--max 0.25] [--suffix _fp]
report:  [--requirements assets/requirements.json] [--manifest draft.json] [--placeholders data/placeholders] [--milestone M1] [--out report.json] [--gaps] [--strict]
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
      manifest: { type: 'string' },
      placeholders: { type: 'string' },
      milestone: { type: 'string' },
      out: { type: 'string' },
      'shopping-list': { type: 'string' },
      assets: { type: 'string' },
      'max-bytes': { type: 'string' },
      class: { type: 'string' },
      max: { type: 'string' },
      suffix: { type: 'string' },
      'no-collision': { type: 'boolean' },
      gaps: { type: 'boolean' },
      strict: { type: 'boolean' },
    },
  });
  const root = resolve(v.root ?? process.cwd());
  const ledger = resolve(root, v.ledger ?? 'LICENSES.md');
  const work = resolve(root, v.work ?? 'work');
  try {
    switch (cmd) {
      case 'ingest': {
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
        const counts: Record<string, number> = { OK: 0, PENDING: 0, FLAGGED: 0, REJECTED: 0 };
        for (const r of l.rows) counts[r.status] = (counts[r.status] ?? 0) + 1;
        out(`ledger-check: ${l.rows.length} row(s): OK ${counts.OK}, PENDING ${counts.PENDING}, FLAGGED ${counts.FLAGGED}, REJECTED ${counts.REJECTED}`);
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
