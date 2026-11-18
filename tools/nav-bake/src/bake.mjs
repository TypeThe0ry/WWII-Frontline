// M1-036: tiled Recast bake + Detour connectivity checks. Build-time only.
// usage: node src/bake.mjs --in tris.bin --params params.json --mode walk|vehicle --out navmesh.bin --report r.json [--points pts.json]
import fs from 'node:fs';
import crypto from 'node:crypto';
import { init, NavMeshQuery, QueryFilter } from 'recast-navigation';
import { generateTiledNavMesh } from 'recast-navigation/generators';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, v, i, all) => (v.startsWith('--') ? [...a, [v.slice(2), all[i + 1]]] : a), []));
const params = JSON.parse(fs.readFileSync(args.params, 'utf8'));
const P = args.mode === 'vehicle' ? params.vehicle : params.walk;

function readTris(path) {
  const b = fs.readFileSync(path);
  const nv = b.readUInt32LE(0), nt = b.readUInt32LE(4);
  const pos = new Float32Array(b.buffer.slice(b.byteOffset + 8, b.byteOffset + 8 + nv * 12));
  const idx = new Uint32Array(b.buffer.slice(b.byteOffset + 8 + nv * 12, b.byteOffset + 8 + nv * 12 + nt * 12));
  return { pos, idx, nv, nt };
}

await init();
const { pos, idx, nv, nt } = readTris(args.in);
const cs = P.cell_size, ch = P.cell_height;
const config = {
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
const res = generateTiledNavMesh(pos, idx, config);
if (!res.success) { console.error('bake failed', res.error); process.exit(2); }
const navMesh = res.navMesh;
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
const he = { x: 3, y: 12, z: 3 };
const report = {
  mode: args.mode, params_hash: paramsHash.toString('hex'), tris_in: nt, verts_in: nv,
  tiles: tiles.length, bytes: bin.length, sha256: crypto.createHash('sha256').update(bin).digest('hex'), bake_ms: bakeMs,
  config,
};
let polys = 0;
for (let i = 0; i < navMesh.getMaxTiles(); i++) { const t = navMesh.getTile(i); const h = t.header(); if (h) polys += h.polyCount(); }
report.polys = polys;
if (args.points) {
  const pts = JSON.parse(fs.readFileSync(args.points, 'utf8'));
  const near = (p) => query.findNearestPoly(p, { halfExtents: he });
  const dist = (a, b) => Math.hypot(a.x - b.x, a.y - b.y, a.z - b.z);
  report.paths = {};
  for (const [name, [a, b]] of Object.entries(pts.paths)) {
    const r = query.computePath({ x: a[0], y: a[1], z: a[2] }, { x: b[0], y: b[1], z: b[2] }, { halfExtents: he, maxPathPolys: 4096, maxStraightPathPoints: 4096 });
    let len = 0;
    if (r.success) for (let i = 1; i < r.path.length; i++) len += dist(r.path[i - 1], r.path[i]);
    // computePath succeeds for *partial* paths too: require the end of the path to reach the goal
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
      const r = query.computePath({ x: b.from[0], y: b.from[1], z: b.from[2] }, { x: b.to[0], y: b.to[1], z: b.to[2] }, { halfExtents: { x: 1.5, y: 6, z: 1.5 }, maxPathPolys: 512, maxStraightPathPoints: 512 });
      const e = r.success && r.path.length ? r.path[r.path.length - 1] : null;
      if (!(e && Math.hypot(e.x - b.to[0], e.z - b.to[2]) < 2.0)) bad.push(b.id);
    }
    report.buildings = { n: pts.buildings.length, unreachable: bad.length, unreachable_ids: bad.slice(0, 50) };
  }
  if (pts.water) { const d = probe(pts.water); report.water = { n: d.length, min_dist_m: Math.min(...d), covered: d.filter((v) => v < 0.5).length }; }
  if (pts.road) { const d = probe(pts.road); report.road = { n: d.length, covered: d.filter((v) => v < 0.5).length }; }
}
fs.writeFileSync(args.report, JSON.stringify(report, null, 1));
console.log(JSON.stringify({ mode: args.mode, tiles: tiles.length, polys, bytes: bin.length, ms: bakeMs, paths: report.paths }));
