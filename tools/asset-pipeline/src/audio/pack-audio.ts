/**
 * `packAudio` (M1-004): wav -> loudness normalisation -> Opus-in-WebM + AAC-LC -> `kind:'audio'` entries (`@opus`, `@aac`) in one
 * `.fpak`, plus `audio-report.json`. Source files never leave the work directory.
 */
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { assertAllowed } from '../allowed.ts';
import { ExitCode, GateError } from '../errors.ts';
import { encodeAudioEntry, isPreload } from '../fpak/entry-audio.ts';
import { Codec, LOGICAL_ID_RE } from '../fpak/format.ts';
import { FpakWriter, type FpakBuild } from '../fpak/writer.ts';
import type { Ledger, LedgerRow } from '../ledger.ts';
import { assertNoForbiddenChunks } from './chunks.ts';
import { bitrateFor, decodeToWav, encodeAac, encodeOpusWebm } from './encode.ts';
import { normalizeLoudness, TARGET_LUFS, type AudioClass } from './loudness.ts';
import { parseWav } from './wav.ts';

/** Ledger `type` values that may enter the audio pipeline. */
export const AUDIO_TYPES = ['sfx', 'music', 'voice', 'ir'] as const;

export interface AudioItem {
  /** Base logical id, `snd_` prefix, without the codec suffix (e.g. `snd_wpn_us_m1_garand_fire_near`). */
  logical_id: string;
  src: string;
  cls: AudioClass;
  loop?: boolean;
  loop_start_ms?: number;
  loop_end_ms?: number;
  /** Ledger asset id; when set, the D-MOD / status / type gate runs (TRANSCODE + LOUDNORM). */
  asset_id?: string;
}

export interface AudioReportItem {
  logical_id: string;
  cls: AudioClass;
  target_lufs: number;
  source_lufs: number;
  /** false for clips too short / quiet for R128 (peak-normalised to -3 dBFS instead; integrated_lufs is then -70). */
  loudness_measured: boolean;
  integrated_lufs: number;
  true_peak_dbtp: number;
  duration_s: number;
  channels: number;
  preload: boolean;
  bitrate_kbps: number;
  opus_bytes: number;
  aac_bytes: number;
}
export interface AudioReport {
  items: AudioReportItem[];
  total_opus_bytes: number;
  total_aac_bytes: number;
}

export interface PackAudioOptions {
  ledger?: Ledger | readonly LedgerRow[];
  /** Write the pack here (the content-addressed publish path is the caller's business, see `publishPath`). */
  outPak?: string;
  /** Write `audio-report.json` here. */
  reportPath?: string;
  /** Parallel ffmpeg jobs (default 4). */
  concurrency?: number;
}

function gate(item: AudioItem, ledger: PackAudioOptions['ledger']): void {
  if (!item.asset_id) return;
  assertAllowed(item.asset_id, 'TRANSCODE', ledger);
  assertAllowed(item.asset_id, 'LOUDNORM', ledger);
  const row = Array.isArray(ledger) ? (ledger as readonly LedgerRow[]).find((r) => r.asset_id === item.asset_id) : (ledger as Ledger | undefined)?.byAssetId.get(item.asset_id);
  if (row && !(AUDIO_TYPES as readonly string[]).includes(row.type)) {
    throw new GateError(ExitCode.NotOk, `${item.asset_id}: ledger type "${row.type}" is not an audio type (${AUDIO_TYPES.join('/')})`, { assetId: item.asset_id });
  }
}

interface Processed {
  report: AudioReportItem;
  opus: Uint8Array;
  aac: Uint8Array;
  item: AudioItem;
  durationMs: number;
}

async function processItem(item: AudioItem, tmp: string): Promise<Processed> {
  if (!LOGICAL_ID_RE.test(item.logical_id) || !item.logical_id.startsWith('snd_') || item.logical_id.includes('@')) {
    throw new GateError(ExitCode.Naming, `${item.logical_id}: audio logical ids are snd_* snake_case without a codec suffix`);
  }
  const safe = item.logical_id;
  const norm = join(tmp, `${safe}.wav`);
  const target = TARGET_LUFS[item.cls];
  const { before, after, measurable } = await normalizeLoudness(item.src, norm, target);
  const wav = parseWav(new Uint8Array(readFileSync(norm)));
  const bitrate = bitrateFor(item.cls);
  const webm = join(tmp, `${safe}.webm`);
  const mp4 = join(tmp, `${safe}.m4a`);
  await encodeOpusWebm(norm, webm, { bitrate, channels: wav.channels });
  await encodeAac(norm, mp4, { bitrate, channels: wav.channels });
  const opus = new Uint8Array(readFileSync(webm));
  const aac = new Uint8Array(readFileSync(mp4));
  assertNoForbiddenChunks(opus, `${safe}@opus`);
  assertNoForbiddenChunks(aac, `${safe}@aac`);
  const seconds = wav.frames / wav.sampleRate;
  return {
    item, opus, aac, durationMs: Math.round(seconds * 1000),
    report: {
      logical_id: item.logical_id, cls: item.cls, target_lufs: target, source_lufs: before.integrated_lufs,
      loudness_measured: measurable, integrated_lufs: after.integrated_lufs, true_peak_dbtp: after.true_peak_dbtp, duration_s: Math.round(seconds * 1000) / 1000,
      channels: wav.channels, preload: isPreload(seconds), bitrate_kbps: bitrate, opus_bytes: opus.length, aac_bytes: aac.length,
    },
  };
}

export async function packAudio(items: AudioItem[], opts: PackAudioOptions = {}): Promise<{ build: FpakBuild; report: AudioReport }> {
  for (const it of items) gate(it, opts.ledger);
  const tmp = mkdtempSync(join(tmpdir(), 'fl-audio-'));
  try {
    const done: Processed[] = new Array(items.length);
    let next = 0;
    const worker = async () => {
      for (;;) {
        const i = next++;
        if (i >= items.length) return;
        done[i] = await processItem(items[i] as AudioItem, tmp);
      }
    };
    await Promise.all(Array.from({ length: Math.min(opts.concurrency ?? 4, Math.max(1, items.length)) }, worker));
    const w = new FpakWriter();
    for (const d of done) {
      const base = {
        preload: d.report.preload, loop: !!d.item.loop, channels: d.report.channels, durationMs: d.durationMs, sampleRate: 48000,
        loopStartMs: d.item.loop_start_ms ?? 0, loopEndMs: d.item.loop_end_ms ?? 0,
      };
      w.add(`${d.item.logical_id}@opus`, 'audio', Codec.audio, encodeAudioEntry({ ...base, container: 'webm-opus', payload: d.opus }));
      w.add(`${d.item.logical_id}@aac`, 'audio', Codec.audio, encodeAudioEntry({ ...base, container: 'mp4-aac', payload: d.aac }));
    }
    const build = w.finish();
    const report: AudioReport = {
      items: done.map((d) => d.report),
      total_opus_bytes: done.reduce((s, d) => s + d.opus.length, 0),
      total_aac_bytes: done.reduce((s, d) => s + d.aac.length, 0),
    };
    if (opts.outPak) {
      mkdirSync(dirname(opts.outPak), { recursive: true });
      writeFileSync(opts.outPak, build.bytes);
    }
    if (opts.reportPath) {
      mkdirSync(dirname(opts.reportPath), { recursive: true });
      writeFileSync(opts.reportPath, JSON.stringify(report, null, 2) + '\n');
    }
    return { build, report };
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

export { decodeToWav };
