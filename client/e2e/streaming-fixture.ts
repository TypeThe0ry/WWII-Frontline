import { AssetLoader, CacheLayer, crc32, detectBackends, type Manifest } from '../src/render/assets';
import { ChunkStreamer, type ChunkCoord } from '../src/render/world/chunk-streamer';
import { fpakChunkSource, loadChunkIndex } from '../src/render/world/chunk-stream-source';
import { chunksAround, R_NEAR } from '../src/render/world/chunk-streamer-core';

const chunk = { bx: 5, bz: 8 };
const indexId = 'chunks_index';
const visualId = 'terrain_b_5_8';
const collisionId = 'terrain_c_5_8';
const path = '/stream-fixture.fpak';

async function deflate(bytes: Uint8Array<ArrayBuffer>) {
  const compressed = new Blob([bytes]).stream().pipeThrough(new CompressionStream('deflate'));
  return new Uint8Array(await new Response(compressed).arrayBuffer());
}

async function bakedVisual() {
  const heights = new Uint8Array(65 * 65 * 2);
  heights.set([255, 2]);
  heights[65 * 65] = 255; // initial sample = 65535, next sample wraps to 1
  heights[65] = 7; // new row is relative to the preceding row's first sample
  const raw = [heights, new Uint8Array(8 * 64 * 64).fill(12), Uint8Array.of(91, 93), Uint8Array.of(0), Uint8Array.of(0)];
  const compressed = await Promise.all(raw.map(deflate));
  const headerSize = 12 + 16 * raw.length;
  const bytes = new Uint8Array(headerSize + compressed.reduce((sum, part) => sum + part.length, 0));
  const d = new DataView(bytes.buffer);
  d.setUint32(0, 0x4643484b);
  d.setUint8(4, 1);
  d.setUint16(6, chunk.bx, true);
  d.setUint16(8, chunk.bz, true);
  d.setUint16(10, raw.length, true);
  let offset = headerSize;
  compressed.forEach((part, i) => {
    const section = 12 + 16 * i;
    d.setUint8(section, i + 1);
    d.setUint8(section + 1, 1);
    d.setUint32(section + 4, offset, true);
    d.setUint32(section + 8, part.length, true);
    d.setUint32(section + 12, raw[i]!.length, true);
    bytes.set(part, offset);
    offset += part.length;
  });
  return bytes;
}

async function bakedCollision() {
  const bytes = new Uint8Array(16 + 65 * 65 * 2 + 64 * 64);
  const d = new DataView(bytes.buffer);
  d.setUint32(0, 0x46434f4c);
  d.setUint8(4, 1);
  d.setUint16(6, 65, true);
  d.setFloat32(12, 16, true);
  return deflate(bytes);
}

async function packFixture() {
  const [visual, collision] = await Promise.all([bakedVisual(), bakedCollision()]);
  const encode = new TextEncoder();
  const index = encode.encode(JSON.stringify({
    schema: 'frontline.map.chunk_index/1', format_version: 1, chunk_m: 64, count: 1,
    chunks: [{ ...chunk, key: 'terrain_carentan/b_5_8', files: ['b_5_8.fchk', 'c_5_8.fcol'], bytes: visual.length + collision.length }],
  }));
  const entries = [[indexId, index], [visualId, visual], [collisionId, collision]] as const;
  const ids = entries.map(([id]) => encode.encode(id));
  const dataEnd = 32 + entries.reduce((sum, [, bytes]) => sum + Math.ceil(bytes.length / 16) * 16, 0);
  const indexEnd = dataEnd + ids.reduce((sum, id) => sum + 26 + id.length, 0);
  const pak = new Uint8Array(indexEnd + 32);
  const d = new DataView(pak.buffer);
  const assets: Manifest['assets'] = {};
  let offset = 32;
  let toc = dataEnd;
  for (const [i, [id, bytes]] of entries.entries()) {
    const entry = ids[i]!;
    const sha = new Uint8Array(await crypto.subtle.digest('SHA-256', Uint8Array.from(bytes)));
    const sha256 = Array.from(sha, (b) => b.toString(16).padStart(2, '0')).join('');
    assets[id] = { kind: 'data', tier: ['medium'], placeholder: false, meta: { entry: id, pak: path, offset, bytes: bytes.length, sha256 } };
    pak.set(bytes, offset);
    d.setUint16(toc, entry.length, true);
    d.setUint8(toc + 2, 3);
    d.setBigUint64(toc + 6, BigInt(offset), true);
    d.setUint32(toc + 14, bytes.length, true);
    pak.set(sha.subarray(0, 8), toc + 18);
    pak.set(entry, toc + 26);
    toc += 26 + entry.length;
    offset += Math.ceil(bytes.length / 16) * 16;
  }
  d.setBigUint64(indexEnd, BigInt(dataEnd), true);
  d.setUint32(indexEnd + 8, indexEnd - dataEnd, true);
  d.setUint32(indexEnd + 12, entries.length, true);
  d.setUint32(indexEnd + 16, crc32(pak.subarray(dataEnd, indexEnd)), true);
  d.setUint16(indexEnd + 20, 1, true);
  d.setUint32(indexEnd + 28, 0x4b415046);
  return { manifest: { schema: 1, game_version: 'test', assets }, pak };
}

export async function packedCache() {
  const { manifest, pak } = await packFixture();
  const ranges: string[] = [];
  const fetcher = async (_url: string, opts?: RequestInit) => {
    const range = new Headers(opts?.headers).get('Range')!;
    ranges.push(range);
    const suffix = /^bytes=-(\d+)$/.exec(range);
    const span = /^bytes=(\d+)-(\d+)$/.exec(range);
    const start = suffix ? pak.length - Number(suffix[1]) : Number(span![1]);
    const end = suffix ? pak.length - 1 : Number(span![2]);
    return new Response(pak.slice(start, end + 1), { status: 206, headers: { 'Content-Range': `bytes ${start}-${end}/${pak.length}` } });
  };
  const backends = await detectBackends('m1-streaming-fixture');
  const run = async () => {
    // Separate in-memory LRUs force the second visit through the persistent browser cache.
    const cache = new CacheLayer({ backends });
    const loader = new AssetLoader({ manifest, fetcher, cache });
    await loader.init({ tier: 'medium' });
    const index = await loadChunkIndex(loader);
    const source = fpakChunkSource(loader, index, () => ({ visual: visualId, collision: collisionId }));
    try {
      const signal = new AbortController().signal;
      const [result, collision] = await Promise.all([source.load(chunk, signal), source.loadCollision(chunk, signal)]);
      return { heights: [result.data.heights[0], result.data.heights[1], result.data.heights[65]], splat: result.data.splat[0],
        collisionBytes: collision.length, uploadBytes: result.uploadBytes, requests: ranges.length, cacheMode: cache.mode };
    } finally {
      source.dispose();
    }
  };
  const cold = await run();
  const warm = await run();
  return { cold, warm };
}

export async function traverse(speed: number) {
  const resident = new Set<string>();
  const loads = new Map<string, number>();
  const tile = ({ bx, bz }: ChunkCoord) => `${bx},${bz}`;
  const missing = '9,16';
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 9;
  const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
  const streamer = new ChunkStreamer({ radius: R_NEAR, retainRadius: R_NEAR,
    async load(c) {
      const key = tile(c);
      loads.set(key, (loads.get(key) ?? 0) + 1);
      if (key === missing) throw new Error('HTTP 404');
      return { data: key, uploadBytes: 120_000 };
    },
    upload: (_c, key) => { resident.add(key); }, evict: (c) => { resident.delete(tile(c)); }, hasFallback: () => true,
  });
  const timeStep = 0.25;
  const frames = 1000 / (timeStep * speed);
  const samples: number[] = [];
  const upload: number[] = [];
  let maxUploadBytes = 0;
  let holes = 0;
  let uncoveredPixels = 0;
  try {
    for (let i = 0; i <= frames; i++) {
      const pos = { x: 256 + speed * i * timeStep, z: 16 * 64 + 32 };
      const t = performance.now();
      streamer.update(pos, { x: speed, z: 0 }, () => true, timeStep);
      samples.push(performance.now() - t);
      holes += streamer.metrics.stream_holes;
      upload.push(streamer.metrics.stream_upload_ms);
      maxUploadBytes = Math.max(maxUploadBytes, streamer.metrics.stream_upload_bytes);
      ctx.clearRect(0, 0, 9, 9);
      const bx = Math.floor(pos.x / 64);
      for (const c of chunksAround(bx, 16, R_NEAR)) {
        ctx.fillStyle = resident.has(tile(c)) ? '#276b30' : '#9c9568';
        ctx.fillRect(c.bx - bx + 4, c.bz - 12, 1, 1);
      }
      const rgba = ctx.getImageData(0, 0, 9, 9).data;
      for (let p = 3; p < rgba.length; p += 4) if (rgba[p] === 0) uncoveredPixels++;
      await Promise.resolve();
    }
    const p99 = (values: number[]) => values.sort((a, b) => a - b)[Math.ceil(values.length * 0.99) - 1];
    return { holes, uncoveredPixels, maxUploadBytes, updateP99Ms: p99(samples), uploadP99Ms: p99(upload), frames: frames + 1,
      resident: resident.size, failedAttempts: loads.get(missing) };
  } finally {
    streamer.dispose();
  }
}
