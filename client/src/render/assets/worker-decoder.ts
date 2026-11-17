/** Main-thread `Decoder` backed by real Workers (browser only; tests use `inlineDecoder`). */
import type { Decoder, Decoded } from './index';
import { handleDecode, transferables, type DecodeInit, type DecodeRequest, type DecodeResponse, type TextureTranscoder } from './decode-worker';

/** Decodes on the calling thread; same code path as the Worker, used in tests and as a no-Worker fallback. */
export const inlineDecoder: Decoder = {
  async decode(entryId, kind, bytes): Promise<Decoded> {
    const r = await handleDecode({ id: 0, entryId, kind, bytes });
    if (!r.ok) throw new Error(r.error);
    return { entryId, kind, bytes, data: r.result };
  },
};

/** Same as `inlineDecoder` but transcoding textures with `transcode` (tests, no-Worker fallback). */
export function inlineDecoderWith(transcode: TextureTranscoder): Decoder {
  return {
    async decode(entryId, kind, bytes): Promise<Decoded> {
      const r = await handleDecode({ id: 0, entryId, kind, bytes }, transcode);
      if (!r.ok) throw new Error(r.error);
      return { entryId, kind, bytes, data: r.result };
    },
  };
}

/** One Worker per pool slot; the pool (`WorkerPool`) guarantees at most one in-flight request per Worker instance it owns. */
export function workerDecoder(size: number, create: () => Worker = () => new Worker(new URL('./decode-worker.ts', import.meta.url), { type: 'module' }), init?: DecodeInit): Decoder & { terminate(): void } {
  const workers = Array.from({ length: size }, () => {
    const w = create();
    if (init) w.postMessage({ init }); // GPU caps + transcoder location; textures are then returned transcoded
    return { w, busy: false };
  });
  const waiting: (() => void)[] = [];
  let seq = 0;
  const acquire = async () => {
    for (;;) {
      const free = workers.find((x) => !x.busy);
      if (free) {
        free.busy = true;
        return free;
      }
      await new Promise<void>((r) => waiting.push(r));
    }
  };
  return {
    async decode(entryId, kind, bytes) {
      const slot = await acquire();
      try {
        const id = ++seq;
        const res = await new Promise<DecodeResponse>((resolve, reject) => {
          slot.w.onmessage = (e: MessageEvent<DecodeResponse>) => resolve(e.data);
          slot.w.onerror = (e) => reject(new Error(e.message));
          const req: DecodeRequest = { id, entryId, kind, bytes };
          slot.w.postMessage(req); // input bytes are cloned: the cache keeps its copy
        });
        if (!res.ok) throw new Error(res.error);
        return { entryId, kind, bytes, data: res.result };
      } finally {
        slot.busy = false;
        waiting.shift()?.();
      }
    },
    terminate: () => workers.forEach((x) => x.w.terminate()),
  };
}

export { transferables };
