import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import { ExitCode, ingest, normalize, processMesh } from '../../src/index.ts';
import { main } from '../../src/cli.ts';
import { BufferSource, FpakReader, assertNoStandalone, assertOpaque, decodeMeshEntry, decodeTextureEntry } from '../../src/fpak/index.ts';
import { ledgerText, tmpRepo, type TmpRepo } from '../helpers.ts';
import { albedoPng, buildDoc, grayPng, normalPng, sphereData, writeGltf } from '../synth.ts';

const ID = 'AST-V-TEST-001-01';
const repos: TmpRepo[] = [];
afterAll(() => repos.forEach((r) => r.cleanup()));

async function prepare(logical: string, cls: object) {
  const r = tmpRepo(ledgerText([{ asset_id: ID, allowed_processing: 'CONVERT,LOD,TEXCOMP' }]));
  repos.push(r);
  const doc = buildDoc(sphereData(60, 60), { baseColor: albedoPng(64), normal: normalPng(64), occlusion: grayPng(64, (u) => u), metalRough: grayPng(64, (_, v) => v) });
  const dir = join(r.root, 'assets/incoming/V-TEST-001/PolyHaven-FREE');
  await import('node:fs').then((fs) => fs.mkdirSync(dir, { recursive: true }));
  await writeGltf(doc, join(dir, 's.gltf'));
  ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
  await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: logical });
  await processMesh({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: logical, cls });
  return r;
}

describe('pack e2e: process -> pack -> read back', () => {
  it('CLI pack writes content-addressed .fpak + packs.json, no standalone files, opaque entries, decodable', async () => {
    const r = await prepare('prop_test_sphere', { class: 'prop', collision: { mode: 'trimesh', max_tris: 200 } });
    const out = join(r.root, 'assets/processed');
    const lines: string[] = [];
    expect(await main(['pack', '--root', r.root], (l) => lines.push(l))).toBe(ExitCode.Ok);
    expect(await main(['assert-no-standalone', '--root', r.root], () => undefined)).toBe(ExitCode.Ok);
    assertNoStandalone(out);
    const idx = JSON.parse(readFileSync(join(out, 'packs.json'), 'utf8')) as { packs: { path: string; sha256: string; entries: { logical_id: string; kind: string }[] }[] };
    expect(idx.packs).toHaveLength(1);
    const p = idx.packs[0]!;
    expect(p.path).toBe(`/a/${p.sha256.slice(0, 2)}/${p.sha256.slice(2, 16)}.fpak`);
    const file = join(out, p.path.slice(1));
    expect(existsSync(file)).toBe(true);
    expect(readdirSync(join(out, 'a'))).toEqual([p.sha256.slice(0, 2)]);
    const bytes = new Uint8Array(readFileSync(file));
    const reader = await FpakReader.open(new BufferSource(bytes));
    const ids = reader.entries().map((e) => e.logical_id).sort();
    expect(ids).toEqual([
      'prop_test_sphere@collision', 'prop_test_sphere@lod0', 'prop_test_sphere@lod1', 'prop_test_sphere@lod2', 'prop_test_sphere@lod3',
      'prop_test_sphere@meta', 'prop_test_sphere@tex_albedo', 'prop_test_sphere@tex_normal', 'prop_test_sphere@tex_orm_mat',
    ].sort());
    // No ledger id / file names / vendor strings anywhere in the file.
    const text = Buffer.from(bytes).toString('latin1');
    for (const s of [ID, 'sphere.gltf', 'PolyHaven', 'V-TEST']) expect(text.includes(s), s).toBe(false);
    for (const e of reader.entries().filter((x) => x.kind !== 'meta')) await assertOpaque(await reader.read(e.logical_id), e.logical_id);
    const lod0 = await decodeMeshEntry(await reader.read('prop_test_sphere@lod0'));
    expect(lod0.materials[0]!.textures.filter(Boolean).every((t) => ids.includes(t))).toBe(true);
    const col = await decodeMeshEntry(await reader.read('prop_test_sphere@collision'));
    expect(col.primitives[0]!.indices.length / 3).toBeLessThanOrEqual(200);
    expect(decodeTextureEntry(await reader.read('prop_test_sphere@tex_normal')).channels).toBe(2);
    expect(JSON.parse(new TextDecoder().decode(await reader.read('prop_test_sphere@meta'))).logical_id).toBe('prop_test_sphere');
  });

  it('weapon _fp / _tp chains are independent entries, readable separately (acceptance 7)', async () => {
    const r = await prepare('wpn_test_gun', { class: 'weapon', variants: [{ suffix: '_fp' }, { suffix: '_tp', lod_ratios: [1, 0.25] }] });
    const res = await import('../../src/pack.ts').then((m) => m.pack({ work: r.work, out: join(r.root, 'assets/processed'), ledger: r.ledger }));
    const bytes = new Uint8Array(readFileSync(res.packs[0]!.file));
    const src = new BufferSource(bytes);
    const reader = await FpakReader.open(src);
    const ids = reader.entries().map((e) => e.logical_id);
    for (const s of ['@lod0_fp', '@lod3_fp', '@lod0_tp', '@lod1_tp', '@tex_albedo_fp', '@tex_albedo_tp']) expect(ids, s).toContain('wpn_test_gun' + s);
    expect(ids).not.toContain('wpn_test_gun@lod2_tp');
    // Reading only _tp touches no byte range of any _fp entry (and vice versa).
    const fpRanges = reader.entries().filter((e) => /_fp$/.test(e.logical_id)).map((e) => [Number(e.offset), Number(e.offset) + e.bytes - 1]);
    src.log.length = 0;
    await reader.read('wpn_test_gun@lod0_tp');
    for (const [a, b] of src.log) for (const [x, y] of fpRanges) expect(b! < x! || a! > y!).toBe(true);
    const tp = await decodeMeshEntry(await reader.read('wpn_test_gun@lod0_tp'));
    expect(tp.materials[0]!.textures[0]).toMatch(/_tp$/);
  });
});
