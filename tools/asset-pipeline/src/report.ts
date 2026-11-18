/**
 * Missing-asset report (ASSET_GAPS §3.4): per requirement ID `real | placeholder | missing | internal-only`, summed per milestone.
 *
 * Inputs
 *  - `assets/requirements.json`  {schema:1, requirements:[{id, milestone, kind}]}  (generated from ASSET_SHOPPING_LIST §4 by
 *    `gen-requirements`; minimal version, see PI-032)
 *  - the ledger (LICENSES.md §3)
 *  - the work dir (`work/{asset_id}/normalized.gltf` marks an asset as processed)
 *  - optional free source index (alternative to work/manifest/placeholders): metadata-only coverage decisions
 *  - optional manifest draft: {assets:{<logical id>:{req_id, placeholder}}} or {entries:[{req_id, placeholder}]}
 *  - optional placeholder data dir: any JSON under it with a `requirement_id` string counts as a grey-box placeholder
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { isBlank, loadLedger, type Ledger, type LedgerRow } from './ledger.ts';
import { checkSourceIndex } from './source-index.ts';
import { ExitCode, GateError } from './errors.ts';

export type ReqStatus = 'real' | 'placeholder' | 'missing' | 'internal-only';

export interface Requirement {
  id: string;
  milestone: string;
  kind: 'visual' | 'audio' | 'other';
}

export interface RequirementsFile {
  schema: 1;
  source: string;
  requirements: Requirement[];
}

export interface ReportRow {
  id: string;
  milestone: string;
  status: ReqStatus;
  detail: string;
  assets: string[];
}

export interface Counts {
  real: number;
  placeholder: number;
  missing: number;
  internal_only: number;
  total: number;
}

export interface Report {
  schema: 1;
  milestone: string | 'all';
  coverage_basis?: 'source-metadata';
  summary: Counts;
  by_milestone: Record<string, Counts>;
  requirements: ReportRow[];
  /** ASSET_GAPS §3.4.2 violations: manifest says real but no OK ledger row backs it. */
  violations: string[];
  /** requirement ids seen in ledger / manifest / placeholders but absent from requirements.json */
  unknown_req_ids: string[];
}

const REQ_ID = /^\|\s*([A-Z]-[A-Z0-9]+(?:-[A-Z0-9]+)*)/;

/** Build the minimal requirements map from the ASSET_SHOPPING_LIST §4.x tables (first column, milestone from the heading). */
export function parseShoppingList(text: string, source = 'docs/ASSET_SHOPPING_LIST.md'): RequirementsFile {
  const reqs = new Map<string, Requirement>();
  let milestone: string | null = null;
  for (const line of text.split(/\r?\n/)) {
    const h = /^###\s+4\.\d+\s+(M\d)\b/.exec(line);
    if (h) {
      milestone = h[1] as string;
      continue;
    }
    if (/^##\s/.test(line)) milestone = null;
    if (!milestone) continue;
    const m = REQ_ID.exec(line);
    if (!m) continue;
    const id = m[1] as string;
    if (reqs.has(id)) continue; // keep the earliest milestone
    reqs.set(id, { id, milestone, kind: id.startsWith('V-') ? 'visual' : /^[A-J]-/.test(id) ? 'audio' : 'other' });
  }
  return { schema: 1, source, requirements: [...reqs.values()] };
}

export function loadRequirements(path: string): RequirementsFile {
  let data: unknown;
  try {
    data = JSON.parse(readFileSync(path, 'utf8'));
  } catch (e) {
    throw new GateError(ExitCode.Usage, `${path}: cannot read requirements (${(e as Error).message}); generate it with "asset-pipeline gen-requirements"`, { file: path });
  }
  const f = data as Partial<RequirementsFile>;
  if (!f || f.schema !== 1 || !Array.isArray(f.requirements)) throw new GateError(ExitCode.Usage, `${path}: not a requirements.json (schema 1)`, { file: path });
  return f as RequirementsFile;
}

function jsonFiles(dir: string, out: string[] = []): string[] {
  if (!existsSync(dir)) return out;
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) jsonFiles(p, out);
    else if (e.name.endsWith('.json')) out.push(p);
  }
  return out.sort();
}

function collectRequirementIds(v: unknown, out: Set<string>): void {
  if (Array.isArray(v)) v.forEach((x) => collectRequirementIds(x, out));
  else if (v && typeof v === 'object') {
    for (const [k, x] of Object.entries(v)) {
      if (k === 'requirement_id' && typeof x === 'string') out.add(x);
      else collectRequirementIds(x, out);
    }
  }
}

interface ManifestEntry {
  req_id: string;
  placeholder: boolean;
}

function loadManifestDraft(path: string): ManifestEntry[] {
  const j = JSON.parse(readFileSync(path, 'utf8')) as { assets?: Record<string, { req_id?: unknown; placeholder?: unknown }>; entries?: { req_id?: unknown; placeholder?: unknown }[] };
  const raw = [...Object.values(j.assets ?? {}), ...(j.entries ?? [])];
  const out: ManifestEntry[] = [];
  for (const e of raw) if (typeof e.req_id === 'string') out.push({ req_id: e.req_id, placeholder: e.placeholder !== false });
  return out;
}

export interface ReportOptions {
  requirements: string | RequirementsFile;
  ledger: string | Ledger;
  work: string;
  manifest?: string;
  placeholders?: string;
  milestone?: string;
  /** Opt into source availability instead of processed/build coverage. */
  sourceIndex?: string;
}

function requirementIds(row: LedgerRow): string[] {
  if (isBlank(row.req_id)) return [];
  return row.status === 'INTERNAL' ? row.req_id.split(',').map((id) => id.trim()) : [row.req_id];
}

export function buildReport(opts: ReportOptions): Report {
  const reqFile = typeof opts.requirements === 'string' ? loadRequirements(opts.requirements) : opts.requirements;
  const ledger = typeof opts.ledger === 'string' ? loadLedger(opts.ledger) : opts.ledger;
  if (opts.sourceIndex && (opts.manifest || opts.placeholders)) {
    throw new GateError(ExitCode.Usage, 'metadata-only reporting cannot be combined with a build manifest or placeholder directory');
  }
  const source = opts.sourceIndex ? checkSourceIndex(opts.sourceIndex, ledger) : undefined;
  const manifest = opts.manifest ? loadManifestDraft(opts.manifest) : [];
  const phIds = new Set<string>();
  if (opts.placeholders) for (const f of jsonFiles(opts.placeholders)) collectRequirementIds(JSON.parse(readFileSync(f, 'utf8')), phIds);

  const known = new Set(reqFile.requirements.map((r) => r.id));
  if (source) {
    for (const id of source.coverage.keys()) {
      if (!known.has(id)) throw new GateError(ExitCode.Usage, `${id}: coverage is absent from requirements.json`);
    }
  }
  const violations: string[] = [];
  const rows: ReportRow[] = [];
  for (const r of reqFile.requirements) {
    const candidates = source?.candidates.get(r.id);
    const rowsOfReq = ledger.rows.filter((l) => requirementIds(l).includes(r.id) || candidates?.has(l.asset_id));
    const internal = rowsOfReq.some((l) => l.status === 'INTERNAL');
    const sourceStatus = source?.coverage.get(r.id);
    const okRows = rowsOfReq.filter((l) => l.status === 'OK');
    const processed = source ? [] : okRows.filter((l) => existsSync(join(opts.work, l.asset_id, 'normalized.gltf')));
    const mEntries = manifest.filter((m) => m.req_id === r.id);
    const manifestReal = mEntries.some((m) => !m.placeholder);
    const manifestPh = mEntries.some((m) => m.placeholder);

    let status: ReqStatus;
    let detail: string;
    if (sourceStatus === 'real') {
      status = sourceStatus;
      detail = 'matching free source registered (metadata only; not built)';
    } else if (processed.length > 0 || (manifestReal && okRows.length > 0)) {
      status = 'real';
      detail = processed.length > 0 ? 'ledger OK, normalized' : 'ledger OK, manifest placeholder:false';
    } else if (internal) {
      status = 'internal-only';
      detail = sourceStatus === 'placeholder' || manifestPh || phIds.has(r.id)
        ? 'internal playtest only; public placeholder remains'
        : 'internal playtest only; excluded from public coverage';
    } else if (sourceStatus === 'placeholder' || manifestPh || phIds.has(r.id)) {
      status = 'placeholder';
      const statuses = [...new Set(rowsOfReq.map((l) => l.status))];
      detail = sourceStatus === 'placeholder' ? 'free proxy candidate' : statuses.length > 0 ? `grey-box; ledger ${statuses.join('/')}, not processed` : 'grey-box';
    } else {
      status = 'missing';
      detail = rowsOfReq.length === 0 ? 'no input' : source ? 'ledger candidates; no matching free source or chosen proxy'
        : `ledger ${[...new Set(rowsOfReq.map((l) => l.status))].join('/')}, not processed`;
    }
    if (manifestReal && okRows.length === 0) {
      violations.push(`${r.id}: manifest marks it real (placeholder:false) but no ledger row with status OK backs it (ASSET_GAPS §3.4 item 2)`);
    }
    rows.push({ id: r.id, milestone: r.milestone, status, detail, assets: rowsOfReq.map((l) => l.asset_id) });
  }

  const unknown = new Set<string>();
  for (const l of ledger.rows) for (const id of requirementIds(l)) if (/^[A-Z]-/.test(id) && !known.has(id)) unknown.add(id);
  for (const m of manifest) if (!known.has(m.req_id)) unknown.add(m.req_id);
  for (const id of phIds) if (!known.has(id)) unknown.add(id);

  const scoped = opts.milestone ? rows.filter((r) => r.milestone === opts.milestone) : rows;
  const count = (rs: ReportRow[]): Counts => ({
    real: rs.filter((r) => r.status === 'real').length,
    placeholder: rs.filter((r) => r.status === 'placeholder').length,
    missing: rs.filter((r) => r.status === 'missing').length,
    internal_only: rs.filter((r) => r.status === 'internal-only').length,
    total: rs.length,
  });
  const by: Record<string, Counts> = {};
  for (const ms of [...new Set(scoped.map((r) => r.milestone))].sort()) by[ms] = count(scoped.filter((r) => r.milestone === ms));
  return {
    schema: 1,
    milestone: opts.milestone ?? 'all',
    ...(source ? { coverage_basis: 'source-metadata' as const } : {}),
    summary: count(scoped),
    by_milestone: by,
    requirements: scoped,
    violations,
    unknown_req_ids: [...unknown].sort(),
  };
}

/** Console rendering of a report. */
export function formatReport(rep: Report, opts: { onlyGaps?: boolean } = {}): string {
  const lines: string[] = [];
  if (rep.coverage_basis) lines.push('Coverage basis: source metadata (no build integration)');
  const rows = opts.onlyGaps ? rep.requirements.filter((r) => r.status !== 'real') : rep.requirements;
  const w = Math.max(14, ...rows.map((r) => r.id.length));
  lines.push(`${'requirement'.padEnd(w)}  ms  ${'status'.padEnd(13)}  detail`);
  for (const r of rows) lines.push(`${r.id.padEnd(w)}  ${r.milestone.padEnd(2)}  ${r.status.padEnd(13)}  ${r.detail}`);
  lines.push('');
  lines.push(`milestone   real  placeholder  missing  internal-only  total`);
  for (const [ms, c] of Object.entries(rep.by_milestone)) {
    lines.push(`${ms.padEnd(10)}  ${String(c.real).padStart(4)}  ${String(c.placeholder).padStart(11)}  ${String(c.missing).padStart(7)}  ${String(c.internal_only).padStart(13)}  ${String(c.total).padStart(5)}`);
  }
  const c = rep.summary;
  lines.push(`${'ALL'.padEnd(10)}  ${String(c.real).padStart(4)}  ${String(c.placeholder).padStart(11)}  ${String(c.missing).padStart(7)}  ${String(c.internal_only).padStart(13)}  ${String(c.total).padStart(5)}`);
  for (const v of rep.violations) lines.push(`VIOLATION: ${v}`);
  if (rep.unknown_req_ids.length) lines.push(`requirement ids not in requirements.json: ${rep.unknown_req_ids.join(', ')}`);
  return lines.join('\n');
}
