import type { AssetLoader } from '../assets';
import { createChunkDecoder, type ChunkDecoder } from './chunk-decoder';
import type { BakedChunk } from './chunk-codec';
import { CHUNK_M, chunkKey, isChunkIndex, type ChunkCoord } from './chunk-streamer-core';
import type { ChunkSource } from './chunk-streamer';

export type { BakedChunk } from './chunk-codec';

const text = new TextDecoder();

export interface ChunkIndexEntry extends ChunkCoord {
  key: string;
  files: readonly [visual: string, collision: string];
  bytes: number;
}

/** The M1-035 v1 bake index. Fpak ranges come from AssetLoader's manifest and pack TOC. */
export class ChunkIndex {
  private readonly entries = new Map<number, ChunkIndexEntry>();

  constructor(raw: unknown) {
    if (!raw || typeof raw !== 'object') throw new Error('Missing chunk index');
    const index = raw as { schema?: unknown; format_version?: unknown; chunk_m?: unknown; chunks?: unknown; count?: unknown };
    if (index.schema !== 'frontline.map.chunk_index/1' || index.format_version !== 1 || index.chunk_m !== CHUNK_M ||
        !Array.isArray(index.chunks) || index.count !== index.chunks.length) throw new Error('Invalid v1 chunk index');
    for (const row of index.chunks as ChunkIndexEntry[]) {
      if (!row || !isChunkIndex(row.bx) || !isChunkIndex(row.bz) || typeof row.key !== 'string' || !row.key ||
          !Array.isArray(row.files) || row.files.length !== 2 || !row.files.every((f) => typeof f === 'string' && f.length > 0) ||
          !Number.isSafeInteger(row.bytes) || row.bytes <= 0) throw new Error('Invalid chunk index entry');
      const key = chunkKey(row.bx, row.bz);
      if (this.entries.has(key)) throw new Error(`Duplicate chunk ${row.bx},${row.bz}`);
      this.entries.set(key, row);
    }
  }

  get(bx: number, bz: number): ChunkIndexEntry | undefined {
    return isChunkIndex(bx) && isChunkIndex(bz) ? this.entries.get(chunkKey(bx, bz)) : undefined;
  }
}

type AssetBytes = Pick<AssetLoader, 'loadBytes' | 'createToken' | 'cancel'>;

async function loadAsset(loader: AssetBytes, id: string, signal?: AbortSignal): Promise<Uint8Array> {
  signal?.throwIfAborted();
  const token = loader.createToken();
  const abort = () => loader.cancel(token);
  signal?.addEventListener('abort', abort, { once: true });
  try {
    const bytes = await loader.loadBytes(id, { token });
    signal?.throwIfAborted();
    return bytes;
  } finally {
    signal?.removeEventListener('abort', abort);
  }
}

// The logical path chunks/index must be mapped to a valid fpak identifier by the manifest.
export async function loadChunkIndex(loader: AssetBytes, entryId = 'chunks_index', signal?: AbortSignal): Promise<ChunkIndex> {
  const bytes = await loadAsset(loader, entryId, signal);
  return new ChunkIndex(JSON.parse(text.decode(bytes)));
}

interface ChunkEntryIds { visual: string; collision: string }

export interface FpakChunkSource extends ChunkSource<BakedChunk> {
  loadCollision(chunk: ChunkCoord, signal: AbortSignal): Promise<Uint8Array>;
  dispose(): void;
}

export function fpakChunkSource(
  loader: AssetBytes,
  index: ChunkIndex,
  /** The pack manifest chooses the two entry IDs for each baked row. */
  entries: (entry: ChunkIndexEntry) => ChunkEntryIds,
  decoder: ChunkDecoder = createChunkDecoder(),
): FpakChunkSource {
  const getIds = ({ bx, bz }: ChunkCoord): ChunkEntryIds => {
    const entry = index.get(bx, bz);
    if (!entry) throw new Error(`No indexed chunk ${bx},${bz}`);
    return entries(entry);
  };
  return {
    retryLoad: false,
    dispose: () => decoder.dispose(),
    hasChunk: ({ bx, bz }) => index.get(bx, bz) !== undefined,
    async load(chunk, signal) {
      const bytes = await loadAsset(loader, getIds(chunk).visual, signal);
      const data = await decoder.visual(chunk, bytes, signal);
      const uploadBytes = data.heights.byteLength + data.splat.byteLength + data.props.byteLength + data.vegetation.byteLength + data.hedges.byteLength;
      return { data, uploadBytes };
    },
    async loadCollision(chunk, signal) {
      return decoder.collision(await loadAsset(loader, getIds(chunk).collision, signal), signal);
    },
  };
}
