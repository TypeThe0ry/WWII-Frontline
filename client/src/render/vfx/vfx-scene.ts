/**
 * `?vfx=stress&vfx_n=800&vfx_tier=high|medium|low` (M1-047b measurement hook): keeps about N particles alive in front of the
 * camera (smoke / dust / explosion mix at 8-40 m) and draws them with the real `ParticleRenderer`, so the per-frame CPU
 * of "pool update + instance write + upload" and the draw cost can be measured on a real GPU (stage `vfx_update`).
 */
import type { UniversalCamera } from '@babylonjs/core/Cameras/universalCamera';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { RenderContext } from '../engine';
import { emitEffect, type EffectName } from './emitters';
import { ParticlePool, type VfxTier } from './particle-pool';
import { ParticleRenderer } from './particle-render';

export interface VfxHandle {
  pool: ParticlePool;
  renderer: ParticleRenderer;
  stats(): { active: number; drawn: number; dropped: number; emitted: number };
}

const MIX: EffectName[] = ['impact_dust', 'explosion', 'tank_shot', 'impact_dust', 'tracer'];

export function attachVfxStress(ctx: RenderContext, params: URLSearchParams): VfxHandle {
  const tierParam = params.get('vfx_tier');
  const tier: VfxTier = tierParam === 'medium' || tierParam === 'low' ? tierParam : 'high';
  const target = Math.max(1, Number(params.get('vfx_n') ?? 800) | 0);
  const pool = new ParticlePool(tier);
  const renderer = new ParticleRenderer(ctx.scene, pool, ctx.kind === 'webgpu');
  const cam = ctx.scene.activeCamera as UniversalCamera;
  let emitted = 0;
  let n = 0;
  ctx.onFrame((dtMs) => {
    // top up to the target with deterministic positions in a 8-40 m cone in front of the camera
    const fx = cam.getDirection(Vector3.Forward());
    let guard = 0;
    while (pool.active < target && guard++ < 64) {
      const s = ++n;
      const d = 8 + ((s * 7919) % 3200) / 100;
      const side = (((s * 104729) % 2000) - 1000) / 1000;
      const px = cam.position.x + fx.x * d - fx.z * side * d * 0.5;
      const pz = cam.position.z + fx.z * d + fx.x * side * d * 0.5;
      emitted += emitEffect(pool, MIX[s % MIX.length]!, px, cam.position.y - 1 + ((s * 31) % 400) / 100, pz, false, d, s);
    }
    renderer.update(cam, dtMs / 1000);
  }, undefined, 'vfx_update');
  return { pool, renderer, stats: () => ({ active: pool.active, drawn: renderer.drawn, dropped: pool.dropped, emitted }) };
}
