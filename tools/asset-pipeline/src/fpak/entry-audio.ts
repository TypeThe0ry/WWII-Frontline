/**
 * Audio entry layout v1 (codec `Codec.audio`): own 32-byte header + the encoded stream (Opus-in-WebM or AAC-LC-in-MP4) that
 * `decodeAudioData` accepts. The header keeps the entry from being a standalone media file; it is not protection.
 *
 * `u8 layout_version=1 | u8 container (0 webm-opus, 1 mp4-aac) | u8 flags (bit0 preload, bit1 loop) | u8 channels |
 *  u32 duration_ms | u32 sample_rate | u32 loop_start_ms | u32 loop_end_ms | u32 payload_bytes | u32 reserved | u32 reserved`
 */
import { FpakFormatError } from './format.ts';

export const AUDIO_LAYOUT_VERSION = 1;
export const AUDIO_HEADER_BYTES = 32;
/** Clips shorter than this are decoded once and kept resident; longer ones are streamed (ARCHITECTURE 12.7). */
export const PRELOAD_MAX_SECONDS = 3;

export type AudioContainer = 'webm-opus' | 'mp4-aac';

export interface AudioEntry {
  container: AudioContainer;
  preload: boolean;
  loop: boolean;
  channels: number;
  durationMs: number;
  sampleRate: number;
  loopStartMs: number;
  loopEndMs: number;
  payload: Uint8Array;
}

/** `preload` rule: strictly shorter than 3 s. */
export function isPreload(durationSeconds: number): boolean {
  return durationSeconds < PRELOAD_MAX_SECONDS;
}

export function encodeAudioEntry(a: AudioEntry): Uint8Array {
  const out = new Uint8Array(AUDIO_HEADER_BYTES + a.payload.byteLength);
  const dv = new DataView(out.buffer);
  dv.setUint8(0, AUDIO_LAYOUT_VERSION);
  dv.setUint8(1, a.container === 'webm-opus' ? 0 : 1);
  dv.setUint8(2, (a.preload ? 1 : 0) | (a.loop ? 2 : 0));
  dv.setUint8(3, a.channels);
  dv.setUint32(4, a.durationMs, true);
  dv.setUint32(8, a.sampleRate, true);
  dv.setUint32(12, a.loopStartMs, true);
  dv.setUint32(16, a.loopEndMs, true);
  dv.setUint32(20, a.payload.byteLength, true);
  out.set(a.payload, AUDIO_HEADER_BYTES);
  return out;
}

export function decodeAudioEntry(b: Uint8Array): AudioEntry {
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  if (b.byteLength < AUDIO_HEADER_BYTES || dv.getUint8(0) !== AUDIO_LAYOUT_VERSION) throw new FpakFormatError('unknown audio layout version');
  const n = dv.getUint32(20, true);
  if (AUDIO_HEADER_BYTES + n !== b.byteLength) throw new FpakFormatError('audio entry length mismatch');
  const flags = dv.getUint8(2);
  return {
    container: dv.getUint8(1) === 0 ? 'webm-opus' : 'mp4-aac', preload: (flags & 1) !== 0, loop: (flags & 2) !== 0, channels: dv.getUint8(3),
    durationMs: dv.getUint32(4, true), sampleRate: dv.getUint32(8, true), loopStartMs: dv.getUint32(12, true), loopEndMs: dv.getUint32(16, true),
    payload: b.slice(AUDIO_HEADER_BYTES),
  };
}
