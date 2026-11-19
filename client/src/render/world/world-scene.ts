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
import { Ray } from '@babylonjs/core/Culling/ray';
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
import { appendSurface, appendWall, buildingAt, type BuildingInterior, type BuildingLayouts } from './building-interiors';
import { BUILDING_LOD_M } from './building-math';
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
  _osm_id?: string;
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
  interiors: number;
  interior_probes: number;
  load_ms: number;
}

export interface WorldHandle {
  stats: WorldStats;
  variants: VariantRegistry;
  /** Preview chunk residency. Counts describe the procedural hedge source. */
  stream: Readonly<StreamMetrics> | null;
  buildingMeshes: Mesh[];
  buildings: readonly BuildingInterior[];
  /** Camera room, updated before scene draw. Does not disable structural occluders. */
  indoorBuilding: string | null;
  /** First structural surface along a unit ray; open doorways do not occlude. */
  buildingOcclusion(ox: number, oy: number, oz: number, dx: number, dy: number, dz: number, maxDist: number): number | null;
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
  const [hedgesJson, buildingsJson, layouts] = await Promise.all([
    fetchJson<Collection<HedgeProps>>(`${base}hedgerows.geojson`), fetchJson<Collection<BuildingProps>>(`${base}buildings.geojson`),
    fetchJson<BuildingLayouts>(`${base}building-interiors.json`),
  ]);
  if (layouts && layouts.version !== 1) throw new Error('Unsupported building-interiors version');

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

  const stats: WorldStats = { hedge_segments: 0, hedge_length_m: 0, hedge_meshes: 0, building_count: 0, building_cells: 0, building_triangles: 0, building_roof_failures: 0, enterable: 0, interiors: 0, interior_probes: 0, load_ms: 0 };
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
  const buildings: BuildingInterior[] = [];
  const roomDetails: Mesh[] = [];
  const layoutById = new Map((layouts?.buildings ?? []).map((b) => [b.id, b]));
  if (buildingsJson) {
    const cells = new Map<number, MeshArrays>();
    const detailCells = new Map<number, MeshArrays>();
    const arraysAt = (cells: Map<number, MeshArrays>, key: number) => {
      let cell = cells.get(key);
      if (!cell) cells.set(key, (cell = newArrays()));
      return cell;
    };
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
      const cell = arraysAt(cells, key);
      const layout = input.enterable && f.properties._osm_id ? layoutById.get(f.properties._osm_id) : undefined;
      if (layout) {
        buildings.push(layout);
        const detail = arraysAt(detailCells, key);
        const before = detail.indices.length + cell.indices.length;
        for (const wall of layout.walls) appendWall(cell, wall, input.style);
        for (const surface of layout.surfaces) appendSurface(surface.kind === 'roof' ? cell : detail, surface, input.style);
        bs.buildings++;
        bs.triangles += (detail.indices.length + cell.indices.length - before) / 3;
        stats.interiors++;
        stats.interior_probes += layout.probes.length;
      } else {
        extrudeBuilding(input, (x, z) => terrain.heightAt(x, z), cell, bs);
      }
    }
    const mat = new StandardMaterial('PH_V-BLD-PLACEHOLDER', scene);
    mat.specularColor = Color3.Black();
    mat.diffuseColor = Color3.White(); // colour comes from the vertex colours
    disposables.push(mat);
    // Two-sided floor/roof surfaces are visible from inside. Other shells retain their original material.
    const detailedMat = mat.clone('PH_V-BLD-PLACEHOLDER_interior');
    detailedMat.backFaceCulling = false;
    detailedMat.twoSidedLighting = true;
    disposables.push(detailedMat);
    const upload = (name: string, arrays: MeshArrays, material: StandardMaterial) => {
      const m = toMesh(name, arrays, scene);
      m.material = material;
      m.useVertexColors = true;
      m.isPickable = true;
      m.metadata = { blocks: true, building: true };
      buildingMeshes.push(m);
      return m;
    };
    for (const [key, cell] of [...cells.entries()].sort((a, b) => a[0] - b[0])) {
      upload(`V-BLD-PLACEHOLDER_cell${key}`, cell, detailCells.has(key) ? detailedMat : mat);
      const detail = detailCells.get(key);
      if (detail) roomDetails.push(upload(`V-BLD-PLACEHOLDER_interior${key}`, detail, detailedMat));
    }
    stats.building_count = bs.buildings;
    stats.building_cells = cells.size;
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
  const ray = new Ray(Vector3.Zero(), Vector3.Zero());
  const handle: WorldHandle = {
    stats,
    variants: new VariantRegistry(params.get('quality') === 'low' || params.get('post') === 'low'),
    stream,
    buildingMeshes,
    buildings,
    indoorBuilding: null,
    buildingOcclusion(ox, oy, oz, dx, dy, dz, maxDist) {
      ray.origin.set(ox, oy, oz);
      ray.direction.set(dx, dy, dz);
      ray.length = maxDist;
      // Use structural geometry even when its detail draw is distance-culled.
      const hit = scene.pickWithRay(ray, (mesh) => mesh.metadata?.building === true);
      return hit?.hit ? hit.distance : null;
    },
    csm,
    dispose() {
      stopRooms();
      stopHedges?.();
      for (const d of disposables) d.dispose();
      csm?.dispose();
      for (const m of buildingMeshes) m.dispose();
    },
  };
  const updateRooms = () => {
    const camera = scene.activeCamera;
    if (!camera) return;
    camera.getViewMatrix();
    const { x, y, z } = camera.globalPosition;
    handle.indoorBuilding = buildingAt(buildings, x, y, z)?.id ?? null;
    for (const mesh of roomDetails) {
      const bounds = mesh.getBoundingInfo().boundingBox;
      const dx = Math.max(bounds.minimumWorld.x - x, x - bounds.maximumWorld.x, 0);
      const dz = Math.max(bounds.minimumWorld.z - z, z - bounds.maximumWorld.z, 0);
      mesh.isVisible = dx * dx + dz * dz <= BUILDING_LOD_M[2] ** 2;
    }
  };
  updateRooms();
  const stopRooms = ctx.onFrame(updateRooms, undefined, 'building_update');
  return handle;
}
