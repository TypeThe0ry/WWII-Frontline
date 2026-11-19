import { Ray } from '@babylonjs/core/Culling/ray';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import { Mesh } from '@babylonjs/core/Meshes/mesh';
import type { Scene } from '@babylonjs/core/scene';
import type { WallProbe } from '../fx/first-person-camera';

/** Pick blocking buildings and individual boxes of the procedural hedge mesh. */
export function createFpWallProbe(scene: Scene, isHedge: (mesh: AbstractMesh) => boolean = () => false): WallProbe {
  const ray = new Ray(Vector3.Zero(), Vector3.Zero());
  const blocks = (mesh: AbstractMesh): boolean => {
    if (!mesh.isEnabled() || !mesh.isVisible) return false;
    if (mesh instanceof Mesh && mesh.hasThinInstances && isHedge(mesh)) {
      mesh.thinInstanceEnablePicking = true;
      return true;
    }
    return mesh.metadata?.blocks === true;
  };
  return (ox, oy, oz, dx, dy, dz, maxDist) => {
    ray.origin.set(ox, oy, oz);
    ray.direction.set(dx, dy, dz);
    ray.length = maxDist;
    // A predicate opts into hedge picking without changing the mesh's general isPickable flag.
    const hit = scene.pickWithRay(ray, blocks);
    return hit?.hit ? hit.distance : null;
  };
}
