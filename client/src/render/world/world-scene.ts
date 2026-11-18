/**
 * Hedgerows + buildings of a map (`?scene=world&map=carentan`, M1-051b / M1-052b): the GeoJSON layers the map pipeline
 * writes (`hedgerows.geojson`, `buildings.geojson`, map-local metres) drawn as
 *  - one thin-instanced box mesh for all hedge segments (placeholder `V-HEDGE-PLACEHOLDER`, 1 draw call), and
 *  - extruded building massing merged per CELL_M cell (placeholder `V-BLD-PLACEHOLDER`, ~100 draw calls).
 * Both are grey-box stand-ins until M1-064 delivers assets; they exist so the M1 camera scenes carry the real
 * object counts and the draw call budget (<= 60 hedges, <= 900 buildings) can be measured.
 */
import { Color3 } from '@babylonjs/core/Maths/math.color';
import { Matrix, Quaternion, Vector3 } from '@babylonjs/core/Maths/math.vector';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import { DirectionalLight } from '@babylonjs/core/Lights/directionalLight';
import { HemisphericLight } from '@babylonjs/core/Lights/hemisphericLight';
import { Mesh } from '@babylonjs/core/Meshes/mesh';
import { VertexData } from '@babylonjs/core/Meshes/mesh.vertexData';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import '@babylonjs/core/Meshes/thinInstanceMesh'; // Mesh.thinInstance* augmentation (not pulled in when vegetation is off)
import type { RenderContext } from '../engine';
import { extrudeBuilding, newArrays, type BuildingInput, type BuildingStats, type MeshArrays } from './building-mesh';
import { polylineToSegments, type HedgeProps, type HedgeSegment } from './hedge-geometry';

export const CELL_M = 128;

interface Feature<P> {
  geometry: { type: string; coordinates: unknown };
  properties: P;
}
interface Collection<P> {
  features: Feature<P>[];
}
interface BuildingProps {
  levels?: number;
  style?: string;
  enterable?: boolean;
}

export interface WorldStats {
  hedge_segments: number;
  hedge_length_m: number;
  hedge_meshes: number;
  building_count: number;
  building_cells: number;
  building_triangles: number;
  building_roof_failures: number;
  enterable: number;
  load_ms: number;
}

export interface WorldHandle {
  stats: WorldStats;
  hedges: Mesh | null;
  buildingMeshes: Mesh[];
  dispose(): void;
}

async function fetchJson<T>(url: string): Promise<T | null> {
  try {
    const r = await fetch(url);
    if (!r.ok) return null;
    return (await r.json()) as T;
  } catch {
    return null;
  }
}

/** Instance matrices (column-major 16 floats each) for hedge segments: unit box scaled to (mound width, height, length). */
export function hedgeMatrices(segs: readonly HedgeSegment[], groundAt: (x: number, z: number) => number): Float32Array {
  const out = new Float32Array(segs.length * 16);
  const m = new Matrix();
  segs.forEach((s, i) => {
    const y = groundAt(s.x, s.z);
    // yaw 0 = +z growing toward +x: rotation about +y by `yaw` maps the box's local +z onto that heading
    Matrix.ComposeToRef(new Vector3(s.moundWidth, s.height, s.len), Quaternion.RotationAxis(Vector3.Up(), s.yaw), new Vector3(s.x, y + s.height / 2 - 0.05, s.z), m);
    m.copyToArray(out, i * 16);
  });
  return out;
}

function toMesh(name: string, a: MeshArrays, scene: RenderContext['scene']): Mesh {
  const mesh = new Mesh(name, scene);
  const vd = new VertexData();
  vd.positions = a.positions;
  vd.normals = a.normals;
  vd.colors = a.colors;
  vd.indices = a.indices;
  vd.applyToMesh(mesh);
  mesh.freezeWorldMatrix();
  mesh.isPickable = false;
  mesh.alwaysSelectAsActiveMesh = false;
  return mesh;
}

export async function attachWorld(ctx: RenderContext, terrain: { heightAt(x: number, z: number): number }, params: URLSearchParams): Promise<WorldHandle> {
  const t0 = performance.now();
  const scene = ctx.scene;
  const mapId = params.get('map') ?? 'carentan';
  const base = `${import.meta.env.BASE_URL}maps/${mapId}/`;
  const [hedgesJson, buildingsJson] = await Promise.all([fetchJson<Collection<HedgeProps>>(`${base}hedgerows.geojson`), fetchJson<Collection<BuildingProps>>(`${base}buildings.geojson`)]);

  // The scene may already carry a directional light; walls facing away from it would be black without an ambient term.
  if (!scene.getLightByName('PH_world_sky')) {
    if (scene.lights.length === 0) {
      const sun = new DirectionalLight('PH_world_sun', new Vector3(-0.4, -1, -0.3), scene);
      sun.intensity = 0.9;
    }
    const sky = new HemisphericLight('PH_world_sky', new Vector3(0, 1, 0), scene);
    sky.intensity = 0.8;
    sky.groundColor = new Color3(0.4, 0.38, 0.35);
  }

  const stats: WorldStats = { hedge_segments: 0, hedge_length_m: 0, hedge_meshes: 0, building_count: 0, building_cells: 0, building_triangles: 0, building_roof_failures: 0, enterable: 0, load_ms: 0 };
  const disposables: { dispose(): void }[] = [];

  // Hedgerows: one mesh, thin instances. Gap segments neither block nor draw.
  let hedges: Mesh | null = null;
  if (hedgesJson) {
    const segs: HedgeSegment[] = [];
    for (const f of hedgesJson.features) {
      if (f.geometry.type !== 'LineString') continue;
      for (const s of polylineToSegments(f.geometry.coordinates as [number, number][], f.properties)) if (!s.gap) segs.push(s);
    }
    stats.hedge_segments = segs.length;
    stats.hedge_length_m = segs.reduce((a, s) => a + s.len, 0);
    if (segs.length > 0) {
      hedges = MeshBuilder.CreateBox('V-HEDGE-PLACEHOLDER', { size: 1 }, scene);
      const mat = new StandardMaterial('PH_V-HEDGE-PLACEHOLDER', scene);
      mat.diffuseColor = Color3.FromHexString('#3f5e2c');
      mat.specularColor = Color3.Black();
      hedges.material = mat;
      hedges.thinInstanceSetBuffer('matrix', hedgeMatrices(segs, (x, z) => terrain.heightAt(x, z)), 16, true);
      hedges.thinInstanceRefreshBoundingInfo(true);
      hedges.isPickable = false;
      stats.hedge_meshes = 1;
      disposables.push(hedges, mat);
    }
  }

  // Buildings: extrude and merge per cell.
  const buildingMeshes: Mesh[] = [];
  if (buildingsJson) {
    const cells = new Map<number, { arrays: MeshArrays; n: number }>();
    const bs: BuildingStats = { buildings: 0, triangles: 0, roofFailures: 0 };
    for (const f of buildingsJson.features) {
      if (f.geometry.type !== 'Polygon') continue;
      const ring = (f.geometry.coordinates as [number, number][][])[0];
      if (!ring || ring.length < 4) continue;
      const input: BuildingInput = { ring, levels: f.properties.levels ?? 2, style: f.properties.style ?? 'default', enterable: f.properties.enterable === true };
      if (input.enterable) stats.enterable++;
      let cx = 0;
      let cz = 0;
      for (const p of ring) {
        cx += p[0];
        cz += p[1];
      }
      const key = Math.floor(cz / ring.length / CELL_M) * 1000 + Math.floor(cx / ring.length / CELL_M);
      let cell = cells.get(key);
      if (!cell) cells.set(key, (cell = { arrays: newArrays(), n: 0 }));
      extrudeBuilding(input, (x, z) => terrain.heightAt(x, z), cell.arrays, bs);
      cell.n++;
    }
    const mat = new StandardMaterial('PH_V-BLD-PLACEHOLDER', scene);
    mat.specularColor = Color3.Black();
    mat.diffuseColor = Color3.White(); // colour comes from the vertex colours
    disposables.push(mat);
    for (const [key, cell] of [...cells.entries()].sort((a, b) => a[0] - b[0])) {
      const m = toMesh(`V-BLD-PLACEHOLDER_cell${key}`, cell.arrays, scene);
      m.material = mat;
      m.useVertexColors = true;
      buildingMeshes.push(m);
    }
    stats.building_count = bs.buildings;
    stats.building_cells = buildingMeshes.length;
    stats.building_triangles = bs.triangles;
    stats.building_roof_failures = bs.roofFailures;
  }
  stats.load_ms = performance.now() - t0;
  return {
    stats,
    hedges,
    buildingMeshes,
    dispose() {
      for (const m of buildingMeshes) m.dispose();
      for (const d of disposables) d.dispose();
    },
  };
}
