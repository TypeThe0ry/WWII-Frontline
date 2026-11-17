import { Frustum } from '@babylonjs/core/Maths/math.frustum';
import { Plane } from '@babylonjs/core/Maths/math.plane';
import type { Matrix } from '@babylonjs/core/Maths/math.vector';

const scratch: Plane[] = Array.from({ length: 6 }, () => new Plane(0, 0, 0, 0));

/** Six inward-pointing planes (a,b,c,d each) of a view-projection matrix, as consumed by `aabbOutsideFrustum`. */
export function frustumPlanes(viewProjection: Matrix, out: Float32Array = new Float32Array(24)): Float32Array {
  Frustum.GetPlanesToRef(viewProjection, scratch);
  for (let i = 0; i < 6; i++) {
    const p = scratch[i]!;
    out[i * 4] = p.normal.x;
    out[i * 4 + 1] = p.normal.y;
    out[i * 4 + 2] = p.normal.z;
    out[i * 4 + 3] = p.d;
  }
  return out;
}
