import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ExitCode, GateError, ingest } from '../src/index.ts';
import { main } from '../src/cli.ts';
import { ledgerText, tmpRepo, type TmpRepo } from './helpers.ts';

const repos: TmpRepo[] = [];
function repo(rows: Parameters<typeof ledgerText>[0]): TmpRepo {
  const r = tmpRepo(ledgerText(rows));
  repos.push(r);
  return r;
}
afterEach(() => {
  while (repos.length) repos.pop()!.cleanup();
  vi.restoreAllMocks();
});

function gate(r: TmpRepo): GateError {
  try {
    ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
  } catch (e) {
    if (e instanceof GateError) return e;
    throw e;
  }
  throw new Error('expected the gate to refuse');
}

const sha = (p: string): string => createHash('sha256').update(readFileSync(p)).digest('hex');

describe('ingest gate (acceptance 1)', () => {
  it('PENDING row -> exit 3 with a PENDING hint', () => {
    const r = repo([{ asset_id: 'AST-V-TEST-001-01', status: 'PENDING' }]);
    const f = r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    const e = gate(r);
    expect(e.code).toBe(ExitCode.NotOk);
    expect(e.code).toBe(3);
    expect(e.message).toContain('PENDING');
    expect(e.message).toContain('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf');
    expect(e.file).toBe(f);
  });

  it('REJECTED row -> exit 3 with a REJECTED hint (distinct from PENDING)', () => {
    const r = repo([{ asset_id: 'AST-V-TEST-001-01', status: 'REJECTED' }]);
    r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    const e = gate(r);
    expect(e.code).toBe(3);
    expect(e.message).toContain('REJECTED');
    expect(e.message).not.toContain('PENDING');
  });

  it('FLAGGED row is treated like PENDING (LICENSES §6.2)', () => {
    const r = repo([{ asset_id: 'AST-V-TEST-001-01', status: 'FLAGGED' }]);
    r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    const e = gate(r);
    expect(e.code).toBe(3);
    expect(e.message).toContain('FLAGGED');
  });

  it('INTERNAL package refuses binary ingest before any file is copied', () => {
    const dir = 'assets/incoming/ue-purchased/internal-pack';
    const r = repo([{ asset_id: 'AST-INT', raw_path: dir, status: 'INTERNAL' }]);
    r.put(`${dir}/nested/model.glb`, 'dummy GLB');
    const e = gate(r);
    expect(e.code).toBe(ExitCode.NotOk);
    expect(e.message).toContain('INTERNAL (internal-only)');
    expect(existsSync(r.work)).toBe(false);
  });

  it('unregistered file -> exit 2 naming the file', () => {
    const r = repo([{ asset_id: 'AST-V-TEST-001-01' }]);
    const f = r.put('assets/incoming/V-OTHER-009/Fab-123/model.glb', 'x');
    const e = gate(r);
    expect(e.code).toBe(ExitCode.Unregistered);
    expect(e.code).toBe(2);
    expect(e.message).toContain('assets/incoming/V-OTHER-009/Fab-123/model.glb');
    expect(e.file).toBe(f);
  });

  it('a path that only shares a name prefix with a row is unregistered', () => {
    const r = repo([{ asset_id: 'AST-V-TEST-001-01', raw_path: 'assets/incoming/V-TEST-001/Poly' }]);
    r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    expect(gate(r).code).toBe(2);
  });

  it('CLI exits 3, 3, 2 for the three sample cases', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const codes: number[] = [];
    for (const row of [
      { asset_id: 'AST-V-TEST-001-01', status: 'PENDING' },
      { asset_id: 'AST-V-TEST-001-01', status: 'REJECTED' },
    ]) {
      const r = repo([row]);
      r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
      codes.push(await main(['ingest', '--root', r.root], () => {}));
    }
    const r = repo([{ asset_id: 'AST-V-TEST-001-01' }]);
    r.put('assets/incoming/unregistered/a.gltf', 'x');
    codes.push(await main(['ingest', '--root', r.root], () => {}));
    expect(codes).toEqual([3, 3, 2]);
  });
});

describe('ingest success (acceptance 2)', () => {
  it('copies an OK asset to work/{asset_id}/raw/ with identical sha256', () => {
    const r = repo([{ asset_id: 'AST-V-TEST-001-01' }]);
    const src = r.putFixture('triangle.gltf', 'assets/incoming/V-TEST-001/PolyHaven-FREE/models/triangle.gltf');
    const res = ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
    expect(res.files).toHaveLength(1);
    const dest = join(r.work, 'AST-V-TEST-001-01', 'raw', 'models', 'triangle.gltf');
    expect(res.files[0]!.dest).toBe(dest);
    expect(existsSync(dest)).toBe(true);
    expect(sha(dest)).toBe(sha(src));
    expect(res.files[0]!.sha256).toBe(sha(src));
    const meta = JSON.parse(readFileSync(join(r.work, 'AST-V-TEST-001-01', 'ingest.json'), 'utf8'));
    expect(meta.files[0].sha256).toBe(sha(src));
  });

  it('a raw_path that names a single file works', () => {
    const r = repo([{ asset_id: 'AST-V-TEST-001-01', raw_path: 'assets/incoming/V-TEST-001/one.gltf' }]);
    const src = r.putFixture('triangle.gltf', 'assets/incoming/V-TEST-001/one.gltf');
    ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
    expect(sha(join(r.work, 'AST-V-TEST-001-01', 'raw', 'one.gltf'))).toBe(sha(src));
  });

  it('is all-or-nothing: one bad file means nothing is copied', () => {
    const r = repo([{ asset_id: 'AST-V-TEST-001-01' }]);
    r.putFixture('triangle.gltf', 'assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf');
    r.put('assets/incoming/zzz/stray.bin', 'x');
    expect(gate(r).code).toBe(2);
    expect(existsSync(r.work)).toBe(false);
  });
});

describe('D-LIC channels (acceptance 3)', () => {
  const dlic = { asset_id: 'AST-V-TEST-001-01', source_channel: 'CGTrader', order_id: 'CGT-884211', license_name_version: 'CGTrader Royalty Free License' };

  it('a paid CGTrader row without confirmation_path is refused with exit 4, and passes once the file exists', () => {
    const r = repo([{ ...dlic, confirmation_path: '—' }]);
    r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    expect(gate(r).code).toBe(ExitCode.ConfirmationMissing);

    const ok = repo([{ ...dlic, confirmation_path: 'licenses/confirmations/AST-V-TEST-001-01.eml' }]);
    ok.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    ok.put('licenses/confirmations/AST-V-TEST-001-01.eml', 'From: seller');
    expect(ingest({ incoming: ok.incoming, ledger: ok.ledger, work: ok.work }).files).toHaveLength(1);
  });

  it.each(['TurboSquid', 'KitBash3D', 'ActorCore', 'A Sound Effect', 'Sketchfab'])('%s (paid, no confirmation) -> 4', (ch) => {
    const r = repo([{ ...dlic, source_channel: ch, license_name_version: 'Standard' }]);
    r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    expect(gate(r).code).toBe(4);
  });

  it('exemptions: free download (order FREE-...) and Sketchfab under a CC licence', () => {
    const free = repo([{ ...dlic, order_id: 'FREE-2026-11-17' }]);
    free.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    expect(ingest({ incoming: free.incoming, ledger: free.ledger, work: free.work }).files).toHaveLength(1);

    const cc = repo([{ ...dlic, source_channel: 'Sketchfab', license_name_version: 'CC BY 4.0' }]);
    cc.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    expect(ingest({ incoming: cc.incoming, ledger: cc.ledger, work: cc.work }).files).toHaveLength(1);
  });

  it('a confirmation_path pointing at a missing file -> exit 5 (LICENSES §4.4); so does a missing license text', () => {
    const r = repo([{ ...dlic, confirmation_path: 'licenses/confirmations/nope.eml' }]);
    r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    const e = gate(r);
    expect(e.code).toBe(ExitCode.ArchiveMissing);
    expect(e.message).toContain('confirmation_path');

    const t = repo([{ asset_id: 'AST-V-TEST-001-01', license_text_path: 'licenses/texts/AST-V-TEST-001-01.txt' }]);
    t.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    expect(gate(t).message).toContain('license_text_path');
  });
});

describe('template ledger (acceptance 4)', () => {
  it('AST-EXAMPLE rows are ignored: nothing registered, nothing processed, no error', () => {
    const r = repo([{ asset_id: 'AST-EXAMPLE-01' }, { asset_id: 'AST-EXAMPLE-02', status: 'PENDING' }]);
    const res = ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
    expect(res).toMatchObject({ files: [], assets: [], scanned: 0 });
    expect(existsSync(r.work)).toBe(false);
  });

  it('a file that only an AST-EXAMPLE row would cover is unregistered', () => {
    const r = repo([{ asset_id: 'AST-EXAMPLE-01' }]);
    r.put('assets/incoming/V-TEST-001/PolyHaven-FREE/a.gltf', 'x');
    expect(gate(r).code).toBe(2);
  });

  it("the repository's real LICENSES.md and (absent) assets/incoming: success, zero files", () => {
    const repoRoot = join(import.meta.dirname, '..', '..', '..');
    const r = tmpRepo('');
    repos.push(r);
    const res = ingest({ incoming: r.incoming, ledger: join(repoRoot, 'LICENSES.md'), work: r.work, repoRoot });
    expect(res.files).toHaveLength(0);
    expect(res.scanned).toBe(0);
  });
});
