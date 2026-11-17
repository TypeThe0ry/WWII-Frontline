import { describe, expect, it } from 'vitest';
import { boundsOf, buildGrass, buildShrub, buildTree, type MeshData } from './placeholder-meshes';

const within = (v: number, target: number, tol = 0.2): boolean => v >= target * (1 - tol) && v <= target * (1 + tol);

describe('placeholder triangle counts (ARCHITECTURE §9.6)', () => {
  const t0 = buildTree(0);
  const t1 = buildTree(1);
  const s0 = buildShrub(0);
  const s1 = buildShrub(1);
  const g0 = buildGrass(0);
  const g1 = buildGrass(1);
  it('tree LOD0 in [2000, 4000], LOD1 about 800 (+-20 %)', () => {
    console.log(`tris: tree ${t0.triangles}/${t1.triangles}, shrub ${s0.triangles}/${s1.triangles}, grass ${g0.triangles}/${g1.triangles}`);
    expect(t0.triangles).toBeGreaterThanOrEqual(2000);
    expect(t0.triangles).toBeLessThanOrEqual(4000);
    expect(within(t1.triangles, 800)).toBe(true);
  });
  it('shrub LOD0 in [300, 800], LOD1 about 120 (+-20 %)', () => {
    expect(s0.triangles).toBeGreaterThanOrEqual(300);
    expect(s0.triangles).toBeLessThanOrEqual(800);
    expect(within(s1.triangles, 120)).toBe(true);
  });
  it('grass tufts are cheap crossed blades; LOD1 is much lighter than LOD0', () => {
    expect(g0.triangles).toBeGreaterThan(20);
    expect(g0.triangles).toBeLessThanOrEqual(64);
    expect(g1.triangles).toBeLessThan(g0.triangles / 4);
  });
});

function check(m: MeshData): void {
  expect(m.positions.length).toBe(m.normals.length);
  expect(m.colors.length / 4).toBe(m.positions.length / 3);
  for (const i of m.indices) expect(i).toBeLessThan(m.positions.length / 3);
  for (let i = 0; i < m.normals.length; i += 3) expect(Math.hypot(m.normals[i]!, m.normals[i + 1]!, m.normals[i + 2]!)).toBeCloseTo(1, 4);
  for (let i = 3; i < m.colors.length; i += 4) {
    expect(m.colors[i]).toBeGreaterThanOrEqual(0);
    expect(m.colors[i]).toBeLessThanOrEqual(1);
  }
}

describe('placeholder geometry sanity', () => {
  it('has valid indices, unit normals and wind weights in [0,1]', () => {
    for (const m of [buildTree(0), buildTree(1), buildShrub(0), buildShrub(1), buildGrass(0), buildGrass(1)]) check(m);
  });
  it('tree crowns have outward normals (winding) and the trunk is fixed in the wind (weight 0)', () => {
    const t = buildTree(0);
    // the top-most vertex (crown cap): normal points up; trunk vertices (y < 4) have weight 0 or crown weight
    let top = 0;
    for (let i = 0; i < t.positions.length / 3; i++) if (t.positions[i * 3 + 1]! > t.positions[top * 3 + 1]!) top = i;
    expect(t.normals[top * 3 + 1]!).toBeGreaterThan(0.5);
    let fixed = 0;
    for (let i = 0; i < t.positions.length / 3; i++) if (t.positions[i * 3 + 1]! < 0.5 && t.colors[i * 4 + 3] === 0) fixed++;
    expect(fixed).toBeGreaterThan(5);
  });
  it('ellipsoid shells face outward: normal . (p - centre) > 0 for the shrub lump vertices', () => {
    const s = buildShrub(0);
    let pos = 0;
    let n = 0;
    for (let i = 0; i < s.positions.length / 3; i++) {
      const dx = s.positions[i * 3]!;
      const dy = s.positions[i * 3 + 1]! - 0.5;
      const dz = s.positions[i * 3 + 2]!;
      if (dx * s.normals[i * 3]! + dy * s.normals[i * 3 + 1]! + dz * s.normals[i * 3 + 2]! > 0) pos++;
      n++;
    }
    expect(pos / n).toBeGreaterThan(0.9);
  });
  it('bounds: the impostor sphere contains the mesh and the tree is ~9.5 m tall', () => {
    const b = boundsOf(buildTree(0));
    expect(b.maxY).toBeGreaterThan(8.5);
    expect(b.maxY).toBeLessThan(10.5);
    expect(b.radius).toBeGreaterThan((b.maxY - b.minY) / 2);
    const g = buildGrass(0);
    expect(boundsOf(g).maxY).toBeLessThan(0.8);
  });
});
