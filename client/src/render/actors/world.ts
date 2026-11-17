import type { Matrix } from '@babylonjs/core/Maths/math.vector';
import type { Node } from '@babylonjs/core/node';

/** World matrix of `n` recomputed from the root of its parent chain (no stale cache, same frame as the writes). */
export function freshWorldMatrix(n: Node): Matrix {
  const chain: Node[] = [];
  for (let p: Node | null = n; p; p = p.parent) chain.push(p);
  for (let i = chain.length - 1; i >= 0; i--) (chain[i] as Node).computeWorldMatrix(true);
  return n.getWorldMatrix();
}
