import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { CdlodQuadtree, buildNodeBounds, makeLayout, makeSelection } from './cdlod-quadtree';
import { HeightGrid, NEAR_BLEND_M, TerrainData, decodeR16, dequantize, type MapMeta } from './terrain-data';
import ref from './fixtures/height_ref_m0.json';

const MAP_DIR = fileURLToPath(new URL('../../../../data/maps/m0/', import.meta.url));
const haveMap = existsSync(`${MAP_DIR}heightmap_1m.r16`) && existsSync(`${MAP_DIR}heightmap_far_8m.r16`) && existsSync(`${MAP_DIR}meta.json`);
const HOW = 'run: PYTHONPATH=tools/map-pipeline python3 -m map_pipeline m0 --seed 1 --out data/maps/m0 (DEVELOPING.md)';

function loadReal(): TerrainData {
  const meta = JSON.parse(readFileSync(`${MAP_DIR}meta.json`, 'utf8')) as MapMeta;
  return new TerrainData(meta, new Uint8Array(readFileSync(`${MAP_DIR}heightmap_1m.r16`)), new Uint8Array(readFileSync(`${MAP_DIR}heightmap_far_8m.r16`)));
}

describe('.r16 decoding', () => {
  it('is little-endian: bytes [0x01, 0x02] are 0x0201', () => {
    expect(Array.from(decodeR16(new Uint8Array([0x01, 0x02, 0xff, 0x00, 0x00, 0x80])))).toEqual([0x0201, 0x00ff, 0x8000]);
  });
  it('rejects odd lengths and wrong sample counts', () => {
    expect(() => decodeR16(new Uint8Array(3))).toThrow(/odd/);
    expect(() => decodeR16(new Uint8Array(4), 3)).toThrow(/expected 3/);
  });
  it('dequantises h = hmin + u16/65535*(hmax-hmin)', () => {
    const h = dequantize(new Uint16Array([0, 65535, 32768]), 1, 11);
    expect(h[0]).toBe(1);
    expect(h[1]).toBeCloseTo(11, 5);
    expect(h[2]).toBeCloseTo(1 + (32768 / 65535) * 10, 5);
  });
});

describe('HeightGrid bilinear', () => {
  // plane h = 2x + 3z over a 5x5 grid with 2 m spacing, origin (10, 20)
  const n = 5;
  const heights = new Float32Array(n * n);
  for (let j = 0; j < n; j++) for (let i = 0; i < n; i++) heights[j * n + i] = 2 * (10 + 2 * i) + 3 * (20 + 2 * j);
  const g = new HeightGrid(heights, n, 2, 10, 20);
  it('reproduces a plane exactly, incl. cell edges and the far corner', () => {
    for (const [x, z] of [[10, 20], [11.3, 25.7], [18, 28], [17.999, 27.5], [14, 22]] as const) {
      expect(g.at(x, z)).toBeCloseTo(2 * x + 3 * z, 4);
    }
  });
  it('clamps outside the grid', () => {
    expect(g.at(-100, -100)).toBe(g.at(10, 20));
    expect(g.at(1e6, 1e6)).toBe(g.at(18, 28));
  });
  it('minMaxRect bounds the surface', () => {
    const [lo, hi] = g.minMaxRect(11, 21, 15, 25);
    for (let t = 0; t < 50; t++) {
      const v = g.at(11 + (t % 7) * (4 / 6), 21 + (t % 5));
      expect(v).toBeGreaterThanOrEqual(lo - 1e-4);
      expect(v).toBeLessThanOrEqual(hi + 1e-4);
    }
  });
});

describe.skipIf(!haveMap)(`real m0 data (${HOW})`, () => {
  const t = haveMap ? loadReal() : (null as never);

  it('files match the golden fixture hashes (pipeline output unchanged)', () => {
    const sha = (f: string) => createHash('sha256').update(readFileSync(`${MAP_DIR}${f}`)).digest('hex');
    expect(sha('heightmap_1m.r16')).toBe(ref.r16_sha256['heightmap_1m.r16']);
    expect(sha('heightmap_far_8m.r16')).toBe(ref.r16_sha256['heightmap_far_8m.r16']);
  });

  it('heightAt matches the independent numpy reference on 1000 points inside the near block (<= 1e-3 m)', () => {
    let worst = 0;
    for (const [x, z, h] of ref.inside) worst = Math.max(worst, Math.abs(t.heightAt(x!, z!) - h!));
    console.log(`heightAt vs numpy reference, near block: ${ref.inside.length} points, max |err| = ${worst.toExponential(2)} m`);
    expect(ref.inside.length).toBe(1000);
    expect(worst).toBeLessThanOrEqual(1e-3);
  });

  it('heightAt matches the reference outside the block (far map + blend) and at corners/edges', () => {
    let worst = 0;
    for (const [x, z, h] of [...ref.outside, ...ref.edge]) worst = Math.max(worst, Math.abs(t.heightAt(x!, z!) - h!));
    console.log(`heightAt vs numpy reference, outside/edges: max |err| = ${worst.toExponential(2)} m`);
    expect(worst).toBeLessThanOrEqual(1e-3);
  });

  it('near/far seam (acceptance 7): no height jump across the block edge', () => {
    const s = t.nearSize;
    let rawJump = 0; // difference between the two maps themselves exactly at the edge
    let surfJump = 0; // jump of the blended surface over a 2 mm step across the edge
    const eps = 0.002;
    for (let i = 0; i <= 256; i++) {
      const u = (i / 256) * s;
      for (const [x, z, nx, nz] of [
        [0, u, -eps, u],
        [s, u, s + eps, u],
        [u, 0, u, -eps],
        [u, s, u, s + eps],
      ] as const) {
        rawJump = Math.max(rawJump, Math.abs(t.far.at(x, z) - t.near.at(x, z)));
        surfJump = Math.max(surfJump, Math.abs(t.heightAt(x, z) - t.heightAt(nx, nz)));
      }
    }
    console.log(`near/far: raw map difference at the block edge max ${rawJump.toFixed(3)} m; blended surface step over 2 mm max ${surfJump.toExponential(2)} m`);
    expect(rawJump).toBeLessThanOrEqual(0.5);
    expect(surfJump).toBeLessThanOrEqual(0.05); // slope * 2 mm, no jump
    // and beyond the blend band the surface is the far map
    expect(t.heightAt(-NEAR_BLEND_M - 10, 100)).toBeCloseTo(t.far.at(-NEAR_BLEND_M - 10, 100), 6);
  });

  it('node bounds contain the surface everywhere they are asked about', () => {
    const layout = makeLayout(t.nearSize, t.farExtent);
    const qt = new CdlodQuadtree(layout, buildNodeBounds(layout, (a, b, c, d) => t.leafRange(a, b, c, d)));
    const sel = makeSelection();
    qt.select(128, 5, 128, null, sel);
    let n = 0;
    for (let i = 0; i < sel.count; i++) {
      const o = i * 8;
      const [ox, oz, size] = [sel.data[o]!, sel.data[o + 1]!, sel.data[o + 2]!];
      const [lo, hi] = t.leafRange(ox, oz, ox + size, oz + size);
      for (let k = 0; k < 40; k++) {
        const h = t.heightAt(ox + ((k * 7919) % 1000) / 1000 * size, oz + ((k * 104729) % 1000) / 1000 * size);
        expect(h).toBeGreaterThanOrEqual(lo - 1e-4);
        expect(h).toBeLessThanOrEqual(hi + 1e-4);
        n++;
      }
    }
    expect(n).toBeGreaterThan(100);
  });

  it('GPU upload: both maps together are < 4 MB as RG8', () => {
    expect(t.uploadBytes).toBeLessThan(4 * 1024 * 1024);
  });

  it('real-data seam check: every shared edge crack-free with the m0 surface', async () => {
    const { morphVertexHeight, GRID_CELLS, EDGE_E, EDGE_N, EDGE_S, EDGE_W, INSTANCE_STRIDE } = await import('./cdlod-quadtree');
    const layout = makeLayout(t.nearSize, t.farExtent);
    const qt = new CdlodQuadtree(layout, buildNodeBounds(layout, (a, b, c, d) => t.leafRange(a, b, c, d)));
    const H = (x: number, z: number) => t.heightAt(x, z);
    let worst = 0;
    let rng = 12345;
    const rand = () => ((rng = (Math.imul(rng, 1664525) + 1013904223) >>> 0) / 4294967296);
    for (let trial = 0; trial < 6; trial++) {
      const cam: [number, number, number] = [rand() * 256, 2 + rand() * 20, rand() * 256];
      const sel = makeSelection();
      qt.select(cam[0], cam[1], cam[2], null, sel);
      const ls = Array.from({ length: sel.count }, (_, i) => Array.from(sel.data.subarray(i * INSTANCE_STRIDE, i * INSTANCE_STRIDE + 6)));
      const prof = (l: number[], side: number, tt: number) => {
        const u = tt * GRID_CELLS;
        const u0 = Math.min(Math.floor(u), GRID_CELLS - 1);
        const v = (a: number) =>
          morphVertexHeight(H, cam, l[0]!, l[1]!, l[2]!, l[3]!, l[4]!, l[5]!, side === EDGE_W ? 0 : side === EDGE_E ? GRID_CELLS : a, side === EDGE_S ? 0 : side === EDGE_N ? GRID_CELLS : a);
        const h0 = v(u0);
        return h0 + (v(u0 + 1) - h0) * (u - u0);
      };
      for (const A of ls) {
        for (const B of ls) {
          if (A === B) continue;
          const zlo = Math.max(A[1]!, B[1]!);
          const zhi = Math.min(A[1]! + A[2]!, B[1]! + B[2]!);
          if (zhi > zlo && A[0]! + A[2]! === B[0]) {
            for (let s = 0; s < 129; s++) {
              const w = zlo + ((s + 0.5) / 129) * (zhi - zlo);
              worst = Math.max(worst, Math.abs(prof(A, EDGE_E, (w - A[1]!) / A[2]!) - prof(B, EDGE_W, (w - B[1]!) / B[2]!)));
            }
          }
        }
      }
    }
    expect(worst).toBeLessThanOrEqual(1e-4);
  });
});
