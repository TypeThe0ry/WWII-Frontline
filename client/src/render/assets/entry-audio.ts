/**
 * Browser-safe `.fpak` audio entry decoder and codec probe parser (M1-004 / M1-006, PROBE.md).
 * Matches `tools/asset-pipeline/src/fpak/entry-audio.ts` layout v1 byte-for-byte; no Node APIs.
 *
 * `u8 layout_version=1 | u8 container (0 webm-opus, 1 mp4-aac) | u8 flags (bit0 preload, bit1 loop) | u8 channels |
 *  u32 duration_ms | u32 sample_rate | u32 loop_start_ms | u32 loop_end_ms | u32 payload_bytes | u32 reserved | u32 reserved`
 */
import { EntryFormatError } from './entry-decode';

export const AUDIO_LAYOUT_VERSION = 1;
export const AUDIO_HEADER_BYTES = 32;

export type AudioContainer = 'webm-opus' | 'mp4-aac';

export interface DecodedAudioEntry {
  container: AudioContainer;
  preload: boolean;
  loop: boolean;
  channels: number;
  durationMs: number;
  sampleRate: number;
  loopStartMs: number;
  loopEndMs: number;
  /** Complete encoded WebM-Opus or MP4-AAC container bytes ready for `AudioContext.decodeAudioData`. */
  payload: Uint8Array;
}

export function decodeAudioEntry(b: Uint8Array): DecodedAudioEntry {
  if (b.byteLength < AUDIO_HEADER_BYTES) throw new EntryFormatError('audio entry truncated');
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  if (dv.getUint8(0) !== AUDIO_LAYOUT_VERSION) throw new EntryFormatError(`unknown audio layout version ${dv.getUint8(0)}`);
  const containerByte = dv.getUint8(1);
  if (containerByte !== 0 && containerByte !== 1) throw new EntryFormatError(`unknown audio container code ${containerByte}`);
  const payloadBytes = dv.getUint32(20, true);
  if (AUDIO_HEADER_BYTES + payloadBytes !== b.byteLength) {
    throw new EntryFormatError(`audio entry length mismatch (${b.byteLength} vs header ${AUDIO_HEADER_BYTES + payloadBytes})`);
  }
  const flags = dv.getUint8(2);
  return {
    container: containerByte === 0 ? 'webm-opus' : 'mp4-aac',
    preload: (flags & 1) !== 0,
    loop: (flags & 2) !== 0,
    channels: dv.getUint8(3),
    durationMs: dv.getUint32(4, true),
    sampleRate: dv.getUint32(8, true),
    loopStartMs: dv.getUint32(12, true),
    loopEndMs: dv.getUint32(16, true),
    payload: b.slice(AUDIO_HEADER_BYTES),
  };
}

export interface ProbeFileJson {
  schema: string;
  timeout_ms: number;
  opus: { mime: string; base64: string; bytes: number };
  aac: { mime: string; base64: string; bytes: number };
}

export interface DecodedProbeSamples {
  timeoutMs: number;
  opus: ArrayBuffer;
  aac: ArrayBuffer;
}

function fromBase64(b64: string): ArrayBuffer {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out.buffer;
}

/** Parse `probe.json` written by `asset-pipeline audio-pack` into ArrayBuffers for `probeCodec()`. */
export function parseProbeJson(raw: unknown): DecodedProbeSamples {
  const p = raw as Partial<ProbeFileJson> | null;
  if (!p || p.schema !== 'frontline.audio-probe/1' || !p.opus?.base64 || !p.aac?.base64) {
    throw new EntryFormatError('invalid audio probe.json');
  }
  const timeoutMs = typeof p.timeout_ms === 'number' && p.timeout_ms > 0 ? p.timeout_ms : 500;
  return { timeoutMs, opus: fromBase64(p.opus.base64), aac: fromBase64(p.aac.base64) };
}
