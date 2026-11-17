import '@babylonjs/core/Engines/Extensions/engine.rawTexture';
import '@babylonjs/core/Engines/Extensions/engine.dynamicBuffer';
import '@babylonjs/core/Engines/WebGPU/Extensions/engine.rawTexture';
import { Buffer, VertexBuffer } from '@babylonjs/core/Buffers/buffer';
import { Constants } from '@babylonjs/core/Engines/constants';
import { Mesh } from '@babylonjs/core/Meshes/mesh';
import { VertexData } from '@babylonjs/core/Meshes/mesh.vertexData';
import { ShaderMaterial } from '@babylonjs/core/Materials/shaderMaterial';
import { ShaderLanguage } from '@babylonjs/core/Materials/shaderLanguage';
import { RawTexture } from '@babylonjs/core/Materials/Textures/rawTexture';
import { Matrix, Vector2, Vector3, Vector4 } from '@babylonjs/core/Maths/math.vector';
import type { Camera } from '@babylonjs/core/Cameras/camera';
import type { Scene } from '@babylonjs/core/scene';
import {
  CdlodQuadtree,
  GRID_CELLS,
  INSTANCE_STRIDE,
  LOD_COUNT,
  buildNodeBounds,
  makeLayout,
  makeSelection,
  type NodeSelection,
} from './cdlod-quadtree';
import { frustumPlanes } from './frustum';
import { buildGridMesh } from './grid-mesh';
import { NEAR_BLEND_M, type TerrainData } from './terrain-data';
import { TERRAIN_FRAGMENT_GLSL, TERRAIN_VERTEX_GLSL } from './terrain-material.glsl';
import { TERRAIN_FRAGMENT_WGSL, TERRAIN_VERTEX_WGSL } from './terrain-material.wgsl';

/** `?terrain_debug=` values. */
export type TerrainDebugMode = 'none' | 'lod' | 'morph' | 'normal' | 'height' | 'flat';
const DEBUG_CODE: Record<TerrainDebugMode, number> = { none: 0, lod: 1, morph: 2, normal: 3, height: 4, flat: 5 };

export function parseTerrainDebug(v: string | null): TerrainDebugMode {
  return v === 'lod' || v === 'morph' || v === 'normal' || v === 'height' || v === 'flat' ? v : 'none';
}

export interface TerrainStats {
  /** Visible (post-culling) instance count. */
  nodes: number;
  /** LOD cut size before frustum culling. */
  leaves: number;
  perLod: number[];
  /** CPU time of the last node selection, ms. */
  selectMs: number;
  /** nodes * 8 192. */
  triangles: number;
  /** Bytes of both height textures as uploaded (RG8). */
  textureBytes: number;
  /** Time from `Terrain` construction start to the first completed `update`, ms (null until then). */
  firstUpdateMs: number | null;
}

export interface TerrainOptions {
  kind: 'webgpu' | 'webgl2';
  debug?: TerrainDebugMode;
}

const MAX_NODES = 2048;

/**
 * CDLOD terrain: one shared 65x65 grid mesh, instanced once per selected quadtree node (a single draw call),
 * heights sampled in the vertex shader. Name prefix `PH_V-NAT-007` = grey-box ground material placeholder (ASSET_GAPS §3).
 */
export class Terrain {
  readonly mesh: Mesh;
  readonly material: ShaderMaterial;
  readonly stats: TerrainStats;
  private readonly data: TerrainData;
  private readonly tree: CdlodQuadtree;
  private readonly selection: NodeSelection;
  private readonly instances: Buffer;
  private readonly textures: RawTexture[];
  private readonly camPos = new Vector3();
  private readonly planes = new Float32Array(24);
  private readonly vp = new Matrix();
  private readonly t0 = performance.now();
  private debug: TerrainDebugMode;

  constructor(scene: Scene, data: TerrainData, opts: TerrainOptions) {
    this.data = data;
    this.debug = opts.debug ?? 'none';
    const engine = scene.getEngine();
    if (data.meta.height.res_m !== 1) throw new Error('Terrain: near height map must be 1 m resolution');

    const layout = makeLayout(data.nearSize, data.farExtent);
    this.tree = new CdlodQuadtree(layout, buildNodeBounds(layout, (x0, z0, x1, z1) => data.leafRange(x0, z0, x1, z1)));
    this.selection = makeSelection(MAX_NODES);

    // Shared grid mesh + instance buffer.
    const grid = buildGridMesh();
    const mesh = new Mesh('PH_V-NAT-007_terrain', scene);
    const vd = new VertexData();
    vd.positions = grid.positions;
    vd.indices = grid.indices;
    vd.applyToMesh(mesh, false);
    this.instances = new Buffer(engine, new Float32Array(MAX_NODES * INSTANCE_STRIDE), true, INSTANCE_STRIDE, false, true);
    mesh.setVerticesBuffer(this.instances.createVertexBuffer('inst0', 0, 4));
    mesh.setVerticesBuffer(this.instances.createVertexBuffer('inst1', 4, 4));
    mesh.alwaysSelectAsActiveMesh = true;
    mesh.isPickable = false;
    mesh.doNotSyncBoundingInfo = true;
    mesh.forcedInstanceCount = 1;
    this.mesh = mesh;

    // Height textures: raw little-endian u16 uploaded as RG8 (R = low byte, G = high byte).
    const mk = (name: string, bytes: Uint8Array, n: number): RawTexture => {
      const t = new RawTexture(bytes, n, n, Constants.TEXTUREFORMAT_RG, scene, false, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE, Constants.TEXTURETYPE_UNSIGNED_BYTE);
      t.name = name;
      t.wrapU = Constants.TEXTURE_CLAMP_ADDRESSMODE;
      t.wrapV = Constants.TEXTURE_CLAMP_ADDRESSMODE;
      return t;
    };
    const near = mk('PH_V-NAT-007_height_near', data.nearBytes, data.meta.height.samples);
    const far = mk('PH_V-NAT-007_height_far', data.farBytes, data.meta.far_ring.samples);
    this.textures = [near, far];

    const wgsl = opts.kind === 'webgpu';
    const mat = new ShaderMaterial(
      'PH_V-NAT-007',
      scene,
      wgsl ? { vertexSource: TERRAIN_VERTEX_WGSL, fragmentSource: TERRAIN_FRAGMENT_WGSL } : { vertexSource: TERRAIN_VERTEX_GLSL, fragmentSource: TERRAIN_FRAGMENT_GLSL },
      {
        attributes: [VertexBuffer.PositionKind, 'inst0', 'inst1'],
        uniforms: ['viewProjection', 'uCam', 'uHeightRange', 'uMap', 'uMap2', 'uDebug'],
        samplers: ['heightNear', 'heightFar'],
        shaderLanguage: wgsl ? ShaderLanguage.WGSL : ShaderLanguage.GLSL,
      },
    );
    mat.backFaceCulling = false;
    mat.setTexture('heightNear', near);
    mat.setTexture('heightFar', far);
    mat.setVector2('uHeightRange', new Vector2(data.hmin, data.hmax));
    mat.setVector4('uMap', new Vector4(data.nearSize, data.meta.height.samples, data.far.originX, data.meta.far_ring.res_m));
    mat.setVector2('uMap2', new Vector2(data.meta.far_ring.samples, NEAR_BLEND_M));
    mat.setFloat('uDebug', DEBUG_CODE[this.debug]);
    mesh.material = mat;
    this.material = mat;

    this.stats = {
      nodes: 0,
      leaves: 0,
      perLod: new Array<number>(LOD_COUNT).fill(0),
      selectMs: 0,
      triangles: 0,
      textureBytes: data.uploadBytes,
      firstUpdateMs: null,
    };
    // keep the cell-count contract visible to type-checkers
    void GRID_CELLS;
  }

  setDebug(mode: TerrainDebugMode): void {
    this.debug = mode;
    this.material.setFloat('uDebug', DEBUG_CODE[mode]);
  }

  /** CPU surface height (same function as the vertex shader; inside the near block it equals sim `height_at`). */
  heightAt(x: number, z: number): number {
    return this.data.heightAt(x, z);
  }

  /** Per-frame: select nodes for `camera` and upload the instance buffer (once). */
  update(camera: Camera): void {
    const view = camera.getViewMatrix(true);
    const proj = camera.getProjectionMatrix(true);
    view.multiplyToRef(proj, this.vp);
    frustumPlanes(this.vp, this.planes);
    const p = camera.globalPosition;
    this.camPos.copyFrom(p);
    this.tree.select(p.x, p.y, p.z, this.planes, this.selection);
    const sel = this.selection;
    if (sel.count > 0) this.instances.update(sel.data.subarray(0, sel.count * INSTANCE_STRIDE));
    this.mesh.forcedInstanceCount = Math.max(sel.count, 1);
    this.mesh.setEnabled(sel.count > 0);
    this.material.setVector3('uCam', this.camPos);

    const s = this.stats;
    s.nodes = sel.count;
    s.leaves = sel.leafCount;
    s.selectMs = sel.selectMs;
    s.triangles = sel.count * GRID_CELLS * GRID_CELLS * 2;
    for (let i = 0; i < LOD_COUNT; i++) s.perLod[i] = sel.perLod[i]!;
    s.firstUpdateMs ??= performance.now() - this.t0;
  }

  /** Copy of the last selection (diagnostics / e2e): ox, oz, size, lod, coarserMask, finerMask per node. */
  debugNodes(): number[][] {
    const out: number[][] = [];
    for (let i = 0; i < this.selection.count; i++) {
      const o = i * INSTANCE_STRIDE;
      out.push(Array.from(this.selection.data.subarray(o, o + 6)));
    }
    return out;
  }

  dispose(): void {
    this.mesh.dispose(false, true);
    this.material.dispose();
    for (const t of this.textures) t.dispose();
    this.instances.dispose();
  }
}
