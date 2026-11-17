import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { ExitCode, GateError, loadLedger, lookupByRawPath, parseLedger } from '../src/index.ts';
import { COLUMNS, ledgerText, rowText } from './helpers.ts';

function err(fn: () => unknown): GateError {
  try {
    fn();
  } catch (e) {
    if (e instanceof GateError) return e;
    throw e;
  }
  throw new Error('expected GateError');
}

describe('ledger.parse', () => {
  it('parses rows by header name and ignores other tables / AST-EXAMPLE rows', () => {
    const l = parseLedger(ledgerText([{ asset_id: 'AST-V-TEST-001-01' }, { asset_id: 'AST-EXAMPLE-01', status: 'bogus' }]));
    expect(l.rows).toHaveLength(1);
    const r = l.rows[0]!;
    expect(r).toMatchObject({ asset_id: 'AST-V-TEST-001-01', req_id: 'V-TEST-001', status: 'OK', allowed_processing: ['CONVERT'] });
    expect(r.raw_path).toBe('assets/incoming/V-TEST-001/PolyHaven-FREE');
    expect(l.byAssetId.get('AST-V-TEST-001-01')).toBe(r);
  });

  it('does not depend on column order', () => {
    const cols = [...COLUMNS].reverse();
    const row = rowText({ asset_id: 'AST-V-TEST-001-01' })
      .split('|')
      .slice(1, -1)
      .map((c) => c.trim());
    const byName = Object.fromEntries(COLUMNS.map((c, i) => [c, row[i]]));
    const text = ledgerText([`| ${cols.map((c) => byName[c]).join(' | ')} |`], { columns: cols });
    expect(parseLedger(text).rows[0]).toMatchObject({ asset_id: 'AST-V-TEST-001-01', status: 'OK' });
  });

  it('reports a missing required column with the header line', () => {
    const cols = COLUMNS.filter((c) => c !== 'raw_path');
    const e = err(() => parseLedger(ledgerText([], { columns: cols }), 'L.md'));
    expect(e.code).toBe(ExitCode.LedgerInvalid);
    expect(e.message).toMatch(/L\.md:\d+: missing required column "raw_path"/);
  });

  it('reports duplicate asset_id with line numbers', () => {
    const text = ledgerText([{ asset_id: 'AST-V-A-01' }, { asset_id: 'AST-V-A-01', raw_path: 'assets/incoming/other/' }]);
    const e = err(() => parseLedger(text, 'L.md'));
    expect(e.code).toBe(ExitCode.LedgerInvalid);
    const lines = text.split('\n');
    const second = lines.findIndex((l, i) => l.includes('AST-V-A-01') && i > lines.findIndex((x) => x.includes('AST-V-A-01')));
    expect(e.message).toContain(`L.md:${second + 1}: duplicate asset_id "AST-V-A-01"`);
  });

  it('rejects statuses outside the enum but accepts FLAGGED (LICENSES §6.2)', () => {
    const e = err(() => parseLedger(ledgerText([{ asset_id: 'AST-V-A-01', status: 'MAYBE' }]), 'L.md'));
    expect(e.message).toMatch(/L\.md:\d+: status "MAYBE"/);
    expect(parseLedger(ledgerText([{ asset_id: 'AST-V-A-01', status: 'FLAGGED' }])).rows[0]!.status).toBe('FLAGGED');
  });

  it('rejects a stray pipe inside a cell (cell count mismatch) and accepts an escaped pipe', () => {
    const bad = rowText({ asset_id: 'AST-V-A-01', notes: 'a | b' });
    const e = err(() => parseLedger(ledgerText([bad]), 'L.md'));
    expect(e.message).toMatch(/L\.md:\d+: row has \d+ cells but the header has \d+/);
    const ok = parseLedger(ledgerText([rowText({ asset_id: 'AST-V-A-01', notes: 'a \\| b' })]));
    expect(ok.rows[0]!.fields.notes).toBe('a | b');
  });

  it('rejects unknown allowed_processing codes; blank means unrestricted (null)', () => {
    const e = err(() => parseLedger(ledgerText([{ asset_id: 'AST-V-A-01', allowed_processing: 'CONVERT,SCULPT' }])));
    expect(e.message).toMatch(/unknown code "SCULPT"/);
    expect(parseLedger(ledgerText([{ asset_id: 'AST-V-A-01', allowed_processing: '—' }])).rows[0]!.allowed_processing).toBeNull();
  });

  it('rejects two rows claiming the same raw_path', () => {
    const e = err(() => parseLedger(ledgerText([{ asset_id: 'AST-V-A-01' }, { asset_id: 'AST-V-B-01' }])));
    expect(e.message).toMatch(/already claimed/);
  });

  it('fails when section 3 or its table is absent', () => {
    expect(err(() => parseLedger('# x\n\n## 2. y\n')).code).toBe(ExitCode.LedgerInvalid);
    expect(err(() => parseLedger('# x\n\n## 3. y\n\nno table\n')).code).toBe(ExitCode.LedgerInvalid);
  });

  it('parses the real LICENSES.md of the repository', () => {
    const path = join(import.meta.dirname, '..', '..', '..', 'LICENSES.md');
    expect(readFileSync(path, 'utf8').length).toBeGreaterThan(1000);
    const l = loadLedger(path);
    expect(l.rows.length).toBeGreaterThan(0);
    expect(l.rows.every((r) => !r.asset_id.startsWith('AST-EXAMPLE-'))).toBe(true);
  });
});

describe('lookupByRawPath', () => {
  const l = parseLedger(
    ledgerText([
      { asset_id: 'AST-V-A-01', raw_path: 'assets/incoming/V-A/Poly-1/' },
      { asset_id: 'AST-V-A-02', raw_path: 'assets/incoming/V-A/Poly-1/special/crate.gltf' },
      { asset_id: 'AST-V-B-01', raw_path: 'assets/incoming/V-B' },
    ]),
  );
  it('matches by whole-segment prefix, longest wins', () => {
    expect(lookupByRawPath(l, 'assets/incoming/V-A/Poly-1/x/y.png')?.asset_id).toBe('AST-V-A-01');
    expect(lookupByRawPath(l, 'assets/incoming/V-A/Poly-1/special/crate.gltf')?.asset_id).toBe('AST-V-A-02');
    expect(lookupByRawPath(l, 'assets/incoming/V-B/a.glb')?.asset_id).toBe('AST-V-B-01');
  });
  it('does not match partial segments or unrelated paths', () => {
    expect(lookupByRawPath(l, 'assets/incoming/V-B2/a.glb')).toBeUndefined();
    expect(lookupByRawPath(l, 'assets/incoming/V-A/Poly-10/a.glb')).toBeUndefined();
    expect(lookupByRawPath(l, 'assets/incoming/other.glb')).toBeUndefined();
  });
});
