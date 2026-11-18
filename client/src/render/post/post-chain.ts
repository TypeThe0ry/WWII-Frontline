/**
 * Post chain (M1-044b, ARCHITECTURE 10.2) on Babylon's pipelines: bloom + ACES tone mapping + FXAA (`DefaultRenderingPipeline`)
 * and temporal anti-aliasing (`TAARenderingPipeline`, Halton jitter, history blend) per quality tier:
 *   high:   TAA + bloom + ACES        medium: FXAA + bloom + ACES        low: FXAA + ACES (no bloom)
 * Exposure is driven by `ExposureController` (rate limited, no overshoot). The luminance *measurement* (64-bin histogram) is not
 * wired yet: `setTargetEv` takes the target from the caller (PI-118). Custom `ShaderMaterial`s (terrain, foliage, PBR) write no
 * motion vectors, so TAA here is jitter + history accumulation (with a 3x3 history clamp) and no reprojection: moving objects can
 * still leave a faint trail. Babylon's PrePass velocity (`reprojectHistory`) hangs the render loop on both backends in this setup and
 * only covers StandardMaterial/PBRMaterial anyway (PI-118). If TAA cannot be created, or the frame budget is missed for a full window,
 * `fallbackToFxaa` switches to FXAA at run time.
 */
import '@babylonjs/core/PostProcesses/RenderPipeline/postProcessRenderPipelineManagerSceneComponent';
import { DefaultRenderingPipeline } from '@babylonjs/core/PostProcesses/RenderPipeline/Pipelines/defaultRenderingPipeline';
import { TAARenderingPipeline } from '@babylonjs/core/PostProcesses/RenderPipeline/Pipelines/taaRenderingPipeline';
import { ImageProcessingConfiguration } from '@babylonjs/core/Materials/imageProcessingConfiguration';
import type { Camera } from '@babylonjs/core/Cameras/camera';
import type { Scene } from '@babylonjs/core/scene';
import { ExposureController } from './post-math';
import { PostBudgetGuard } from './post-guard';

export type PostTier = 'high' | 'medium' | 'low';

export interface PostOptions {
  tier: PostTier;
  /** Overrides (debug / measurement): force a stage on or off regardless of the tier. */
  taa?: boolean;
  bloom?: boolean;
  fxaa?: boolean;
  tonemap?: boolean;
  /** TAA history clamp and blend factor (debug / measurement). */
  clamp?: boolean;
  taaFactor?: number;
  /** Frame-interval limit (ms) of the TAA -> FXAA budget guard; `0` disables the guard. */
  guardMs?: number;
}

export interface PostHandle {
  stages: { taa: boolean; bloom: boolean; fxaa: boolean; tonemap: boolean };
  exposure: ExposureController;
  /** Current exposure in EV and the pipelines (diagnostics). */
  ev(): number;
  setTargetEv(ev: number): void;
  update(dtSec: number): void;
  /** Why TAA was replaced by FXAA, or null while TAA is (or never was) active. */
  fallbackReason(): string | null;
  /** Switches TAA off and FXAA on (idempotent); returns false when TAA was not active. */
  fallbackToFxaa(reason: string): boolean;
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
  let fallback: string | null = null;
  const fallbackToFxaa = (reason: string): boolean => {
    if (!stages.taa) return false;
    taa?.dispose();
    taa = null;
    stages.taa = false;
    stages.fxaa = true;
    def.fxaaEnabled = true;
    fallback = reason;
    return true;
  };
  if (stages.taa) {
    try {
      taa = new TAARenderingPipeline('post_taa', scene, [camera]);
      taa.samples = 8; // matches JITTER_PHASES
      taa.msaaSamples = 1;
      taa.factor = o.taaFactor ?? 0.9;
      taa.clampHistory = o.clamp ?? true; // 3x3 neighbourhood clamp of the history: cuts the ghost trail of moving objects (reports/m1-post.md)
      taa.isEnabled = true;
    } catch (e) {
      fallbackToFxaa(`TAA unavailable: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  const guard = o.guardMs === 0 ? null : new PostBudgetGuard(o.guardMs ?? 25);
  const exposure = new ExposureController(0);
  let targetEv = 0;
  return {
    stages,
    exposure,
    ev: () => exposure.ev,
    setTargetEv: (ev) => {
      targetEv = ev;
    },
    fallbackReason: () => fallback,
    fallbackToFxaa,
    update(dtSec) {
      if (guard && stages.taa && guard.observe(dtSec * 1000)) fallbackToFxaa(`frame budget: P95 over ${guard.limitMs} ms for ${guard.window} frames`);
      exposure.update(dtSec, targetEv);
      ip.exposure = 2 ** exposure.ev;
    },
    dispose() {
      taa?.dispose();
      def.dispose();
    },
  };
}
