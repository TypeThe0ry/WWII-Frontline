/**
 * Built-in codec probe samples (PROBE.md): a very short silent clip in each codec, written to `probe.json` for the `core` pack.
 * The client decodes them with `decodeAudioData` at start-up to choose between `@opus` and `@aac` entries.
 */
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { ffmpeg } from './ffmpeg.ts';

export interface ProbeResult {
  opus: boolean;
  aac: boolean;
}

/** Probe protocol decision (PROBE.md): prefer Opus, then AAC; both failing is an error the caller must surface. */
export function chooseCodec(p: ProbeResult): { codec: 'opus' | 'aac' | 'none'; error?: string } {
  if (p.opus) return { codec: 'opus' };
  if (p.aac) return { codec: 'aac' };
  return { codec: 'none', error: 'neither Opus-in-WebM nor AAC could be decoded by decodeAudioData' };
}

export interface ProbeFile {
  schema: 'frontline.audio-probe/1';
  timeout_ms: number;
  opus: { mime: string; base64: string; bytes: number };
  aac: { mime: string; base64: string; bytes: number };
}

export async function buildProbe(): Promise<ProbeFile> {
  const dir = mkdtempSync(join(tmpdir(), 'fl-probe-'));
  try {
    const silence = ['-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=mono', '-t', '0.05', '-map_metadata', '-1', '-fflags', '+bitexact', '-flags:a', '+bitexact'];
    await ffmpeg([...silence, '-c:a', 'libopus', '-b:a', '16k', '-f', 'webm', join(dir, 'p.webm')]);
    await ffmpeg([...silence, '-c:a', 'aac', '-b:a', '16k', '-movflags', '+faststart', '-f', 'mp4', join(dir, 'p.m4a')]);
    const o = readFileSync(join(dir, 'p.webm'));
    const a = readFileSync(join(dir, 'p.m4a'));
    return {
      schema: 'frontline.audio-probe/1', timeout_ms: 500,
      opus: { mime: 'audio/webm; codecs=opus', base64: o.toString('base64'), bytes: o.length },
      aac: { mime: 'audio/mp4; codecs=mp4a.40.2', base64: a.toString('base64'), bytes: a.length },
    };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

export async function writeProbe(path: string): Promise<ProbeFile> {
  const p = await buildProbe();
  writeFileSync(path, JSON.stringify(p, null, 2) + '\n');
  return p;
}
