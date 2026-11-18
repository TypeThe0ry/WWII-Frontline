/**
 * Streaming core (M1-049, ARCHITECTURE 9.3): ring selection, priority score, the per-chunk state
 * machine with bounded retry and exponential backoff, and the upload budget. It does no I/O: the
 * host feeds results back with `fetched` / `failed` / `uploaded` and takes work with `nextFetches`
 * / `nextUploads`, which keeps all of it unit-testable.
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
  bytes: number;
}

export const chunkOf = (m: number): number => Math.min(GRID - 1, Math.max(0, Math.floor(m / CHUNK_M)));
export const chunkKey = (bx: number, bz: number): number => bz * GRID + bx;

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
  /** Chunks missing inside R_near (must stay 0 once resident: "no holes"). */
  holes = 0;
  fetching = 0;
  spawnTarget: { bx: number; bz: number } | undefined;
  uploadsPerFrameMs: number[] = [];

  state(bx: number, bz: number): ChunkState {
    return this.chunks.get(chunkKey(bx, bz))?.state ?? 'unloaded';
  }

  private chunk(bx: number, bz: number): Chunk {
    const k = chunkKey(bx, bz);
    let c = this.chunks.get(k);
    if (!c) {
      c = { state: 'unloaded', retries: 0, retryAt: 0, bytes: 0 };
      this.chunks.set(k, c);
    }
    return c;
  }

  /** Wanted chunks (ring <= R_MID around the player), nearest first by priority. */
  wanted(px: number, pz: number, dx: number, dz: number, inFrustum: (bx: number, bz: number) => boolean): { bx: number; bz: number; score: number }[] {
    const pbx = chunkOf(px);
    const pbz = chunkOf(pz);
    const out: { bx: number; bz: number; score: number }[] = [];
    for (let bz = Math.max(0, pbz - R_MID); bz <= Math.min(GRID - 1, pbz + R_MID); bz++) {
      for (let bx = Math.max(0, pbx - R_MID); bx <= Math.min(GRID - 1, pbx + R_MID); bx++) {
        const d = ringDist(bx, bz, pbx, pbz);
        const st = this.spawnTarget;
        out.push({
          bx,
          bz,
          score: priority({
            d,
            inFrustum: inFrustum(bx, bz),
            headingAhead: headingAhead(px, pz, dx, dz, bx, bz),
            spawnTarget: st !== undefined && ringDist(bx, bz, st.bx, st.bz) <= R_NEAR,
          }),
        });
      }
    }
    return out.sort((a, b) => b.score - a.score || a.bz - b.bz || a.bx - b.bx);
  }

  /** Starts up to the concurrency limit of fetches from `queue` (already priority sorted). */
  nextFetches(queue: readonly { bx: number; bz: number }[], now: number): { bx: number; bz: number }[] {
    const out: { bx: number; bz: number }[] = [];
    for (const q of queue) {
      if (this.fetching + out.length >= MAX_CONCURRENT_FETCH) break;
      const c = this.chunk(q.bx, q.bz);
      const startable = c.state === 'unloaded' || (c.state === 'failed' && c.retries <= MAX_RETRIES && now >= c.retryAt);
      if (!startable) continue;
      c.state = 'fetching';
      out.push(q);
    }
    this.fetching += out.length;
    return out;
  }

  fetched(bx: number, bz: number, bytes: number): void {
    const c = this.chunk(bx, bz);
    if (c.state !== 'fetching') return;
    this.fetching--;
    c.state = 'uploading'; // decode is assumed done by the worker pool before this call
    c.bytes = bytes;
  }

  /** Fetch or decode failed: retry with exponential backoff, give up after MAX_RETRIES. */
  failed(bx: number, bz: number, now: number): void {
    const c = this.chunk(bx, bz);
    if (c.state === 'fetching') this.fetching--;
    c.retries++;
    c.state = 'failed';
    c.retryAt = now + BACKOFF_BASE_S * 2 ** (c.retries - 1);
  }

  /** True when a chunk is permanently failed (retries exhausted). */
  dead(bx: number, bz: number): boolean {
    const c = this.chunks.get(chunkKey(bx, bz));
    return c !== undefined && c.state === 'failed' && c.retries > MAX_RETRIES;
  }

  /** Picks chunks to upload this frame within the budget: <= 1.5 ms (estimated) or <= 2 MB. */
  nextUploads(estMsPerMb = 2): { bx: number; bz: number }[] {
    const out: { bx: number; bz: number }[] = [];
    let ms = 0;
    let bytes = 0;
    for (const [k, c] of this.chunks) {
      if (c.state !== 'uploading') continue;
      const cost = (c.bytes / 1048576) * estMsPerMb;
      if (out.length > 0 && (ms + cost > UPLOAD_BUDGET_MS || bytes + c.bytes > UPLOAD_BUDGET_BYTES)) break;
      ms += cost;
      bytes += c.bytes;
      out.push({ bx: k % GRID, bz: Math.floor(k / GRID) });
    }
    return out;
  }

  uploaded(bx: number, bz: number): void {
    const c = this.chunk(bx, bz);
    if (c.state === 'uploading') c.state = 'resident';
  }

  /** Evicts resident chunks farther than R_NEAR + 2 from the player; returns the evicted ones. */
  evict(px: number, pz: number): { bx: number; bz: number }[] {
    const pbx = chunkOf(px);
    const pbz = chunkOf(pz);
    const out: { bx: number; bz: number }[] = [];
    for (const [k, c] of this.chunks) {
      if (c.state !== 'resident') continue;
      const bx = k % GRID;
      const bz = Math.floor(k / GRID);
      if (ringDist(bx, bz, pbx, pbz) > R_NEAR + 2) {
        c.state = 'unloaded';
        c.retries = 0;
        out.push({ bx, bz });
      }
    }
    return out;
  }

  /** Counts chunks inside R_near of the player that are not resident (the "holes"). */
  countHoles(px: number, pz: number): number {
    const pbx = chunkOf(px);
    const pbz = chunkOf(pz);
    let n = 0;
    for (let bz = Math.max(0, pbz - R_NEAR); bz <= Math.min(GRID - 1, pbz + R_NEAR); bz++) {
      for (let bx = Math.max(0, pbx - R_NEAR); bx <= Math.min(GRID - 1, pbx + R_NEAR); bx++) if (this.state(bx, bz) !== 'resident') n++;
    }
    this.holes = n;
    return n;
  }

  /** Whether the sim may predict into chunk (bx,bz): its collision must be resident (task 6). */
  collisionReady(bx: number, bz: number): boolean {
    const s = this.state(bx, bz);
    return s === 'resident' || s === 'uploading';
  }
}
