/**
 * Velocity pass for TAA reprojection (PI-118 follow-up). Babylon's own velocity comes from PrePass, which only StandardMaterial /
 * PBRMaterial feed and which hangs this render loop; instead one extra render target (RG float: screen-space motion in UV units,
 * `uv_prev - uv_now`) is drawn by an orthogonal pass over the visible meshes with generic velocity materials:
 *  - plain meshes: previous world matrix remembered per mesh (so moving objects get their own motion),
 *  - instanced / thin-instanced meshes (soldiers, hedges, vegetation): treated as static (camera motion only; their own motion is not known),
 *  - the terrain: its own velocity material (the CDLOD vertex displacement), camera motion only.
 * Both view-projection matrices are the *unjittered* ones; the raster position still uses the jittered `viewProjection`, so the
 * velocity pixels line up with the colour pixels. TAA is driven by Babylon's own post process (`taa-chain` in post-chain.ts).
 */
import { Constants } from '@babylonjs/core/Engines/constants';
import { Color4 } from '@babylonjs/core/Maths/math.color';
import { Matrix } from '@babylonjs/core/Maths/math.vector';
import { RenderTargetTexture } from '@babylonjs/core/Materials/Textures/renderTargetTexture';
import { ShaderLanguage } from '@babylonjs/core/Materials/shaderLanguage';
import { ShaderMaterial } from '@babylonjs/core/Materials/shaderMaterial';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import { Camera } from '@babylonjs/core/Cameras/camera';
import type { Scene } from '@babylonjs/core/scene';

const vertexGlsl = (instanced: boolean): string => `
precision highp float;
attribute vec3 position;
${instanced ? 'attribute vec4 world0;\nattribute vec4 world1;\nattribute vec4 world2;\nattribute vec4 world3;' : 'uniform mat4 world;\nuniform mat4 uPrevWorld;'}
uniform mat4 viewProjection;
uniform mat4 uVP;
uniform mat4 uPrevVP;
varying vec4 vCur;
varying vec4 vPrev;
void main() {
  ${instanced ? 'mat4 W = mat4(world0, world1, world2, world3); mat4 PW = W;' : 'mat4 W = world; mat4 PW = uPrevWorld;'}
  vec4 wp = W * vec4(position, 1.0);
  vCur = uVP * wp;
  vPrev = uPrevVP * (PW * vec4(position, 1.0));
  gl_Position = viewProjection * wp;
}
`;
const fragmentGlsl = `
precision highp float;
uniform float uFlipY;
varying vec4 vCur;
varying vec4 vPrev;
void main() {
  vec2 d = (vPrev.xy / vPrev.w - vCur.xy / vCur.w) * 0.5;
  gl_FragColor = vec4(d.x, d.y * uFlipY, 0.0, 1.0);
}
`;
const vertexWgsl = (instanced: boolean): string => `
attribute position: vec3f;
${instanced ? 'attribute world0: vec4f;\nattribute world1: vec4f;\nattribute world2: vec4f;\nattribute world3: vec4f;' : 'uniform world: mat4x4f;\nuniform uPrevWorld: mat4x4f;'}
uniform viewProjection: mat4x4f;
uniform uVP: mat4x4f;
uniform uPrevVP: mat4x4f;
varying vCur: vec4f;
varying vPrev: vec4f;
@vertex
fn main(input: VertexInputs) -> FragmentInputs {
  ${instanced ? 'let W = mat4x4f(vertexInputs.world0, vertexInputs.world1, vertexInputs.world2, vertexInputs.world3); let PW = W;' : 'let W = uniforms.world; let PW = uniforms.uPrevWorld;'}
  let wp = W * vec4f(vertexInputs.position, 1.0);
  vertexOutputs.vCur = uniforms.uVP * wp;
  vertexOutputs.vPrev = uniforms.uPrevVP * (PW * vec4f(vertexInputs.position, 1.0));
  vertexOutputs.position = uniforms.viewProjection * wp;
}
`;
const fragmentWgsl = `
uniform uFlipY: f32;
varying vCur: vec4f;
varying vPrev: vec4f;
@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  let d = (fragmentInputs.vPrev.xy / fragmentInputs.vPrev.w - fragmentInputs.vCur.xy / fragmentInputs.vCur.w) * 0.5;
  fragmentOutputs.color = vec4f(d.x, d.y * uniforms.uFlipY, 0.0, 1.0);
}
`;

/** Terrain velocity fragment (the terrain vertex shader provides `vWorld`); `uVP` / `uPrevVP` are the unjittered matrices. */
export const TERRAIN_VELOCITY_FRAGMENT_GLSL = `
precision highp float;
varying vec3 vWorld;
uniform mat4 uVP;
uniform mat4 uPrevVP;
uniform float uFlipY;
void main() {
  vec4 c = uVP * vec4(vWorld, 1.0);
  vec4 p = uPrevVP * vec4(vWorld, 1.0);
  vec2 d = (p.xy / p.w - c.xy / c.w) * 0.5;
  gl_FragColor = vec4(d.x, d.y * uFlipY, 0.0, 1.0);
}
`;
export const TERRAIN_VELOCITY_FRAGMENT_WGSL = `
varying vWorld: vec3f;
uniform uVP: mat4x4f;
uniform uPrevVP: mat4x4f;
uniform uFlipY: f32;
@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  let c = uniforms.uVP * vec4f(fragmentInputs.vWorld, 1.0);
  let p = uniforms.uPrevVP * vec4f(fragmentInputs.vWorld, 1.0);
  let d = (p.xy / p.w - c.xy / c.w) * 0.5;
  fragmentOutputs.color = vec4f(d.x, d.y * uniforms.uFlipY, 0.0, 1.0);
}
`;

export interface VelocityOptions {
  wgsl: boolean;
  /** Terrain mesh and its velocity material (set up by the terrain). */
  terrain?: { mesh: AbstractMesh; material: ShaderMaterial };
  /** Meshes not drawn into the velocity target (e.g. the first-person weapon layer, depth passes). */
  exclude?: (m: AbstractMesh) => boolean;
  /** Flip the sign of the vertical velocity component (the vUV orientation of Babylon's post processes differs per backend). */
  flipY?: boolean;
}

export class VelocityPass {
  readonly rtt: RenderTargetTexture;
  private readonly plain: ShaderMaterial;
  private readonly inst: ShaderMaterial;
  private readonly prevWorld = new WeakMap<AbstractMesh, Matrix>();
  private readonly vp = new Matrix();
  private readonly prevVP = new Matrix();
  private readonly proj = new Matrix();
  private haveFrame = false;
  drawn = 0;

  constructor(
    private readonly scene: Scene,
    private readonly o: VelocityOptions,
  ) {
    const eng = scene.getEngine();
    const mk = (instanced: boolean): ShaderMaterial => {
      const m = new ShaderMaterial(
        `PH_V-GEN-001__velocity${instanced ? '_inst' : ''}`,
        scene,
        o.wgsl ? { vertexSource: vertexWgsl(instanced), fragmentSource: fragmentWgsl } : { vertexSource: vertexGlsl(instanced), fragmentSource: fragmentGlsl },
        {
          attributes: ['position'],
          uniforms: instanced ? ['viewProjection', 'uVP', 'uPrevVP', 'uFlipY'] : ['world', 'uPrevWorld', 'viewProjection', 'uVP', 'uPrevVP', 'uFlipY'],
          shaderLanguage: o.wgsl ? ShaderLanguage.WGSL : ShaderLanguage.GLSL,
        },
      );
      m.backFaceCulling = false;
      m.setFloat('uFlipY', o.flipY ? -1 : 1);
      if (!instanced) m.onBindObservable.add((mesh) => m.setMatrix('uPrevWorld', this.prevWorld.get(mesh) ?? mesh.getWorldMatrix()));
      return m;
    };
    this.plain = mk(false);
    this.inst = mk(true);
    this.rtt = new RenderTargetTexture('velocity', { width: eng.getRenderWidth(), height: eng.getRenderHeight() }, scene, false, true, Constants.TEXTURETYPE_FLOAT, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE);
    this.rtt.clearColor = new Color4(0, 0, 0, 0);
    this.rtt.refreshRate = 1;
    this.rtt.renderList = [];
    scene.customRenderTargets.push(this.rtt);
    eng.onResizeObservable.add(() => this.rtt.resize({ width: eng.getRenderWidth(), height: eng.getRenderHeight() }));
  }

  /** Per frame, before the scene renders: unjittered view-projection of this and the previous frame, and the mesh list. */
  update(camera: Camera): void {
    const eng = this.scene.getEngine();
    // unjittered projection from the camera parameters (the cached matrix carries the TAA jitter)
    Matrix.PerspectiveFovLHToRef(camera.fov, eng.getRenderWidth() / Math.max(1, eng.getRenderHeight()), camera.minZ, camera.maxZ, this.proj, camera.fovMode === Camera.FOVMODE_VERTICAL_FIXED, eng.isNDCHalfZRange);
    if (this.haveFrame) this.prevVP.copyFrom(this.vp);
    camera.getViewMatrix(true).multiplyToRef(this.proj, this.vp);
    if (!this.haveFrame) {
      this.prevVP.copyFrom(this.vp);
      this.haveFrame = true;
    }
    for (const m of [this.plain, this.inst]) {
      m.setMatrix('uVP', this.vp);
      m.setMatrix('uPrevVP', this.prevVP);
    }
    const t = this.o.terrain;
    if (t) {
      t.material.setMatrix('uVP', this.vp);
      t.material.setMatrix('uPrevVP', this.prevVP);
    }
    const list = this.rtt.renderList!;
    list.length = 0;
    let n = 0;
    for (const m of this.scene.meshes) {
      if (!m.isEnabled() || m.material === null || m.name.startsWith('PH_V-GEN-001__csm') || m.name === 'PH_V-FX-001__particles') continue;
      if (m.getClassName() === 'InstancedMesh' ? !(m as unknown as { visibility: number }).visibility : !m.isVisible) continue;
      if (this.o.exclude?.(m)) continue;
      if (t && m === t.mesh) {
        this.rtt.setMaterialForRendering(m, t.material);
      } else {
        const src = (m as unknown as { sourceMesh?: AbstractMesh }).sourceMesh ?? m;
        const instanced = m.getClassName() === 'InstancedMesh' || (m as unknown as { hasThinInstances?: boolean }).hasThinInstances === true;
        this.rtt.setMaterialForRendering(src, instanced ? this.inst : this.plain);
      }
      list.push(m);
      n++;
    }
    this.drawn = n;
  }

  /** After the scene rendered: remember this frame's world matrices (the next frame's `uPrevWorld`). */
  endFrame(): void {
    for (const m of this.rtt.renderList!) {
      if (m.getClassName() === 'InstancedMesh') continue;
      const w = m.getWorldMatrix();
      const p = this.prevWorld.get(m);
      if (p) p.copyFrom(w);
      else this.prevWorld.set(m, w.clone());
    }
  }

  dispose(): void {
    this.scene.customRenderTargets.splice(this.scene.customRenderTargets.indexOf(this.rtt), 1);
    this.rtt.dispose();
    this.plain.dispose();
    this.inst.dispose();
  }
}
