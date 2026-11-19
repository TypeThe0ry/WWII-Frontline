import { afterEach, describe, expect, it } from 'vitest';
import { FreeCamera } from '@babylonjs/core/Cameras/freeCamera';
import { NullEngine } from '@babylonjs/core/Engines/nullEngine';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Scene } from '@babylonjs/core/scene';
import { Vegetation, type ProceduralVegetationSource, type VegetationOptions } from './vegetation';
import { INSTANCE_BYTES, SPECIES, SPECIES_GRASS, SPECIES_TREE } from './species';
import { scatterChunk } from './scatter';

const flat = { heightAt: () => 2 };
const engines: NullEngine[] = [];
afterEach(() => {
  for (const engine of engines.splice(0)) engine.dispose();
});

async function setup(source: Partial<ProceduralVegetationSource> = {}, opts: Partial<VegetationOptions> = {}) {
  const engine = new NullEngine();
  engines.push(engine);
  const scene = new Scene(engine);
  const cam = new FreeCamera('cam', new Vector3(64, 3, 64), scene);
  cam.setTarget(new Vector3(64, 3, 128));
  const veg = await Vegetation.createProcedural(scene, flat, { sizeM: 384, seed: 1, grassScale: 0.4, ...source }, {
    kind: 'webgl2', bake: false, groupChunks: 2, rebucketBudgetMs: Infinity, ...opts,
  });
  return { scene, cam, veg };
}

function grassCount(veg: Vegetation): number {
  return veg.stats.perLod.grass.reduce((total, n) => total + n, 0);
}

function expectResidentStats(veg: Vegetation): void {
  const total = { grass: 0, shrub: 0, tree: 0 };
  const perLod = { grass: [0, 0], shrub: [0, 0, 0], tree: [0, 0, 0] };
  let bytes = 0;
  const groups = new Set<string>();
  for (const b of veg.bucketList) {
    const key = SPECIES[b.species]!.key;
    total[key] += b.buckets.inst.count;
    b.buckets.buffers.forEach((buffer, lod) => {
      bytes += buffer.byteLength;
      perLod[key][lod]! += b.meshes[lod]!.thinInstanceCount;
    });
    groups.add(`${b.cx},${b.cz}`);
  }
  expect(veg.stats.total).toEqual(total);
  expect(veg.stats.perLod).toEqual(perLod);
  expect(veg.stats.bufferBytes).toBe(bytes);
  expect(veg.stats.residentGroups).toBe(groups.size);
}

const zeroDraws = { drawCalls: 0, instances: 0, tris: 0 };

describe('procedural group residency (NullEngine)', () => {
  it('creates grouped meshes lazily, evicts them on exit, and regenerates identical instances on re-entry', async () => {
    const { scene, cam, veg } = await setup();
    expect(veg.meshes).toHaveLength(0);
    expectResidentStats(veg);
    expect(veg.perfStats()).toEqual(zeroDraws); // Initialize the reporting path before any group exists.
    const sharedMeshes = [...scene.meshes];
    const sharedMaterials = [...scene.materials];
    veg.update(cam, 0);
    const first = veg.bucketList.find((b) => b.species === SPECIES_GRASS)!;
    const oldMeshes = [...veg.meshes];
    const oldTotals = { ...veg.stats.total };
    const oldBytes = veg.stats.bufferBytes;
    expect(veg.bucketList.filter((b) => b.species === SPECIES_GRASS)).toEqual([first]);
    expect(first.meshes).toHaveLength(2);
    const oldGeometry = first.meshes.map((m) => m.geometry!);
    // One pair of grass LOD meshes covers all four 64 m chunks in this group.
    expect(first.buckets.box.minX).toBeLessThan(64);
    expect(first.buckets.box.maxX).toBeGreaterThan(64);
    expect(first.buckets.box.minZ).toBeLessThan(64);
    expect(first.buckets.box.maxZ).toBeGreaterThan(64);
    expectResidentStats(veg);
    scene.render();
    const draw = veg.perfStats();
    expect(draw.drawCalls).toBeGreaterThan(0);
    expect(draw.instances).toBeGreaterThan(0);
    expect(draw.tris).toBeGreaterThan(0);

    const retainedTree = veg.bucketList.find((b) => b.species === SPECIES_TREE && b.cx === 0 && b.cz === 0)!;
    cam.position.x = 180; // grass group at x=0 is out of range, within its exit hysteresis.
    veg.update(cam, 0);
    expect(first.meshes.every((m) => !m.isDisposed() && m.thinInstanceCount === 0)).toBe(true);
    cam.position.x = 245;
    veg.update(cam, 0);
    expect(veg.bucketList).not.toContain(first);
    expect(veg.bucketList).toContain(retainedTree);
    expect(first.meshes.every((m) => m.isDisposed() && m.thinInstanceCount === 0)).toBe(true);
    expect(oldGeometry.every((g) => g.isDisposed())).toBe(true);
    expectResidentStats(veg);

    cam.position.x = -3000;
    veg.update(cam, 0);
    expect(veg.meshes).toHaveLength(0);
    expect(veg.bucketList).toHaveLength(0);
    expect(veg.stats.drawn).toEqual(zeroDraws); // Scene's last-rendered active list still contains old entries.
    expect(scene.meshes).toEqual(sharedMeshes);
    expect(scene.materials).toEqual(sharedMaterials);
    expectResidentStats(veg);

    cam.position.x = 64;
    veg.update(cam, 0);
    const regenerated = veg.bucketList.find((b) => b.species === SPECIES_GRASS)!;
    expect(regenerated).not.toBe(first);
    expect(regenerated.buckets.inst).toEqual(first.buckets.inst);
    expect(veg.meshes.every((m) => !oldMeshes.includes(m))).toBe(true);
    expect(veg.stats.total).toEqual(oldTotals);
    expect(veg.stats.bufferBytes).toBe(oldBytes);
    expectResidentStats(veg);
    scene.render();
    expect(veg.perfStats()).toEqual(draw);

    veg.dispose();
    veg.dispose();
    veg.update(cam, 1);
    expect(scene.meshes).toHaveLength(0);
    expect(sharedMaterials.every((m) => !scene.materials.includes(m))).toBe(true);
    expect(veg.perfStats()).toEqual(zeroDraws);
    expectResidentStats(veg);
  });

  it('uses active Low reach and stable grass ranks while accepting runtime quality changes', async () => {
    const { cam, veg } = await setup({ sizeM: 64 }, { quality: 'low' });
    cam.position.set(-32, 2, 32);
    veg.update(cam, 0);
    expect(veg.stats.total.grass).toBe(0);
    expect(veg.stats.total.tree).toBeGreaterThan(0);

    veg.setQuality('high'); // Same view can now reach the group.
    veg.update(cam, 0);
    const grass = veg.bucketList.find((b) => b.species === SPECIES_GRASS)!;
    expect(grassCount(veg)).toBeGreaterThan(0);
    veg.setQuality('low');
    veg.update(cam, 0);
    expect(grassCount(veg)).toBe(0);

    cam.position.set(32, 2, 32);
    veg.update(cam, 0);
    const { pos, rank } = grass.buckets.inst;
    let expected = 0;
    for (let i = 0; i < rank.length; i++) {
      if (rank[i]! < Math.round(0.35 * 256) && (pos[i * 3]! - 32) ** 2 + (pos[i * 3 + 2]! - 32) ** 2 <= 20 ** 2) expected++;
    }
    expect(expected).toBeGreaterThan(0);
    expect(grassCount(veg)).toBe(expected);
    expect(veg.bucketList).toContain(grass);

    cam.position.set(-900, 2, 32);
    veg.update(cam, 0);
    expect(veg.stats.residentGroups).toBe(0);
    veg.setQuality('high');
    veg.update(cam, 0);
    expect(veg.stats.residentGroups).toBe(1);
    expect(veg.stats.perLod.tree[2]).toBeGreaterThan(0);
    veg.dispose();
  });

  it('remembers a masked-out group until it leaves residency', async () => {
    let maskCalls = 0;
    const { cam, veg } = await setup({ sizeM: 64, mask: { blocked: () => { maskCalls++; return true; }, forest: () => false } });
    expect(maskCalls).toBe(0);
    veg.update(cam, 0);
    expect(maskCalls).toBeGreaterThan(0);
    expect(veg.meshes).toHaveLength(0);
    const calls = maskCalls;
    cam.position.x--;
    veg.update(cam, 0);
    expect(maskCalls).toBe(calls);
    cam.position.x = -3000;
    veg.update(cam, 0);
    cam.position.x = 64;
    veg.update(cam, 0);
    expect(maskCalls).toBe(calls * 2);
    veg.dispose();
  });

  it('generates the exact eager scatter entries when loading only one species', () => {
    const input = { chunkX: 4, chunkZ: 3, seed: 42, grassScale: 0.4, heightAt: flat.heightAt, mask: { blocked: (x: number) => x < 270, forest: () => true } };
    const eager = scatterChunk(input);
    for (const species of SPECIES) {
      const entries = [];
      for (let i = 0; i < eager.length; i += INSTANCE_BYTES) {
        if (eager[i + 3] === species.id) entries.push(...eager.subarray(i, i + INSTANCE_BYTES));
      }
      expect(entries.length).toBeGreaterThan(0);
      expect(scatterChunk({ ...input, species: species.id })).toEqual(Uint8Array.from(entries));
    }
  });
});
