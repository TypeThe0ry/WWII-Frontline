import {
  CHUNK_M, GRID, MAX_CONCURRENT_FETCH, R_MID, R_NEAR, R_PHYS, UPLOAD_BUDGET_BYTES, UPLOAD_BUDGET_MS, StreamerCore,
  chunkKey, chunkOf, chunksAround, ringDist, type ChunkCoord, type ChunkFrustum, type ChunkState, type WantedChunk,
} from './chunk-streamer-core';

export type { ChunkCoord, ChunkFrustum, ChunkState } from './chunk-streamer-core';

type Horizontal = { x: number; z: number };
const RESCORE_S = 0.25;
const RETAIN_RING = R_NEAR + 2;
const COLLISION_RING = R_PHYS + 1;
const MAX_WAITING = 4 * MAX_CONCURRENT_FETCH;
const alwaysVisible: ChunkFrustum = () => true;

export interface ChunkSource<T> {
  /** Resolve a decoded payload with its upload footprint. Honour abort before committing I/O results. */
  load(chunk: ChunkCoord, signal: AbortSignal): Promise<{ data: T; uploadBytes: number }>;
  hasChunk?(chunk: ChunkCoord): boolean;
  /** Set false when the source already owns its retry policy (e.g. AssetLoader). */
  retryLoad?: boolean;
}

export interface ChunkStreamerOptions<T> extends ChunkSource<T> {
  upload(chunk: ChunkCoord, data: T): void;
  evict(chunk: ChunkCoord): void;
  /** Whether visible terrain is covered by a coarse parent while its detail chunk is missing. */
  hasFallback(chunk: ChunkCoord): boolean;
  collision?: {
    load(chunk: ChunkCoord, signal: AbortSignal): Promise<Uint8Array>;
    install(chunk: ChunkCoord, data: Uint8Array): void;
    remove(chunk: ChunkCoord): void;
  };
  /** Visual scheduling radius in chunks. Defaults to R_MID. */
  radius?: number;
  /** Extra ahead-only ring. Enabled by default. */
  headingPrefetch?: boolean;
  /** Hard residency limit in chunks. By default residency is governed by the soft budget. */
  retainRadius?: number;
  softBudgetBytes?: number;
  /** Aborts an outstanding load after this much active scene time. Defaults to 10 s. */
  loadTimeoutSeconds?: number;
  onProgress?(event: PrefetchProgress): void;
  /** Monotonic wall clock in milliseconds. */
  now?(): number;
}

export interface PrefetchProgress {
  type: 'stream.progress';
  total: number;
  ready: number;
  failed: number;
}

export interface PrefetchResult extends PrefetchProgress {
  cancelled: boolean;
  elapsedMs: number;
}

export interface StreamMetrics {
  stream_resident: number;
  stream_queue_len: number;
  stream_upload_ms: number;
  stream_upload_bytes: number;
  stream_holes: number;
}

type Job = { chunk: WantedChunk; collision: boolean };
const loadTimeout = Symbol('stream load timeout');

type ReadyJob = { job: Job; bytes: number; commit: () => void };
type Prefetch = {
  chunks: ChunkCoord[];
  unavailable: number;
  started: number;
  resolve: (result: PrefetchResult) => void;
  previous?: PrefetchProgress;
};

export class ChunkStreamer<T> {
  private readonly visual: StreamerCore;
  private readonly physics: StreamerCore;
  private readonly now: () => number;
  private readonly softBudget: number;
  private readonly fetching = new Map<Job, { controller: AbortController; deadline: number }>();
  private readonly radius: number;
  private readonly ready = new Map<number, ReadyJob>();
  private readonly resident = new Map<number, { chunk: ChunkCoord; bytes: number }>();
  private readonly installed = new Map<number, ChunkCoord>();
  private readonly evicted = new Set<number>();
  private residentBytes = 0;
  private queue: Job[] = [];
  private center: ChunkCoord | undefined;
  private nextScore = 0;
  private time = 0;
  private estimateMsPerMiB = 2;
  private pos: Horizontal | undefined;
  private vel: Horizontal = { x: 0, z: 0 };
  private frustum = alwaysVisible;
  private prefetch: Prefetch | undefined;
  private disposed = false;
  readonly metrics: StreamMetrics = {
    stream_resident: 0, stream_queue_len: 0, stream_upload_ms: 0, stream_upload_bytes: 0, stream_holes: 0,
  };

  constructor(private readonly options: ChunkStreamerOptions<T>) {
    this.radius = options.radius ?? R_MID;
    for (const radius of [this.radius, options.retainRadius ?? this.radius]) {
      if (!Number.isInteger(radius) || radius < 0 || radius >= GRID) throw new RangeError('Invalid streaming radius');
    }
    this.visual = new StreamerCore(options.retryLoad === false ? 0 : undefined);
    this.physics = new StreamerCore(options.retryLoad === false ? 0 : undefined);
    this.now = options.now ?? (() => performance.now());
    this.softBudget = options.softBudgetBytes ?? 64 * 1024 * 1024;
  }

  getState(bx: number, bz: number): ChunkState {
    return this.visual.state(bx, bz);
  }

  /** Prediction gates on successfully installed collision, independently of visual residency. */
  collisionReady(bx: number, bz: number): boolean {
    return this.physics.state(bx, bz) === 'resident';
  }

  update(playerPos: Horizontal, vel: Horizontal, inFrustum: ChunkFrustum, dtSeconds: number): void {
    if (this.disposed) return;
    this.pos = playerPos;
    this.vel = vel;
    this.frustum = inFrustum;
    this.time += dtSeconds;
    const bx = chunkOf(playerPos.x);
    const bz = chunkOf(playerPos.z);
    const target = this.visual.spawnTarget;
    if (!this.prefetch && target?.bx === bx && target.bz === bz) this.clearTarget();
    if (this.center?.bx !== bx || this.center.bz !== bz || this.time >= this.nextScore) this.replan(playerPos);
    this.releaseOutsideRings(bx, bz);
    for (const { controller, deadline } of this.fetching.values()) {
      if (this.time >= deadline) controller.abort(loadTimeout);
    }
    this.uploadFrame();
    this.releaseOutsideRings(bx, bz);
    this.startLoads();
    let holes = 0;
    for (const c of chunksAround(bx, bz, Math.min(R_NEAR, this.radius))) {
      if (inFrustum(c.bx, c.bz) && this.getState(c.bx, c.bz) !== 'resident' && !this.options.hasFallback(c)) holes++;
    }
    this.metrics.stream_holes = holes;
    this.metrics.stream_resident = this.resident.size;
    let pending = 0;
    for (const job of this.queue) if (this.pending(job)) pending++;
    this.metrics.stream_queue_len = pending;
    this.reportPrefetch();
  }

  prefetchAround(pos: Horizontal, opts: { ring?: number } = {}): Promise<PrefetchResult> {
    const ring = opts.ring ?? R_NEAR;
    if (!Number.isInteger(ring) || ring < 0 || ring >= GRID) throw new RangeError('Invalid prefetch ring');
    this.cancelPrefetch();
    const bx = chunkOf(pos.x);
    const bz = chunkOf(pos.z);
    // Count unbaked in-grid chunks as failed without attempting their I/O.
    const requested = [...chunksAround(bx, bz, ring)];
    const chunks = requested.filter((c) => this.options.hasChunk?.(c) !== false);
    if (this.disposed) return Promise.resolve({ type: 'stream.progress', total: requested.length, ready: 0, failed: 0, cancelled: true, elapsedMs: 0 });
    this.visual.spawnTarget = this.physics.spawnTarget = { bx, bz, ring };
    const result = new Promise<PrefetchResult>((resolve) => {
      this.prefetch = { chunks, unavailable: requested.length - chunks.length, resolve, started: this.now() };
    });
    this.replan(this.pos ?? pos);
    this.startLoads();
    this.reportPrefetch();
    return result;
  }

  cancelPrefetch(): void {
    this.clearTarget();
    this.reportPrefetch(true);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.cancelPrefetch();
    for (const [job, { controller }] of this.fetching) {
      controller.abort();
      this.core(job).unload(job.chunk.bx, job.chunk.bz);
    }
    this.fetching.clear();
    for (const { job } of this.ready.values()) this.core(job).unload(job.chunk.bx, job.chunk.bz);
    this.ready.clear();
    for (const { chunk } of this.resident.values()) this.releaseVisual(chunk);
    for (const c of this.installed.values()) this.releaseCollision(c);
    this.queue = [];
    Object.assign(this.metrics, { stream_resident: 0, stream_queue_len: 0, stream_upload_ms: 0, stream_upload_bytes: 0, stream_holes: 0 });
  }

  private core(job: Job): StreamerCore {
    return job.collision ? this.physics : this.visual;
  }

  private jobKey(job: Job): number {
    return 2 * chunkKey(job.chunk.bx, job.chunk.bz) + Number(job.collision);
  }

  private clearTarget(): void {
    if (!this.visual.spawnTarget) return;
    this.visual.spawnTarget = this.physics.spawnTarget = undefined;
    this.nextScore = 0;
  }

  private replan(pos: Horizontal): void {
    const center = { bx: chunkOf(pos.x), bz: chunkOf(pos.z) };
    this.center = center;
    this.nextScore = this.time + RESCORE_S;
    const wanted = this.visual.wanted(pos.x, pos.z, this.vel.x, this.vel.z, this.frustum, this.radius, this.options.headingPrefetch);
    const { retainRadius } = this.options;
    this.queue = [];
    for (const chunk of wanted) {
      if (this.options.hasChunk?.(chunk) === false) continue;
      if (retainRadius !== undefined && !chunk.spawn && ringDist(chunk.bx, chunk.bz, center.bx, center.bz) > retainRadius) continue;
      this.queue.push({ chunk, collision: false });
    }
    if (this.options.collision) {
      const collision = this.physics.wanted(pos.x, pos.z, this.vel.x, this.vel.z, this.frustum, COLLISION_RING, false);
      for (const chunk of collision) {
        if (this.options.hasChunk?.(chunk) !== false) this.queue.push({ chunk, collision: true });
      }
      // Spawn jobs go first, interleaved by score. Nearby collision has priority over other local work.
      this.queue.sort((a, b) => {
        if (a.chunk.spawn !== b.chunk.spawn) return Number(b.chunk.spawn) - Number(a.chunk.spawn);
        if (!a.chunk.spawn && a.collision !== b.collision) return Number(b.collision) - Number(a.collision);
        return b.chunk.score - a.chunk.score || Number(b.collision) - Number(a.collision);
      });
    }
    const keys = new Set(this.queue.map((job) => this.jobKey(job)));
    for (const [job, { controller }] of this.fetching) if (!keys.has(this.jobKey(job))) controller.abort();
    for (const [key, { job }] of this.ready) {
      if (keys.has(key)) continue;
      this.ready.delete(key);
      this.core(job).unload(job.chunk.bx, job.chunk.bz);
    }
  }

  private pending(job: Job): boolean {
    const { bx, bz } = job.chunk;
    const core = this.core(job);
    const state = core.state(bx, bz);
    return state !== 'resident' && !core.dead(bx, bz) && this.admitted(job);
  }

  private admitted(job: Job): boolean {
    if (job.collision || job.chunk.spawn || !this.center) return true;
    const { bx, bz } = job.chunk;
    if (ringDist(bx, bz, this.center.bx, this.center.bz) <= RETAIN_RING) {
      this.evicted.delete(chunkKey(bx, bz));
      return true;
    }
    return this.residentBytes < this.softBudget && !this.evicted.has(chunkKey(bx, bz));
  }

  private startLoads(): void {
    for (const job of this.queue) {
      if (this.fetching.size >= MAX_CONCURRENT_FETCH || this.ready.size + this.fetching.size >= MAX_WAITING) break;
      const { bx, bz } = job.chunk;
      if (!this.admitted(job) || !this.core(job).start(bx, bz, this.time)) continue;
      const controller = new AbortController();
      this.fetching.set(job, { controller, deadline: this.time + (this.options.loadTimeoutSeconds ?? 10) });
      void this.load(job, controller.signal);
    }
  }

  private async load(job: Job, signal: AbortSignal): Promise<void> {
    const { chunk } = job;
    const { bx, bz } = chunk;
    const core = this.core(job);
    try {
      let item: ReadyJob;
      if (job.collision && this.options.collision) {
        const collision = this.options.collision;
        const data = await collision.load(chunk, signal);
        item = { job, bytes: 0, commit: () => {
          collision.install(chunk, data);
          this.installed.set(chunkKey(bx, bz), chunk);
        } };
      } else {
        const { data, uploadBytes } = await this.options.load(chunk, signal);
        if (!Number.isSafeInteger(uploadBytes) || uploadBytes < 0) throw new RangeError('Invalid chunk uploadBytes');
        item = { job, bytes: uploadBytes, commit: () => {
          this.options.upload(chunk, data);
          this.resident.set(chunkKey(bx, bz), { chunk, bytes: uploadBytes });
          this.residentBytes += uploadBytes;
        } };
      }
      if (signal.aborted) return;
      core.fetched(bx, bz);
      this.ready.set(this.jobKey(job), item);
    } catch {
      if (!signal.aborted) core.failed(bx, bz, this.time);
    } finally {
      this.fetching.delete(job);
      if (signal.aborted && !this.disposed) {
        if (signal.reason === loadTimeout) core.failed(bx, bz, this.time);
        else core.unload(bx, bz);
      }
    }
  }

  private uploadFrame(): void {
    let ms = 0;
    let bytes = 0;
    let count = 0;
    for (const [key, item] of this.ready) {
      const { job, bytes: payload } = item;
      const { bx, bz } = job.chunk;
      const core = this.core(job);
      if (!this.admitted(job)) {
        this.ready.delete(key);
        core.unload(bx, bz);
        continue;
      }
      const estimated = (payload / (1024 * 1024)) * this.estimateMsPerMiB;
      // A whole chunk is a synchronous commit. Give the first job a slice to guarantee progress.
      if (count > 0 && (ms >= UPLOAD_BUDGET_MS || ms + estimated > UPLOAD_BUDGET_MS || bytes + payload > UPLOAD_BUDGET_BYTES)) break;
      this.ready.delete(key);
      const start = this.now();
      try {
        item.commit();
        core.uploaded(bx, bz);
      } catch {
        core.failed(bx, bz, this.time);
      }
      const elapsed = this.now() - start;
      if (payload > 0) {
        const rate = (elapsed * 1024 * 1024) / payload;
        this.estimateMsPerMiB += 0.2 * (rate - this.estimateMsPerMiB);
      }
      ms += elapsed;
      bytes += payload;
      count++;
    }
    this.metrics.stream_upload_ms = ms;
    this.metrics.stream_upload_bytes = bytes;
  }

  private releaseOutsideRings(bx: number, bz: number): void {
    const target = this.visual.spawnTarget;
    const pinned = (c: ChunkCoord) => target && ringDist(c.bx, c.bz, target.bx, target.bz) <= (target.ring ?? R_NEAR);
    for (const c of this.installed.values()) {
      if (!pinned(c) && ringDist(c.bx, c.bz, bx, bz) > COLLISION_RING) this.releaseCollision(c);
    }
    const { retainRadius } = this.options;
    if (retainRadius !== undefined) {
      for (const { chunk } of this.resident.values()) {
        if (!pinned(chunk) && ringDist(chunk.bx, chunk.bz, bx, bz) > retainRadius) this.releaseVisual(chunk);
      }
    }
    if (this.residentBytes <= this.softBudget) return;
    const candidates = [...this.resident.values()].map(({ chunk }) => ({ chunk, d: ringDist(chunk.bx, chunk.bz, bx, bz) }))
      .filter(({ chunk, d }) => d > RETAIN_RING && !pinned(chunk)).sort((a, b) => b.d - a.d);
    for (const { chunk } of candidates) {
      if (this.residentBytes <= this.softBudget) break;
      this.evicted.add(chunkKey(chunk.bx, chunk.bz));
      this.releaseVisual(chunk);
    }
  }

  private releaseVisual(chunk: ChunkCoord): void {
    const key = chunkKey(chunk.bx, chunk.bz);
    this.options.evict(chunk);
    this.residentBytes -= this.resident.get(key)!.bytes;
    this.resident.delete(key);
    this.visual.unload(chunk.bx, chunk.bz);
  }

  private releaseCollision(chunk: ChunkCoord): void {
    this.options.collision?.remove(chunk);
    this.installed.delete(chunkKey(chunk.bx, chunk.bz));
    this.physics.unload(chunk.bx, chunk.bz);
  }

  private reportPrefetch(cancelled = false): void {
    const current = this.prefetch;
    if (!current) return;
    let ready = 0;
    let failed = current.unavailable;
    for (const { bx, bz } of current.chunks) {
      if (this.visual.dead(bx, bz) || (this.options.collision && this.physics.dead(bx, bz))) failed++;
      else if (this.getState(bx, bz) === 'resident' && (!this.options.collision || this.collisionReady(bx, bz))) ready++;
    }
    const progress: PrefetchProgress = { type: 'stream.progress', total: current.chunks.length + current.unavailable, ready, failed };
    if (cancelled || ready + failed === progress.total) {
      this.prefetch = undefined;
      current.resolve({ ...progress, cancelled, elapsedMs: this.now() - current.started });
    }
    if (current.previous?.ready !== ready || current.previous.failed !== failed) {
      current.previous = progress;
      this.options.onProgress?.(progress);
    }
  }
}

export { CHUNK_M };
