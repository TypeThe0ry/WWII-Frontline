/**
 * Decode Worker entry (M1-006b): `postMessage({id, entryId, kind, bytes})` -> `{id, ok, result}` with Transferable buffers.
 * Textures are parsed and, once the worker has been configured with `{init: {caps, basisBase}}`, transcoded to the best GPU
 * format with the self-hosted Basis transcoder (M1-006c, PI-044); without it they come back as `DecodedTexture`.
 */
import { MeshoptDecoder } from 'meshoptimizer';
import { decodeMesh, decodeTexture, type DecodedMesh, type DecodedTexture } from './entry-decode';
import { loadBasisModule, transcodeTexture, type BasisModule, type GpuCaps, type TranscodedTexture } from './transcode';

export interface DecodeRequest {
  id: number;
  entryId: string;
  kind: 'mesh' | 'texture';
  bytes: Uint8Array;
}
export type DecodeResponse = { id: number; ok: true; kind: 'mesh'; result: DecodedMesh } | { id: number; ok: true; kind: 'texture'; result: DecodedTexture | TranscodedTexture } | { id: number; ok: false; error: string };

/** Buffers of a decoded result that can be moved to the main thread without a copy. */
export function transferables(r: DecodedMesh | DecodedTexture | TranscodedTexture): ArrayBuffer[] {
  const out: ArrayBuffer[] = [];
  if ('primitives' in r) for (const p of r.primitives) {
    out.push(p.indices.buffer as ArrayBuffer);
    for (const a of p.attributes) out.push(a.data.buffer as ArrayBuffer);
  } else for (const l of r.levels) out.push(l.data.buffer as ArrayBuffer);
  return out;
}

/** Configuration sent once to every worker. */
export interface DecodeInit {
  caps: GpuCaps;
  /** URL prefix of `basis_transcoder.{js,wasm}`. */
  basisBase: string;
}

/** Turns a parsed texture into GPU data; absent = textures stay as `DecodedTexture`. */
export type TextureTranscoder = (t: DecodedTexture) => Promise<TranscodedTexture> | TranscodedTexture;

/** Transcoder that loads the Basis module on first use (once per worker). */
export function basisTranscoder(init: DecodeInit, load: (base: string) => Promise<BasisModule> = loadBasisModule): TextureTranscoder {
  let mod: Promise<BasisModule> | undefined;
  return async (t) => transcodeTexture(await (mod ??= load(init.basisBase)), t, init.caps);
}

export async function handleDecode(req: DecodeRequest, transcode?: TextureTranscoder): Promise<DecodeResponse> {
  try {
    if (req.kind === 'mesh') return { id: req.id, ok: true, kind: 'mesh', result: await decodeMesh(req.bytes, MeshoptDecoder) };
    const tex = decodeTexture(req.bytes);
    return { id: req.id, ok: true, kind: 'texture', result: transcode ? await transcode(tex) : tex };
  } catch (e) {
    return { id: req.id, ok: false, error: `${req.entryId}: ${e instanceof Error ? e.message : String(e)}` };
  }
}

// Only wire the message loop when running as a dedicated worker.
if (typeof self !== 'undefined' && typeof (self as { document?: unknown }).document === 'undefined' && typeof (self as { postMessage?: unknown }).postMessage === 'function' && 'onmessage' in self) {
  const scope = self as unknown as { onmessage: ((e: MessageEvent<DecodeRequest | { init: DecodeInit }>) => void) | null; postMessage(m: unknown, t?: Transferable[]): void };
  let transcode: TextureTranscoder | undefined;
  scope.onmessage = (e) => {
    if ('init' in e.data) {
      transcode = basisTranscoder(e.data.init);
      return;
    }
    void handleDecode(e.data, transcode).then((r) => scope.postMessage(r, r.ok ? transferables(r.result) : []));
  };
}
