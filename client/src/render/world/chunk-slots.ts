import { GRID, R_NEAR, chunkKey, isChunkIndex } from './chunk-streamer-core';

export const MEDIUM_CHUNK_SLOTS = (2 * R_NEAR + 1) ** 2;
export const ULTRA_CHUNK_SLOTS = 13 ** 2;

/** Stable layer indices shared by the renderer's height and splat arrays. */
export class ChunkSlots {
  private readonly assigned = new Map<number, number>();
  private readonly free: number[];

  constructor(readonly capacity = MEDIUM_CHUNK_SLOTS) {
    if (!Number.isInteger(capacity) || capacity < 1 || capacity > GRID ** 2) throw new RangeError('Invalid chunk slot capacity');
    this.free = Array.from({ length: capacity }, (_, i) => capacity - i - 1);
  }

  get(bx: number, bz: number): number | undefined {
    return isChunkIndex(bx) && isChunkIndex(bz) ? this.assigned.get(chunkKey(bx, bz)) : undefined;
  }

  acquire(bx: number, bz: number): number | undefined {
    if (!isChunkIndex(bx) || !isChunkIndex(bz)) return undefined;
    const key = chunkKey(bx, bz);
    const assigned = this.assigned.get(key);
    if (assigned !== undefined) return assigned;
    const slot = this.free.pop();
    if (slot !== undefined) this.assigned.set(key, slot);
    return slot;
  }

  release(bx: number, bz: number): void {
    if (!isChunkIndex(bx) || !isChunkIndex(bz)) return;
    const key = chunkKey(bx, bz);
    const slot = this.assigned.get(key);
    if (slot === undefined) return;
    this.assigned.delete(key);
    this.free.push(slot);
  }
}
