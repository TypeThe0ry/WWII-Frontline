/**
 * Variant prewarm (M1-042b): compiles every legal PBR variant before the first frame that needs it and reports the time.
 * `isReady` is polled because Babylon compiles shaders asynchronously (WebGPU pipelines / WebGL parallel compile).
 */
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import type { ShaderMaterial } from '@babylonjs/core/Materials/shaderMaterial';
import { enumerateVariants, variantKey, type MaterialFactory } from './variants';

export interface PrewarmResult {
  variants: number;
  totalMs: number;
  perVariantMs: number[];
  maxMs: number;
  /** Variants that did not compile within the timeout. */
  failed: string[];
}

const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

/** Compiles each legal variant of the factory's tier against `probe` (a mesh that has the vertex data the lit variants need). */
export async function prewarmVariants(factory: MaterialFactory<ShaderMaterial>, probe: Mesh, timeoutMs = 15000): Promise<PrewarmResult> {
  const low = factory.registry.low;
  const descs = enumerateVariants(low);
  const per: number[] = [];
  const failed: string[] = [];
  const t0 = performance.now();
  for (const d of descs) {
    const m = factory.getMaterial(d);
    const t = performance.now();
    let ok = false;
    for (;;) {
      if (m.isReady(probe)) {
        ok = true;
        break;
      }
      if (performance.now() - t > timeoutMs) break;
      await sleep(1);
    }
    per.push(performance.now() - t);
    if (!ok) failed.push(variantKey(d, low));
  }
  return { variants: descs.length, totalMs: performance.now() - t0, perVariantMs: per, maxMs: per.length ? Math.max(...per) : 0, failed };
}
