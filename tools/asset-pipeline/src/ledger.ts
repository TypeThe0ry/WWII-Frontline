import { readFileSync } from 'node:fs';
import { ExitCode, GateError } from './errors.ts';

/** Ledger statuses from LICENSES §1 and §6.2. Only OK is build-eligible. */
export const STATUSES = ['OK', 'PENDING', 'REJECTED', 'FLAGGED', 'INTERNAL'] as const;
export type LedgerStatus = (typeof STATUSES)[number];

/** D-MOD processing codes (LICENSES §2 `allowed_processing`). */
export const VISUAL_PROCESSING_CODES = ['CONVERT', 'LOD', 'TEXCOMP', 'RETARGET', 'REBIND', 'MATPARAM', 'ASSEMBLE'] as const;
export const AUDIO_PROCESSING_CODES = ['TRANSCODE', 'LOUDNORM', 'EDIT', 'LAYERMIX', 'EQCOMP', 'CONVOLVE'] as const;
/** SUBSET / RENAME: font processing codes added by PI-009. */
export const FONT_PROCESSING_CODES = ['SUBSET', 'RENAME'] as const;
export const PROCESSING_CODES = [...VISUAL_PROCESSING_CODES, ...AUDIO_PROCESSING_CODES, ...FONT_PROCESSING_CODES] as const;
export type ProcessingCode = (typeof PROCESSING_CODES)[number];

/** Columns the gate cannot work without. Others (LICENSES §2) are optional in the table (LICENSES §6.1). */
export const REQUIRED_COLUMNS = [
  'asset_id',
  'req_id',
  'type',
  'source_channel',
  'license_name_version',
  'license_text_path',
  'order_id',
  'confirmation_path',
  'raw_path',
  'allowed_processing',
  'status',
] as const;

export interface LedgerRow {
  asset_id: string;
  req_id: string;
  type: string;
  status: LedgerStatus;
  source_channel: string;
  license_name_version: string;
  license_text_path: string;
  order_id: string;
  confirmation_path: string;
  /** Normalised: posix separators, no leading "./"; may be a directory prefix or a file. Empty if unusable (e.g. "—"). */
  raw_path: string;
  /**
   * Parsed `allowed_processing`. `null` = column empty or "—": LICENSES §6.5 makes it optional for free assets, so
   * every D-MOD code is permitted (explicit lists always restrict).
   */
  allowed_processing: ProcessingCode[] | null;
  /** 1-based line of the row in LICENSES.md. */
  line: number;
  /** Every column of the row by header name (including the ones above, raw text). */
  fields: Readonly<Record<string, string>>;
}

export interface Ledger {
  path: string;
  rows: readonly LedgerRow[];
  byAssetId: ReadonlyMap<string, LedgerRow>;
}

const DASHES = new Set(['', '—', '-', '–', '--', 'n/a', 'N/A']);

/** True for the "no value" placeholders used in the ledger table. */
export function isBlank(v: string): boolean {
  return DASHES.has(v.trim());
}

function splitRow(line: string): string[] {
  const cells: string[] = [];
  let cur = '';
  for (let i = 0; i < line.length; i++) {
    const ch = line[i] as string;
    if (ch === '\\' && line[i + 1] === '|') {
      cur += '|';
      i++;
    } else if (ch === '|') {
      cells.push(cur);
      cur = '';
    } else cur += ch;
  }
  cells.push(cur);
  // A well-formed row starts and ends with a pipe: drop the empty edge cells.
  if (cells.length > 0 && (cells[0] as string).trim() === '') cells.shift();
  if (cells.length > 0 && (cells[cells.length - 1] as string).trim() === '') cells.pop();
  return cells.map((c) => c.trim());
}

function fail(path: string, line: number, msg: string): never {
  throw new GateError(ExitCode.LedgerInvalid, `${path}:${line}: ${msg}`, { file: path });
}

/** Normalise a repo-relative path from the ledger: backslashes, "./", duplicate slashes, trailing slash kept out. */
export function normalizeLedgerPath(p: string): string {
  let s = p.trim().replace(/\\/g, '/').replace(/\/{2,}/g, '/');
  while (s.startsWith('./')) s = s.slice(2);
  return s.replace(/\/+$/, '');
}

/** Extract the raw_path value: the ledger sometimes appends a prose note after a full-width bracket ("—（未入库…）"). */
function parseRawPath(v: string): string {
  const s = v.trim();
  if (!s.startsWith('assets/')) return '';
  // Cut free-text notes such as "assets/incoming/x/（说明）".
  const cut = s.search(/[（(\s]/);
  return normalizeLedgerPath(cut === -1 ? s : s.slice(0, cut));
}

function parseAllowed(v: string, path: string, line: number): ProcessingCode[] | null {
  if (isBlank(v)) return null;
  const out: ProcessingCode[] = [];
  for (const tok of v.split(/[,\s/;]+/).filter(Boolean)) {
    const code = tok.toUpperCase();
    if (!(PROCESSING_CODES as readonly string[]).includes(code)) {
      fail(path, line, `allowed_processing: unknown code "${tok}" (known: ${PROCESSING_CODES.join('/')})`);
    }
    if (!out.includes(code as ProcessingCode)) out.push(code as ProcessingCode);
  }
  return out;
}

/** Parse the section-3 table of a LICENSES.md text. Throws GateError(LedgerInvalid) with `path:line` on any defect. */
export function parseLedger(text: string, path = 'LICENSES.md'): Ledger {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex((l) => /^##\s+3[.\s]/.test(l));
  if (start === -1) fail(path, 1, 'section "## 3." (ledger) not found');

  // First table in section 3 whose header has an asset_id column.
  let header: string[] | null = null;
  let headerLine = 0;
  const rows: { cells: string[]; line: number }[] = [];
  for (let i = start + 1; i < lines.length; i++) {
    const l = lines[i] as string;
    if (/^##\s/.test(l)) break;
    if (!l.trimStart().startsWith('|')) {
      if (header) break; // a non-table line ends the table
      continue;
    }
    const cells = splitRow(l.trim());
    if (!header) {
      if (cells.includes('asset_id')) {
        header = cells;
        headerLine = i + 1;
      }
      continue;
    }
    if (cells.every((c) => /^:?-{3,}:?$/.test(c))) continue; // separator row
    rows.push({ cells, line: i + 1 });
  }
  if (!header) fail(path, start + 1, 'ledger table header (a row containing "asset_id") not found in section 3');

  const col = new Map<string, number>();
  header.forEach((h, idx) => {
    if (col.has(h)) fail(path, headerLine, `duplicate column "${h}"`);
    col.set(h, idx);
  });
  for (const c of REQUIRED_COLUMNS) {
    if (!col.has(c)) fail(path, headerLine, `missing required column "${c}"`);
  }

  const out: LedgerRow[] = [];
  const seen = new Map<string, number>();
  const seenRaw = new Map<string, number>();
  for (const { cells, line } of rows) {
    const idCell = cells[col.get('asset_id') as number] ?? '';
    if (idCell.startsWith('AST-EXAMPLE-')) continue; // LICENSES §2: example rows are ignored
    if (cells.length !== header.length) {
      fail(path, line, `row has ${cells.length} cells but the header has ${header.length} (a "|" inside a cell?)`);
    }
    const fields: Record<string, string> = {};
    header.forEach((h, idx) => (fields[h] = cells[idx] as string));
    const get = (k: string): string => fields[k] as string;

    const assetId = get('asset_id');
    if (!/^AST-[A-Za-z0-9][A-Za-z0-9-]*$/.test(assetId)) fail(path, line, `asset_id "${assetId}" is not of the form AST-{req}-{NN}`);
    const first = seen.get(assetId);
    if (first !== undefined) fail(path, line, `duplicate asset_id "${assetId}" (first at line ${first})`);
    seen.set(assetId, line);

    const status = get('status');
    if (!(STATUSES as readonly string[]).includes(status)) {
      fail(path, line, `status "${status}" is not one of ${STATUSES.join('/')}`);
    }

    const rawPath = parseRawPath(get('raw_path'));
    if (rawPath !== '') {
      const dup = seenRaw.get(rawPath);
      if (dup !== undefined) fail(path, line, `raw_path "${rawPath}" is already claimed by the row at line ${dup}`);
      seenRaw.set(rawPath, line);
    }

    out.push({
      asset_id: assetId,
      req_id: get('req_id'),
      type: get('type'),
      status: status as LedgerStatus,
      source_channel: get('source_channel'),
      license_name_version: get('license_name_version'),
      license_text_path: get('license_text_path'),
      order_id: get('order_id'),
      confirmation_path: get('confirmation_path'),
      raw_path: rawPath,
      allowed_processing: parseAllowed(get('allowed_processing'), path, line),
      line,
      fields,
    });
  }
  return { path, rows: out, byAssetId: new Map(out.map((r) => [r.asset_id, r])) };
}

/** Read and parse a LICENSES.md file. */
export function loadLedger(path: string): Ledger {
  let text: string;
  try {
    text = readFileSync(path, 'utf8');
  } catch (e) {
    throw new GateError(ExitCode.LedgerInvalid, `${path}: cannot read ledger (${(e as Error).message})`, { file: path });
  }
  return parseLedger(text, path);
}

/**
 * Find the ledger row owning a logical raw path (`assets/incoming/...`, posix). The row's raw_path must be a whole-segment
 * prefix of the path (or equal to it); the longest prefix wins. Returns undefined if no row claims the path.
 */
export function lookupByRawPath(ledger: Ledger, rawPath: string): LedgerRow | undefined {
  const p = normalizeLedgerPath(rawPath);
  let best: LedgerRow | undefined;
  for (const row of ledger.rows) {
    const rp = row.raw_path;
    if (rp === '' || !(p === rp || p.startsWith(rp + '/'))) continue;
    if (!best || rp.length > best.raw_path.length) best = row;
  }
  return best;
}
