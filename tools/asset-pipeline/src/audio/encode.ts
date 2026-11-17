/** Opus-in-WebM (primary) and AAC-LC-in-MP4 (fallback) encoding, 48 kHz, all metadata dropped (ARCHITECTURE 12.7, D-AUD). */
import { ffmpeg } from './ffmpeg.ts';
import type { AudioClass } from './loudness.ts';

export const SAMPLE_RATE = 48000;

/** Bit rate in kbit/s per class: near gun layer 96, ambience 32, everything else 64 (card: 48-64 [to be tuned]). */
export function bitrateFor(cls: AudioClass): number {
  return cls === 'gun_near' ? 96 : cls === 'ambience' ? 32 : 64;
}

const COMMON = ['-ar', String(SAMPLE_RATE), '-map_metadata', '-1', '-fflags', '+bitexact', '-flags:a', '+bitexact'];

export async function encodeOpusWebm(wav: string, out: string, o: { bitrate: number; channels?: number }): Promise<void> {
  await ffmpeg(['-i', wav, ...(o.channels ? ['-ac', String(o.channels)] : []), ...COMMON, '-c:a', 'libopus', '-b:a', `${o.bitrate}k`, '-vbr', 'off', // CBR: predictable pack sizes (VBR overshoots the nominal rate on tonal material)
     '-application', 'audio', '-f', 'webm', out]);
}

export async function encodeAac(wav: string, out: string, o: { bitrate: number; channels?: number }): Promise<void> {
  await ffmpeg(['-i', wav, ...(o.channels ? ['-ac', String(o.channels)] : []), ...COMMON, '-c:a', 'aac', '-b:a', `${o.bitrate}k`, '-movflags', '+faststart', '-f', 'mp4', out]);
}

/** Decode any audio file to mono 48 kHz PCM WAV bytes (tests and verification). */
export async function decodeToWav(file: string, out: string): Promise<void> {
  await ffmpeg(['-i', file, '-ac', '1', '-ar', String(SAMPLE_RATE), '-c:a', 'pcm_s16le', '-map_metadata', '-1', out]);
}
