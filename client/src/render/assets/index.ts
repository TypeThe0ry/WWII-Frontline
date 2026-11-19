/**
 * Asset loader (M1-006, ARCHITECTURE 11.5): manifest -> pak Range read (cache first) -> pool decode -> budgeted GPU upload.
 * Everything environment specific (fetch, timers, decoder, uploader) is injectable so the logic runs in vitest without a browser.
 */
import { CacheLayer, type CacheMode } from './cache';
import { decodeAudioEntry, type DecodedAudioEntry } from './entry-audio';
import { FetchRangeSource, FpakReader, type Fetcher } from './fpak-reader';
import { loadManifest, ManifestIndex, type Manifest, type Tier } from './manifest';
import { isPlaceholder, makePlaceholder, OncePerSession, silence, type Placeholder } from './placeholder';
import { CancelledError, WorkerPool } from './worker-pool';
import { UploadScheduler } from './upload-scheduler';

export * from './cache';
export * from './entry-audio';
export * from './fpak-reader';
export * from './manifest';
export * from './placeholder';
export * from './upload-scheduler';
export * from './worker-pool';

export type Phase = 'download' | 'decode' | 'upload';
export interface ProgressEvent {
  loaded: number;
  total: number;
  phase: Phase;
}

export interface Decoded {
  entryId: string;
  kind: 'mesh' | 'texture';
  bytes: Uint8Array;
  /** Decoder specific payload (meshopt-decoded buffers, transcoded KTX2 levels...). */
  data?: unknown;
}
export type MeshData = Decoded & { kind: 'mesh' };
export type GpuTextureHandle = Decoded & { kind: 'texture' };
export interface Silence {
  silent: true;
  samples: Float32Array;
}

export interface Decoder {
  decode(entryId: string, kind: 'mesh' | 'texture', bytes: Uint8Array): Promise<Decoded>;
}

export interface LoaderDeps {
  fetcher?: Fetcher;
  sleep?: (ms: number) => Promise<void>;
  now?: () => number;
  /** Prefix for pak paths from the manifest (CDN origin). */
  base?: string;
  cache?: CacheLayer;
  decoder?: Decoder;
  uploads?: UploadScheduler;
  poolSize?: number;
  /** Maximum concurrent network downloads (ARCHITECTURE 9.3: <= 6). */
  maxDownloadConcurrency?: number;
  log?: (msg: string) => void;
  /** Called to schedule `uploads.runFrame()`; the render loop normally calls it itself. Default: never (caller drives it). */
  onProgress?: (e: ProgressEvent) => void;
  /** Manifest object instead of `manifestUrl` (tests). */
  manifest?: Manifest;
}

export interface LoadToken {
  cancelled: boolean;
  readonly abort: AbortController;
}
export interface LoadOptions {
  prio?: number;
  token?: LoadToken;
}

export const RETRY_DELAYS_MS = [200, 400, 800] as const;
export const PROGRESS_INTERVAL_MS = 100;
export const DEFAULT_MAX_DOWNLOAD_CONCURRENCY = 6;

const identityDecoder: Decoder = { decode: async (entryId, kind, bytes) => ({ entryId, kind, bytes }) };

export class AssetLoader {
  #index!: ManifestIndex;
  readonly #paks = new Map<string, Promise<FpakReader>>();
  readonly #sources = new Map<string, FetchRangeSource>();
  readonly #bytesInflight = new Map<string, Promise<Uint8Array>>();
  readonly #decoded = new Map<string, Promise<unknown>>();
  readonly #pool: WorkerPool<{ entryId: string; kind: 'mesh' | 'texture'; bytes: Uint8Array }, Decoded>;
  readonly #maxDownloads: number;
  #activeDownloads = 0;
  readonly #downloadQueue: { run: () => void; cancel: () => void }[] = [];
  readonly #maxDecodeInflight: number;
  #activeDecodes = 0;
  readonly #decodeQueue: { prio: number; seq: number; run: () => void; cancel: () => void }[] = [];
  #decodeSeq = 0;
  readonly cache: CacheLayer;
  readonly uploads: UploadScheduler;
  readonly #deps: LoaderDeps;
  readonly #logOnce: OncePerSession;
  readonly #progress: Record<Phase, { loaded: number; total: number }> = { download: { loaded: 0, total: 0 }, decode: { loaded: 0, total: 0 }, upload: { loaded: 0, total: 0 } };
  #lastEmit = -Infinity;
  /** Requests actually sent to the network (Range reads + whole-file downloads); used by the cache tests. */
  networkReads = 0;
  attempts = new Map<string, number>();

  constructor(deps: LoaderDeps = {}) {
    this.#deps = deps;
    this.cache = deps.cache ?? new CacheLayer();
    this.uploads = deps.uploads ?? new UploadScheduler();
    this.#maxDownloads = Math.max(1, deps.maxDownloadConcurrency ?? DEFAULT_MAX_DOWNLOAD_CONCURRENCY);
    const dec = deps.decoder ?? identityDecoder;
    this.#pool = new WorkerPool((t) => dec.decode(t.entryId, t.kind, t.bytes), deps.poolSize);
    this.#maxDecodeInflight = Math.max(this.#pool.size * 2, this.#maxDownloads);
    this.#logOnce = new OncePerSession(deps.log ?? ((m) => console.warn(m)));
  }

  get #now(): () => number {
    return this.#deps.now ?? (() => performance.now());
  }

  async init(opts: { manifestUrl?: string; tier: Tier }): Promise<void> {
    if (this.#deps.manifest) this.#index = new ManifestIndex(this.#deps.manifest, opts.tier);
    else if (opts.manifestUrl) this.#index = await loadManifest(opts.manifestUrl, opts.tier, this.#deps.fetcher);
    else throw new Error('AssetLoader.init needs manifestUrl or deps.manifest');
  }

  createToken(): LoadToken {
    return { cancelled: false, abort: new AbortController() };
  }

  cancel(token: LoadToken): void {
    token.cancelled = true;
    token.abort.abort();
  }

  stats(): { cacheMode: CacheMode; inflight: number; hitRate: number } {
    return { cacheMode: this.cache.mode, inflight: this.#bytesInflight.size + this.#pool.inflight, hitRate: this.cache.hitRate };
  }

  // ---- progress (<= 10 Hz, monotonic) ----------------------------------------------------------------------------------

  #bump(phase: Phase, loaded: number, total: number): void {
    const p = this.#progress[phase];
    p.loaded += loaded;
    p.total += total;
    const t = this.#now();
    if (t - this.#lastEmit < PROGRESS_INTERVAL_MS) return;
    this.#lastEmit = t;
    this.#deps.onProgress?.({ phase, loaded: p.loaded, total: p.total });
  }

  /** Emit the final state of every phase regardless of the throttle (call when a load batch settles). */
  flushProgress(): void {
    for (const phase of ['download', 'decode', 'upload'] as const) this.#deps.onProgress?.({ phase, ...this.#progress[phase] });
  }

  // ---- bytes ----------------------------------------------------------------------------------------------------------

  async #reader(pak: string, token?: LoadToken): Promise<FpakReader> {
    let r = this.#paks.get(pak);
    if (!r) {
      const src = new FetchRangeSource((this.#deps.base ?? '') + pak, async (u, i) => {
        this.networkReads++;
        return (this.#deps.fetcher ?? fetch)(u, i);
      });
      this.#sources.set(pak, src);
      r = FpakReader.open(src, token?.abort.signal);
      this.#paks.set(pak, r);
      r.catch(() => this.#paks.delete(pak)); // a failed open is retried by the next request
    }
    return r;
  }

  async #withRetry<T>(key: string, fn: () => Promise<T>, token?: LoadToken): Promise<T> {
    let last: unknown;
    for (let attempt = 0; attempt <= RETRY_DELAYS_MS.length; attempt++) {
      if (token?.cancelled) throw new CancelledError();
      this.attempts.set(key, attempt + 1);
      try {
        return await fn();
      } catch (e) {
        if (token?.cancelled) throw new CancelledError();
        last = e;
        const delay = RETRY_DELAYS_MS[attempt];
        if (delay === undefined) break;
        await (this.#deps.sleep ?? ((ms) => new Promise<void>((r) => setTimeout(r, ms))))(delay);
      }
    }
    throw last;
  }

  async #withDownloadSlot<T>(fn: () => Promise<T>, token?: LoadToken): Promise<T> {
    if (token?.cancelled) throw new CancelledError();
    if (this.#activeDownloads >= this.#maxDownloads) {
      await new Promise<void>((resolve, reject) => {
        const entry = {
          run: () => {
            token?.abort.signal.removeEventListener('abort', onAbort);
            resolve();
          },
          cancel: () => reject(new CancelledError()),
        };
        const onAbort = () => {
          const idx = this.#downloadQueue.indexOf(entry);
          if (idx >= 0) this.#downloadQueue.splice(idx, 1);
          entry.cancel();
        };
        token?.abort.signal.addEventListener('abort', onAbort, { once: true });
        this.#downloadQueue.push(entry);
      });
    }
    if (token?.cancelled) throw new CancelledError();
    this.#activeDownloads++;
    try {
      return await fn();
    } finally {
      this.#activeDownloads--;
      const next = this.#downloadQueue.shift();
      next?.run();
    }
  }

  /** Raw entry bytes (cache first). Concurrent requests for one entry share one read. Throws after 3 retries. */
  loadBytes(entryId: string, opts: LoadOptions = {}): Promise<Uint8Array> {
    const hit = this.#bytesInflight.get(entryId);
    if (hit) return hit;
    const p = (async () => {
      const ref = this.#index.resolve(entryId);
      if (!ref) throw new Error(`${entryId}: not in the manifest for this tier`);
      const key = `${ref.sha256}`;
      // Fast synchronous memory LRU check avoids queuing when already in RAM.
      const mem = this.cache.memory.get(key);
      this.#bump('download', 0, ref.bytes);
      if (mem) {
        this.cache.hits++;
        this.#bump('download', mem.byteLength, 0);
        return mem;
      }
      // Bound concurrent persistent-cache (OPFS/Cache API) + network I/O to maxDownloadConcurrency (default 6)
      // so warm OPFS bursts cannot flood the main thread with hundreds of MB at once.
      return await this.#withDownloadSlot(async () => {
        const cached = await this.cache.get(key);
        if (cached) {
          this.#bump('download', cached.byteLength, 0);
          return cached;
        }
        const bytes = await this.#withRetry(
          entryId,
          async () => (await this.#reader(ref.pak, opts.token)).read(entryId, opts.token?.abort.signal),
          opts.token,
        );
        await this.cache.put(key, bytes);
        this.#bump('download', bytes.byteLength, 0);
        return bytes;
      }, opts.token);
    })();
    this.#bytesInflight.set(entryId, p);
    const done = () => this.#bytesInflight.delete(entryId);
    p.then(done, done);
    return p;
  }

  // ---- decode + upload -------------------------------------------------------------------------------------------------

  /** Backpressure gate so fast warm OPFS hits don't pile up hundreds of MB of raw buffers waiting for decode workers. */
  async #withDecodeSlot<T>(prio: number, fn: () => Promise<T>, token?: LoadToken): Promise<T> {
    if (token?.cancelled) throw new CancelledError();
    if (this.#activeDecodes >= this.#maxDecodeInflight) {
      await new Promise<void>((resolve, reject) => {
        const entry = {
          prio,
          seq: this.#decodeSeq++,
          run: () => {
            token?.abort.signal.removeEventListener('abort', onAbort);
            resolve();
          },
          cancel: () => reject(new CancelledError()),
        };
        const onAbort = () => {
          const idx = this.#decodeQueue.indexOf(entry);
          if (idx >= 0) this.#decodeQueue.splice(idx, 1);
          entry.cancel();
        };
        token?.abort.signal.addEventListener('abort', onAbort, { once: true });
        this.#decodeQueue.push(entry);
        this.#decodeQueue.sort((a, b) => a.prio - b.prio || a.seq - b.seq);
      });
    }
    if (token?.cancelled) throw new CancelledError();
    this.#activeDecodes++;
    try {
      return await fn();
    } finally {
      this.#activeDecodes--;
      const next = this.#decodeQueue.shift();
      next?.run();
    }
  }

  async #decodeUpload(entryId: string, kind: 'mesh' | 'texture', opts: LoadOptions): Promise<Decoded | Placeholder> {
    try {
      const prio = opts.prio ?? 5;
      const decoded = await this.#withDecodeSlot(
        prio,
        async () => {
          const bytes = await this.loadBytes(entryId, opts);
          this.#bump('decode', 0, bytes.byteLength);
          const task = this.#pool.submit({ entryId, kind, bytes }, prio);
          opts.token?.abort.signal.addEventListener('abort', () => task.cancel(), { once: true });
          const out = await task.promise;
          this.#bump('decode', bytes.byteLength, 0);
          return out;
        },
        opts.token,
      );
      this.#bump('upload', 0, decoded.bytes.byteLength);
      return await new Promise<Decoded>((resolve, reject) => {
        this.uploads.enqueue({
          bytes: decoded.bytes.byteLength,
          prio,
          run: () => {
            if (opts.token?.cancelled) return reject(new CancelledError());
            this.#bump('upload', decoded.bytes.byteLength, 0);
            resolve(decoded);
          },
        });
      });
    } catch (e) {
      if (e instanceof CancelledError) throw e;
      const req = this.#index.reqIdOf(entryId);
      this.#logOnce.log(req ?? entryId, `assets: ${entryId} failed (${e instanceof Error ? e.message : String(e)}); using placeholder PH_${req ?? 'UNKNOWN'}`);
      return makePlaceholder(entryId, req, entryId.replace(/[^A-Za-z0-9_.-]/g, '_'), String(e));
    }
  }

  #dedup<T>(key: string, make: () => Promise<T>): Promise<T> {
    const hit = this.#decoded.get(key);
    if (hit) return hit as Promise<T>;
    const p = make();
    this.#decoded.set(key, p);
    // A placeholder or cancelled result must not be cached for the whole session: the next request retries.
    p.then((v) => isPlaceholder(v) && this.#decoded.delete(key), () => this.#decoded.delete(key));
    return p;
  }

  loadMesh(id: string, lod: number, prio?: number | LoadOptions): Promise<MeshData | Placeholder> {
    const o = typeof prio === 'object' ? prio : { ...(prio !== undefined ? { prio } : {}) };
    const entryId = `${id}@lod${lod}`;
    return this.#dedup(entryId, () => this.#decodeUpload(entryId, 'mesh', o) as Promise<MeshData | Placeholder>);
  }

  /** `id` is a texture entry id (`prop_crate@tex_albedo`). */
  loadTexture(id: string, prio?: number | LoadOptions): Promise<GpuTextureHandle | Placeholder> {
    const o = typeof prio === 'object' ? prio : { ...(prio !== undefined ? { prio } : {}) };
    return this.#dedup(id, () => this.#decodeUpload(id, 'texture', o) as Promise<GpuTextureHandle | Placeholder>);
  }

  /** Audio bytes only (`snd_x@opus|aac`); a missing entry yields 0.1 s of silence and one log line per requirement id. */
  async loadAudioBytes(id: string, opts: LoadOptions = {}): Promise<Uint8Array | Silence> {
    try {
      return await this.loadBytes(id, opts);
    } catch (e) {
      if (e instanceof CancelledError) throw e;
      const base = id.split('@')[0] as string;
      this.#logOnce.log(this.#index.reqIdOf(id) ?? base, `assets: audio ${id} missing (${e instanceof Error ? e.message : String(e)}); playing silence`);
      return { silent: true, samples: silence() };
    }
  }

  /** Decoded `.fpak` audio entry (header stripped, container payload ready for `AudioContext.decodeAudioData`). */
  async loadAudioEntry(id: string, opts: LoadOptions = {}): Promise<DecodedAudioEntry | Silence> {
    try {
      return decodeAudioEntry(await this.loadBytes(id, opts));
    } catch (e) {
      if (e instanceof CancelledError) throw e;
      const base = id.split('@')[0] as string;
      this.#logOnce.log(this.#index.reqIdOf(id) ?? base, `assets: audio ${id} missing/invalid (${e instanceof Error ? e.message : String(e)}); playing silence`);
      return { silent: true, samples: silence() };
    }
  }
}
