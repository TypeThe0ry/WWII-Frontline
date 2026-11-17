/**
 * Octahedral impostor atlas (ARCHITECTURE §9.6): 12 x 12 views over the upper hemisphere (hemi-octahedral mapping),
 * 128 px per view -> 1536 x 1536. Two atlases per species: albedo + alpha, and object-space normals.
 *
 * Baked once at start-up from the placeholder LOD0 mesh by `bakeImpostorAtlas`: a single instanced draw per atlas
 * (see BAKE_VERTEX_* in impostor-material.*) into a RenderTargetTexture, so WebGL2 and WebGPU share the code path and no
 * pixels are read back. The pure mapping functions below are mirrored in the shaders and unit-tested.
 */
import '@babylonjs/core/Meshes/thinInstanceMesh';
import { Constants } from '@babylonjs/core/Engines/constants';
import { Color4 } from '@babylonjs/core/Maths/math.color';
import { Vector3, Vector4 } from '@babylonjs/core/Maths/math.vector';
import { FreeCamera } from '@babylonjs/core/Cameras/freeCamera';
import { Mesh } from '@babylonjs/core/Meshes/mesh';
import { VertexData } from '@babylonjs/core/Meshes/mesh.vertexData';
import { ShaderMaterial } from '@babylonjs/core/Materials/shaderMaterial';
import { ShaderLanguage } from '@babylonjs/core/Materials/shaderLanguage';
import { RenderTargetTexture } from '@babylonjs/core/Materials/Textures/renderTargetTexture';
import { Scene } from '@babylonjs/core/scene';
import type { AbstractEngine } from '@babylonjs/core/Engines/abstractEngine';
import type { MeshBounds, MeshData } from './placeholder-meshes';
import { BAKE_FRAGMENT_GLSL, BAKE_VERTEX_GLSL } from './impostor-material.glsl';
import { BAKE_FRAGMENT_WGSL, BAKE_VERTEX_WGSL } from './impostor-material.wgsl';

export const ATLAS_CELLS = 12;
export const CELL_PX = 128;
export const CELL_PX_LOWRES = 64;
export const ATLAS_PX = ATLAS_CELLS * CELL_PX;

type Vec3 = readonly [number, number, number];

/** Cell centre coordinates (a, b) in [-1, 1]^2 of cell (ci, cj). */
export function cellCenter(ci: number, cj: number, cells = ATLAS_CELLS): [number, number] {
  return [((ci + 0.5) / cells) * 2 - 1, ((cj + 0.5) / cells) * 2 - 1];
}

/** Hemi-octahedral decode: (a, b) in [-1,1]^2 -> unit direction with y >= 0. */
export function hemiOctDecode(a: number, b: number): Vec3 {
  const x = (a + b) * 0.5;
  const z = (a - b) * 0.5;
  const y = Math.max(1 - Math.abs(x) - Math.abs(z), 0);
  const l = Math.hypot(x, y, z);
  return [x / l, y / l, z / l];
}

/** Inverse of `hemiOctDecode` (directions below the horizon are clamped to it). */
export function hemiOctEncode(dx: number, dy: number, dz: number): [number, number] {
  const y = Math.max(dy, 0);
  const l1 = Math.abs(dx) + y + Math.abs(dz);
  const x = dx / l1;
  const z = dz / l1;
  return [x + z, x - z];
}

/** Atlas cell (ci, cj) used for a view direction (what the impostor fragment shader does). */
export function cellOfDirection(dx: number, dy: number, dz: number, cells = ATLAS_CELLS): [number, number] {
  const [a, b] = hemiOctEncode(dx, dy, dz);
  const f = (v: number) => Math.min(cells - 1, Math.max(0, Math.floor((v * 0.5 + 0.5) * cells)));
  return [f(a), f(b)];
}

/** Camera-facing basis used by both the bake and the runtime billboard: right = d x up, camUp = right x d. */
export function impostorBasis(d: Vec3): { right: Vec3; up: Vec3 } {
  const rx = d[1] * 0 - d[2] * 1; // cross(d, (0,1,0)) = (d.y*0 - d.z*1, d.z*0 - d.x*0, d.x*1 - d.y*0)
  const ry = 0;
  const rz = d[0];
  const l = Math.hypot(rx, ry, rz);
  const r: Vec3 = l > 1e-4 ? [rx / l, ry / l, rz / l] : [1, 0, 0];
  const u: Vec3 = [r[1] * d[2] - r[2] * d[1], r[2] * d[0] - r[0] * d[2], r[0] * d[1] - r[1] * d[0]];
  return { right: r, up: u };
}

export interface ImpostorAtlas {
  albedo: RenderTargetTexture;
  normal: RenderTargetTexture;
  cellPx: number;
  cells: number;
  /** Wall time of the bake including waiting for the GPU to finish, ms. */
  bakeMs: number;
  dispose(): void;
}

export interface BakeOptions {
  /** Species requirement id for names, e.g. `V-NAT-002`. */
  reqId: string;
  kind: 'webgpu' | 'webgl2';
  cellPx: number;
  /** Atlas clear colour (alpha 0): leaks into mip borders, so use a foliage-like colour. */
  clear: [number, number, number];
}

/**
 * Bake albedo + normal atlases from `mesh`. Uses a private Scene on the same engine so nothing shows in the main view;
 * the returned textures stay valid until `dispose()`.
 */
export async function bakeImpostorAtlas(engine: AbstractEngine, mesh: MeshData, bounds: MeshBounds, opts: BakeOptions): Promise<ImpostorAtlas> {
  const t0 = performance.now();
  const scene = new Scene(engine);
  scene.autoClear = false;
  new FreeCamera('PH_bake_cam', Vector3.Zero(), scene);
  const size = ATLAS_CELLS * opts.cellPx;
  const wgsl = opts.kind === 'webgpu';
  const makeRtt = (name: string, clear: Color4): RenderTargetTexture => {
    const rtt = new RenderTargetTexture(name, { width: size, height: size }, scene, {
      generateMipMaps: true,
      type: Constants.TEXTURETYPE_UNSIGNED_BYTE,
      samplingMode: Constants.TEXTURE_TRILINEAR_SAMPLINGMODE,
      generateDepthBuffer: true,
      samples: 4,
    });
    rtt.clearColor = clear;
    rtt.wrapU = Constants.TEXTURE_CLAMP_ADDRESSMODE;
    rtt.wrapV = Constants.TEXTURE_CLAMP_ADDRESSMODE;
    rtt.refreshRate = RenderTargetTexture.REFRESHRATE_RENDER_ONCE;
    scene.customRenderTargets.push(rtt);
    return rtt;
  };
  const albedo = makeRtt(`PH_${opts.reqId}_atlas_albedo`, new Color4(opts.clear[0], opts.clear[1], opts.clear[2], 0));
  const normal = makeRtt(`PH_${opts.reqId}_atlas_normal`, new Color4(0.5, 1, 0.5, 0));

  const cells = ATLAS_CELLS;
  const matrices = new Float32Array(cells * cells * 16);
  for (let i = 0; i < cells * cells; i++) matrices[i * 16] = i; // world0.x = cell index (ci + cells * cj)
  const mk = (mode: number, rtt: RenderTargetTexture): Mesh => {
    const m = new Mesh(`PH_${opts.reqId}__bake_${mode === 0 ? 'albedo' : 'normal'}`, scene);
    const vd = new VertexData();
    vd.positions = mesh.positions;
    vd.normals = mesh.normals;
    vd.colors = mesh.colors;
    vd.indices = mesh.indices;
    vd.applyToMesh(m, false);
    const mat = new ShaderMaterial(
      `PH_${opts.reqId}_bake_${mode}`,
      scene,
      wgsl ? { vertexSource: BAKE_VERTEX_WGSL, fragmentSource: BAKE_FRAGMENT_WGSL } : { vertexSource: BAKE_VERTEX_GLSL, fragmentSource: BAKE_FRAGMENT_GLSL },
      {
        attributes: ['position', 'normal', 'color'],
        uniforms: ['uImp', 'uMode'],
        shaderLanguage: wgsl ? ShaderLanguage.WGSL : ShaderLanguage.GLSL,
      },
    );
    mat.backFaceCulling = false;
    mat.setVector4('uImp', new Vector4(bounds.radius, bounds.centerY, cells, wgsl ? 1 : 0));
    mat.setFloat('uMode', mode);
    m.material = mat;
    m.thinInstanceSetBuffer('matrix', matrices, 16, true);
    m.doNotSyncBoundingInfo = true;
    m.alwaysSelectAsActiveMesh = true;
    m.isPickable = false;
    rtt.renderList = [m];
    return m;
  };
  mk(0, albedo);
  mk(1, normal);

  scene.render();
  // Wait for the GPU: read one pixel back (forces completion of the queued bake on both backends).
  await albedo.readPixels(0, 0, null, true, false, 0, 0, 1, 1);
  await normal.readPixels(0, 0, null, true, false, 0, 0, 1, 1);
  const bakeMs = performance.now() - t0;
  return {
    albedo,
    normal,
    cellPx: opts.cellPx,
    cells,
    bakeMs,
    dispose() {
      scene.customRenderTargets.length = 0;
      scene.dispose();
    },
  };
}
