import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const FIXTURES = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');

export const COLUMNS = [
  'asset_id', 'req_id', 'name', 'type', 'source_channel', 'author_vendor', 'url', 'license_name_version', 'license_text_path',
  'order_id', 'purchase_date', 'confirmation_path', 'raw_path', 'allowed_processing', 'status', 'notes',
] as const;

export type RowInput = Partial<Record<(typeof COLUMNS)[number], string>> & { asset_id: string };

export function rowText(r: RowInput): string {
  const base: Record<string, string> = {
    req_id: 'V-TEST-001', name: 'Test asset', type: 'model', source_channel: 'Poly Haven', author_vendor: 'tester', url: 'https://example.org/x',
    license_name_version: 'CC0 1.0', license_text_path: '—', order_id: 'FREE-2026-11-17', purchase_date: '2026-11-17', confirmation_path: '—',
    raw_path: `assets/incoming/V-TEST-001/PolyHaven-FREE/`, allowed_processing: 'CONVERT', status: 'OK', notes: 'test',
  };
  const all = { ...base, ...r };
  return '| ' + COLUMNS.map((c) => all[c] ?? '—').join(' | ') + ' |';
}

export function ledgerText(rows: (RowInput | string)[], opts: { columns?: readonly string[]; extraSectionAfter?: boolean } = {}): string {
  const cols = opts.columns ?? COLUMNS;
  const head = `| ${cols.join(' | ')} |\n|${cols.map(() => '---').join('|')}|`;
  const body = rows.map((r) => (typeof r === 'string' ? r : rowText(r))).join('\n');
  return `# LICENSES.md\n\n## 2. fields\n\n| a | b |\n|---|---|\n| x | y |\n\n## 3. 台账\n\ntext\n\n${head}\n${body}\n\n## 4. next\n\n| asset_id | zzz |\n|---|---|\n| nope | nope |\n`;
}

export interface TmpRepo {
  root: string;
  ledger: string;
  incoming: string;
  work: string;
  put(rel: string, content: string | Buffer): string;
  putFixture(name: string, rel: string): string;
  cleanup(): void;
}

/** A throw-away repository root with LICENSES.md, assets/incoming/ and work/. */
export function tmpRepo(text: string): TmpRepo {
  const root = mkdtempSync(join(tmpdir(), 'fl-ap-'));
  const ledger = join(root, 'LICENSES.md');
  writeFileSync(ledger, text);
  const incoming = join(root, 'assets', 'incoming');
  mkdirSync(incoming, { recursive: true });
  return {
    root,
    ledger,
    incoming,
    work: join(root, 'work'),
    put(rel, content) {
      const p = join(root, rel);
      mkdirSync(dirname(p), { recursive: true });
      writeFileSync(p, content);
      return p;
    },
    putFixture(name, rel) {
      const p = join(root, rel);
      mkdirSync(dirname(p), { recursive: true });
      copyFileSync(join(FIXTURES, name), p);
      return p;
    },
    cleanup() {
      rmSync(root, { recursive: true, force: true });
    },
  };
}
