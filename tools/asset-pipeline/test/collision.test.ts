import { describe, expect, it } from 'vitest';
import { collectSoup, convexHull, simplifyCollision } from '../src/collision.ts';
import { buildDoc, seamCubeData, sphereData } from './synth.ts';

describe('simplifyCollision (acceptance 6)', () => {
  const sphere = () => buildDoc(sphereData(100, 100, 2));

  it('trimesh: 20k triangles -> <= 400 (vehicle budget), no UV / normals / material', async () => {
    const r = await simplifyCollision(sphere(), { mode: 'trimesh', kind: 'vehicle' });
    expect(r.sourceTris).toBeGreaterThan(19000);
    expect(r.tris).toBeLessThanOrEqual(400);
    expect(r.tris).toBeGreaterThan(50);
    const prim = r.doc.getRoot().listMeshes()[0]!.listPrimitives()[0]!;
    expect(prim.listSemantics()).toEqual(['POSITION']);
    expect(prim.getMaterial()).toBeNull();
    expect(r.doc.getRoot().listMaterials()).toHaveLength(0);
  });

  it('trimesh: explicit maxTris and class budgets (building 800)', async () => {
    expect((await simplifyCollision(sphere(), { mode: 'trimesh', maxTris: 120 })).tris).toBeLessThanOrEqual(120);
    const b = await simplifyCollision(sphere(), { mode: 'trimesh', kind: 'building' });
    expect(b.tris).toBeLessThanOrEqual(800);
  });

  it('convex: <= 32 hulls, each hull is a closed triangle mesh', async () => {
    const r = await simplifyCollision(sphere(), { mode: 'convex' });
    expect(r.hulls).toBeLessThanOrEqual(32);
    expect(r.hulls).toBeGreaterThan(1);
    for (const m of r.doc.getRoot().listMeshes()) {
      const p = m.listPrimitives()[0]!;
      expect(p.listSemantics()).toEqual(['POSITION']);
      expect((p.getIndices()!.getCount() / 3) % 1).toBe(0);
      // closed mesh: every edge is shared by exactly two triangles
      const ix = Array.from(p.getIndices()!.getArray()!);
      const edges = new Map<string, number>();
      for (let t = 0; t < ix.length; t += 3) for (let e = 0; e < 3; e++) {
        const a = ix[t + e] as number;
        const b = ix[t + ((e + 1) % 3)] as number;
        edges.set(`${Math.min(a, b)},${Math.max(a, b)}`, (edges.get(`${Math.min(a, b)},${Math.max(a, b)}`) ?? 0) + 1);
      }
      for (const n of edges.values()) expect(n).toBe(2);
    }
  });

  it('convex: maxHulls is honoured and capped at 32', async () => {
    expect((await simplifyCollision(buildDoc(seamCubeData(8)), { mode: 'convex', maxHulls: 4 })).hulls).toBeLessThanOrEqual(4);
    expect((await simplifyCollision(sphere(), { mode: 'convex', maxHulls: 500 })).hulls).toBeLessThanOrEqual(32);
  });

  it('convexHull: cube corners -> 12 triangles; coplanar points -> null', () => {
    const cube: [number, number, number][] = [];
    for (let i = 0; i < 8; i++) cube.push([i & 1, (i >> 1) & 1, (i >> 2) & 1]);
    cube.push([0.5, 0.5, 0.5]); // interior point is dropped
    expect(convexHull(cube)!.length).toBe(12);
    expect(convexHull([[0, 0, 0], [1, 0, 0], [0, 1, 0], [1, 1, 0], [0.5, 0.5, 0]])).toBeNull();
  });

  it('collectSoup welds shared positions', () => {
    const s = collectSoup(buildDoc(sphereData(10, 10)));
    expect(s.positions.length / 3).toBeLessThan((11 * 11));
  });
});
