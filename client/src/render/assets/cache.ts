/**
 * `CacheLayer` (ARCHITECTURE 11.6): memory LRU in front of optional persistent backends (OPFS, then Cache API). The game must run
 * with no persistence at all: any backend failure disables that backend and the layer continues in memory.
 */
export type CacheMode = 'opfs' | 'cache' | 'memory';

export interface CacheBackend {
  readonly name: 'opfs' | 'cache';
  get(key: string): Promise<Uint8Array | undefined>;
  put(key: string, bytes: Uint8Array): Promise<void>;
  /** Bytes currently stored (for LRU / quota decisions). */
  usage?(): Promise<number>;
}

export class MemoryLru {
  readonly #map = new Map<string, Uint8Array>();
  #bytes = 0;
  constructor(readonly maxBytes: number) {}
  get bytes(): number {
    return this.#bytes;
  }
  get size(): number {
    return this.#map.size;
  }
  get(key: string): Uint8Array | undefined {
    const v = this.#map.get(key);
    if (v) {
      this.#map.delete(key);
      this.#map.set(key, v); // most recently used last
    }
    return v;
  }
  put(key: string, v: Uint8Array): void {
    const old = this.#map.get(key);
    if (old) {
      this.#bytes -= old.byteLength;
      this.#map.delete(key);
    }
    if (v.byteLength > this.maxBytes) return;
    this.#map.set(key, v);
    this.#bytes += v.byteLength;
    for (const [k, x] of this.#map) {
      if (this.#bytes <= this.maxBytes) break;
      this.#map.delete(k);
      this.#bytes -= x.byteLength;
    }
  }
}

export interface CacheLayerOptions {
  backends?: CacheBackend[];
  memoryBytes?: number;
  onInfo?: (msg: string) => void;
}

export class CacheLayer {
  readonly memory: MemoryLru;
  #backends: CacheBackend[];
  hits = 0;
  misses = 0;
  constructor(private readonly o: CacheLayerOptions = {}) {
    this.#backends = [...(o.backends ?? [])];
    this.memory = new MemoryLru(o.memoryBytes ?? 256 * 1024 * 1024);
  }

  get mode(): CacheMode {
    return this.#backends[0]?.name ?? 'memory';
  }

  get hitRate(): number {
    const n = this.hits + this.misses;
    return n === 0 ? 0 : this.hits / n;
  }

  #drop(b: CacheBackend, err: unknown): void {
    this.#backends = this.#backends.filter((x) => x !== b);
    this.o.onInfo?.(`cache: ${b.name} disabled (${err instanceof Error ? err.message : String(err)}); mode ${this.mode}`);
  }

  async get(key: string): Promise<Uint8Array | undefined> {
    const m = this.memory.get(key);
    if (m) {
      this.hits++;
      return m;
    }
    for (const b of [...this.#backends]) {
      try {
        const v = await b.get(key);
        if (v) {
          this.memory.put(key, v);
          this.hits++;
          return v;
        }
      } catch (e) {
        this.#drop(b, e);
      }
    }
    this.misses++;
    return undefined;
  }

  async put(key: string, bytes: Uint8Array): Promise<void> {
    this.memory.put(key, bytes);
    for (const b of [...this.#backends]) {
      try {
        await b.put(key, bytes);
      } catch (e) {
        this.#drop(b, e);
      }
    }
  }
}

// ---- backends & feature detection -------------------------------------------------------------------------------------

const safeName = (key: string) => key.replace(/[^A-Za-z0-9._-]+/g, '_');

/** OPFS (async API; the Worker-only sync handle variant is a later optimisation, see PI-037). */
export function opfsBackend(root: FileSystemDirectoryHandle): CacheBackend {
  return {
    name: 'opfs',
    async get(key) {
      try {
        const h = await root.getFileHandle(safeName(key));
        return new Uint8Array(await (await h.getFile()).arrayBuffer());
      } catch (e) {
        if (e instanceof DOMException && e.name === 'NotFoundError') return undefined;
        throw e;
      }
    },
    async put(key, bytes) {
      const h = await root.getFileHandle(safeName(key), { create: true });
      const w = await h.createWritable();
      await w.write(bytes as unknown as BufferSource);
      await w.close();
    },
  };
}

export function cacheApiBackend(cache: Cache, versionKey: string): CacheBackend {
  const url = (key: string) => `https://cache.invalid/${versionKey}/${encodeURIComponent(key)}`;
  return {
    name: 'cache',
    async get(key) {
      const r = await cache.match(url(key));
      return r ? new Uint8Array(await r.arrayBuffer()) : undefined;
    },
    async put(key, bytes) {
      await cache.put(url(key), new Response(bytes as unknown as BodyInit));
    },
  };
}

export interface DetectEnv {
  storage?: { getDirectory?: () => Promise<FileSystemDirectoryHandle>; estimate?: () => Promise<{ quota?: number; usage?: number }> };
  caches?: { open(name: string): Promise<Cache> };
}

/** Quota floor below which persistence is not worth the risk (the default OPFS budget is 1.5 GB, ARCHITECTURE 11.6). */
export const MIN_QUOTA_BYTES = 256 * 1024 * 1024;

export async function detectBackends(gameVersion: string, env: DetectEnv = { storage: globalThis.navigator?.storage as DetectEnv['storage'], caches: (globalThis as { caches?: DetectEnv['caches'] }).caches }): Promise<CacheBackend[]> {
  const out: CacheBackend[] = [];
  try {
    const est = await env.storage?.estimate?.();
    const free = est?.quota !== undefined ? est.quota - (est.usage ?? 0) : Infinity;
    if (env.storage?.getDirectory && free >= MIN_QUOTA_BYTES) out.push(opfsBackend(await env.storage.getDirectory()));
  } catch {
    // private mode / no permission: fall through to the next backend
  }
  try {
    if (env.caches) out.push(cacheApiBackend(await env.caches.open(`frontline-assets-v${gameVersion}`), gameVersion));
  } catch {
    // Cache API unavailable
  }
  return out;
}
