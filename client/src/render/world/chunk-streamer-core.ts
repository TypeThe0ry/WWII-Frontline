/**
 * Streaming core (M1-049, ARCHITECTURE 9.3): ring selection, priority score, the per-chunk state
 * machine with bounded retry and exponential backoff. I/O and frame scheduling live in ChunkStreamer.
 */
export type ChunkState = 'unloaded' | 'requested' | 'fetching' | 'decoding' | 'uploading' | 'resident' | 'evicting' | 'failed';

export const CHUNK_M = 64;
export const GRID = 32;
export const R_PHYS = 1;
export const R_NEAR = 4;
export const R_MID = 12;
export const MAX_RETRIES = 3;
export const MAX_CONCURRENT_FETCH = 6;
export const UPLOAD_BUDGET_MS = 1.5;
export const UPLOAD_BUDGET_BYTES = 2 * 1024 * 1024;
const BACKOFF_BASE_S = 0.25;

export interface Chunk {
  state: ChunkState;
  retries: number;
  /** Time before which a failed chunk is not retried. */
  retryAt: number;
}

export interface ChunkCoord { bx: number; bz: number }
export interface WantedChunk extends ChunkCoord { score: number; spawn: boolean }
export type ChunkFrustum = (bx: number, bz: number) => boolean;

export const chunkOf = (m: number): number => Math.min(GRID - 1, Math.max(0, Math.floor(m / CHUNK_M)));
export const chunkKey = (bx: number, bz: number): number => bz * GRID + bx;
export const isChunkIndex = (n: number): boolean => Number.isInteger(n) && n >= 0 && n < GRID;

export function* chunksAround(bx: number, bz: number, ring: number): Iterable<ChunkCoord> {
  for (let z = Math.max(0, bz - ring); z <= Math.min(GRID - 1, bz + ring); z++) {
    for (let x = Math.max(0, bx - ring); x <= Math.min(GRID - 1, bx + ring); x++) yield { bx: x, bz: z };
  }
}

/** Ring (Chebyshev) distance between two chunks. */
export const ringDist = (ax: number, az: number, bx: number, bz: number): number => Math.max(Math.abs(ax - bx), Math.abs(az - bz));

export interface PriorityInput {
  /** Chebyshev distance in chunks. */
  d: number;
  inFrustum: boolean;
  headingAhead: boolean;
  spawnTarget: boolean;
}

/** `1/(d+1) x (in_frustum ? 1.5 : 1) x (heading_ahead ? 1.3 : 1) x (spawn_target ? 3 : 1)`. */
export const priority = (p: PriorityInput): number => (1 / (p.d + 1)) * (p.inFrustum ? 1.5 : 1) * (p.headingAhead ? 1.3 : 1) * (p.spawnTarget ? 3 : 1);

/** Whether chunk `(bx,bz)` lies in the +-45 degree cone around the movement direction (dx,dz). */
export function headingAhead(px: number, pz: number, dx: number, dz: number, bx: number, bz: number): boolean {
  const len = Math.hypot(dx, dz);
  if (len < 0.5) return false;
  const cx = (bx + 0.5) * CHUNK_M - px;
  const cz = (bz + 0.5) * CHUNK_M - pz;
  const cl = Math.hypot(cx, cz);
  if (cl < 1e-6) return true;
  return (cx * dx + cz * dz) / (cl * len) >= Math.SQRT1_2;
}

export class StreamerCore {
  private readonly chunks = new Map<number, Chunk>();
  fetching = 0;
  spawnTarget: (ChunkCoord & { ring?: number }) | undefined;

  constructor(private readonly maxRetries = MAX_RETRIES) {}

  state(bx: number, bz: number): ChunkState {
    if (!isChunkIndex(bx) || !isChunkIndex(bz)) return 'unloaded';
    return this.chunks.get(chunkKey(bx, bz))?.state ?? 'unloaded';
  }

  wanted(px: number, pz: number, dx: number, dz: number, inFrustum: ChunkFrustum,
    radius = R_MID, extendHeading = true,
  ): WantedChunk[] {
    const pbx = chunkOf(px);
    const pbz = chunkOf(pz);
    const target = this.spawnTarget;
    const spawnRing = target?.ring ?? R_NEAR;
    const out: WantedChunk[] = [];
    const add = (bx: number, bz: number, spawn: boolean) => {
      out.push({ bx, bz, spawn, score: priority({
        d: spawn && target ? ringDist(bx, bz, target.bx, target.bz) : ringDist(bx, bz, pbx, pbz),
        inFrustum: inFrustum(bx, bz),
        headingAhead: headingAhead(px, pz, dx, dz, bx, bz),
        spawnTarget: spawn,
      }) });
    };
    if (target) for (const { bx, bz } of chunksAround(target.bx, target.bz, spawnRing)) add(bx, bz, true);
    for (const { bx, bz } of chunksAround(pbx, pbz, radius + Number(extendHeading))) {
      if (target && ringDist(bx, bz, target.bx, target.bz) <= spawnRing) continue;
      if (ringDist(bx, bz, pbx, pbz) > radius && !headingAhead(px, pz, dx, dz, bx, bz)) continue;
      add(bx, bz, false);
    }
    return out.sort((a, b) => Number(b.spawn) - Number(a.spawn) || b.score - a.score || a.bz - b.bz || a.bx - b.bx);
  }

  /** Starts one request if its state and backoff permit it. */
  start(bx: number, bz: number, now: number): boolean {
    if (this.fetching >= MAX_CONCURRENT_FETCH) return false;
    const k = chunkKey(bx, bz);
    const c = this.chunks.get(k) ?? { state: 'unloaded', retries: 0, retryAt: 0 };
    if (c.state !== 'unloaded' && (c.state !== 'failed' || c.retries > this.maxRetries || now < c.retryAt)) return false;
    c.state = 'fetching';
    this.chunks.set(k, c);
    this.fetching++;
    return true;
  }

  nextFetches(queue: readonly ChunkCoord[], now: number): ChunkCoord[] {
    const out: ChunkCoord[] = [];
    for (const q of queue) {
      if (this.fetching >= MAX_CONCURRENT_FETCH) break;
      if (this.start(q.bx, q.bz, now)) out.push(q);
    }
    return out;
  }

  fetched(bx: number, bz: number): void {
    const c = this.chunks.get(chunkKey(bx, bz));
    if (c?.state !== 'fetching') return;
    this.fetching--;
    c.state = 'uploading'; // Decode is part of the load job (PI-101).
  }

  failed(bx: number, bz: number, now: number): void {
    const c = this.chunks.get(chunkKey(bx, bz));
    if (c?.state !== 'fetching' && c?.state !== 'uploading') return;
    if (c.state === 'fetching') this.fetching--;
    c.retries++;
    c.state = 'failed';
    c.retryAt = now + BACKOFF_BASE_S * 2 ** (c.retries - 1);
  }

  dead(bx: number, bz: number): boolean {
    const c = this.chunks.get(chunkKey(bx, bz));
    return c?.state === 'failed' && c.retries > this.maxRetries;
  }

  uploaded(bx: number, bz: number): void {
    const c = this.chunks.get(chunkKey(bx, bz));
    if (c?.state === 'uploading') c.state = 'resident';
  }

  unload(bx: number, bz: number): void {
    const k = chunkKey(bx, bz);
    if (this.chunks.get(k)?.state === 'fetching') this.fetching--;
    this.chunks.delete(k);
  }
}
