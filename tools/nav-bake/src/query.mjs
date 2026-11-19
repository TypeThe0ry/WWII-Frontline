// M1-036 behaviour 8: load an FNAV binary and query a path between two (x,z) points.
// usage: node src/query.mjs --in navmesh.bin --from x,z --to x,z
import fs from 'node:fs';
import { init, NavMesh, NavMeshParams, NavMeshQuery, QueryFilter, UnsignedCharArray, statusFailed } from 'recast-navigation';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, v, i, all) => (v.startsWith('--') ? [...a, [v.slice(2), all[i + 1]]] : a), []));
const parseXZ = (s) => s.split(',').map(Number);
const [fx, fz] = parseXZ(args.from);
const [tx, tz] = parseXZ(args.to);

await init();
const b = fs.readFileSync(args.in);
if (b.subarray(0, 4).toString() !== 'FNAV') throw new Error('not an FNAV file');
const tileCount = b.readUInt32LE(16);
let o = 36, maxPolys = 0, orig = null, tw = 0, th = 0;
const tiles = [];
for (let i = 0; i < tileCount; i++) {
  const x = b.readInt32LE(o), z = b.readInt32LE(o + 4), len = b.readUInt32LE(o + 8);
  const d = b.subarray(o + 12, o + 12 + len);
  o += 12 + len;
  maxPolys = Math.max(maxPolys, d.readInt32LE(24));
  if (!orig) {
    tw = d.readFloatLE(84) - d.readFloatLE(72);
    th = d.readFloatLE(92) - d.readFloatLE(80);
    orig = { x: d.readFloatLE(72) - x * tw, y: d.readFloatLE(76), z: d.readFloatLE(80) - z * th };
  }
  tiles.push(d);
}
const nm = new NavMesh();
const maxTiles = 1 << Math.ceil(Math.log2(Math.max(1, tileCount)));
const maxPolysPow2 = 1 << Math.ceil(Math.log2(Math.max(1, maxPolys)));
if (!nm.initTiled(NavMeshParams.create({ orig, tileWidth: tw, tileHeight: th, maxTiles, maxPolys: maxPolysPow2 }))) throw new Error('initTiled failed');
for (const d of tiles) {
  const arr = new UnsignedCharArray();
  arr.copy(new Uint8Array(d));
  if (statusFailed(nm.addTile(arr, 0, 0).status)) throw new Error('addTile failed');
}
const q = new NavMeshQuery(nm, { maxNodes: 65535 });
const filter = new QueryFilter();
filter.setAreaCost(1, 1.0);
filter.setAreaCost(2, 2.0);
filter.setAreaCost(3, 1.0);
const he = { x: 4, y: 40, z: 4 };
const r = q.computePath({ x: fx, y: orig.y + 5, z: fz }, { x: tx, y: orig.y + 5, z: tz }, { halfExtents: he, filter, maxPathPolys: 4096, maxStraightPathPoints: 4096 });
let len = 0;
if (r.success) for (let i = 1; i < r.path.length; i++) len += Math.hypot(r.path[i].x - r.path[i - 1].x, r.path[i].y - r.path[i - 1].y, r.path[i].z - r.path[i - 1].z);
const end = r.success && r.path.length ? r.path[r.path.length - 1] : null;
const gap = end ? Math.hypot(end.x - tx, end.z - tz) : 1e9;
const ok = !!r.success && r.path.length > 0 && gap < 3.0;
console.log(JSON.stringify({ ok, length_m: Math.round(len * 10) / 10, end_gap_m: Math.round(gap * 10) / 10, points: r.path.length }));
process.exit(ok ? 0 : 1);
