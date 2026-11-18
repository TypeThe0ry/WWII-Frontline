/**
 * Custom cascaded shadow maps that terrain and PBR shaders can sample (PI-117 follow-up). Each cascade is a float render target
 * drawn by an orthographic camera fitted to a slice of the view frustum (`cascades.ts`); casters are drawn with a plain depth
 * material (a thin-instanced twin for hedges). The receivers (terrain, PBR materials) get the matrices, splits and the four depth
 * maps through uniforms (`csm-shader.ts`). This replaces Babylon's `CascadedShadowGenerator` for the custom shaders; Babylon's
 * stays available for StandardMaterial receivers (`?csm=1`).
 */
import { FreeCamera } from '@babylonjs/core/Cameras/freeCamera';
import { Camera } from '@babylonjs/core/Cameras/camera';
import { Constants } from '@babylonjs/core/Engines/constants';
import { Color4 } from '@babylonjs/core/Maths/math.color';
import { Matrix, Vector3, Vector4 } from '@babylonjs/core/Maths/math.vector';
import { RenderTargetTexture } from '@babylonjs/core/Materials/Textures/renderTargetTexture';
import { RawTexture } from '@babylonjs/core/Materials/Textures/rawTexture';
import { ShaderLanguage } from '@babylonjs/core/Materials/shaderLanguage';
import { ShaderMaterial } from '@babylonjs/core/Materials/shaderMaterial';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import type { Scene } from '@babylonjs/core/scene';
import type { RenderContext } from '../engine';
import { csmConfig, type ShadowTier } from './csm-math';
import { fitCascade, type CameraPose, type V3 } from './cascades';
import { DEPTH_FRAGMENT_GLSL, DEPTH_FRAGMENT_WGSL, DEPTH_VERTEX_GLSL, DEPTH_VERTEX_WGSL, SHADOW_SAMPLERS } from './csm-shader';

export interface CustomCsmOptions {
  tier: ShadowTier;
  /** Direction the light travels (unit). */
  lightDir: V3;
  /** Depth bias (in the [0,1] depth range) and normal offset (in texels). */
  bias?: number;
  normalOffset?: number;
  /** Casters are included this far behind the cascade sphere (towards the light), metres. */
  depthBack?: number;
  /** Use the WebGL2 cascade config (3 cascades, 2048/1024/1024) on any backend: for backend comparisons at equal settings. */
  webgl2Config?: boolean;
  /** Debug overrides of the backend conventions (flip V of the depth map lookup, depth already in [0,1]). */
  flipY?: boolean;
  halfZ?: boolean;
}

export interface CustomCsm {
  cascades: number;
  /** Adds a caster; `instanced` selects the thin-instance depth material. */
  addCaster(mesh: AbstractMesh, instanced?: boolean): void;
  /**
   * Registers a dynamic caster class: every frame the scene's meshes that satisfy `pick` (returning 'plain' or 'instanced' for the
   * depth material to use, or null) are put into the cascades' render lists next to the static casters (characters, the tank,
   * visible vegetation LODs). `maxDistM` skips casters farther from the camera than any cascade can show.
   */
  addDynamicCasters(pick: (m: AbstractMesh) => 'plain' | 'instanced' | null, maxDistM?: number): void;
  /** Binds the shadow maps / uniforms to a material (call once per material, then `update` keeps the uniforms fresh). */
  attach(material: ShaderMaterial): void;
  /** Per frame, before the render: refits the cascades to `camera` and pushes the uniforms. */
  update(camera: Camera): void;
  setEnabled(on: boolean): void;
  stats(): { cascades: number; sizes: number[]; renderListSize: number };
  dispose(): void;
}

/** A 1x1 depth-1 texture for materials that are bound to the shadow contract while no CSM exists (WebGPU needs every sampler bound). */
export function dummyShadowTexture(scene: Scene): RawTexture {
  return RawTexture.CreateRGBATexture(new Uint8Array([255, 255, 255, 255]), 1, 1, scene, false, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE);
}

function depthMaterial(scene: Scene, instanced: boolean, wgsl: boolean): ShaderMaterial {
  const m = new ShaderMaterial(
    `PH_V-GEN-001__csm_depth${instanced ? '_inst' : ''}`,
    scene,
    wgsl ? { vertexSource: DEPTH_VERTEX_WGSL(instanced), fragmentSource: DEPTH_FRAGMENT_WGSL } : { vertexSource: DEPTH_VERTEX_GLSL(instanced), fragmentSource: DEPTH_FRAGMENT_GLSL },
    {
      attributes: ['position'], // thin-instance matrices (world0..3) are bound by Babylon from the shader's declarations
      uniforms: instanced ? ['viewProjection'] : ['world', 'viewProjection'],
      shaderLanguage: wgsl ? ShaderLanguage.WGSL : ShaderLanguage.GLSL,
    },
  );
  m.backFaceCulling = false;
  return m;
}

export function attachCustomCsm(ctx: RenderContext, o: CustomCsmOptions): CustomCsm {
  const scene = ctx.scene;
  const wgsl = ctx.kind === 'webgpu';
  const cfg = csmConfig(o.tier, o.webgl2Config ?? !wgsl);
  const n = Math.min(4, cfg.splits.length);
  const depthBack = o.depthBack ?? 150;
  const plain = depthMaterial(scene, false, wgsl);
  const inst = depthMaterial(scene, true, wgsl);
  const rtts: RenderTargetTexture[] = [];
  const cams: FreeCamera[] = [];
  const casters: AbstractMesh[] = [];
  const instancedCasters = new Set<AbstractMesh>();
  const dynamicPicks: { pick: (m: AbstractMesh) => 'plain' | 'instanced' | null; maxDist: number }[] = [];
  const overridden = new WeakSet<AbstractMesh>();
  let staticCount = 0;
  let dynamicCount = 0;
  const materials: ShaderMaterial[] = [];
  const dummy = dummyShadowTexture(scene);
  for (let i = 0; i < n; i++) {
    const size = cfg.sizes[i] as number;
    const rtt = new RenderTargetTexture(`csm_depth_${i}`, size, scene, false, true, Constants.TEXTURETYPE_FLOAT, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE);
    rtt.clearColor = new Color4(1, 1, 1, 1);
    rtt.wrapU = Constants.TEXTURE_CLAMP_ADDRESSMODE;
    rtt.wrapV = Constants.TEXTURE_CLAMP_ADDRESSMODE;
    rtt.renderList = [];
    rtt.refreshRate = 1; // every frame
    const cam = new FreeCamera(`csm_cam_${i}`, new Vector3(0, 100, 0), scene);
    cam.mode = Camera.ORTHOGRAPHIC_CAMERA;
    cam.minZ = 0.1;
    rtt.activeCamera = cam;
    scene.customRenderTargets.push(rtt);
    rtts.push(rtt);
    cams.push(cam);
  }
  const vp: Matrix[] = Array.from({ length: 4 }, () => new Matrix());
  const splits = new Vector4(1e9, 1e9, 1e9, 1e9);
  const info = new Vector4(n, (o.flipY ?? false) ? 1 : 0, (o.halfZ ?? wgsl) ? 1 : 0, 0);
  const texel = new Vector4(1, 1, 1, 1);
  const world = new Vector4(1, 1, 1, 1);
  const params = new Vector4(o.bias ?? 0.0015, o.normalOffset ?? 1.5, 0, 0);
  const edges = cfg.splits.slice(0, n);
  const lightDir = o.lightDir;
  let enabled = true;

  const push = (m: ShaderMaterial): void => {
    for (let i = 0; i < 4; i++) m.setMatrix(`uShadowVP${i}`, vp[i]!);
    m.setVector4('uShadowSplits', splits);
    m.setVector4('uShadowInfo', enabled ? info : new Vector4(0, info.y, info.z, 0));
    m.setVector4('uShadowTexel', texel);
    m.setVector4('uShadowWorld', world);
    m.setVector4('uShadowParams', params);
  };

  return {
    cascades: n,
    addDynamicCasters(pick, maxDistM) {
      dynamicPicks.push({ pick, maxDist: maxDistM ?? 400 });
    },
    addCaster(mesh, instanced = false) {
      staticCount++;
      casters.push(mesh);
      if (instanced) instancedCasters.add(mesh);
      for (const r of rtts) {
        r.renderList!.push(mesh);
        r.setMaterialForRendering(mesh, instanced ? inst : plain);
      }
    },
    attach(m) {
      for (let i = 0; i < 4; i++) m.setTexture(SHADOW_SAMPLERS[i]!, i < n ? rtts[i]! : dummy);
      materials.push(m);
      push(m);
    },
    update(camera) {
      const view = camera.getViewMatrix(true);
      const proj = camera.getProjectionMatrix(true);
      void view;
      void proj;
      const pos = camera.globalPosition;
      // camera basis from the inverse view matrix (rows of the world matrix)
      const wm = camera.getWorldMatrix().m;
      const right: V3 = [wm[0]!, wm[1]!, wm[2]!];
      const up: V3 = [wm[4]!, wm[5]!, wm[6]!];
      const forward: V3 = [wm[8]!, wm[9]!, wm[10]!];
      const fov = camera.fov;
      const eng = scene.getEngine();
      const aspect = eng.getRenderWidth() / Math.max(1, eng.getRenderHeight());
      const pose: CameraPose = { pos: [pos.x, pos.y, pos.z], forward, right, up, fov, aspect };
      let near = 0.1;
      for (let i = 0; i < n; i++) {
        const far = edges[i] as number;
        const c = fitCascade(pose, near, far, lightDir, cfg.sizes[i] as number);
        const cam = cams[i]!;
        const eye = new Vector3(c.snappedCenter[0] - c.dir[0] * (c.radius + depthBack), c.snappedCenter[1] - c.dir[1] * (c.radius + depthBack), c.snappedCenter[2] - c.dir[2] * (c.radius + depthBack));
        cam.position.copyFrom(eye);
        cam.setTarget(new Vector3(eye.x + c.dir[0], eye.y + c.dir[1], eye.z + c.dir[2]));
        cam.orthoLeft = -c.radius;
        cam.orthoRight = c.radius;
        cam.orthoBottom = -c.radius;
        cam.orthoTop = c.radius;
        cam.maxZ = depthBack + 2 * c.radius;
        cam.getViewMatrix(true).multiplyToRef(cam.getProjectionMatrix(true), vp[i]!);
        if (i === 0) {
          splits.x = far;
          texel.x = 1 / (cfg.sizes[0] as number);
          world.x = c.texel;
        } else if (i === 1) {
          splits.y = far;
          texel.y = 1 / (cfg.sizes[1] as number);
          world.y = c.texel;
        } else if (i === 2) {
          splits.z = far;
          texel.z = 1 / (cfg.sizes[2] as number);
          world.z = c.texel;
        } else {
          splits.w = far;
          texel.w = 1 / (cfg.sizes[3] as number);
          world.w = c.texel;
        }
        near = far;
      }
      for (const m of materials) push(m);
      if (dynamicPicks.length > 0) {
        // rebuild the dynamic part of the render lists: static casters first, then the picked meshes of this frame
        for (const r of rtts) r.renderList!.length = staticCount;
        dynamicCount = 0;
        const maxD2 = Math.max(...dynamicPicks.map((d) => d.maxDist)) ** 2;
        for (const m of scene.meshes) {
          if (!m.isEnabled() || !m.isVisible && m.getClassName() !== 'InstancedMesh') continue;
          if (m.getClassName() === 'InstancedMesh' && !(m as unknown as { visibility: number }).visibility) continue;
          let kind: 'plain' | 'instanced' | null = null;
          let maxDist = 0;
          for (const d of dynamicPicks) {
            const k = d.pick(m);
            if (k) {
              kind = k;
              maxDist = d.maxDist;
              break;
            }
          }
          if (!kind) continue;
          const bs = m.getBoundingInfo().boundingSphere;
          const bp = bs.centerWorld;
          const dx = bp.x - pos.x;
          const dy = bp.y - pos.y;
          const dz = bp.z - pos.z;
          // distance to the sphere's surface: grouped vegetation meshes cover 256 m, their centre can be far while trees are near
          const dd = Math.max(0, Math.sqrt(dx * dx + dy * dy + dz * dz) - bs.radiusWorld);
          if (dd * dd > Math.min(maxD2, maxDist * maxDist)) continue;
          const src = (m as unknown as { sourceMesh?: AbstractMesh }).sourceMesh ?? m;
          if (!overridden.has(src)) {
            overridden.add(src);
            for (const r of rtts) r.setMaterialForRendering(src, kind === 'instanced' ? inst : plain);
          }
          for (const r of rtts) r.renderList!.push(m);
          dynamicCount++;
        }
      }
    },
    setEnabled(on) {
      enabled = on;
      for (const r of rtts) r.refreshRate = on ? 1 : 0;
      for (const m of materials) push(m);
    },
    stats: () => ({ cascades: n, sizes: cfg.sizes.slice(0, n) as number[], renderListSize: staticCount + dynamicCount }),
    dispose() {
      for (const r of rtts) {
        scene.customRenderTargets.splice(scene.customRenderTargets.indexOf(r), 1);
        r.dispose();
      }
      for (const c of cams) c.dispose();
      plain.dispose();
      inst.dispose();
      dummy.dispose();
    },
  };
}
