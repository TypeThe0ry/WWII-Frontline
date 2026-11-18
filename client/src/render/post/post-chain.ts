/**
 * Post chain (M1-044b, ARCHITECTURE 10.2) on Babylon's pipelines: bloom + ACES tone mapping + FXAA (`DefaultRenderingPipeline`)
 * and temporal anti-aliasing (`TAARenderingPipeline`, Halton jitter, history blend) per quality tier:
 *   high:   TAA + bloom + ACES        medium: FXAA + bloom + ACES        low: FXAA + ACES (no bloom)
 * Exposure is driven by `ExposureController` (rate limited, no overshoot). The luminance *measurement* (64-bin histogram) is not
 * wired yet: `setTargetEv` takes the target from the caller (PI-118). Custom `ShaderMaterial`s (terrain, foliage, PBR) write no
 * motion vectors, so TAA here is jitter + history accumulation without per-object reprojection: moving objects can ghost.
 */
import '@babylonjs/core/PostProcesses/RenderPipeline/postProcessRenderPipelineManagerSceneComponent';
import { DefaultRenderingPipeline } from '@babylonjs/core/PostProcesses/RenderPipeline/Pipelines/defaultRenderingPipeline';
import { TAARenderingPipeline } from '@babylonjs/core/PostProcesses/RenderPipeline/Pipelines/taaRenderingPipeline';
import { ImageProcessingConfiguration } from '@babylonjs/core/Materials/imageProcessingConfiguration';
import type { Camera } from '@babylonjs/core/Cameras/camera';
import type { Scene } from '@babylonjs/core/scene';
import { ExposureController } from './post-math';

export type PostTier = 'high' | 'medium' | 'low';

export interface PostOptions {
  tier: PostTier;
  /** Overrides (debug / measurement): force a stage on or off regardless of the tier. */
  taa?: boolean;
  bloom?: boolean;
  fxaa?: boolean;
  tonemap?: boolean;
}

export interface PostHandle {
  stages: { taa: boolean; bloom: boolean; fxaa: boolean; tonemap: boolean };
  exposure: ExposureController;
  /** Current exposure in EV and the pipelines (diagnostics). */
  ev(): number;
  setTargetEv(ev: number): void;
  update(dtSec: number): void;
  dispose(): void;
}

export function attachPost(scene: Scene, camera: Camera, o: PostOptions): PostHandle {
  const stages = {
    taa: o.taa ?? o.tier === 'high',
    bloom: o.bloom ?? o.tier !== 'low',
    fxaa: o.fxaa ?? o.tier !== 'high',
    tonemap: o.tonemap ?? true,
  };
  const def = new DefaultRenderingPipeline('post_default', true, scene, [camera]);
  def.fxaaEnabled = stages.fxaa;
  def.bloomEnabled = stages.bloom;
  if (stages.bloom) {
    def.bloomThreshold = 0.9;
    def.bloomWeight = 0.25;
    def.bloomKernel = 48;
    def.bloomScale = 0.5;
  }
  def.imageProcessingEnabled = true;
  const ip = def.imageProcessing;
  ip.toneMappingEnabled = stages.tonemap;
  ip.toneMappingType = ImageProcessingConfiguration.TONEMAPPING_ACES;
  ip.exposure = 1;
  let taa: TAARenderingPipeline | null = null;
  if (stages.taa) {
    taa = new TAARenderingPipeline('post_taa', scene, [camera]);
    taa.samples = 8; // matches JITTER_PHASES
    taa.msaaSamples = 1;
    taa.factor = 0.9;
    taa.isEnabled = true;
  }
  const exposure = new ExposureController(0);
  let targetEv = 0;
  return {
    stages,
    exposure,
    ev: () => exposure.ev,
    setTargetEv: (ev) => {
      targetEv = ev;
    },
    update(dtSec) {
      exposure.update(dtSec, targetEv);
      ip.exposure = 2 ** exposure.ev;
    },
    dispose() {
      taa?.dispose();
      def.dispose();
    },
  };
}
