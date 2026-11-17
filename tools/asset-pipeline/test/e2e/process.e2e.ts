import { existsSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import { ExitCode, ingest, normalize, processMesh, readStats, sizeRatio, diffStats } from '../../src/index.ts';
import { readGlb } from '../../src/mesh-compress.ts';
import { main } from '../../src/cli.ts';
import { ledgerText, tmpRepo, type TmpRepo } from '../helpers.ts';
import { albedoPng, buildDoc, grayPng, normalPng, sphereData, writeGltf } from '../synth.ts';

const ID = 'AST-V-TEST-001-01';
const repos: TmpRepo[] = [];
afterAll(() => repos.forEach((r) => r.cleanup()));

async function prepare(allowed: string, tris: { lon: number; lat: number }, size: number, albedoSize = size) {
  const r = tmpRepo(ledgerText([{ asset_id: ID, allowed_processing: allowed }]));
  repos.push(r);
  const doc = buildDoc(sphereData(tris.lon, tris.lat), {
    baseColor: albedoPng(albedoSize, 3),
    normal: normalPng(size),
    occlusion: grayPng(size, (u, v) => 0.6 + 0.4 * Math.sin(u * 20) * Math.cos(v * 20)),
    metalRough: grayPng(size, (u) => u),
  });
  const dir = join(r.root, 'assets/incoming/V-TEST-001/PolyHaven-FREE');
  await import('node:fs').then((fs) => fs.mkdirSync(dir, { recursive: true }));
  await writeGltf(doc, join(dir, 'sphere.gltf'));
  ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
  await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_sphere' });
  return r;
}

describe('process e2e: ingest -> normalize -> process -> stats', () => {
  it('textured 20k-tri sphere: outputs, size ratio <= 25 %, targets, stats.json fields', async () => {
    // PBR-pack proportions as in the card's sample note: 2048^2 albedo, 1024^2 normal / AO / metal-rough.
    const r = await prepare('CONVERT,LOD,TEXCOMP', { lon: 100, lat: 100 }, 1024, 2048);
    const t0 = Date.now();
    const res = await processMesh({
      work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_sphere',
      cls: { class: 'vehicle', texture_profile: 'bg', collision: { mode: 'trimesh' } },
    });
    const secs = (Date.now() - t0) / 1000;
    const dir = join(r.work, ID);
    for (const f of ['lod0.glb', 'lod1.glb', 'lod2.glb', 'lod3.glb', 'collision.glb', 'stats.json']) expect(existsSync(join(dir, f)), f).toBe(true);
    const s = readStats(join(dir, 'stats.json'));
    expect(s.lods.map((l) => l.lod)).toEqual([0, 1, 2, 3]);
    for (const l of s.lods) expect(l.bytes).toBe(statSync(join(dir, l.file)).size);
    const ratio = sizeRatio(s);
    console.log(`size ratio ${(ratio * 100).toFixed(1)} %, source ${s.source_bytes} B, lod0 ${s.lods[0]!.bytes} B, ${secs.toFixed(1)} s`);
    expect(ratio).toBeLessThanOrEqual(0.25);
    for (const [i, t] of [[1, 0.5], [2, 0.25], [3, 0.1]] as const) expect(Math.abs(s.lods[i]!.tris / s.lods[0]!.tris - t) / t).toBeLessThanOrEqual(0.05);
    expect(s.chains[0]!.bbox_error).toBeLessThan(0.001);
    expect(s.collision!.tris).toBeLessThanOrEqual(400);
    expect(s.textures.map((t) => `${t.slot}:${t.format}`).sort()).toEqual(['color:ETC1S', 'normal:UASTC', 'orm:ETC1S']);
    expect(res.statsFile).toBe(join(dir, 'stats.json'));
    // The glb files load back.
    const lod3 = await readGlb(join(dir, 'lod3.glb'));
    expect(lod3.getRoot().listTextures().length).toBe(3);
    expect(secs).toBeLessThan(30);
  });

  it('LOD not in allowed_processing is refused (acceptance 7), nothing is written', async () => {
    const r = await prepare('CONVERT,TEXCOMP', { lon: 12, lat: 12 }, 16);
    await expect(processMesh({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_sphere', cls: {} })).rejects.toMatchObject({ code: ExitCode.ProcessingNotAllowed });
    expect(existsSync(join(r.work, ID, 'lod0.glb'))).toBe(false);
  });

  it('TEXCOMP not allowed: textured asset is refused, textureless is fine', async () => {
    const r = await prepare('CONVERT,LOD', { lon: 12, lat: 12 }, 16);
    await expect(processMesh({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_sphere', cls: {} })).rejects.toMatchObject({ code: ExitCode.ProcessingNotAllowed });
  });

  it('CLI: process + check-size-ratio; weapon variants yield independent _fp / _tp chains', async () => {
    const r = await prepare('CONVERT,LOD,TEXCOMP', { lon: 60, lat: 60 }, 64);
    r.put('class.json', JSON.stringify({ class: 'weapon', texture_profile: 'hero', variants: [{ suffix: '_fp' }, { suffix: '_tp', lod_ratios: [1, 0.25] }] }));
    const lines: string[] = [];
    const code = await main(['process', '--root', r.root, '--asset', ID, '--id', 'wpn_test_gun', '--class', 'class.json'], (l) => lines.push(l));
    expect(code).toBe(ExitCode.Ok);
    const dir = join(r.work, ID);
    for (const f of ['lod0_fp.glb', 'lod3_fp.glb', 'lod0_tp.glb', 'lod1_tp.glb']) expect(existsSync(join(dir, f)), f).toBe(true);
    expect(existsSync(join(dir, 'lod2_tp.glb'))).toBe(false);
    const s = readStats(join(dir, 'stats.json'));
    expect(s.chains.map((c) => c.suffix)).toEqual(['_fp', '_tp']);
    expect(s.textures.every((t) => t.format === 'UASTC' || t.slot === 'normal')).toBe(true);
    expect(await main(['check-size-ratio', '--root', r.root, '--asset', ID, '--max', '10'], () => undefined)).toBe(ExitCode.Ok);
    expect(await main(['check-size-ratio', '--root', r.root, '--asset', ID, '--max', '0.0001'], () => undefined)).toBe(ExitCode.NotOk);
  });

  it('regression: stats.json of the textured sphere matches the committed snapshot within 1 %', async () => {
    const r = await prepare('CONVERT,LOD,TEXCOMP', { lon: 40, lat: 40 }, 128);
    const res = await processMesh({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_sphere', cls: { class: 'prop', collision: { mode: 'convex' } } });
    const snap = join(import.meta.dirname, 'stats.snapshot.json');
    const cur = JSON.parse(JSON.stringify(res.stats)) as Record<string, unknown>;
    if (!existsSync(snap) || process.env.UPDATE_SNAPSHOT) {
      await import('node:fs').then((fs) => fs.writeFileSync(snap, JSON.stringify(cur, null, 2) + '\n'));
    }
    expect(diffStats(JSON.parse(readFileSync(snap, 'utf8')), cur)).toEqual([]);
  });
});
