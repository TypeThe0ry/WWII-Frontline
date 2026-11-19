import { WorkerPool } from '../assets/worker-pool';
import { decodeBakedChunk, decodeBakedCollision, type BakedChunk } from './chunk-codec';
import type { DecodeRequest, DecodeResponse } from './chunk-decode-worker';
import type { ChunkCoord } from './chunk-streamer-core';

export interface ChunkDecoder {
  visual(chunk: ChunkCoord, bytes: Uint8Array, signal: AbortSignal): Promise<BakedChunk>;
  collision(bytes: Uint8Array, signal: AbortSignal): Promise<Uint8Array>;
  dispose(): void;
}

/** Lazy workers, one decode per slot. Cached compressed bytes stay owned by AssetLoader. */
export function createChunkDecoder(): ChunkDecoder {
  let disposed = false;
  const idle: Worker[] = [];
  const active = new Map<Worker, (err: Error) => void>();
  const pool = new WorkerPool(async (request: DecodeRequest): Promise<BakedChunk | Uint8Array> => {
    if (disposed) throw new Error('Chunk decoder disposed');
    if (typeof Worker === 'undefined') {
      return request.chunk ? decodeBakedChunk(request.chunk, request.bytes) : decodeBakedCollision(request.bytes);
    }
    const worker = idle.pop() ?? new Worker(new URL('./chunk-decode-worker.ts', import.meta.url), { type: 'module' });
    try {
      const reply = await new Promise<DecodeResponse>((resolve, reject) => {
        active.set(worker, reject);
        worker.onmessage = (event: MessageEvent<DecodeResponse>) => resolve(event.data);
        worker.onerror = (event) => reject(new Error(event.message));
        worker.onmessageerror = () => reject(new Error('Invalid decode worker message'));
        worker.postMessage(request);
      });
      if (!disposed) idle.push(worker);
      if ('error' in reply) throw new Error(reply.error);
      return reply.data;
    } catch (err) {
      // A codec error leaves the worker reusable. Transport failures discard the slot.
      if (!idle.includes(worker)) worker.terminate();
      throw err;
    } finally {
      worker.onmessage = worker.onerror = worker.onmessageerror = null;
      active.delete(worker);
    }
  });
  async function submit(request: DecodeRequest, signal: AbortSignal) {
    signal.throwIfAborted();
    const task = pool.submit(request);
    signal.addEventListener('abort', task.cancel, { once: true });
    try {
      return await task.promise;
    } finally {
      signal.removeEventListener('abort', task.cancel);
    }
  }
  return {
    async visual(chunk, bytes, signal) {
      const data = await submit({ chunk, bytes }, signal);
      if (data instanceof Uint8Array) throw new Error('Expected visual chunk');
      return data;
    },
    async collision(bytes, signal) {
      const data = await submit({ bytes }, signal);
      if (!(data instanceof Uint8Array)) throw new Error('Expected collision chunk');
      return data;
    },
    dispose() {
      disposed = true;
      for (const [worker, reject] of active) {
        worker.terminate();
        reject(new Error('Chunk decoder disposed'));
      }
      for (const worker of idle) worker.terminate();
      idle.length = 0;
    },
  };
}
