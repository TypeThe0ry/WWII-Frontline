/**
 * Hedgerows + buildings of a map (`?scene=world&map=carentan`, M1-051b / M1-052b): the GeoJSON layers the map pipeline
 * writes (`hedgerows.geojson`, `buildings.geojson`, map-local metres) drawn as
 *  - one thin-instanced box mesh per occupied near hedge chunk, and
 *  - extruded building massing merged per CELL_M cell (placeholder `V-BLD-PLACEHOLDER`, ~100 draw calls).
 * Both are grey-box stand-ins until M1-064 delivers assets; they exist so the M1 camera scenes carry the real
 * object counts and the draw call budget (<= 60 hedges, <= 900 buildings) can be measured.
 */
import { Color3 } from '@babylonjs/core/Maths/math.color';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import { DirectionalLight } from '@babylonjs/core/Lights/directionalLight';
import { HemisphericLight } from '@babylonjs/core/Lights/hemisphericLight';
import { Mesh } from '@babylonjs/core/Meshes/mesh';
import { VertexData } from '@babylonjs/core/Meshes/mesh.vertexData';
import type { RenderContext } from '../engine';
import { VariantRegistry } from '../materials/variants';
import { attachCsm, type CsmHandle } from '../lighting/csm';
import type { ShadowTier } from '../lighting/csm-math';
import { extrudeBuilding, newArrays, type BuildingInput, type BuildingStats, type MeshArrays } from './building-mesh';
import { polylineToSegments, type HedgeProps, type HedgeSegment } from './hedge-geometry';
import { HedgeChunks } from './hedge-chunks';
import type { StreamMetrics } from './chunk-streamer';

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
  variants: VariantRegistry;
  /** Preview chunk residency. Counts describe the procedural hedge source. */
  stream: Readonly<StreamMetrics> | null;
  buildingMeshes: Mesh[];
  /** Cascaded shadow maps when `?csm=1` (M1-043b). */
  csm: CsmHandle | null;
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
      const sun = new DirectionalLight('PH_world_sun', new Vector3(0.45, -0.8, 0.35), scene);
      sun.intensity = 0.9;
    }
    const sky = new HemisphericLight('PH_world_sky', new Vector3(0, 1, 0), scene);
    sky.intensity = 0.8;
    sky.groundColor = new Color3(0.4, 0.38, 0.35);
  }

  const stats: WorldStats = { hedge_segments: 0, hedge_length_m: 0, hedge_meshes: 0, building_count: 0, building_cells: 0, building_triangles: 0, building_roof_failures: 0, enterable: 0, load_ms: 0 };
  const disposables: { dispose(): void }[] = [];

  const segments: HedgeSegment[] = [];
  for (const f of hedgesJson?.features ?? []) {
    if (f.geometry.type !== 'LineString') continue;
    for (const segment of polylineToSegments(f.geometry.coordinates as [number, number][], f.properties)) {
      if (!segment.gap) segments.push(segment);
    }
  }
  stats.hedge_segments = segments.length;
  stats.hedge_length_m = segments.reduce((sum, segment) => sum + segment.len, 0);

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
      m.isPickable = true; // wall probe of the first-person weapon (M1-046b)
      m.metadata = { blocks: true };
      buildingMeshes.push(m);
    }
    stats.building_count = bs.buildings;
    stats.building_cells = buildingMeshes.length;
    stats.building_triangles = bs.triangles;
    stats.building_roof_failures = bs.roofFailures;
  }
  // M1-043b: `?csm=1` puts cascaded shadow maps on the scene's directional light; buildings and hedges cast and receive
  let csm: CsmHandle | null = null;
  if (params.get('csm') === '1') {
    const sun = scene.lights.find((l): l is DirectionalLight => l instanceof DirectionalLight);
    if (sun) {
      const tier = (params.get('csm_tier') ?? 'high') as ShadowTier;
      const casc = params.get('csm_cascades');
      csm = attachCsm(sun, tier, ctx.kind === 'webgl2', casc ? { cascades: Number(casc) | 0 } : {});
      for (const m of buildingMeshes) csm.add(m);
    }
  }
  let stream: StreamMetrics | null = null;
  let stopHedges: (() => void) | undefined;
  if (segments.length > 0) {
    const material = new StandardMaterial('PH_V-HEDGE-PLACEHOLDER', scene);
    material.diffuseColor = Color3.FromHexString('#3f5e2c');
    material.specularColor = Color3.Black();
    const hedges = new HedgeChunks(scene, material, segments, (x, z) => terrain.heightAt(x, z),
      (mesh) => csm?.add(mesh), (mesh) => csm?.generator.removeShadowCaster(mesh, false));
    stream = hedges.streamer.metrics;
    let previous: { x: number; z: number } | undefined;
    const update = (dtMs: number) => {
      const camera = scene.activeCamera;
      if (!camera) return;
      camera.getViewMatrix();
      const { x, z } = camera.globalPosition;
      const pos = { x, z };
      const dt = dtMs / 1000;
      const vel = previous && dt > 0 ? { x: (x - previous.x) / dt, z: (z - previous.z) / dt } : { x: 0, z: 0 };
      hedges.streamer.update(pos, vel, () => true, dt);
      previous = pos;
      stats.hedge_meshes = hedges.activeMeshes;
    };
    update(0);
    stopHedges = ctx.onFrame(update, undefined, 'hedgerow_update');
    disposables.push(hedges, material);
  }
  stats.load_ms = performance.now() - t0;
  return {
    stats,
    variants: new VariantRegistry(params.get('quality') === 'low' || params.get('post') === 'low'),
    stream,
    buildingMeshes,
    csm,
    dispose() {
      stopHedges?.();
      for (const d of disposables) d.dispose();
      csm?.dispose();
      for (const m of buildingMeshes) m.dispose();
    },
  };
}
