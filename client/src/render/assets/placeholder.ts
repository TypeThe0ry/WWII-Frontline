/**
 * Grey-box fallback naming (ASSET_GAPS 3.1): material `PH_{reqId}`, node `PH_{reqId}__{instance}`. The loader only decides names
 * and whether a request needs a placeholder; building Babylon objects belongs to the render layer.
 */
export const PH_RE = /^PH_[A-Z0-9][A-Z0-9-]*(__[A-Za-z0-9_.-]+)?$/;

export interface Placeholder {
  placeholder: true;
  entryId: string;
  reqId: string;
  materialName: string;
  nodeName: string;
  reason: string;
}

/** Entries without a manifest requirement id fall back to `UNKNOWN`-prefixed names derived from the logical id. */
export function makePlaceholder(entryId: string, reqId: string | undefined, instance: string, reason: string): Placeholder {
  const req = reqId ?? 'UNKNOWN';
  return { placeholder: true, entryId, reqId: req, materialName: `PH_${req}`, nodeName: `PH_${req}__${instance}`, reason };
}

export function isPlaceholder(v: unknown): v is Placeholder {
  return typeof v === 'object' && v !== null && (v as { placeholder?: unknown }).placeholder === true;
}

/** 0.1 s of silence at 48 kHz mono (ASSET_GAPS 3.3) as raw samples; `decodeAudioData` is not needed for it. */
export const SILENCE_SECONDS = 0.1;
export function silence(sampleRate = 48000): Float32Array {
  return new Float32Array(Math.round(SILENCE_SECONDS * sampleRate));
}

/** One log line per requirement id per session. */
export class OncePerSession {
  readonly #seen = new Set<string>();
  constructor(private readonly sink: (msg: string) => void) {}
  log(key: string, msg: string): void {
    if (this.#seen.has(key)) return;
    this.#seen.add(key);
    this.sink(msg);
  }
}
