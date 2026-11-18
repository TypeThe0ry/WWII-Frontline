import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { main } from '../src/cli.ts';
import { ExitCode } from '../src/errors.ts';
import { loadLedger } from '../src/ledger.ts';
import { checkSourceIndex } from '../src/source-index.ts';
import { ledgerText, tmpRepo, type RowInput, type TmpRepo } from './helpers.ts';

const sha = 'a'.repeat(64);
const file = {
  path: 'assets/incoming/V-TEST-001/asset.zip',
  req_ids: ['V-TEST-001', 'V-TEST-002', 'V-TEST-003'],
  license: 'CC0 1.0',
  source_url: 'https://example.org/x',
  sha256: sha,
};
const archive = 'licenses/texts/asset.txt';
const repos: TmpRepo[] = [];
afterEach(() => {
  while (repos.length) repos.pop()!.cleanup();
  vi.restoreAllMocks();
});

function fixture(row: Partial<RowInput> = {}) {
  const r = tmpRepo(ledgerText([{
    asset_id: 'AST-FREE', raw_path: file.path, notes: `SHA-256=${sha}`, license_text_path: archive, ...row,
  }, {
    asset_id: 'AST-INT', raw_path: 'assets/incoming/ue-purchased/internal', req_id: 'V-TEST-003', status: 'INTERNAL',
  }]));
  repos.push(r);
  r.put(archive, 'test licence text');
  return r;
}

function putIndex(r: TmpRepo, updated: Partial<typeof file> = {}) {
  return r.put('index.json', JSON.stringify({ files: [{ ...file, ...updated }] }));
}

describe('metadata-only ingest and source coverage', () => {
  it('validates the index without source bytes and reports only explicit coverage decisions', async () => {
    const r = fixture();
    r.put('index.json', JSON.stringify({ files: [file], coverage: { 'V-TEST-001': 'real', 'V-TEST-002': 'placeholder' } }));
    r.put('requirements.json', JSON.stringify({ schema: 1, requirements: ['V-TEST-001', 'V-TEST-002', 'V-TEST-003', 'V-TEST-004'].map((id) => ({ id, milestone: 'M1' })) }));
    const lines: string[] = [];
    const base = ['--root', r.root, '--metadata-only', 'index.json'];
    const out = (s: string) => lines.push(s);
    expect(await main(['ingest', ...base], out)).toBe(ExitCode.Ok);
    expect(lines.at(-1)).toContain('1 file record(s)');
    expect(await main(['report', ...base, '--requirements', 'requirements.json', '--out', 'report.json', '--strict'], out)).toBe(ExitCode.Ok);
    const report = JSON.parse(readFileSync(join(r.root, 'report.json'), 'utf8'));
    expect(report.coverage_basis).toBe('source-metadata');
    expect(report.summary).toEqual({ real: 1, placeholder: 1, missing: 1, internal_only: 1, total: 4 });
    expect(report.requirements[2]).toMatchObject({ status: 'internal-only', assets: ['AST-FREE', 'AST-INT'] });
    expect(existsSync(join(r.root, file.path))).toBe(false);
    expect(existsSync(r.work)).toBe(false);
  });

  it.each([
    ['hash', { sha256: 'b'.repeat(64) }],
    ['source', { source_url: 'https://example.org/y' }],
    ['requirements', { req_ids: ['V-TEST-002'] }],
  ])('rejects mismatched %s metadata', (_field, data) => {
    const r = fixture();
    expect(() => checkSourceIndex(putIndex(r, data), loadLedger(r.ledger))).toThrow('invalid or mismatched');
  });

  it('rejects a licence outside the allowlist even if its ledger row is OK', () => {
    const license = 'CC BY-NC 4.0';
    const r = fixture({ license_name_version: license });
    expect(() => checkSourceIndex(putIndex(r, { license }), loadLedger(r.ledger))).toThrow('invalid or mismatched');
  });

  it('requires archived licence evidence even for OK rows', () => {
    const r = fixture({ license_text_path: '—' });
    expect(() => checkSourceIndex(putIndex(r), loadLedger(r.ledger))).toThrow('no archived licence text');
  });

  it('rejects a GLB under an INTERNAL package through the metadata-only CLI too', async () => {
    const r = fixture();
    putIndex(r, { path: 'assets/incoming/ue-purchased/internal/model.glb' });
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(await main(['ingest', '--root', r.root, '--metadata-only', 'index.json'])).toBe(ExitCode.NotOk);
    expect(err).toHaveBeenCalledWith(expect.stringContaining('INTERNAL (internal-only)'));
    expect(existsSync(r.work)).toBe(false);
  });
});
