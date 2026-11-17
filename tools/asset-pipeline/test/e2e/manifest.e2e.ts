import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import { ExitCode, ingest, normalize, processMesh, readManifest, checkConsistency } from '../../src/index.ts';
import { main } from '../../src/cli.ts';
import { ledgerText, tmpRepo, type TmpRepo } from '../helpers.ts';
import { albedoPng, buildDoc, grayPng, normalPng, sphereData, writeGltf } from '../synth.ts';

const ID = 'AST-V-TEST-001-01';
const repos: TmpRepo[] = [];
afterAll(() => repos.forEach((r) => r.cleanup()));

describe('manifest e2e: ingest -> process -> pack -> manifest -> budget', () => {
  it('builds a consistent manifest and the budget gate passes, then fails when the budget shrinks', async () => {
    const r = tmpRepo(ledgerText([{ asset_id: ID, allowed_processing: 'CONVERT,LOD,TEXCOMP' }]));
    repos.push(r);
    const doc = buildDoc(sphereData(60, 60), { baseColor: albedoPng(64), normal: normalPng(64), occlusion: grayPng(64, (u) => u), metalRough: grayPng(64, (_, v) => v) });
    const dir = join(r.root, 'assets/incoming/V-TEST-001/PolyHaven-FREE');
    await import('node:fs').then((fs) => fs.mkdirSync(dir, { recursive: true }));
    await writeGltf(doc, join(dir, 's.gltf'));
    ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
    await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_sphere' });
    await processMesh({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_sphere', cls: { class: 'prop', collision: { mode: 'trimesh', max_tris: 200 } } });
    expect(await main(['pack', '--root', r.root], () => undefined)).toBe(ExitCode.Ok);
    r.put('assets/manifests/budgets.json', JSON.stringify({ core_bytes: 8388608, map_carentan_medium_bytes: 157286400 }));
    r.put('assets/manifests/bundles.json', JSON.stringify({ core: ['ui_*'], map_carentan_medium: ['prop_*'] }));
    expect(await main(['manifest', '--root', r.root], () => undefined)).toBe(ExitCode.Ok);
    const m = readManifest(join(r.root, 'assets/manifests/manifest.json'));
    const a = m.assets.prop_test_sphere!;
    expect(a.kind).toBe('model');
    expect(a.placeholder).toBe(false);
    expect(a.lods!.length).toBe(4);
    expect(a.lods![0]!.tris).toBeGreaterThan(a.lods![3]!.tris!);
    expect(a.collision).toBeDefined();
    expect(a.textures!.length).toBe(3);
    expect(await checkConsistency(m, join(r.root, 'assets/processed'))).toEqual([]);
    const lines: string[] = [];
    expect(await main(['manifest-budget', '--root', r.root], (l) => lines.push(l))).toBe(ExitCode.Ok);
    expect(lines.join('\n')).toContain('map_carentan_medium');
    // Shrink the budget to 1 KB: the gate must fail and name the bundle.
    r.put('assets/manifests/budgets.json', JSON.stringify({ core_bytes: 8388608, map_carentan_medium_bytes: 1024 }));
    const bad: string[] = [];
    expect(await main(['manifest-budget', '--root', r.root], (l) => bad.push(l))).not.toBe(ExitCode.Ok);
    expect(bad.join('\n')).toMatch(/FAIL map_carentan_medium.*over/);
    expect(JSON.parse(readFileSync(join(r.root, 'assets/manifests/manifest.json'), 'utf8')).schema).toBe(1);
  });
});
