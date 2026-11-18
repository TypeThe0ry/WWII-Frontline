/**
 * Cascaded shadow maps on the scene's directional light (M1-043b), driven by `csm-math.ts` (tier, cascade count, map sizes,
 * PCF width). Babylon's `CascadedShadowGenerator` computes its own splits from a lambda (it has no explicit split list), so the
 * data splits `[12,40,120,350]` / `[15,60,250]` are approximated by `shadowMaxZ = last split` and `lambda` (PI-117). Only
 * `StandardMaterial` meshes receive shadows: the terrain and PBR `ShaderMaterial`s are custom shaders that do not sample the
 * maps yet.
 */
import '@babylonjs/core/Lights/Shadows/shadowGeneratorSceneComponent';
import { CascadedShadowGenerator } from '@babylonjs/core/Lights/Shadows/cascadedShadowGenerator';
import { ShadowGenerator } from '@babylonjs/core/Lights/Shadows/shadowGenerator';
import type { DirectionalLight } from '@babylonjs/core/Lights/directionalLight';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import { csmConfig, type CsmConfig, type ShadowTier } from './csm-math';

export interface CsmHandle {
  generator: CascadedShadowGenerator;
  config: CsmConfig;
  cascades: number;
  /** Add a caster (and optionally make it a receiver). */
  add(mesh: AbstractMesh, receive?: boolean): void;
  dispose(): void;
}

export function attachCsm(light: DirectionalLight, tier: ShadowTier, webgl2: boolean, opts: { cascades?: number } = {}): CsmHandle {
  const cfg = csmConfig(tier, webgl2);
  const cascades = opts.cascades ?? cfg.splits.length;
  const gen = new CascadedShadowGenerator(cfg.sizes[0] as number, light);
  gen.numCascades = cascades;
  gen.shadowMaxZ = cfg.splits[Math.min(cascades, cfg.splits.length) - 1] as number;
  gen.lambda = 0.85;
  gen.stabilizeCascades = true;
  gen.autoCalcDepthBounds = false;
  gen.filter = ShadowGenerator.FILTER_PCF;
  gen.filteringQuality = cfg.pcf === 5 ? ShadowGenerator.QUALITY_HIGH : ShadowGenerator.QUALITY_MEDIUM;
  gen.bias = 0.003;
  gen.depthClamp = true;
  return {
    generator: gen,
    config: cfg,
    cascades,
    add(mesh, receive = true) {
      gen.addShadowCaster(mesh, true);
      if (receive) mesh.receiveShadows = true;
    },
    dispose() {
      gen.dispose();
    },
  };
}
