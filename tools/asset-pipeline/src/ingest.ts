import { copyFileSync, mkdirSync, openSync, readSync, closeSync, readdirSync, existsSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { basename, dirname, join, relative, resolve, sep } from 'node:path';
import { ExitCode, GateError } from './errors.ts';
import { checkRow, repoRootOf } from './gate.ts';
import { setDefaultLedger } from './allowed.ts';
import { loadLedger, lookupByRawPath, type Ledger, type LedgerRow } from './ledger.ts';

/** Logical prefix every incoming file is addressed by in the ledger (`raw_path`). */
export const INCOMING_PREFIX = 'assets/incoming';

/** Files that are scaffolding rather than assets. */
const IGNORED_NAMES = new Set(['.gitkeep', '.gitignore', '.DS_Store', 'Thumbs.db']);

export interface IngestOptions {
  /** Directory holding the incoming files (normally `<repo>/assets/incoming`). */
  incoming: string;
  /** Path of LICENSES.md. */
  ledger: string;
  /** Work directory (`work/{asset_id}/raw/` is created under it). */
  work: string;
  /** Root that ledger archive paths resolve against; defaults to the directory of the ledger file. */
  repoRoot?: string;
}

export interface IngestedFile {
  assetId: string;
  reqId: string;
  /** Ledger-style path, `assets/incoming/...`. */
  logicalPath: string;
  /** Destination under `work/{asset_id}/raw/`. */
  dest: string;
  sha256: string;
  bytes: number;
}

export interface IngestResult {
  files: IngestedFile[];
  /** Asset ids that had at least one file. */
  assets: string[];
  /** Number of files seen under incoming/ (equals files.length on success; the gate is all-or-nothing). */
  scanned: number;
}

export function sha256File(path: string): { sha256: string; bytes: number } {
  const h = createHash('sha256');
  const fd = openSync(path, 'r');
  const buf = Buffer.allocUnsafe(1 << 20);
  let bytes = 0;
  try {
    for (;;) {
      const n = readSync(fd, buf, 0, buf.length, null);
      if (n === 0) break;
      h.update(buf.subarray(0, n));
      bytes += n;
    }
  } finally {
    closeSync(fd);
  }
  return { sha256: h.digest('hex'), bytes };
}

function walk(dir: string, out: string[]): void {
  for (const e of readdirSync(dir, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))) {
    const p = join(dir, e.name);
    if (e.isSymbolicLink()) {
      throw new GateError(ExitCode.Usage, `${p}: symbolic links are not allowed under incoming/`, { file: p });
    }
    if (e.isDirectory()) walk(p, out);
    else if (e.isFile() && !IGNORED_NAMES.has(e.name)) out.push(p);
  }
}

/**
 * The license gate (ARCHITECTURE §11.1 step 1, §11.7; LICENSES §1).
 * Every file under `incoming` must (2) be claimed by a ledger row, (3) whose status is OK, (4) with a seller
 * confirmation if the channel is a D-LIC one, (5) with the archive files it names present. The check covers ALL files
 * before anything is copied, so a refusal leaves `work/` untouched. Throws GateError{code, file}.
 */
export function ingest(opts: IngestOptions): IngestResult {
  const ledger: Ledger = loadLedger(opts.ledger);
  setDefaultLedger(ledger);
  const repoRoot = opts.repoRoot ?? repoRootOf(opts.ledger);
  const incoming = resolve(opts.incoming);

  const abs: string[] = [];
  if (existsSync(incoming)) walk(incoming, abs);

  const plan: { row: LedgerRow; src: string; logical: string }[] = [];
  const checked = new Set<string>();
  for (const src of abs) {
    const logical = `${INCOMING_PREFIX}/${relative(incoming, src).split(sep).join('/')}`;
    const row = lookupByRawPath(ledger, logical);
    if (!row) {
      throw new GateError(ExitCode.Unregistered, `${logical}: not registered in LICENSES.md section 3 (no row's raw_path covers it); refusing to process (LICENSES §1 rule 1)`, {
        file: src,
      });
    }
    if (!checked.has(row.asset_id)) {
      try {
        checkRow(row, repoRoot, logical);
      } catch (e) {
        if (e instanceof GateError) throw new GateError(e.code, e.message, { file: src, assetId: row.asset_id });
        throw e;
      }
      checked.add(row.asset_id);
    }
    plan.push({ row, src, logical });
  }

  const files: IngestedFile[] = [];
  const work = resolve(opts.work);
  for (const { row, src, logical } of plan) {
    const inner = logical === row.raw_path ? basename(logical) : logical.slice(row.raw_path.length + 1);
    const dest = join(work, row.asset_id, 'raw', ...inner.split('/'));
    const { sha256, bytes } = sha256File(src);
    mkdirSync(dirname(dest), { recursive: true });
    copyFileSync(src, dest);
    const copied = sha256File(dest);
    if (copied.sha256 !== sha256) {
      throw new GateError(ExitCode.Usage, `${dest}: copy verification failed (sha256 differs from source)`, { file: dest, assetId: row.asset_id });
    }
    files.push({ assetId: row.asset_id, reqId: row.req_id, logicalPath: logical, dest, sha256, bytes });
  }

  const assets = [...new Set(files.map((f) => f.assetId))];
  for (const id of assets) {
    const mine = files.filter((f) => f.assetId === id);
    const row = ledger.byAssetId.get(id) as LedgerRow;
    writeFileSync(
      join(work, id, 'ingest.json'),
      JSON.stringify(
        {
          asset_id: id,
          req_id: row.req_id,
          files: mine.map((f) => ({ path: f.logicalPath, sha256: f.sha256, bytes: f.bytes })),
        },
        null,
        2,
      ) + '\n',
    );
  }
  return { files, assets, scanned: abs.length };
}
