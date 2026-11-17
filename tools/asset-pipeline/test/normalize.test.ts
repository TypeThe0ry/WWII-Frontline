import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { NodeIO } from '@gltf-transform/core';
import { afterEach, describe, expect, it } from 'vitest';
import { ExitCode, GateError, ingest, normalize } from '../src/index.ts';
import { ledgerText, tmpRepo, type RowInput, type TmpRepo } from './helpers.ts';

const repos: TmpRepo[] = [];
afterEach(() => {
  while (repos.length) repos.pop()!.cleanup();
});

const ID = 'AST-V-TEST-001-01';
async function setup(fixture: string, row: Partial<RowInput> = {}): Promise<TmpRepo> {
  const r = tmpRepo(ledgerText([{ asset_id: ID, ...row }]));
  repos.push(r);
  r.putFixture(fixture, `assets/incoming/V-TEST-001/PolyHaven-FREE/${fixture}`);
  ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
  return r;
}

async function bounds(path: string) {
  const doc = await new NodeIO().read(path);
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  let tris = 0;
  const prims = doc.getRoot().listMeshes().flatMap((m) => m.listPrimitives());
  for (const p of prims) {
    const pos = p.getAttribute('POSITION')!;
    const v = [0, 0, 0];
    for (let i = 0; i < pos.getCount(); i++) {
      pos.getElement(i, v);
      for (let k = 0; k < 3; k++) {
        min[k] = Math.min(min[k]!, v[k]!);
        max[k] = Math.max(max[k]!, v[k]!);
      }
    }
    tris += p.getIndices()!.getCount() / 3;
  }
  return { doc, min, max, tris, prims };
}

describe('normalize (acceptance 5)', () => {
  it('1 cm-unit Z-up cube -> 1 m edge, Y-up, bbox error < 1e-4 m, origin at bottom centre', async () => {
    const r = await setup('cube_cm_zup.gltf');
    const res = await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_cube' });
    expect(res.mode).toBe('baked');
    expect(res.unitScale).toBe(0.01);
    expect(res.up).toBe('Z');
    const b = await bounds(res.output);
    const eps = 1e-4;
    const size = b.max.map((m, i) => m - b.min[i]!);
    for (const s of size) expect(Math.abs(s - 1)).toBeLessThan(eps);
    expect(Math.abs(b.min[1]!)).toBeLessThan(eps); // sits on the ground
    expect(Math.abs(b.min[0]! + 0.5)).toBeLessThan(eps);
    expect(Math.abs(b.max[2]! - 0.5)).toBeLessThan(eps);
    expect(res.triangles).toBe(12);
    // Reported bbox equals the written one.
    for (let k = 0; k < 3; k++) {
      expect(Math.abs(res.bbox.min[k]! - b.min[k]!)).toBeLessThan(1e-5);
      expect(Math.abs(res.bbox.max[k]! - b.max[k]!)).toBeLessThan(1e-5);
    }
    // normals unit length and the written file is a valid single-mesh scene
    const nor = b.prims[0]!.getAttribute('NORMAL')!;
    const n = [0, 0, 0];
    nor.getElement(0, n);
    expect(Math.hypot(n[0]!, n[1]!, n[2]!)).toBeCloseTo(1, 5);
    expect(b.doc.getRoot().listMaterials()).toHaveLength(1); // material kept
    expect(b.prims[0]!.getMaterial()?.getName()).toBe('grey');
    expect(b.doc.getRoot().listNodes()).toHaveLength(1);
    expect(b.doc.getRoot().listMeshes()).toHaveLength(1);
  });

  it('--unit-scale overrides the declared unit', async () => {
    const r = await setup('cube_cm_zup.gltf');
    const res = await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_cube', unitScale: 0.02 });
    const b = await bounds(res.output);
    expect(b.max[0]! - b.min[0]!).toBeCloseTo(2, 4);
  });

  it('axis conversion: Z-up box with +X forward -> height on Y, forward length on -Z', async () => {
    const r = await setup('box_cm_zup.gltf');
    const res = await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_box' });
    expect(res.forward).toBe('+X');
    const b = await bounds(res.output);
    // source: x 2 m (forward), y 1 m (depth/right), z 0.5 m (height)  ->  x 1 m, y 0.5 m, z 2 m
    expect(b.max[0]! - b.min[0]!).toBeCloseTo(1, 4);
    expect(b.max[1]! - b.min[1]!).toBeCloseTo(0.5, 4);
    expect(b.max[2]! - b.min[2]!).toBeCloseTo(2, 4);
    expect(b.min[1]).toBeCloseTo(0, 5);
  });

  it('origin modes: center and none', async () => {
    const r = await setup('cube_cm_zup.gltf');
    const c = await bounds((await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_cube', origin: 'center' })).output);
    expect(c.min[1]).toBeCloseTo(-0.5, 4);
    const n = await bounds((await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_cube', origin: 'none' })).output);
    // Z-up default forward is -Y (Blender), so source +X is the left-hand side: x 5 m -> -5 m; source y 3 m is behind -> z +3 m.
    expect(n.max[0]).toBeCloseTo(-4.5, 4);
    expect(n.min[1]).toBeCloseTo(0, 4);
    expect(n.min[2]).toBeCloseTo(2.5, 4);
    expect(n.max[2]).toBeCloseTo(3.5, 4);
  });

  it('a default Y-up glTF faces +Z in the source; the output faces -Z (180 deg turn about Y)', async () => {
    const r = await setup('triangle.gltf');
    const res = await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_tri', origin: 'none' });
    expect(res.up).toBe('Y');
    expect(res.forward).toBe('+Z');
    // vertices (1,0,0) and (0,1,0) -> (-1,0,0) and (0,1,0)
    expect(res.bbox.min[0]).toBeCloseTo(-1, 5);
    expect(res.bbox.max[0]).toBeCloseTo(0, 5);
    const keep = await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_tri', origin: 'none', forward: '-Z' });
    expect(keep.bbox.min[0]).toBeCloseTo(0, 5);
    expect(keep.bbox.max[0]).toBeCloseTo(1, 5);
  });

  it('merges same-material sub-meshes across instances; scale and translation are baked in', async () => {
    const r = await setup('two_material_instanced.gltf');
    const res = await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_merge', forward: '-Z', origin: 'none' });
    const b = await bounds(res.output);
    expect(b.prims).toHaveLength(2); // material a, material b
    expect(b.tris).toBe(6);
    expect(res.triangles).toBe(6);
    expect(b.max[0]).toBeCloseTo(4, 5); // 2 + 2*1
    expect(b.max[1]).toBeCloseTo(2, 5); // 2*1
    expect(b.doc.getRoot().listMaterials()).toHaveLength(2);
  });

  it('animated assets are passed through under a wrapper node (unit + axis only)', async () => {
    const r = await setup('animated_triangle.gltf');
    const res = await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'chr_test_mover' });
    expect(res.mode).toBe('wrapped');
    expect(res.warnings.join(' ')).toMatch(/passed through/);
    const doc = await new NodeIO().read(res.output);
    expect(doc.getRoot().listAnimations()).toHaveLength(1);
    const wrapper = doc.getRoot().listScenes()[0]!.listChildren()[0]!;
    expect(wrapper.getScale()[0]).toBeCloseTo(0.01, 6);
    expect(wrapper.listChildren()).toHaveLength(1);
  });

  it('writes normalized.json and records the conversion in asset.extras', async () => {
    const r = await setup('cube_cm_zup.gltf');
    await normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_cube' });
    const meta = JSON.parse(readFileSync(join(r.work, ID, 'normalized.json'), 'utf8'));
    expect(meta).toMatchObject({ asset_id: ID, logical_id: 'prop_test_cube', mode: 'baked', output: 'normalized.gltf' });
    const j = JSON.parse(readFileSync(join(r.work, ID, 'normalized.gltf'), 'utf8'));
    expect(j.asset.extras.frontline).toMatchObject({ unit: 'm', up: 'Y', forward: '-Z' });
    expect(j.accessors).toHaveLength(3); // position, normal, indices: no stale accessors from the source
    expect(j.materials).toHaveLength(1);
  });
});

describe('normalize gate integration', () => {
  const code = async (p: Promise<unknown>): Promise<number> => {
    try {
      await p;
    } catch (e) {
      if (e instanceof GateError) return e.code;
      throw e;
    }
    return 0;
  };

  it('CONVERT is checked against allowed_processing (exit 8)', async () => {
    const r = await setup('triangle.gltf', { allowed_processing: 'LOD,TEXCOMP' });
    expect(await code(normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_tri' }))).toBe(ExitCode.ProcessingNotAllowed);
  });

  it('a bad logical ID is refused (exit 7) before anything is read', async () => {
    const r = await setup('triangle.gltf');
    expect(await code(normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'Bad-Name' }))).toBe(ExitCode.Naming);
  });

  it('a ledger row that is no longer OK is refused (exit 3)', async () => {
    const r = await setup('triangle.gltf');
    r.put('LICENSES.md', ledgerText([{ asset_id: ID, status: 'REJECTED' }]));
    expect(await code(normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_tri' }))).toBe(ExitCode.NotOk);
  });

  it('an asset that was never ingested is refused', async () => {
    const r = tmpRepo(ledgerText([{ asset_id: ID }]));
    repos.push(r);
    expect(await code(normalize({ work: r.work, ledger: r.ledger, assetId: ID, logicalId: 'prop_test_tri' }))).toBe(ExitCode.Usage);
  });
});
