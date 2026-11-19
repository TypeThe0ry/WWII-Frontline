/**
 * Dedicated OPFS cache Worker (ARCHITECTURE §11.6, M1-006 / PI-037): uses `FileSystemSyncAccessHandle` inside a Worker for
 * fast, non-blocking persistent storage with LRU quota eviction (default 1.5 GiB).
 */

export const DEFAULT_OPFS_MAX_BYTES = 1536 * 1024 * 1024;
const INDEX_FILE = '_lru.json';
const PROBE_FILE = '_sync_probe.tmp';

export interface LruRecord {
  size: number;
  atime: number;
}

/** Pure LRU metadata tracker (unit-tested in vitest without a browser). */
export class OpfsLruTracker {
  readonly #entries = new Map<string, LruRecord>();
  #totalBytes = 0;
  #clock = 0;

  constructor(readonly maxBytes: number = DEFAULT_OPFS_MAX_BYTES) {}

  get totalBytes(): number {
    return this.#totalBytes;
  }

  get size(): number {
    return this.#entries.size;
  }

  has(key: string): boolean {
    return this.#entries.has(key);
  }

  touch(key: string): void {
    const rec = this.#entries.get(key);
    if (!rec) return;
    rec.atime = ++this.#clock;
    // Re-insert so Map iteration order stays oldest -> newest.
    this.#entries.delete(key);
    this.#entries.set(key, rec);
  }

  remove(key: string): void {
    const old = this.#entries.get(key);
    if (!old) return;
    this.#totalBytes -= old.size;
    this.#entries.delete(key);
  }

  /**
   * Record `key` with `bytes`. Returns the keys that must be deleted from OPFS to keep `totalBytes <= maxBytes`.
   * If a single entry exceeds `maxBytes`, it is not stored and `{ store: false, evict: [] }` is returned.
   */
  planPut(key: string, bytes: number): { store: boolean; evict: string[] } {
    if (bytes > this.maxBytes) return { store: false, evict: [] };
    this.remove(key);
    const evict: string[] = [];
    for (const [k, rec] of this.#entries) {
      if (this.#totalBytes + bytes <= this.maxBytes) break;
      evict.push(k);
      this.#totalBytes -= rec.size;
      this.#entries.delete(k);
    }
    this.#entries.set(key, { size: bytes, atime: ++this.#clock });
    this.#totalBytes += bytes;
    return { store: true, evict };
  }

  serialize(): string {
    const items: [string, number, number][] = [];
    for (const [k, v] of this.#entries) items.push([k, v.size, v.atime]);
    return JSON.stringify({ v: 1, clock: this.#clock, items });
  }

  load(json: string): void {
    this.#entries.clear();
    this.#totalBytes = 0;
    this.#clock = 0;
    const parsed = JSON.parse(json) as { v?: number; clock?: number; items?: [string, number, number][] };
    if (!Array.isArray(parsed.items)) return;
    const sorted = [...parsed.items].sort((a, b) => a[2] - b[2]);
    for (const [k, sz, at] of sorted) {
      if (typeof k !== 'string' || typeof sz !== 'number' || sz < 0) continue;
      this.#entries.set(k, { size: sz, atime: at });
      this.#totalBytes += sz;
      if (at > this.#clock) this.#clock = at;
    }
    if (typeof parsed.clock === 'number' && parsed.clock > this.#clock) this.#clock = parsed.clock;
  }
}

export type OpfsRequestBody =
  | { op: 'init'; dirName: string; maxBytes: number }
  | { op: 'get'; key: string }
  | { op: 'put'; key: string; bytes: Uint8Array }
  | { op: 'usage' }
  | { op: 'clear' };

export type OpfsRequest = OpfsRequestBody & { id: number };

export type OpfsResponse =
  | { id: number; ok: true; sync?: boolean; bytes?: Uint8Array; usage?: number }
  | { id: number; ok: false; error: string };

interface SyncAccessHandle {
  getSize(): number;
  read(buffer: ArrayBufferView, options?: { at?: number }): number;
  write(buffer: ArrayBufferView, options?: { at?: number }): number;
  truncate(newSize: number): void;
  flush(): void;
  close(): void;
}

interface WorkerFileHandle extends FileSystemFileHandle {
  createSyncAccessHandle?: () => Promise<SyncAccessHandle>;
}

interface WorkerDirHandle extends FileSystemDirectoryHandle {
  getFileHandle(name: string, options?: FileSystemGetFileOptions): Promise<WorkerFileHandle>;
  getDirectoryHandle(name: string, options?: FileSystemGetDirectoryOptions): Promise<WorkerDirHandle>;
}

const safeName = (key: string) => key.replace(/[^A-Za-z0-9._-]+/g, '_');

async function readSync(h: WorkerFileHandle): Promise<Uint8Array> {
  const sync = await h.createSyncAccessHandle!();
  try {
    const size = sync.getSize();
    const out = new Uint8Array(size);
    if (size > 0) sync.read(out, { at: 0 });
    return out;
  } finally {
    sync.close();
  }
}

async function writeSync(h: WorkerFileHandle, bytes: Uint8Array): Promise<void> {
  const sync = await h.createSyncAccessHandle!();
  try {
    sync.truncate(0);
    if (bytes.byteLength > 0) sync.write(bytes, { at: 0 });
    sync.flush();
  } finally {
    sync.close();
  }
}

// Wire the dedicated worker message loop only when running inside a Worker.
if (
  typeof self !== 'undefined' &&
  typeof (self as { document?: unknown }).document === 'undefined' &&
  typeof (self as { postMessage?: unknown }).postMessage === 'function' &&
  'onmessage' in self
) {
  const scope = self as unknown as {
    onmessage: ((e: MessageEvent<OpfsRequest>) => void) | null;
    postMessage(m: OpfsResponse, transfer?: Transferable[]): void;
  };
  let dir: WorkerDirHandle | undefined;
  let lru = new OpfsLruTracker();
  let queue: Promise<void> = Promise.resolve();

  const saveIndex = async () => {
    if (!dir) return;
    const fh = await dir.getFileHandle(INDEX_FILE, { create: true });
    await writeSync(fh, new TextEncoder().encode(lru.serialize()));
  };

  const handle = async (req: OpfsRequest): Promise<{ res: OpfsResponse; transfer: Transferable[] }> => {
    try {
      if (req.op === 'init') {
        const storage = (self as unknown as { navigator?: { storage?: { getDirectory?: () => Promise<WorkerDirHandle> } } }).navigator?.storage;
        if (!storage?.getDirectory) return { res: { id: req.id, ok: false, error: 'OPFS getDirectory unavailable' }, transfer: [] };
        const root = await storage.getDirectory();
        dir = await root.getDirectoryHandle(safeName(req.dirName), { create: true });
        const probe = await dir.getFileHandle(PROBE_FILE, { create: true });
        if (typeof probe.createSyncAccessHandle !== 'function') {
          await dir.removeEntry(PROBE_FILE).catch(() => undefined);
          return { res: { id: req.id, ok: false, error: 'FileSystemSyncAccessHandle unavailable' }, transfer: [] };
        }
        const sh = await probe.createSyncAccessHandle();
        sh.close();
        await dir.removeEntry(PROBE_FILE).catch(() => undefined);
        lru = new OpfsLruTracker(req.maxBytes);
        try {
          const idxH = await dir.getFileHandle(INDEX_FILE);
          lru.load(new TextDecoder().decode(await readSync(idxH)));
        } catch {
          // Fresh cache directory: index will be created on first put.
        }
        return { res: { id: req.id, ok: true, sync: true }, transfer: [] };
      }
      if (!dir) return { res: { id: req.id, ok: false, error: 'OPFS worker not initialized' }, transfer: [] };
      if (req.op === 'usage') return { res: { id: req.id, ok: true, usage: lru.totalBytes }, transfer: [] };
      if (req.op === 'clear') {
        if (dir.entries) {
          for await (const [name] of dir.entries()) await dir.removeEntry(name, { recursive: true }).catch(() => undefined);
        }
        lru = new OpfsLruTracker(lru.maxBytes);
        return { res: { id: req.id, ok: true }, transfer: [] };
      }
      const name = safeName(req.key);
      if (req.op === 'get') {
        try {
          const fh = await dir.getFileHandle(name);
          const bytes = await readSync(fh);
          if (lru.has(name)) lru.touch(name);
          else lru.planPut(name, bytes.byteLength);
          return { res: { id: req.id, ok: true, bytes }, transfer: [bytes.buffer] };
        } catch (e) {
          if (e instanceof DOMException && e.name === 'NotFoundError') return { res: { id: req.id, ok: true, bytes: undefined }, transfer: [] };
          throw e;
        }
      }
      if (req.op === 'put') {
        const plan = lru.planPut(name, req.bytes.byteLength);
        if (!plan.store) return { res: { id: req.id, ok: true }, transfer: [] };
        for (const old of plan.evict) await dir.removeEntry(old).catch(() => undefined);
        const fh = await dir.getFileHandle(name, { create: true });
        await writeSync(fh, req.bytes);
        await saveIndex();
        return { res: { id: req.id, ok: true }, transfer: [] };
      }
      return { res: { id: (req as { id: number }).id, ok: false, error: 'unknown op' }, transfer: [] };
    } catch (e) {
      return { res: { id: req.id, ok: false, error: e instanceof Error ? e.message : String(e) }, transfer: [] };
    }
  };

  scope.onmessage = (e) => {
    const req = e.data;
    // Serialize OPFS handle operations inside the worker to prevent concurrent SyncAccessHandle lock contention.
    queue = queue.then(async () => {
      const { res, transfer } = await handle(req);
      scope.postMessage(res, transfer);
    });
  };
}
