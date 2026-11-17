/**
 * Decode Worker entry (M1-006b): `postMessage({id, entryId, kind, bytes})` -> `{id, ok, result}` with Transferable buffers.
 * KTX2 -> GPU format transcoding (BasisU transcoder WASM) is NOT here yet (PI-038): textures are parsed and returned as
 * `DecodedTexture` for a transcoder stage to consume.
 */
import { MeshoptDecoder } from 'meshoptimizer';
import { decodeMesh, decodeTexture, type DecodedMesh, type DecodedTexture } from './entry-decode';

export interface DecodeRequest {
  id: number;
  entryId: string;
  kind: 'mesh' | 'texture';
  bytes: Uint8Array;
}
export type DecodeResponse = { id: number; ok: true; kind: 'mesh'; result: DecodedMesh } | { id: number; ok: true; kind: 'texture'; result: DecodedTexture } | { id: number; ok: false; error: string };

/** Buffers of a decoded result that can be moved to the main thread without a copy. */
export function transferables(r: DecodedMesh | DecodedTexture): ArrayBuffer[] {
  const out: ArrayBuffer[] = [];
  if ('primitives' in r) for (const p of r.primitives) {
    out.push(p.indices.buffer as ArrayBuffer);
    for (const a of p.attributes) out.push(a.data.buffer as ArrayBuffer);
  } else for (const l of r.levels) out.push(l.data.buffer as ArrayBuffer);
  return out;
}

export async function handleDecode(req: DecodeRequest): Promise<DecodeResponse> {
  try {
    if (req.kind === 'mesh') return { id: req.id, ok: true, kind: 'mesh', result: await decodeMesh(req.bytes, MeshoptDecoder) };
    return { id: req.id, ok: true, kind: 'texture', result: decodeTexture(req.bytes) };
  } catch (e) {
    return { id: req.id, ok: false, error: `${req.entryId}: ${e instanceof Error ? e.message : String(e)}` };
  }
}

// Only wire the message loop when running as a dedicated worker.
if (typeof self !== 'undefined' && typeof (self as { document?: unknown }).document === 'undefined' && typeof (self as { postMessage?: unknown }).postMessage === 'function' && 'onmessage' in self) {
  const scope = self as unknown as { onmessage: ((e: MessageEvent<DecodeRequest>) => void) | null; postMessage(m: unknown, t?: Transferable[]): void };
  scope.onmessage = (e) => {
    void handleDecode(e.data).then((r) => scope.postMessage(r, r.ok ? transferables(r.result) : []));
  };
}
