import { describe, expect, it } from 'vitest';
import { NullEngine } from '@babylonjs/core/Engines/nullEngine';
import { Scene } from '@babylonjs/core/scene';
import { FreeCamera } from '@babylonjs/core/Cameras/freeCamera';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Vegetation, type VegetationOptions } from './vegetation';
import { SPECIES } from './species';

const flat = { heightAt: () => 2 };

async function setup(opts: Partial<VegetationOptions> = {}) {
  const engine = new NullEngine();
  const scene = new Scene(engine);
  const cam = new FreeCamera('cam', new Vector3(128, 3.7, 20), scene);
  const tables = Vegetation.scatterTables(256, 1, flat, [{ x: 128, z: 56 }]);
  const veg = await Vegetation.create(scene, flat, tables, { kind: 'webgl2', bake: false, rebucketBudgetMs: Infinity, ...opts });
  return { engine, scene, cam, veg };
}

const sum = (a: number[]): number => a.reduce((x, y) => x + y, 0);

describe('Vegetation (NullEngine, no GPU)', () => {
  it('names materials PH_V-NAT-* and every vegetation node PH_V-NAT-...__{instance} (acceptance 7)', async () => {
    const { scene, veg } = await setup();
    const mats = scene.materials.map((m) => m.name);
    expect(mats.length).toBeGreaterThan(0);
    for (const n of mats) expect(n, `material ${n}`).toMatch(/^PH_V-NAT-00[24](_|$)/);
    expect(scene.meshes.length).toBeGreaterThan(30);
    for (const m of scene.meshes) expect(m.name, `mesh ${m.name}`).toMatch(/^PH_V-NAT-00[24]__.+/);
    expect(mats).toContain('PH_V-NAT-002');
    expect(mats).toContain('PH_V-NAT-004');
    veg.dispose();
  });

  it('buckets instances per LOD as the camera moves; one mesh per chunk x species x LOD, empty ones disabled', async () => {
    const { cam, veg } = await setup();
    veg.update(cam, 1 / 60);
    const s = veg.stats;
    expect(s.total.grass).toBeGreaterThan(1000);
    expect(s.total.tree).toBeGreaterThan(50);
    expect(sum(s.perLod.grass)).toBeGreaterThan(0);
    expect(sum(s.perLod.grass)).toBeLessThan(s.total.grass); // beyond 45 m is dropped
    expect(sum(s.perLod.tree)).toBe(s.total.tree); // trees never vanish inside the block (impostors to 1500 m)
    const enabled = veg.meshes.filter((m) => m.isEnabled());
    expect(enabled.length).toBeLessThanOrEqual(16 * 8); // <= 16 chunks x (2 grass + 3 shrub + 3 tree)
    expect(enabled.length).toBeGreaterThan(0);
    for (const m of veg.meshes) expect(m.isEnabled()).toBe(m.thinInstanceCount > 0);
    // far camera: trees become impostors, grass disappears
    cam.position = new Vector3(128, 3.7, -400);
    veg.update(cam, 1 / 60);
    expect(sum(s.perLod.grass)).toBe(0);
    expect(s.perLod.tree[2]).toBeGreaterThan(s.total.tree * 0.9);
    veg.dispose();
  });

  it('quality low draws 35 % of the grass within 20 m and switches trees to impostors early', async () => {
    const hi = await setup();
    hi.veg.update(hi.cam, 1 / 60);
    const gHi = sum(hi.veg.stats.perLod.grass);
    const tHi0 = hi.veg.stats.perLod.tree[0]!;
    const lo = await setup({ quality: 'low' });
    lo.veg.update(lo.cam, 1 / 60);
    const gLo = sum(lo.veg.stats.perLod.grass);
    expect(gLo).toBeLessThan(gHi * 0.2);
    expect(lo.veg.stats.perLod.tree[0]!).toBeLessThanOrEqual(tHi0);
    lo.veg.setQuality('high'); // switching at runtime re-buckets
    lo.veg.update(lo.cam, 1 / 60);
    expect(sum(lo.veg.stats.perLod.grass)).toBe(gHi);
    hi.veg.dispose();
    lo.veg.dispose();
  });

  it('no tree is lost or duplicated while flying 10 s through the block', async () => {
    const { cam, veg } = await setup();
    for (let f = 0; f < 600; f++) {
      cam.position = new Vector3(128, 3.7, 10 + (6.4 * f) / 60);
      veg.update(cam, 1 / 60);
      expect(sum(veg.stats.perLod.tree)).toBe(veg.stats.total.tree);
    }
    expect(veg.refreshRebucketStats().count).toBe(600);
    veg.dispose();
  });

  it('wind clock advances; a bounded budget defers work instead of exceeding it', async () => {
    const { cam, veg } = await setup({ rebucketBudgetMs: 0 });
    veg.update(cam, 0.5);
    expect(veg.windState.time).toBeCloseTo(0.5, 6);
    expect(veg.stats.deferred).toBeGreaterThan(0);
    veg.setBudget(Infinity);
    veg.update(cam, 0.5);
    expect(veg.stats.deferred).toBe(0);
    veg.dispose();
  });

  it('SPECIES table matches the spec caps', () => {
    expect(SPECIES.map((s) => s.cap)).toEqual([4096, 256, 192]);
  });
});

describe('no external texture/model URLs in vegetation code (acceptance 7)', () => {
  it('source files contain no http(s) URLs or asset file references', () => {
    const files = import.meta.glob<string>(['./*.ts', '!./*.test.ts'], { query: '?raw', import: 'default', eager: true });
    const names = Object.keys(files);
    expect(names.length).toBeGreaterThan(8);
    for (const f of names) {
      const src = files[f]!;
      expect(src, f).not.toMatch(/https?:\/\//);
      expect(src, f).not.toMatch(/\.(png|jpe?g|ktx2?|dds|basis|webp|glb|gltf)['"`]/i);
    }
  });
});
