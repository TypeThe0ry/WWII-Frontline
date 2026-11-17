/**
 * KTX2 (UASTC / ETC1S) -> GPU format transcoding with the self-hosted Basis Universal transcoder
 * (`public/basis/basis_transcoder.{js,wasm}`, Apache-2.0, see LICENSES.md section 5, PI-044). Runs inside the decode
 * Worker. The transcoder module is injected so tests (and a future main-thread fallback) can load it their own way.
 */
import type { DecodedTexture } from './entry-decode';
import { rebuildKtx2 } from './ktx2-rebuild';

export type GpuTextureFormat = 'bc7' | 'astc4x4' | 'etc2' | 'bc3' | 'rgba8';

export interface GpuCaps {
  bc7?: boolean;
  astc?: boolean;
  etc2?: boolean;
  bc3?: boolean;
}

/** Basis `transcoder_texture_format` values. */
const BASIS_FORMAT: Record<GpuTextureFormat, number> = { etc2: 1, bc3: 3, bc7: 7, astc4x4: 10, rgba8: 13 };

/** Preference order: quality first (BC7 / ASTC), then the older block formats, RGBA8 always works. */
export function pickFormat(caps: GpuCaps): GpuTextureFormat {
  if (caps.bc7) return 'bc7';
  if (caps.astc) return 'astc4x4';
  if (caps.etc2) return 'etc2';
  if (caps.bc3) return 'bc3';
  return 'rgba8';
}

export interface TranscodedTexture {
  format: GpuTextureFormat;
  width: number;
  height: number;
  /** Largest mip first. */
  levels: { data: Uint8Array; width: number; height: number }[];
}

/** The slice of the Emscripten `BASIS` module that we use. */
export interface BasisKtx2File {
  isValid(): boolean;
  getWidth(): number;
  getHeight(): number;
  getLevels(): number;
  getHasAlpha(): boolean;
  startTranscoding(): boolean;
  getImageTranscodedSizeInBytes(level: number, layer: number, face: number, format: number): number;
  transcodeImage(dst: Uint8Array, level: number, layer: number, face: number, format: number, getAlphaForOpaqueFormats: number, channel0: number, channel1: number): boolean;
  close(): void;
  delete(): void;
}
export interface BasisModule {
  initializeBasis(): void;
  KTX2File: new (data: Uint8Array) => BasisKtx2File;
}

export class TranscodeError extends Error {}

export function transcodeTexture(basis: BasisModule, tex: DecodedTexture, caps: GpuCaps): TranscodedTexture {
  const format = pickFormat(caps);
  const file = new basis.KTX2File(rebuildKtx2(tex));
  try {
    if (!file.isValid()) throw new TranscodeError('invalid KTX2 data');
    if (!file.startTranscoding()) throw new TranscodeError('transcoder failed to start');
    const code = BASIS_FORMAT[format];
    const levels: TranscodedTexture['levels'] = [];
    const n = file.getLevels();
    for (let l = 0; l < n; l++) {
      const size = file.getImageTranscodedSizeInBytes(l, 0, 0, code);
      const data = new Uint8Array(size);
      if (!file.transcodeImage(data, l, 0, 0, code, 0, -1, -1)) throw new TranscodeError(`level ${l} failed to transcode`);
      levels.push({ data, width: Math.max(1, file.getWidth() >> l), height: Math.max(1, file.getHeight() >> l) });
    }
    return { format, width: file.getWidth(), height: file.getHeight(), levels };
  } finally {
    file.close();
    file.delete();
  }
}

/**
 * Loads the module in a (module) Worker or the main thread: the Emscripten wrapper is a classic script, so it is fetched
 * as text and evaluated instead of `importScripts` (not available in module workers).
 */
export async function loadBasisModule(baseUrl: string, fetchFn: typeof fetch = fetch): Promise<BasisModule> {
  const [js, wasm] = await Promise.all([
    fetchFn(`${baseUrl}/basis_transcoder.js`).then((r) => r.text()),
    fetchFn(`${baseUrl}/basis_transcoder.wasm`).then((r) => r.arrayBuffer()),
  ]);
  return instantiateBasis(js, wasm);
}

/** `scope` is only for tests: the Emscripten wrapper takes its Node code path (`require`, `__dirname`...) when it detects Node. */
export async function instantiateBasis(js: string, wasmBinary: ArrayBuffer, scope: Record<string, unknown> = {}): Promise<BasisModule> {
  // The script ends with `module.exports = BASIS` when `exports`/`module` exist; give it ours.
  const mod: { exports: unknown } = { exports: {} };
  new Function('module', 'exports', ...Object.keys(scope), js)(mod, mod.exports, ...Object.values(scope));
  const factory = mod.exports as (opts: { wasmBinary: ArrayBuffer }) => Promise<BasisModule>;
  const m = await factory({ wasmBinary });
  m.initializeBasis();
  return m;
}
