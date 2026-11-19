// M1-036: tiled Recast bake (with ground/wade/door area marking, per-tile timing, island check,
// off-mesh validation) + Detour connectivity checks. Build-time only.
// usage: node src/bake.mjs --in tris.bin --params params.json --mode walk|vehicle --out navmesh.bin --report r.json [--points pts.json] [--areas areas.json] [--offmesh offmesh.json]
import fs from 'node:fs';
import crypto from 'node:crypto';
import {
  init, Recast, Detour, RecastBuildContext, NavMesh, NavMeshParams, RecastChunkyTriMesh,
  VerticesArray, TrianglesArray, ChunkIdsArray, TriangleAreasArray,
  allocHeightfield, freeHeightfield, createHeightfield,
  markWalkableTriangles, rasterizeTriangles,
  filterLowHangingWalkableObstacles, filterLedgeSpans, filterWalkableLowHeightSpans,
  allocCompactHeightfield, freeCompactHeightfield, buildCompactHeightfield,
  erodeWalkableArea, markBoxArea, buildDistanceField, buildRegions,
  allocContourSet, freeContourSet, buildContours,
  allocPolyMesh, freePolyMesh, buildPolyMesh,
  allocPolyMeshDetail, freePolyMeshDetail, buildPolyMeshDetail,
  NavMeshCreateParams, createNavMeshData, cloneRcConfig, statusFailed,
  NavMeshQuery, QueryFilter,
} from 'recast-navigation';
import {
  buildTiledNavMeshRcConfig, getBoundingBox, tiledNavMeshGeneratorConfigDefaults,
} from 'recast-navigation/generators';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, v, i, all) => (v.startsWith('--') ? [...a, [v.slice(2), all[i + 1]]] : a), []));
const params = JSON.parse(fs.readFileSync(args.params, 'utf8'));
const P = args.mode === 'vehicle' ? params.vehicle : params.walk;
const AREAS = params.areas ?? { ground: 1, wade: 2, door: 3, water: 0 };
const areaBoxes = args.areas && fs.existsSync(args.areas) ? JSON.parse(fs.readFileSync(args.areas, 'utf8')) : [];

function readTris(path) {
  const b = fs.readFileSync(path);
  const nv = b.readUInt32LE(0), nt = b.readUInt32LE(4);
  const pos = new Float32Array(b.buffer.slice(b.byteOffset + 8, b.byteOffset + 8 + nv * 12));
  const idx = new Uint32Array(b.buffer.slice(b.byteOffset + 8 + nv * 12, b.byteOffset + 8 + nv * 12 + nt * 12));
  return { pos, idx, nv, nt };
}

function buildTile(verticesArray, trianglesArray, rcConfig, chunkyTriMesh, tile, boxes, buildContext) {
  const tileConfig = cloneRcConfig(rcConfig);
  const bmin = [tile.bmin[0] - tileConfig.borderSize * tileConfig.cs, tile.bmin[1], tile.bmin[2] - tileConfig.borderSize * tileConfig.cs];
  const bmax = [tile.bmax[0] + tileConfig.borderSize * tileConfig.cs, tile.bmax[1], tile.bmax[2] + tileConfig.borderSize * tileConfig.cs];
  for (let i = 0; i < 3; i++) { tileConfig.set_bmin(i, bmin[i]); tileConfig.set_bmax(i, bmax[i]); }

  let hf = allocHeightfield();
  let chf = null, cset = null, pmesh = null, dmesh = null;
  const cleanup = () => {
    if (hf) { freeHeightfield(hf); hf = null; }
    if (chf) { freeCompactHeightfield(chf); chf = null; }
    if (cset) { freeContourSet(cset); cset = null; }
    if (pmesh) { freePolyMesh(pmesh); pmesh = null; }
    if (dmesh) { freePolyMeshDetail(dmesh); dmesh = null; }
  };
  if (!createHeightfield(buildContext, hf, tileConfig.width, tileConfig.height, bmin, bmax, tileConfig.cs, tileConfig.ch)) {
    cleanup(); return null;
  }
  const chunkIds = new ChunkIdsArray();
  chunkIds.resize(512);
  const nChunks = chunkyTriMesh.getChunksOverlappingRect([bmin[0], bmin[2]], [bmax[0], bmax[2]], chunkIds, 512);
  if (nChunks === 0) { chunkIds.destroy(); cleanup(); return null; }
  for (let i = 0; i < nChunks; i++) {
    const nodeId = chunkIds.get(i);
    const nTris = chunkyTriMesh.nodes(nodeId).n;
    const nodeTris = chunkyTriMesh.getNodeTris(nodeId);
    const triAreas = new TriangleAreasArray();
    triAreas.resize(nTris);
    markWalkableTriangles(buildContext, tileConfig.walkableSlopeAngle, verticesArray, trianglesArray.size, nodeTris, nTris, triAreas);
    const ok = rasterizeTriangles(buildContext, verticesArray, trianglesArray.size, nodeTris, triAreas, nTris, hf, tileConfig.walkableClimb);
    nodeTris.destroy(); triAreas.destroy();
    if (!ok) { chunkIds.destroy(); cleanup(); return null; }
  }
  chunkIds.destroy();

  filterLowHangingWalkableObstacles(buildContext, tileConfig.walkableClimb, hf);
  filterLedgeSpans(buildContext, tileConfig.walkableHeight, tileConfig.walkableClimb, hf);
  filterWalkableLowHeightSpans(buildContext, tileConfig.walkableHeight, hf);

  chf = allocCompactHeightfield();
  if (!buildCompactHeightfield(buildContext, tileConfig.walkableHeight, tileConfig.walkableClimb, hf, chf)) { cleanup(); return null; }
  freeHeightfield(hf); hf = null;
  if (!erodeWalkableArea(buildContext, tileConfig.walkableRadius, chf)) { cleanup(); return null; }

  // Mark wade (2) / door (3) area volumes overlapping this tile's expanded bounds
  for (const box of boxes) {
    if (box.bmax[0] < bmin[0] || box.bmin[0] > bmax[0] || box.bmax[2] < bmin[2] || box.bmin[2] > bmax[2]) continue;
    markBoxArea(buildContext, box.bmin, box.bmax, box.area, chf);
  }

  if (!buildDistanceField(buildContext, chf)) { cleanup(); return null; }
  if (!buildRegions(buildContext, chf, tileConfig.borderSize, tileConfig.minRegionArea, tileConfig.mergeRegionArea)) { cleanup(); return null; }
  cset = allocContourSet();
  if (!buildContours(buildContext, chf, tileConfig.maxSimplificationError, tileConfig.maxEdgeLen, cset, Recast.RC_CONTOUR_TESS_WALL_EDGES)) { cleanup(); return null; }
  pmesh = allocPolyMesh();
  if (!buildPolyMesh(buildContext, cset, tileConfig.maxVertsPerPoly, pmesh)) { cleanup(); return null; }
  dmesh = allocPolyMeshDetail();
  if (!buildPolyMeshDetail(buildContext, pmesh, chf, tileConfig.detailSampleDist, tileConfig.detailSampleMaxError, dmesh)) { cleanup(); return null; }
  freeCompactHeightfield(chf); chf = null;
  freeContourSet(cset); cset = null;

  for (let i = 0; i < pmesh.npolys(); i++) {
    const a = pmesh.areas(i);
    if (a === Recast.RC_WALKABLE_AREA) pmesh.setAreas(i, AREAS.ground);
    if (pmesh.areas(i) !== 0) pmesh.setFlags(i, 1);
  }
  const createParams = new NavMeshCreateParams();
  createParams.setPolyMeshCreateParams(pmesh);
  createParams.setPolyMeshDetailCreateParams(dmesh);
  createParams.setWalkableHeight(tileConfig.walkableHeight * tileConfig.ch);
  createParams.setWalkableRadius(tileConfig.walkableRadius * tileConfig.cs);
  createParams.setWalkableClimb(tileConfig.walkableClimb * tileConfig.ch);
  createParams.setCellSize(tileConfig.cs);
  createParams.setCellHeight(tileConfig.ch);
  createParams.setBuildBvTree(true);
  createParams.setTileX(tile.x);
  createParams.setTileY(tile.y);
  const created = createNavMeshData(createParams);
  createParams.destroy();
  cleanup();
  return created.success ? created.navMeshData : null;
}

function computeIslands(tiles) {
  // Parse Detour v7 tile blobs to count polygon area distribution & disconnected components > 4 m²
  const HDR = 100, POLY = 32, EXT = 0x8000;
  const parsed = tiles.map(({ tx, tz, data: d }) => {
    const pc = d.readInt32LE(24), vc = d.readInt32LE(28);
    const verts = [];
    for (let i = 0; i < vc; i++) { const o = HDR + i * 12; verts.push([d.readFloatLE(o), d.readFloatLE(o + 8)]); }
    const polys = [];
    const po = HDR + vc * 12;
    for (let i = 0; i < pc; i++) {
      const o = po + i * POLY;
      const n = d[o + 30], area = d[o + 31] & 0x3f;
      const vs = [], neis = [];
      for (let k = 0; k < n; k++) { vs.push(verts[d.readUInt16LE(o + 4 + k * 2)]); neis.push(d.readUInt16LE(o + 16 + k * 2)); }
      let a2 = 0;
      for (let k = 0; k < n; k++) { const p = vs[k], q = vs[(k + 1) % n]; a2 += p[0] * q[1] - q[0] * p[1]; }
      polys.push({ vs, neis, area, m2: Math.abs(a2) * 0.5 });
    }
    return { tx, tz, polys };
  });
  const byKey = new Map(parsed.map((t, i) => [`${t.tx},${t.tz}`, i]));
  const base = []; let acc = 0;
  for (const t of parsed) { base.push(acc); acc += t.polys.length; }
  const parent = Int32Array.from({ length: acc }, (_, i) => i);
  const find = (x) => { while (parent[x] !== x) { parent[x] = parent[parent[x]]; x = parent[x]; } return x; };
  const unite = (a, b) => { const ra = find(a), rb = find(b); if (ra !== rb) parent[ra] = rb; };
  const areaPolys = {};
  for (let ti = 0; ti < parsed.length; ti++) {
    const t = parsed[ti];
    for (let pi = 0; pi < t.polys.length; pi++) {
      const p = t.polys[pi], g = base[ti] + pi;
      areaPolys[p.area] = (areaPolys[p.area] ?? 0) + 1;
      for (let e = 0; e < p.vs.length; e++) {
        const nei = p.neis[e];
        if (!nei) continue;
        if ((nei & EXT) === 0) unite(g, base[ti] + nei - 1);
        else {
          const side = nei & 0xff;
          const dxy = side === 0 ? [1, 0] : side === 2 ? [0, 1] : side === 4 ? [-1, 0] : side === 6 ? [0, -1] : null;
          if (!dxy) continue;
          const nt = byKey.get(`${t.tx + dxy[0]},${t.tz + dxy[1]}`);
          if (nt === undefined) continue;
          const want = EXT | ((side + 4) & 7);
          const ax = dxy[0] !== 0 ? 1 : 0;
          const a = p.vs[e], bb = p.vs[(e + 1) % p.vs.length];
          const lo = Math.min(a[ax], bb[ax]), hi = Math.max(a[ax], bb[ax]);
          const qlist = parsed[nt].polys;
          for (let qi = 0; qi < qlist.length; qi++) {
            const q = qlist[qi];
            for (let f = 0; f < q.vs.length; f++) {
              if (q.neis[f] !== want) continue;
              const qa = q.vs[f], qb = q.vs[(f + 1) % q.vs.length];
              if (Math.min(hi, Math.max(qa[ax], qb[ax])) - Math.max(lo, Math.min(qa[ax], qb[ax])) > 1e-3) {
                unite(g, base[nt] + qi);
              }
            }
          }
        }
      }
    }
  }
  const compArea = new Map();
  for (let ti = 0; ti < parsed.length; ti++) {
    for (let pi = 0; pi < parsed[ti].polys.length; pi++) {
      const r = find(base[ti] + pi);
      compArea.set(r, (compArea.get(r) ?? 0) + parsed[ti].polys[pi].m2);
    }
  }
  const sorted = [...compArea.values()].sort((a, b) => b - a);
  const islandsOver4 = sorted.slice(1).filter((a) => a > 4.0);
  return {
    area_polys: areaPolys,
    components: sorted.length,
    main_component_m2: Math.round((sorted[0] ?? 0) * 10) / 10,
    islands_over_4m2: islandsOver4.length,
    largest_island_m2: Math.round((islandsOver4[0] ?? 0) * 10) / 10,
  };
}

await init();
const { pos, idx, nv, nt } = readTris(args.in);
const cs = P.cell_size, ch = P.cell_height;
const generatorConfig = {
  ...tiledNavMeshGeneratorConfigDefaults,
  cs, ch,
  walkableSlopeAngle: P.max_slope,
  walkableHeight: Math.ceil(P.agent_height / ch),
  walkableClimb: Math.floor(P.max_climb / ch),
  walkableRadius: Math.ceil(P.agent_radius / cs),
  maxEdgeLen: Math.round((P.max_edge_len_m ?? 3.6) / cs), maxSimplificationError: 1.3, minRegionArea: 8, mergeRegionArea: 20,
  maxVertsPerPoly: 6, detailSampleDist: 6, detailSampleMaxError: 1,
  tileSize: Math.round(params.tile_size_m / cs),
  borderSize: Math.ceil(P.agent_radius / cs) + 3,
};

const t0 = Date.now();
const buildContext = new RecastBuildContext();
const verticesArray = new VerticesArray(); verticesArray.copy(pos);
const trianglesArray = new TrianglesArray(); trianglesArray.copy(idx);
const { bbMin, bbMax } = getBoundingBox(pos, idx);
const { config: rcConfig, tileWidth, tileHeight, tcs, orig, maxTiles, maxPolysPerTile } = buildTiledNavMeshRcConfig({
  recastConfig: generatorConfig, navMeshBounds: [bbMin, bbMax],
});
const navMesh = new NavMesh();
const navMeshParams = NavMeshParams.create({
  orig, tileWidth: generatorConfig.tileSize * cs, tileHeight: generatorConfig.tileSize * cs,
  maxTiles, maxPolys: maxPolysPerTile,
});
if (!navMesh.initTiled(navMeshParams)) { console.error('initTiled failed'); process.exit(2); }
const chunkyTriMesh = new RecastChunkyTriMesh();
if (!chunkyTriMesh.init(verticesArray, trianglesArray, nt, generatorConfig.chunkyTriMeshTrisPerChunk)) { console.error('chunkyTriMesh failed'); process.exit(2); }

const tileMs = [];
for (let ty = 0; ty < tileHeight; ty++) {
  for (let tx = 0; tx < tileWidth; tx++) {
    const tbmin = [bbMin[0] + tx * tcs, bbMin[1], bbMin[2] + ty * tcs];
    const tbmax = [bbMin[0] + (tx + 1) * tcs, bbMax[1], bbMin[2] + (ty + 1) * tcs];
    const ts0 = performance.now();
    const data = buildTile(verticesArray, trianglesArray, rcConfig, chunkyTriMesh, { x: tx, y: ty, bmin: tbmin, bmax: tbmax }, areaBoxes, buildContext);
    tileMs.push(performance.now() - ts0);
    if (data) {
      navMesh.removeTile(navMesh.getTileRefAt(tx, ty, 0));
      const addRes = navMesh.addTile(data, Detour.DT_TILE_FREE_DATA, 0);
      if (statusFailed(addRes.status)) data.destroy();
    }
  }
}
chunkyTriMesh.destroy();
verticesArray.destroy();
trianglesArray.destroy();
buildContext.destroy();
const bakeMs = Date.now() - t0;

// ---- serialise: FNAV header + tile blobs
const tiles = [];
for (let i = 0; i < navMesh.getMaxTiles(); i++) {
  const t = navMesh.getTile(i);
  const n = t.dataSize();
  const h = t.header();
  if (!n || !h) continue;
  const d = Buffer.alloc(n);
  for (let k = 0; k < n; k++) d[k] = t.data(k);
  tiles.push({ tx: h.x(), tz: h.y(), data: d });
}
const paramsHash = args['params-hash'] ? Buffer.from(args['params-hash'], 'hex') : crypto.createHash('sha256').update(JSON.stringify(params)).digest().subarray(0, 8);
const head = Buffer.alloc(4 + 2 + 2 + 8 + 4 + 4 * 3 + 4);
let o = 0;
head.write('FNAV', o); o += 4;
head.writeUInt16LE(1, o); o += 2;
head.writeUInt16LE(args.mode === 'vehicle' ? 1 : 0, o); o += 2;
paramsHash.copy(head, o); o += 8;
head.writeUInt32LE(tiles.length, o); o += 4;
head.writeFloatLE(params.origin[0], o); o += 4;
head.writeFloatLE(params.origin[1], o); o += 4;
head.writeFloatLE(params.origin[2], o); o += 4;
head.writeFloatLE(params.tile_size_m, o); o += 4;
const parts = [head];
for (const t of tiles) {
  const th = Buffer.alloc(12);
  th.writeInt32LE(t.tx, 0); th.writeInt32LE(t.tz, 4); th.writeUInt32LE(t.data.length, 8);
  parts.push(th, t.data);
}
const bin = Buffer.concat(parts);
fs.writeFileSync(args.out, bin);

// ---- queries
const query = new NavMeshQuery(navMesh, { maxNodes: 65535 });
const filter = new QueryFilter();
filter.setAreaCost(AREAS.ground, 1.0);
filter.setAreaCost(AREAS.wade, 2.0);
filter.setAreaCost(AREAS.door, 1.0);
const he = { x: 3, y: 12, z: 3 };
const sortedTileMs = [...tileMs].sort((a, b) => a - b);
const islandStats = computeIslands(tiles);
const report = {
  mode: args.mode, params_hash: paramsHash.toString('hex'), tris_in: nt, verts_in: nv,
  tiles: tiles.length, bytes: bin.length, sha256: crypto.createHash('sha256').update(bin).digest('hex'), bake_ms: bakeMs,
  max_tile_ms: Math.round((sortedTileMs[sortedTileMs.length - 1] ?? 0) * 10) / 10,
  p50_tile_ms: Math.round((sortedTileMs[Math.floor(sortedTileMs.length / 2)] ?? 0) * 10) / 10,
  area_polys: {
    ground: islandStats.area_polys[AREAS.ground] ?? 0,
    wade: islandStats.area_polys[AREAS.wade] ?? 0,
    door: islandStats.area_polys[AREAS.door] ?? 0,
  },
  islands: {
    components: islandStats.components,
    main_component_m2: islandStats.main_component_m2,
    islands_over_4m2: islandStats.islands_over_4m2,
    largest_island_m2: islandStats.largest_island_m2,
  },
  config: generatorConfig,
};
let polys = 0;
for (let i = 0; i < navMesh.getMaxTiles(); i++) { const t = navMesh.getTile(i); const h = t.header(); if (h) polys += h.polyCount(); }
report.polys = polys;
if (args.points) {
  const pts = JSON.parse(fs.readFileSync(args.points, 'utf8'));
  const near = (p) => query.findNearestPoly(p, { halfExtents: he, filter });
  const dist = (a, b) => Math.hypot(a.x - b.x, a.y - b.y, a.z - b.z);
  report.paths = {};
  for (const [name, [a, b]] of Object.entries(pts.paths)) {
    const r = query.computePath({ x: a[0], y: a[1], z: a[2] }, { x: b[0], y: b[1], z: b[2] }, { halfExtents: he, filter, maxPathPolys: 4096, maxStraightPathPoints: 4096 });
    let len = 0;
    if (r.success) for (let i = 1; i < r.path.length; i++) len += dist(r.path[i - 1], r.path[i]);
    const end = r.success && r.path.length ? r.path[r.path.length - 1] : null;
    const gap = end ? Math.hypot(end.x - b[0], end.z - b[2]) : 1e9;
    report.paths[name] = { ok: !!r.success && r.path.length > 1 && gap < 3.0, length_m: Math.round(len * 10) / 10, end_gap_m: Math.round(gap * 10) / 10 };
  }
  const probe = (list) => list.map(([x, y, z]) => {
    const r = near({ x, y, z });
    return r.success ? Math.hypot(r.nearestPoint.x - x, r.nearestPoint.z - z) : 1e9;
  });
  if (pts.buildings) {
    const bad = [];
    for (const b of pts.buildings) {
      const r = query.computePath({ x: b.from[0], y: b.from[1], z: b.from[2] }, { x: b.to[0], y: b.to[1], z: b.to[2] }, { halfExtents: { x: 1.5, y: 6, z: 1.5 }, filter, maxPathPolys: 512, maxStraightPathPoints: 512 });
      const e = r.success && r.path.length ? r.path[r.path.length - 1] : null;
      if (!(e && Math.hypot(e.x - b.to[0], e.z - b.to[2]) < 2.0)) bad.push(b.id);
    }
    report.buildings = { n: pts.buildings.length, unreachable: bad.length, unreachable_ids: bad.slice(0, 50) };
  }
  if (pts.water) { const d = probe(pts.water); report.water = { n: d.length, min_dist_m: Math.min(...d), covered: d.filter((v) => v < 0.5).length }; }
  if (pts.road) { const d = probe(pts.road); report.road = { n: d.length, covered: d.filter((v) => v < 0.5).length }; }
}
if (args.offmesh && fs.existsSync(args.offmesh)) {
  const om = JSON.parse(fs.readFileSync(args.offmesh, 'utf8')).offmesh ?? [];
  const invalid = [];
  let maxDist = 0;
  for (const l of om) {
    for (const k of ['start', 'end']) {
      const [x, y, z] = l[k];
      const r = query.findNearestPoly({ x, y, z }, { halfExtents: { x: 2, y: 8, z: 2 }, filter });
      const d = r.success ? Math.hypot(r.nearestPoint.x - x, r.nearestPoint.z - z) : 1e9;
      if (d > maxDist) maxDist = d;
      if (d > 0.5) invalid.push({ id: l.id, end: k, dist_m: Math.round(d * 1000) / 1000 });
    }
  }
  report.offmesh = { count: om.length, violations: invalid.length, max_dist_m: Math.round(maxDist * 1000) / 1000, invalid: invalid.slice(0, 20) };
}
fs.writeFileSync(args.report, JSON.stringify(report, null, 1));
console.log(JSON.stringify({ mode: args.mode, tiles: tiles.length, polys, area_polys: report.area_polys, bytes: bin.length, ms: bakeMs, max_tile_ms: report.max_tile_ms, paths: report.paths }));
