import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { buildReport, formatReport, parseLedger, parseShoppingList } from '../src/index.ts';
import { main } from '../src/cli.ts';
import { ledgerText, tmpRepo, type TmpRepo } from './helpers.ts';

const repos: TmpRepo[] = [];
afterEach(() => {
  while (repos.length) repos.pop()!.cleanup();
});

const reqs = { schema: 1 as const, source: 'test', requirements: [
  { id: 'V-REAL-001', milestone: 'M1', kind: 'visual' as const },
  { id: 'V-GREY-001', milestone: 'M1', kind: 'visual' as const },
  { id: 'A-NONE-001', milestone: 'M1', kind: 'audio' as const },
  { id: 'V-LATE-001', milestone: 'M2', kind: 'visual' as const },
] };

function fixture(): TmpRepo {
  const r = tmpRepo(
    ledgerText([
      { asset_id: 'AST-V-REAL-001-01', req_id: 'V-REAL-001', raw_path: 'assets/incoming/V-REAL-001/x' },
      { asset_id: 'AST-V-GREY-001-01', req_id: 'V-GREY-001', raw_path: 'assets/incoming/V-GREY-001/x', status: 'PENDING' },
      { asset_id: 'AST-V-OTHER-001-01', req_id: 'V-OTHER-001', raw_path: 'assets/incoming/V-OTHER-001/x' },
    ]),
  );
  repos.push(r);
  mkdirSync(join(r.work, 'AST-V-REAL-001-01'), { recursive: true });
  writeFileSync(join(r.work, 'AST-V-REAL-001-01', 'normalized.gltf'), '{}');
  return r;
}

describe('report (acceptance 6)', () => {
  it('real / placeholder / missing with correct counts', () => {
    const r = fixture();
    const ph = join(r.root, 'ph');
    mkdirSync(ph);
    writeFileSync(join(ph, 'w.json'), JSON.stringify({ weapons: { a: { requirement_id: 'V-GREY-001' } } }));
    const rep = buildReport({ requirements: reqs, ledger: r.ledger, work: r.work, placeholders: ph });
    const byId = Object.fromEntries(rep.requirements.map((x) => [x.id, x.status]));
    expect(byId).toEqual({ 'V-REAL-001': 'real', 'V-GREY-001': 'placeholder', 'A-NONE-001': 'missing', 'V-LATE-001': 'missing' });
    expect(rep.summary).toEqual({ real: 1, placeholder: 1, missing: 2, total: 4 });
    expect(rep.by_milestone.M1).toEqual({ real: 1, placeholder: 1, missing: 1, total: 3 });
    expect(rep.by_milestone.M2).toEqual({ real: 0, placeholder: 0, missing: 1, total: 1 });
    expect(rep.unknown_req_ids).toEqual(['V-OTHER-001']);
    expect(formatReport(rep)).toContain('V-REAL-001');
  });

  it('manifest draft: placeholder flags; real without an OK ledger row is a violation', () => {
    const r = fixture();
    const m = join(r.root, 'm.json');
    writeFileSync(m, JSON.stringify({ assets: { a: { req_id: 'V-GREY-001', placeholder: true }, b: { req_id: 'A-NONE-001', placeholder: false } } }));
    const rep = buildReport({ requirements: reqs, ledger: r.ledger, work: r.work, manifest: m });
    expect(rep.requirements.find((x) => x.id === 'V-GREY-001')!.status).toBe('placeholder');
    expect(rep.requirements.find((x) => x.id === 'A-NONE-001')!.status).toBe('missing');
    expect(rep.violations).toHaveLength(1);
    expect(rep.violations[0]).toContain('A-NONE-001');
  });

  it('milestone filter', () => {
    const r = fixture();
    const rep = buildReport({ requirements: reqs, ledger: r.ledger, work: r.work, milestone: 'M1' });
    expect(rep.requirements).toHaveLength(3);
    expect(rep.summary.total).toBe(3);
  });

  it('CLI writes the JSON report', async () => {
    const r = fixture();
    writeFileSync(join(r.root, 'requirements.json'), JSON.stringify(reqs));
    const lines: string[] = [];
    const code = await main(['report', '--root', r.root, '--requirements', 'requirements.json', '--out', 'out/report.json'], (s) => lines.push(s));
    expect(code).toBe(0);
    const j = JSON.parse(require_('node:fs').readFileSync(join(r.root, 'out/report.json'), 'utf8'));
    expect(j.summary.total).toBe(4);
  });
});

import { createRequire } from 'node:module';
const require_ = createRequire(import.meta.url);

describe('parseShoppingList / ledger of real repo', () => {
  it('extracts requirement ids with milestones from the §4 tables', () => {
    const f = parseShoppingList(
      ['## 4. x', '### 4.1 M0 a', '| V-AAA-001 | x |', '| A-BBB-002（M1 | y |', '### 4.2 M1 b', '| V-AAA-001 | dup |', '| D-TNK-001 | z |', '## 5. out', '| V-ZZZ-001 | no |'].join('\n'),
    );
    expect(f.requirements).toEqual([
      { id: 'V-AAA-001', milestone: 'M0', kind: 'visual' },
      { id: 'A-BBB-002', milestone: 'M0', kind: 'audio' },
      { id: 'D-TNK-001', milestone: 'M1', kind: 'audio' },
    ]);
    expect(parseLedger(ledgerText([])).rows).toHaveLength(0);
  });
});
