import { GRID_CELLS, GRID_VERTS } from './cdlod-quadtree';

export interface GridMeshData {
  /** xyz per vertex; x = column i, z = row j (integers 0..64), y = 0. The vertex shader scales them per instance. */
  positions: Float32Array;
  indices: Uint16Array;
  vertexCount: number;
  triangleCount: number;
}

/**
 * The single shared 65x65 vertex grid (64x64 cells, 8 192 triangles). Every cell is split along the
 * BL-TR diagonal: (i,j)-(i+1,j+1). The morph (cdlod-quadtree.ts `morphVertexHeight`) relies on that.
 */
export function buildGridMesh(): GridMeshData {
  const positions = new Float32Array(GRID_VERTS * GRID_VERTS * 3);
  for (let j = 0; j < GRID_VERTS; j++) {
    for (let i = 0; i < GRID_VERTS; i++) {
      const o = (j * GRID_VERTS + i) * 3;
      positions[o] = i;
      positions[o + 1] = 0;
      positions[o + 2] = j;
    }
  }
  const indices = new Uint16Array(GRID_CELLS * GRID_CELLS * 6);
  let k = 0;
  for (let j = 0; j < GRID_CELLS; j++) {
    for (let i = 0; i < GRID_CELLS; i++) {
      const bl = j * GRID_VERTS + i;
      const br = bl + 1;
      const tl = bl + GRID_VERTS;
      const tr = tl + 1;
      indices[k++] = bl;
      indices[k++] = tl;
      indices[k++] = tr;
      indices[k++] = bl;
      indices[k++] = tr;
      indices[k++] = br;
    }
  }
  return { positions, indices, vertexCount: GRID_VERTS * GRID_VERTS, triangleCount: indices.length / 3 };
}
