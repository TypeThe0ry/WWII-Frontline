// Generates the self-made minimal glTF fixtures (M1-001). No third-party content: plain triangles and boxes written
// by this script (CC0 / public domain dedication by the project). Run `node make-fixtures.mjs` and refresh SHA256SUMS:
//   sha256sum *.gltf > SHA256SUMS
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

function boxGeometry(sx, sy, sz, offset = [0, 0, 0]) {
  // Axis-aligned box centred on `offset`, 24 vertices with normals, 36 indices, outward CCW winding.
  const hx = sx / 2, hy = sy / 2, hz = sz / 2;
  const faces = [
    { n: [1, 0, 0], v: [[hx, -hy, -hz], [hx, hy, -hz], [hx, hy, hz], [hx, -hy, hz]] },
    { n: [-1, 0, 0], v: [[-hx, -hy, hz], [-hx, hy, hz], [-hx, hy, -hz], [-hx, -hy, -hz]] },
    { n: [0, 1, 0], v: [[-hx, hy, -hz], [-hx, hy, hz], [hx, hy, hz], [hx, hy, -hz]] },
    { n: [0, -1, 0], v: [[-hx, -hy, hz], [-hx, -hy, -hz], [hx, -hy, -hz], [hx, -hy, hz]] },
    { n: [0, 0, 1], v: [[-hx, -hy, hz], [hx, -hy, hz], [hx, hy, hz], [-hx, hy, hz]] },
    { n: [0, 0, -1], v: [[hx, -hy, -hz], [-hx, -hy, -hz], [-hx, hy, -hz], [hx, hy, -hz]] },
  ];
  const pos = [], nor = [], idx = [];
  for (const f of faces) {
    const base = pos.length / 3;
    for (const p of f.v) {
      pos.push(p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]);
      nor.push(...f.n);
    }
    idx.push(base, base + 1, base + 2, base, base + 2, base + 3);
  }
  return { pos: new Float32Array(pos), nor: new Float32Array(nor), idx: new Uint16Array(idx) };
}

function pack(chunks) {
  // chunks: array of TypedArray; returns {uri, views[{byteOffset, byteLength}]}
  const bufs = chunks.map((c) => Buffer.from(c.buffer, c.byteOffset, c.byteLength));
  const views = [];
  let off = 0;
  const padded = [];
  for (const b of bufs) {
    views.push({ buffer: 0, byteOffset: off, byteLength: b.length });
    const pad = (4 - (b.length % 4)) % 4;
    padded.push(b, Buffer.alloc(pad));
    off += b.length + pad;
  }
  const all = Buffer.concat(padded);
  return { uri: 'data:application/octet-stream;base64,' + all.toString('base64'), length: all.length, views };
}

function minmax(arr, n) {
  const mn = new Array(n).fill(Infinity), mx = new Array(n).fill(-Infinity);
  for (let i = 0; i < arr.length; i += n) for (let k = 0; k < n; k++) { mn[k] = Math.min(mn[k], arr[i + k]); mx[k] = Math.max(mx[k], arr[i + k]); }
  return { min: mn, max: mx };
}

function write(name, gltf) {
  writeFileSync(join(here, name), JSON.stringify(gltf, null, 2) + '\n');
}

// 1. Single triangle, Y-up metres, no declarations.
{
  const pos = new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]);
  const p = pack([pos]);
  write('triangle.gltf', {
    asset: { version: '2.0', generator: 'frontline make-fixtures.mjs' },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [{ name: 'tri', mesh: 0 }],
    meshes: [{ primitives: [{ attributes: { POSITION: 0 } }] }],
    accessors: [{ bufferView: 0, componentType: 5126, count: 3, type: 'VEC3', ...minmax(pos, 3) }],
    bufferViews: [{ ...p.views[0], target: 34962 }],
    buffers: [{ byteLength: p.length, uri: p.uri }],
  });
}

// 2. 1 m cube authored in centimetres, Z-up (edge = 100 units), shifted away from the origin by a node translation.
{
  const g = boxGeometry(100, 100, 100, [0, 0, 50]);
  const p = pack([g.pos, g.nor, g.idx]);
  write('cube_cm_zup.gltf', {
    asset: { version: '2.0', generator: 'frontline make-fixtures.mjs', extras: { unit: 'cm', up_axis: 'Z' } },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [{ name: 'cube', mesh: 0, translation: [500, 300, 0] }],
    meshes: [{ primitives: [{ attributes: { POSITION: 0, NORMAL: 1 }, indices: 2, material: 0 }] }],
    materials: [{ name: 'grey', pbrMetallicRoughness: { baseColorFactor: [0.5, 0.5, 0.5, 1], metallicFactor: 0 } }],
    accessors: [
      { bufferView: 0, componentType: 5126, count: 24, type: 'VEC3', ...minmax(g.pos, 3) },
      { bufferView: 1, componentType: 5126, count: 24, type: 'VEC3' },
      { bufferView: 2, componentType: 5123, count: 36, type: 'SCALAR' },
    ],
    bufferViews: [{ ...p.views[0], target: 34962 }, { ...p.views[1], target: 34962 }, { ...p.views[2], target: 34963 }],
    buffers: [{ byteLength: p.length, uri: p.uri }],
  });
}

// 3. Non-symmetric box 200 x 100 x 50 cm, Z-up (x = width, y = depth, z = height), declared in the file.
{
  const g = boxGeometry(200, 100, 50, [0, 0, 25]);
  const p = pack([g.pos, g.idx]);
  write('box_cm_zup.gltf', {
    asset: { version: '2.0', generator: 'frontline make-fixtures.mjs', extras: { unit: 'cm', up_axis: 'Z', forward_axis: '+X' } },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [{ name: 'box', mesh: 0 }],
    meshes: [{ primitives: [{ attributes: { POSITION: 0 }, indices: 1 }] }],
    accessors: [
      { bufferView: 0, componentType: 5126, count: 24, type: 'VEC3', ...minmax(g.pos, 3) },
      { bufferView: 1, componentType: 5123, count: 36, type: 'SCALAR' },
    ],
    bufferViews: [{ ...p.views[0], target: 34962 }, { ...p.views[1], target: 34963 }],
    buffers: [{ byteLength: p.length, uri: p.uri }],
  });
}

// 4. Instanced mesh: 3 primitives (2x material A, 1x material B) used by two nodes -> merge to 2 primitives, 6 triangles.
{
  const pos = new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]);
  const p = pack([pos]);
  const prim = (material) => ({ attributes: { POSITION: 0 }, material });
  write('two_material_instanced.gltf', {
    asset: { version: '2.0', generator: 'frontline make-fixtures.mjs' },
    scene: 0,
    scenes: [{ nodes: [0, 1] }],
    nodes: [
      { name: 'a', mesh: 0 },
      { name: 'b', mesh: 0, translation: [2, 0, 0], scale: [2, 2, 2] },
    ],
    meshes: [{ primitives: [prim(0), prim(0), prim(1)] }],
    materials: [{ name: 'a' }, { name: 'b' }],
    accessors: [{ bufferView: 0, componentType: 5126, count: 3, type: 'VEC3', ...minmax(pos, 3) }],
    bufferViews: [{ ...p.views[0], target: 34962 }],
    buffers: [{ byteLength: p.length, uri: p.uri }],
  });
}

// 5. Animated node (translation channel) -> normalize must pass it through under a wrapper node.
{
  const pos = new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]);
  const times = new Float32Array([0, 1]);
  const trans = new Float32Array([0, 0, 0, 0, 1, 0]);
  const p = pack([pos, times, trans]);
  write('animated_triangle.gltf', {
    asset: { version: '2.0', generator: 'frontline make-fixtures.mjs', extras: { unit: 'cm' } },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [{ name: 'mover', mesh: 0 }],
    meshes: [{ primitives: [{ attributes: { POSITION: 0 } }] }],
    animations: [{ channels: [{ sampler: 0, target: { node: 0, path: 'translation' } }], samplers: [{ input: 1, output: 2, interpolation: 'LINEAR' }] }],
    accessors: [
      { bufferView: 0, componentType: 5126, count: 3, type: 'VEC3', ...minmax(pos, 3) },
      { bufferView: 1, componentType: 5126, count: 2, type: 'SCALAR', ...minmax(times, 1) },
      { bufferView: 2, componentType: 5126, count: 2, type: 'VEC3' },
    ],
    bufferViews: [{ ...p.views[0], target: 34962 }, p.views[1], p.views[2]],
    buffers: [{ byteLength: p.length, uri: p.uri }],
  });
}
console.log('fixtures written to', here);
