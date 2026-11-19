import '@babylonjs/core/Meshes/thinInstanceMesh';
import type { Material } from '@babylonjs/core/Materials/material';
import { Matrix, Quaternion, Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import type { Scene } from '@babylonjs/core/scene';
import { chunkKey, chunkOf, R_NEAR } from './chunk-streamer-core';
import { ChunkStreamer } from './chunk-streamer';
import type { HedgeSegment } from './hedge-geometry';

export const HEDGE_MESH_PREFIX = 'V-HEDGE-PLACEHOLDER_';

export function hedgeMatrices(segs: readonly HedgeSegment[], groundAt: (x: number, z: number) => number): Float32Array {
  const out = new Float32Array(segs.length * 16);
  const matrix = new Matrix();
  const scale = new Vector3();
  const rotation = new Quaternion();
  const position = new Vector3();
  const up = Vector3.UpReadOnly;
  segs.forEach((s, i) => {
    scale.set(s.moundWidth, s.height, s.len);
    Quaternion.RotationAxisToRef(up, s.yaw, rotation);
    position.set(s.x, groundAt(s.x, s.z) + s.height / 2 - 0.05, s.z);
    Matrix.ComposeToRef(scale, rotation, position, matrix).copyToArray(out, i * 16);
  });
  return out;
}

/** Grey-box hedges are bucketed by segment centre. A whole segment stays in one cell across boundaries. */
export class HedgeChunks {
  private readonly meshByChunk = new Map<number, Mesh>();
  readonly streamer: ChunkStreamer<readonly HedgeSegment[]>;

  constructor(
    scene: Scene,
    material: Material,
    segments: readonly HedgeSegment[],
    groundAt: (x: number, z: number) => number,
    onAdd?: (mesh: Mesh) => void,
    onRemove?: (mesh: Mesh) => void,
  ) {
    const byChunk = new Map<number, HedgeSegment[]>();
    for (const segment of segments) {
      if (segment.gap) continue;
      const key = chunkKey(chunkOf(segment.x), chunkOf(segment.z));
      const group = byChunk.get(key);
      if (group) group.push(segment);
      else byChunk.set(key, [segment]);
    }
    this.streamer = new ChunkStreamer<readonly HedgeSegment[]>({
      radius: R_NEAR,
      retainRadius: R_NEAR,
      hasChunk: ({ bx, bz }) => byChunk.has(chunkKey(bx, bz)),
      // The world preview retains the continuous CDLOD heightmap at all times.
      hasFallback: () => true,
      load: ({ bx, bz }) => {
        const group = byChunk.get(chunkKey(bx, bz))!;
        return Promise.resolve({ data: group, uploadBytes: group.length * 16 * Float32Array.BYTES_PER_ELEMENT });
      },
      upload: ({ bx, bz }, group) => {
        const mesh = MeshBuilder.CreateBox(`${HEDGE_MESH_PREFIX}${bx}_${bz}`, { size: 1 }, scene);
        mesh.material = material;
        mesh.thinInstanceSetBuffer('matrix', hedgeMatrices(group, groundAt), 16, true);
        mesh.thinInstanceRefreshBoundingInfo(true);
        mesh.isPickable = false;
        mesh.freezeWorldMatrix();
        this.meshByChunk.set(chunkKey(bx, bz), mesh);
        onAdd?.(mesh);
      },
      evict: ({ bx, bz }) => {
        const key = chunkKey(bx, bz);
        const mesh = this.meshByChunk.get(key);
        if (!mesh) return;
        onRemove?.(mesh);
        mesh.dispose();
        this.meshByChunk.delete(key);
      },
    });
  }

  get activeMeshes(): number { return this.meshByChunk.size; }

  dispose(): void { this.streamer.dispose(); }
}
