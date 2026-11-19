import { decodeBakedChunk, decodeBakedCollision, type BakedChunk } from './chunk-codec';
import type { ChunkCoord } from './chunk-streamer-core';

export type DecodeRequest = { chunk?: ChunkCoord; bytes: Uint8Array };
export type DecodeResponse = { data: BakedChunk | Uint8Array } | { error: string };

const scope = globalThis as unknown as {
  onmessage: ((ev: MessageEvent<DecodeRequest>) => Promise<void>) | null;
  postMessage(data: DecodeResponse, transfer?: Transferable[]): void;
};
scope.onmessage = async ({ data: { chunk, bytes } }) => {
  try {
    if (!chunk) {
      const data = await decodeBakedCollision(bytes);
      scope.postMessage({ data }, [data.buffer]);
      return;
    }
    const data = await decodeBakedChunk(chunk, bytes);
    scope.postMessage({ data }, [data.heights.buffer, data.splat.buffer, data.props.buffer, data.vegetation.buffer, data.hedges.buffer]);
  } catch (error) {
    scope.postMessage({ error: String(error) });
  }
};
