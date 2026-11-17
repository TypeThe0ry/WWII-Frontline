/** EBU R128 measurement and two-pass `loudnorm` normalisation (ffmpeg). Targets per class are [to be tuned]. */
import { ffmpeg } from './ffmpeg.ts';

export type AudioClass = 'gun_near' | 'gun_mid' | 'gun_far' | 'foley' | 'ambience' | 'ui' | 'music' | 'voice';

/** Integrated loudness targets in LUFS (card: near SFX -18, ambience -24, UI -20; the rest follow the nearest class). */
export const TARGET_LUFS: Record<AudioClass, number> = {
  gun_near: -18, gun_mid: -20, gun_far: -22, foley: -20, ambience: -24, ui: -20, music: -20, voice: -18,
};
/** True-peak ceiling handed to loudnorm (the acceptance limit is -1 dBTP). */
export const TRUE_PEAK_TARGET = -1.5;

export interface Loudness {
  integrated_lufs: number;
  true_peak_dbtp: number;
  lra: number;
}

function parseJson(stderr: string): Record<string, string> {
  const i = stderr.lastIndexOf('{');
  const j = stderr.lastIndexOf('}');
  if (i < 0 || j < i) throw new Error('loudnorm printed no JSON');
  return JSON.parse(stderr.slice(i, j + 1)) as Record<string, string>;
}

const num = (s: string | undefined): number => {
  const v = Number(s);
  return Number.isFinite(v) ? v : -70;
};

/** Measure integrated loudness / true peak of a file (loudnorm analysis pass). */
export async function measure(file: string): Promise<Loudness> {
  const r = await ffmpeg(['-i', file, '-af', `loudnorm=I=-23:TP=${TRUE_PEAK_TARGET}:LRA=11:print_format=json`, '-f', 'null', '-']);
  const j = parseJson(r.stderr);
  return { integrated_lufs: num(j.input_i), true_peak_dbtp: num(j.input_tp), lra: num(j.input_lra) };
}

/**
 * Two-pass loudness normalisation to `targetLufs`; writes 48 kHz 16-bit PCM WAV to `outWav` (an intermediate: it never
 * leaves the work directory). Metadata is dropped.
 */
export async function normalizeLoudness(input: string, outWav: string, targetLufs: number): Promise<{ before: Loudness; after: Loudness; measurable: boolean }> {
  const a = await ffmpeg(['-i', input, '-af', `loudnorm=I=${targetLufs}:TP=${TRUE_PEAK_TARGET}:LRA=11:print_format=json`, '-f', 'null', '-']);
  const j = parseJson(a.stderr);
  const measured = Number(j.input_i);
  if (!Number.isFinite(measured) || measured <= -70) {
    // Clips shorter than the 400 ms momentary window (or silent ones) have no R128 loudness: fall back to peak normalisation.
    const v = await ffmpeg(['-i', input, '-af', 'volumedetect', '-f', 'null', '-']);
    const peak = Number(/max_volume:\s*(-?[\d.]+) dB/.exec(v.stderr)?.[1] ?? -90);
    const gain = peak <= -80 ? 0 : -3 - peak;
    await ffmpeg(['-i', input, '-af', `volume=${gain}dB`, '-ar', '48000', '-c:a', 'pcm_s16le', '-map_metadata', '-1', '-fflags', '+bitexact', '-flags:a', '+bitexact', outWav]);
    return { before: { integrated_lufs: -70, true_peak_dbtp: peak, lra: 0 }, after: { integrated_lufs: -70, true_peak_dbtp: peak + gain, lra: 0 }, measurable: false };
  }
  const filter =
    `loudnorm=I=${targetLufs}:TP=${TRUE_PEAK_TARGET}:LRA=11:measured_I=${j.input_i}:measured_TP=${j.input_tp}:measured_LRA=${j.input_lra}` +
    `:measured_thresh=${j.input_thresh}:offset=${j.target_offset}:linear=true`;
  await ffmpeg(['-i', input, '-af', filter, '-ar', '48000', '-c:a', 'pcm_s16le', '-map_metadata', '-1', '-fflags', '+bitexact', '-flags:a', '+bitexact', outWav]);
  return { before: { integrated_lufs: num(j.input_i), true_peak_dbtp: num(j.input_tp), lra: num(j.input_lra) }, after: await measure(outWav), measurable: true };
}
