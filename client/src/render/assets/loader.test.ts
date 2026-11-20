import { describe, expect, it } from 'vitest';
import {
  AssetLoader, CacheLayer, CancelledError, FetchRangeSource, FpakReader, MemoryLru, UploadScheduler, WorkerPool, BytesSource,
  crc32, decodeAudioEntry, defaultPoolSize, detectBackends, isPlaceholder, parseProbeJson, PH_RE, silence,
  type CacheBackend, type Manifest, type ProgressEvent,
} from './index';
import { OpfsLruTracker } from './opfs-worker';

// ---- a minimal .fpak writer (SPEC.md layout) so the tests need no Node-only code from the asset pipeline --------------------
function buildPak(entries: { id: string; kind: number; payload: Uint8Array }[]): { bytes: Uint8Array; refs: Map<string, { offset: number; bytes: number }> } {
  const parts: Uint8Array[] = [new Uint8Array(32)];
  let pos = 32;
  const refs = new Map<string, { offset: number; bytes: number }>();
  const idx: number[] = [];
  const enc = new TextEncoder();
  for (const e of entries) {
    refs.set(e.id, { offset: pos, bytes: e.payload.byteLength });
    parts.push(e.payload);
    const pad = (16 - (e.payload.byteLength % 16)) % 16;
    parts.push(new Uint8Array(pad));
    const id = enc.encode(e.id);
    const rec = new DataView(new ArrayBuffer(26));
    rec.setUint16(0, id.length, true);
    rec.setUint8(2, e.kind);
    rec.setUint16(4, 0, true);
    rec.setBigUint64(6, BigInt(pos), true);
    rec.setUint32(14, e.payload.byteLength, true);
    idx.push(...new Uint8Array(rec.buffer), ...id);
    pos += e.payload.byteLength + pad;
  }
  const index = Uint8Array.from(idx);
  const footer = new DataView(new ArrayBuffer(32));
  footer.setBigUint64(0, BigInt(pos), true);
  footer.setUint32(8, index.length, true);
  footer.setUint32(12, entries.length, true);
  footer.setUint32(16, crc32(index), true);
  footer.setUint16(20, 1, true);
  new Uint8Array(footer.buffer).set([0x4b, 0x41, 0x50, 0x46], 28);
  const all = new Uint8Array(pos + index.length + 32);
  let o = 0;
  for (const p of [...parts, index, new Uint8Array(footer.buffer)]) {
    all.set(p, o);
    o += p.byteLength;
  }
  return { bytes: all, refs };
}

const PAK_PATH = '/a/ab/cdef0123456789.fpak';
const payload = (n: number, v: number) => new Uint8Array(n).fill(v);

function world(opts: { honourRange?: boolean; missing?: string[] } = {}) {
  const defs = [
    { id: 'prop_crate@lod0', kind: 0, payload: payload(1000, 1) },
    { id: 'prop_crate@lod1', kind: 0, payload: payload(400, 2) },
    { id: 'prop_crate@tex_albedo', kind: 1, payload: payload(2000, 3) },
    { id: 'snd_gun@opus', kind: 2, payload: payload(300, 4) },
  ];
  const { bytes, refs } = buildPak(defs);
  const ref = (id: string) => ({ pak: PAK_PATH, entry: id, ...refs.get(id)!, sha256: `sha_${id}` });
  const manifest: Manifest = {
    schema: 1, game_version: 't',
    assets: {
      prop_crate: { kind: 'model', tier: ['low', 'medium'], placeholder: true, req_id: 'V-PROP-001', lods: [{ lod: 0, suffix: '', ...ref('prop_crate@lod0') }, { lod: 1, suffix: '', ...ref('prop_crate@lod1') }], textures: [{ name: 'tex_albedo', ...ref('prop_crate@tex_albedo') }] },
      snd_gun: { kind: 'audio', tier: ['medium'], placeholder: true, req_id: 'A-GUN-001', audio: [{ codec: 'opus', ...ref('snd_gun@opus') }] },
      ghost: { kind: 'model', tier: ['medium'], placeholder: true, req_id: 'V-GHOST-001', lods: [{ lod: 0, suffix: '', pak: '/a/zz/missing.fpak', entry: 'ghost@lod0', offset: 32, bytes: 10, sha256: 'x' }] },
    },
  };
  const requests: { url: string; range: string | null }[] = [];
  const fetcher = async (url: string, init?: RequestInit): Promise<Response> => {
    const range = new Headers(init?.headers).get('Range');
    requests.push({ url, range });
    if (!url.endsWith(PAK_PATH) || opts.missing?.includes(url)) return new Response('nope', { status: 404 });
    if (range && opts.honourRange !== false) {
      const m = /bytes=(-)?(\d+)(?:-(\d+))?/.exec(range)!;
      const [s, e] = m[1] ? [bytes.length - Number(m[2]), bytes.length - 1] : [Number(m[2]), Number(m[3])];
      return new Response(bytes.slice(s, e + 1) as unknown as BodyInit, { status: 206, headers: { 'Content-Range': `bytes ${s}-${e}/${bytes.length}` } });
    }
    return new Response(bytes as unknown as BodyInit, { status: 200 });
  };
  return { bytes, manifest, fetcher, requests };
}

const noSleep = async () => undefined;
const loaderFor = (w: ReturnType<typeof world>, extra: Partial<ConstructorParameters<typeof AssetLoader>[0]> = {}) =>
  new AssetLoader({ fetcher: w.fetcher, sleep: noSleep, manifest: w.manifest, log: () => undefined, ...extra });

async function drain(l: AssetLoader, p: Promise<unknown>): Promise<unknown> {
  let done = false;
  void p.finally(() => (done = true)).catch(() => undefined);
  while (!done) {
    l.uploads.runFrame();
    await new Promise((r) => setTimeout(r, 0));
  }
  return p;
}

describe('fpak reader', () => {
  it('opens through Range reads: footer, index, then one read per entry', async () => {
    const w = world();
    const src = new FetchRangeSource('http://x' + PAK_PATH, w.fetcher);
    const r = await FpakReader.open(src);
    expect(r.entries.map((e) => e.id)).toEqual(['prop_crate@lod0', 'prop_crate@lod1', 'prop_crate@tex_albedo', 'snd_gun@opus']);
    expect((await r.read('prop_crate@lod1'))[0]).toBe(2);
    expect(src.rangeSupported).toBe(true);
    expect(w.requests.every((q) => q.range !== null)).toBe(true);
  });

  it('server ignoring Range (200): downloads once, still reads entries correctly (acceptance 6)', async () => {
    const w = world({ honourRange: false });
    const src = new FetchRangeSource('http://x' + PAK_PATH, w.fetcher);
    const r = await FpakReader.open(src);
    expect((await r.read('snd_gun@opus')).byteLength).toBe(300);
    expect((await r.read('prop_crate@lod0'))[0]).toBe(1);
    expect(w.requests).toHaveLength(1);
    expect(src.rangeSupported).toBe(false);
  });

  it('rejects a corrupt index checksum', async () => {
    const w = world();
    const bad = w.bytes.slice();
    bad[bad.length - 40] = (bad[bad.length - 40] ?? 0) ^ 0xff; // inside the index
    await expect(FpakReader.open(new BytesSource(bad))).rejects.toThrow(/checksum|truncated|unknown/);
  });
});

describe('loader', () => {
  it('loads a mesh and a texture, dedups concurrent requests, resolves tier', async () => {
    const w = world();
    const l = loaderFor(w);
    await l.init({ tier: 'medium' });
    const [a, b] = await Promise.all([drain(l, l.loadMesh('prop_crate', 0)), l.loadMesh('prop_crate', 0)]);
    expect(a).toBe(b);
    expect(isPlaceholder(a)).toBe(false);
    expect((a as { bytes: Uint8Array }).bytes.byteLength).toBe(1000);
    const t = await drain(l, l.loadTexture('prop_crate@tex_albedo'));
    expect((t as { bytes: Uint8Array }).bytes[0]).toBe(3);
    const open = w.requests.filter((q) => q.range === 'bytes=-1').length;
    expect(open).toBe(1); // one pak, opened once
  });

  it('404 entry: exactly 3 retries (4 attempts) then a PH_{reqId} placeholder (acceptance 5)', async () => {
    const w = world();
    const logs: string[] = [];
    const l = loaderFor(w, { log: (m) => logs.push(m) });
    await l.init({ tier: 'medium' });
    const r = await drain(l, l.loadMesh('ghost', 0));
    expect(isPlaceholder(r)).toBe(true);
    const ph = r as { materialName: string; nodeName: string; reqId: string };
    expect(ph.materialName).toMatch(PH_RE);
    expect(ph.nodeName).toMatch(PH_RE);
    expect(ph.materialName).toBe('PH_V-GHOST-001');
    expect(l.attempts.get('ghost@lod0')).toBe(4);
    expect(logs).toHaveLength(1);
  });

  it('second load from the same cache sends no network request (acceptance 4)', async () => {
    const w = world();
    const cache = new CacheLayer();
    const first = loaderFor(w, { cache });
    await first.init({ tier: 'medium' });
    await drain(first, first.loadMesh('prop_crate', 0));
    const before = w.requests.length;
    expect(before).toBeGreaterThan(0);
    const second = loaderFor(w, { cache });
    await second.init({ tier: 'medium' });
    await drain(second, second.loadMesh('prop_crate', 0));
    expect(w.requests.length).toBe(before);
    expect(cache.hits).toBe(1);
  });

  it('no persistent backend: memory mode, runs fine (acceptance 2)', async () => {
    const w = world();
    const info: string[] = [];
    const backends = await detectBackends('t', { storage: { getDirectory: async () => { throw new Error('SecurityError'); } }, caches: undefined });
    expect(backends).toEqual([]);
    const l = loaderFor(w, { cache: new CacheLayer({ backends, onInfo: (m) => info.push(m) }) });
    await l.init({ tier: 'medium' });
    expect(l.stats().cacheMode).toBe('memory');
    const r = await drain(l, l.loadMesh('prop_crate', 1));
    expect(isPlaceholder(r)).toBe(false);
  });

  it('a failing backend is dropped and the layer falls back to memory without error', async () => {
    const broken: CacheBackend = { name: 'opfs', get: async () => { throw new Error('quota'); }, put: async () => { throw new Error('quota'); } };
    const info: string[] = [];
    const cache = new CacheLayer({ backends: [broken], onInfo: (m) => info.push(m) });
    expect(cache.mode).toBe('opfs');
    await cache.put('k', payload(4, 9));
    expect(cache.mode).toBe('memory');
    expect((await cache.get('k'))![0]).toBe(9);
    expect(info[0]).toContain('opfs disabled');
  });

  it('OPFS is skipped when the free quota is below the floor', async () => {
    const env = { storage: { getDirectory: async () => ({}) as FileSystemDirectoryHandle, estimate: async () => ({ quota: 10 * 1024 * 1024, usage: 0 }) } };
    expect(await detectBackends('t', env)).toEqual([]);
  });

  it('progress: <= 10 Hz and loaded never decreases per phase (acceptance 7)', async () => {
    const w = world();
    let t = 0;
    const events: (ProgressEvent & { t: number })[] = [];
    const l = loaderFor(w, { now: () => t, onProgress: (e) => events.push({ ...e, t }) });
    await l.init({ tier: 'medium' });
    for (let i = 0; i < 40; i++) {
      t += 10; // 100 Hz of load activity
      await drain(l, l.loadMesh('prop_crate', i % 2));
      await l.loadAudioBytes('snd_gun@opus');
    }
    const times = events.map((e) => e.t);
    for (let i = 1; i < times.length; i++) expect(times[i]! - times[i - 1]!).toBeGreaterThanOrEqual(100);
    for (const phase of ['download', 'decode', 'upload'] as const) {
      const ls = events.filter((e) => e.phase === phase).map((e) => e.loaded);
      for (let i = 1; i < ls.length; i++) expect(ls[i]!).toBeGreaterThanOrEqual(ls[i - 1]!);
    }
    expect(events.length).toBeGreaterThan(0);
  });

  it('missing audio: 100 requests -> 1 log line, 0.1 s of silence (acceptance 8)', async () => {
    const w = world();
    const logs: string[] = [];
    const l = loaderFor(w, { log: (m) => logs.push(m) });
    await l.init({ tier: 'medium' });
    for (let i = 0; i < 100; i++) {
      const r = await l.loadAudioBytes('snd_missing@opus');
      expect(r).toMatchObject({ silent: true });
      expect((r as { samples: Float32Array }).samples.length).toBe(4800);
    }
    expect(logs).toHaveLength(1);
    expect(silence(44100).length).toBe(4410);
  });

  it('cancel: aborts a pending request', async () => {
    const w = world();
    const l = loaderFor(w);
    await l.init({ tier: 'medium' });
    const token = l.createToken();
    const p = l.loadMesh('prop_crate', 0, { token });
    l.cancel(token);
    await expect(p).rejects.toBeInstanceOf(CancelledError);
  });
});

describe('upload scheduler (acceptance 3)', () => {
  it('stops at the byte budget and carries the rest to the next frame', () => {
    const s = new UploadScheduler(1.5, 2 * 1024 * 1024, () => 0);
    for (let i = 0; i < 10; i++) s.enqueue({ bytes: 700 * 1024, run: () => undefined });
    expect(s.runFrame().jobs).toBe(2); // 1.4 MB; the third would exceed 2 MB
    expect(s.pending).toBe(8);
    let frames = 1;
    while (s.pending > 0) {
      s.runFrame();
      frames++;
    }
    expect(frames).toBe(5);
    expect(s.frames.every((f) => f.bytes <= 2 * 1024 * 1024)).toBe(true);
  });

  it('stops at the time budget; the first job of a frame always runs', () => {
    let t = 0;
    const s = new UploadScheduler(1.5, 1e9, () => t);
    for (let i = 0; i < 6; i++) s.enqueue({ bytes: 10, run: () => { t += 0.8; } });
    expect(s.runFrame().jobs).toBe(2); // 0.8 ms, 1.6 ms >= 1.5 -> stop
    const big = new UploadScheduler(1.5, 100, () => 0);
    big.enqueue({ bytes: 5000, run: () => undefined });
    expect(big.runFrame().jobs).toBe(1);
  });

  it('runs lower priority numbers first, FIFO within one priority', () => {
    const s = new UploadScheduler(100, 1e9, () => 0);
    const order: string[] = [];
    s.enqueue({ bytes: 1, prio: 5, run: () => order.push('a') });
    s.enqueue({ bytes: 1, prio: 1, run: () => order.push('b') });
    s.enqueue({ bytes: 1, prio: 5, run: () => order.push('c') });
    s.runFrame();
    expect(order).toEqual(['b', 'a', 'c']);
  });

  it('200 small entries: P95 per-frame upload time stays within budget', () => {
    let t = 0;
    const s = new UploadScheduler(1.5, 2 * 1024 * 1024, () => t);
    for (let i = 0; i < 200; i++) s.enqueue({ bytes: 20 * 1024, run: () => { t += 0.125; } });
    while (s.pending > 0) s.runFrame();
    const ms = s.frames.map((f) => f.ms).sort((a, b) => a - b);
    expect(ms[Math.floor(ms.length * 0.95)]!).toBeLessThanOrEqual(1.5);
  });
});

describe('worker pool', () => {
  it('size is min(4, cores - 2), at least 1', () => {
    expect([defaultPoolSize(2), defaultPoolSize(4), defaultPoolSize(8), defaultPoolSize(32)]).toEqual([1, 2, 4, 4]);
  });

  it('runs at most `size` tasks at once, by priority', async () => {
    let live = 0;
    let peak = 0;
    const order: number[] = [];
    const pool = new WorkerPool<number, number>(async (n) => {
      live++;
      peak = Math.max(peak, live);
      order.push(n);
      await new Promise((r) => setTimeout(r, 5));
      live--;
      return n;
    }, 2);
    const ts = [5, 4, 3, 2, 1].map((n) => pool.submit(n, n));
    await Promise.all(ts.map((t) => t.promise));
    expect(peak).toBe(2);
        expect(order.indexOf(1)).toBeLessThan(order.indexOf(5));
  });

  it('cancelling a queued task rejects it and never runs it', async () => {
    const ran: number[] = [];
    const pool = new WorkerPool<number, number>(async (n) => { ran.push(n); await new Promise((r) => setTimeout(r, 5)); return n; }, 1);
    const a = pool.submit(1);
    const b = pool.submit(2);
    b.cancel();
    await expect(b.promise).rejects.toBeInstanceOf(CancelledError);
    await a.promise;
    expect(ran).toEqual([1]);
  });
});

describe('memory LRU', () => {
  it('evicts least recently used first and ignores oversized values', () => {
    const m = new MemoryLru(100);
    m.put('a', payload(40, 1));
    m.put('b', payload(40, 2));
    m.get('a'); // a is now most recent
    m.put('c', payload(40, 3));
    expect(m.get('b')).toBeUndefined();
    expect(m.get('a')).toBeDefined();
    m.put('big', payload(500, 9));
    expect(m.get('big')).toBeUndefined();
    expect(m.bytes).toBeLessThanOrEqual(100);
  });
});

describe('download concurrency cap (ARCHITECTURE 9.3: <= 6)', () => {
  it('limits concurrent network reads to 6 even under 20 simultaneous requests', async () => {
    const defs = Array.from({ length: 20 }, (_, i) => ({ id: `prop_c_${i}@lod0`, kind: 0, payload: payload(64, i) }));
    const { bytes, refs } = buildPak(defs);
    const assets: Manifest['assets'] = {};
    for (let i = 0; i < 20; i++) {
      const id = `prop_c_${i}@lod0`;
      assets[`prop_c_${i}`] = {
        kind: 'model', tier: ['medium'], placeholder: true,
        lods: [{ lod: 0, suffix: '', pak: PAK_PATH, entry: id, ...refs.get(id)!, sha256: `sha_${i}` }],
      };
    }
    let inflight = 0;
    let peak = 0;
    const fetcher = async (_url: string, init?: RequestInit): Promise<Response> => {
      inflight++;
      peak = Math.max(peak, inflight);
      await new Promise((r) => setTimeout(r, 8));
      inflight--;
      const range = new Headers(init?.headers).get('Range')!;
      const m = /bytes=(-)?(\d+)(?:-(\d+))?/.exec(range)!;
      const [s, e] = m[1] ? [bytes.length - Number(m[2]), bytes.length - 1] : [Number(m[2]), Number(m[3])];
      return new Response(bytes.slice(s, e + 1) as unknown as BodyInit, { status: 206, headers: { 'Content-Range': `bytes ${s}-${e}/${bytes.length}` } });
    };
    const l = new AssetLoader({ fetcher, sleep: noSleep, manifest: { schema: 1, game_version: 't', assets }, log: () => undefined });
    await l.init({ tier: 'medium' });
    const out = await Promise.all(Array.from({ length: 20 }, (_, i) => l.loadBytes(`prop_c_${i}@lod0`)));
    expect(out).toHaveLength(20);
    expect(peak).toBeLessThanOrEqual(6);
    expect(peak).toBeGreaterThan(1);
  });
});

describe('OPFS LRU tracker & sync fallback', () => {
  it('evicts oldest keys when maxBytes is exceeded and round-trips JSON index', () => {
    const t = new OpfsLruTracker(100);
    expect(t.planPut('a', 40)).toEqual({ store: true, evict: [] });
    expect(t.planPut('b', 40)).toEqual({ store: true, evict: [] });
    t.touch('a');
    expect(t.planPut('c', 40)).toEqual({ store: true, evict: ['b'] });
    expect(t.totalBytes).toBe(80);
    expect(t.planPut('huge', 101)).toEqual({ store: false, evict: [] });
    const t2 = new OpfsLruTracker(100);
    t2.load(t.serialize());
    expect(t2.totalBytes).toBe(80);
    expect(t2.has('a')).toBe(true);
    expect(t2.has('b')).toBe(false);
    expect(t2.has('c')).toBe(true);
  });

  it('falls back to async OPFS when sync OPFS worker init fails', async () => {
    const fakeDir = {} as FileSystemDirectoryHandle;
    const failingWorker = () => {
      const w = {
        onmessage: null as ((e: MessageEvent) => void) | null,
        onerror: null as ((e: ErrorEvent) => void) | null,
        postMessage(req: { id: number }) {
          queueMicrotask(() => w.onmessage?.({ data: { id: req.id, ok: false, error: 'no sync handle' } } as MessageEvent));
        },
        terminate() {},
      };
      return w as unknown as Worker;
    };
    const backends = await detectBackends('t', {
      storage: { getDirectory: async () => fakeDir, estimate: async () => ({ quota: 2 * 1024 * 1024 * 1024, usage: 0 }) },
      opfsSync: failingWorker,
    });
    expect(backends).toHaveLength(1);
    expect(backends[0]!.name).toBe('opfs');
    expect(backends[0]!.sync).toBeUndefined();
  });
});

describe('audio entry header & probe.json decoding', () => {
  it('decodes .fpak audio layout v1 header and strips it in loadAudioEntry', async () => {
    const body = Uint8Array.of(0x1a, 0x45, 0xdf, 0xa3, 1, 2, 3, 4);
    const entry = new Uint8Array(32 + body.length);
    const dv = new DataView(entry.buffer);
    dv.setUint8(0, 1); // layout_version
    dv.setUint8(1, 0); // webm-opus
    dv.setUint8(2, 3); // preload + loop
    dv.setUint8(3, 2); // stereo
    dv.setUint32(4, 1500, true);
    dv.setUint32(8, 48000, true);
    dv.setUint32(12, 100, true);
    dv.setUint32(16, 1400, true);
    dv.setUint32(20, body.length, true);
    entry.set(body, 32);

    const dec = decodeAudioEntry(entry);
    expect(dec).toMatchObject({ container: 'webm-opus', preload: true, loop: true, channels: 2, durationMs: 1500, sampleRate: 48000, loopStartMs: 100, loopEndMs: 1400 });
    expect(Array.from(dec.payload)).toEqual(Array.from(body));

    const { bytes, refs } = buildPak([{ id: 'snd_test@opus', kind: 2, payload: entry }]);
    const manifest: Manifest = {
      schema: 1, game_version: 't',
      assets: { snd_test: { kind: 'audio', tier: ['medium'], placeholder: false, req_id: 'A-WPN-001', audio: [{ codec: 'opus', pak: PAK_PATH, entry: 'snd_test@opus', ...refs.get('snd_test@opus')!, sha256: 's1' }] } },
    };
    const l = new AssetLoader({
      manifest,
      fetcher: async () => new Response(bytes as unknown as BodyInit, { status: 200 }),
      sleep: noSleep,
      log: () => undefined,
    });
    await l.init({ tier: 'medium' });
    const loaded = await l.loadAudioEntry('snd_test@opus');
    expect('silent' in loaded).toBe(false);
    if (!('silent' in loaded)) expect(Array.from(loaded.payload)).toEqual(Array.from(body));

    const parsed = parseProbeJson({
      schema: 'frontline.audio-probe/1', timeout_ms: 500,
      opus: { mime: 'audio/webm; codecs=opus', base64: btoa('OPUS'), bytes: 4 },
      aac: { mime: 'audio/mp4; codecs=mp4a.40.2', base64: btoa('AAC!'), bytes: 4 },
    });
    expect(parsed.timeoutMs).toBe(500);
    expect(parsed.opus.byteLength).toBe(4);
    expect(parsed.aac.byteLength).toBe(4);
  });

  it('resolves chunk and collision entries from terrain_carentan-style manifests', async () => {
    const chunkPayload = payload(32, 7);
    const colPayload = payload(24, 11);
    const { bytes, refs } = buildPak([
      { id: 'terrain_carentan_b_10_22@chunk', kind: 3, payload: chunkPayload },
      { id: 'terrain_carentan_b_10_22@collision', kind: 3, payload: colPayload },
    ]);
    const manifest: Manifest = {
      schema: 1,
      game_version: '0.1.0',
      assets: {
        terrain_carentan_b_10_22: {
          kind: 'data',
          tier: ['medium'],
          placeholder: true,
          chunk: { pak: PAK_PATH, entry: 'terrain_carentan_b_10_22@chunk', ...refs.get('terrain_carentan_b_10_22@chunk')!, sha256: 'c1' },
          collision: { pak: PAK_PATH, entry: 'terrain_carentan_b_10_22@collision', ...refs.get('terrain_carentan_b_10_22@collision')!, sha256: 'c2' },
        },
      },
    };
    const l = new AssetLoader({
      manifest,
      fetcher: async () => new Response(bytes as unknown as BodyInit, { status: 200 }),
      sleep: noSleep,
      log: () => undefined,
    });
    await l.init({ tier: 'medium' });
    expect(Array.from(await l.loadBytes('terrain_carentan_b_10_22@chunk'))).toEqual(Array.from(chunkPayload));
    expect(Array.from(await l.loadBytes('terrain_carentan_b_10_22@collision'))).toEqual(Array.from(colPayload));
  });
});
