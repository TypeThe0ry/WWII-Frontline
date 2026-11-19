import { NullEngine } from '@babylonjs/core/Engines/nullEngine';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import { Scene } from '@babylonjs/core/scene';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import { expect, it } from 'vitest';
import { HedgeChunks } from './hedge-chunks';
import { polylineToSegments } from './hedge-geometry';

it('owns one thin-instance mesh per occupied cell and detaches casters before eviction and rebuild', async () => {
  const engine = new NullEngine();
  const scene = new Scene(engine);
  const material = new StandardMaterial('hedges', scene);
  const add = new Set<Mesh>();
  const removed: Mesh[] = [];
  const props = { mound_width_m: 1, height_m: 2 };
  const segments = [
    ...polylineToSegments([[63, 32], [68, 32]], props), // split across a cell edge, but keep the segment whole
    ...polylineToSegments([[70, 32], [82, 32]], props),
    ...polylineToSegments([[600, 32], [606, 32]], props),
    ...polylineToSegments([[10, 5], [15, 5]], { ...props, gap: true }),
  ];
  const hedges = new HedgeChunks(scene, material, segments, () => 0, (mesh) => add.add(mesh), (mesh) => {
    expect(mesh.isDisposed()).toBe(false);
    add.delete(mesh);
    removed.push(mesh);
  });
  const move = async (x: number, z: number) => {
    for (let i = 0; i < 4; i++) {
      hedges.streamer.update({ x, z }, { x: 0, z: 0 }, () => true, 1 / 60);
      await Promise.resolve();
    }
  };
  try {
    await move(10, 10);
    expect(hedges.activeMeshes).toBe(1);
    const [near] = add;
    expect(near?.thinInstanceCount).toBe(3);
    await move(20, 20);
    expect([...add]).toEqual([near]);
    await move(600, 30);
    expect(near?.isDisposed()).toBe(true);
    expect(removed).toEqual([near]);
    expect([...add][0]?.thinInstanceCount).toBe(1);
    await move(10, 10);
    expect([...add][0]).not.toBe(near);
    expect([...add][0]?.thinInstanceCount).toBe(3);
    expect(scene.meshes).toHaveLength(1);
  } finally {
    hedges.dispose();
    expect(add.size).toBe(0);
    expect(scene.meshes).toHaveLength(0);
    engine.dispose();
  }
});
