import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import {
  AUDIO_TYPES, BufferSource, FpakReader, assertNoStandalone, bitrateFor, buildProbe, buildReport, chooseCodec, decodeAudioEntry, decodeToWav, forbiddenChunks, isPreload,
  packAudio, parseLedger, parseWav, samples, writeWav, TARGET_LUFS, ExitCode, type AudioClass,
} from '../src/index.ts';
import { ledgerText } from './helpers.ts';

const tmp = mkdtempSync(join(tmpdir(), 'fl-audio-test-'));
afterAll(() => rmSync(tmp, { recursive: true, force: true }));

function sine(seconds: number, hz = 440, amp = 0.1, rate = 48000): Float32Array {
  return Float32Array.from({ length: Math.round(seconds * rate) }, (_, i) => amp * Math.sin((2 * Math.PI * hz * i) / rate));
}
function wavFile(name: string, s: Float32Array, extra?: { id: string; data: Uint8Array }[]): string {
  const p = join(tmp, name);
  writeFileSync(p, writeWav({ sampleRate: 48000, samples: s, ...(extra ? { extraChunks: extra } : {}) }));
  return p;
}

/** Normalised cross-correlation of two signals, best over lags +-`maxLag` samples. */
function correlation(a: Float32Array, b: Float32Array, maxLag = 480): number {
  let best = -1;
  const n = Math.min(a.length, b.length) - maxLag * 2;
  for (let lag = -maxLag; lag <= maxLag; lag += 1) {
    let ab = 0, aa = 0, bb = 0;
    for (let i = maxLag; i < maxLag + n; i++) {
      const x = a[i] as number;
      const y = b[i + lag] as number;
      ab += x * y; aa += x * x; bb += y * y;
    }
    best = Math.max(best, ab / Math.sqrt(aa * bb || 1));
  }
  return best;
}

describe('audio pack (acceptance 1, 2, 4, 5)', () => {
  it('sine WAV -> Opus pack -> read back -> decode: duration within 20 ms, correlation >= 0.95; loudness on target; size and metadata', async () => {
    const injected = [
      { id: 'bext', data: new Uint8Array(602).fill(65) },
      { id: 'iXML', data: new TextEncoder().encode('<BWFXML><USER>secret-name</USER></BWFXML>') },
    ];
    const src = wavFile('sine1.wav', sine(1), injected);
    expect(parseWav(new Uint8Array(readFileSync(src))).chunks).toEqual(expect.arrayContaining(['bext', 'iXML']));
    const { build, report } = await packAudio([{ logical_id: 'snd_test_sine_foley', src, cls: 'foley' }]);
    const reader = await FpakReader.open(new BufferSource(build.bytes));
    expect(reader.entries().map((e) => e.logical_id).sort()).toEqual(['snd_test_sine_foley@aac', 'snd_test_sine_foley@opus']);
    expect(reader.entries().every((e) => e.kind === 'audio' && e.codec === 3)).toBe(true);

    // 1. round trip
    const opus = decodeAudioEntry(await reader.read('snd_test_sine_foley@opus'));
    const webm = join(tmp, 'rt.webm');
    writeFileSync(webm, opus.payload);
    await decodeToWav(webm, join(tmp, 'rt.wav'));
    const dec = parseWav(new Uint8Array(readFileSync(join(tmp, 'rt.wav'))));
    expect(Math.abs(dec.frames / dec.sampleRate - 1)).toBeLessThanOrEqual(0.02);
    expect(correlation(sine(1), samples(dec))).toBeGreaterThanOrEqual(0.95);
    expect(opus.durationMs).toBeGreaterThanOrEqual(990);
    expect(opus.durationMs).toBeLessThanOrEqual(1010);

    // 2. loudness
    const it0 = report.items[0]!;
    expect(it0.target_lufs).toBe(-20);
    expect(Math.abs(it0.integrated_lufs - it0.target_lufs)).toBeLessThanOrEqual(1);
    expect(it0.true_peak_dbtp).toBeLessThanOrEqual(-1);
    expect(it0).toMatchObject({ logical_id: 'snd_test_sine_foley', cls: 'foley', preload: true, bitrate_kbps: 64, channels: 1 });
    expect(it0.opus_bytes).toBeGreaterThan(0);
    expect(it0.aac_bytes).toBeGreaterThan(0);

    // 4. no source metadata anywhere in the pack
    expect(forbiddenChunks(build.bytes)).toEqual([]);
    expect(Buffer.from(build.bytes).includes('secret-name')).toBe(false);

    // 5. size: 1 s mono at 64 kbps
    expect(reader.entry('snd_test_sine_foley@opus').bytes).toBeLessThanOrEqual(9 * 1024);
  }, 60_000);

  it('AAC entry decodes too, and the pack dir is free of standalone audio files (acceptance 6)', async () => {
    const out = join(tmp, 'processed');
    mkdirSync(join(out, 'a', 'ab'), { recursive: true });
    const { build } = await packAudio([{ logical_id: 'snd_test_sine_ui', src: wavFile('s2.wav', sine(0.5)), cls: 'ui' }], { outPak: join(out, 'a', 'ab', 'cdef.fpak') });
    const reader = await FpakReader.open(new BufferSource(build.bytes));
    const aac = decodeAudioEntry(await reader.read('snd_test_sine_ui@aac'));
    expect(aac.container).toBe('mp4-aac');
    writeFileSync(join(tmp, 'rt.m4a'), aac.payload);
    await decodeToWav(join(tmp, 'rt.m4a'), join(tmp, 'rt2.wav'));
    const dec = parseWav(new Uint8Array(readFileSync(join(tmp, 'rt2.wav'))));
    expect(Math.abs(dec.frames / dec.sampleRate - 0.5)).toBeLessThanOrEqual(0.06); // AAC adds encoder delay / padding
    expect(() => assertNoStandalone(out)).not.toThrow();
  }, 60_000);

  it('preload threshold is < 3 s; longer clips stream', async () => {
    expect(isPreload(2.99)).toBe(true);
    expect(isPreload(3.0)).toBe(false);
    const { report } = await packAudio([
      { logical_id: 'snd_test_short', src: wavFile('p1.wav', sine(2.99)), cls: 'foley' },
      { logical_id: 'snd_test_long', src: wavFile('p2.wav', sine(3.2)), cls: 'ambience', loop: true, loop_start_ms: 100, loop_end_ms: 3000 },
    ]);
    expect(report.items.map((i) => [i.logical_id, i.preload])).toEqual([['snd_test_short', true], ['snd_test_long', false]]);
  }, 60_000);

  it('clips too short for R128 are peak-normalised and flagged', async () => {
    const { report } = await packAudio([{ logical_id: 'snd_test_blip', src: wavFile('blip.wav', sine(0.2)), cls: 'ui' }]);
    expect(report.items[0]).toMatchObject({ loudness_measured: false, integrated_lufs: -70 });
    expect(report.items[0]!.true_peak_dbtp).toBeLessThanOrEqual(-1);
  }, 60_000);

  it('bit rate and loudness target tables', () => {
    const rates: [AudioClass, number][] = [['gun_near', 96], ['gun_mid', 64], ['gun_far', 64], ['foley', 64], ['ambience', 32], ['ui', 64], ['music', 64], ['voice', 64]];
    for (const [c, r] of rates) expect(bitrateFor(c), c).toBe(r);
    expect([TARGET_LUFS.gun_near, TARGET_LUFS.ambience, TARGET_LUFS.ui]).toEqual([-18, -24, -20]);
  });

  it('logical ids: snd_ prefix, no codec suffix', async () => {
    const src = wavFile('n.wav', sine(0.2));
    await expect(packAudio([{ logical_id: 'wpn_bad', src, cls: 'ui' }])).rejects.toMatchObject({ code: ExitCode.Naming });
    await expect(packAudio([{ logical_id: 'snd_bad@opus', src, cls: 'ui' }])).rejects.toMatchObject({ code: ExitCode.Naming });
  });
});

describe('gate (acceptance: ledger OK, type, TRANSCODE/LOUDNORM)', () => {
  const ledger = parseLedger(ledgerText([
    { asset_id: 'AST-A-OK-01', raw_path: 'assets/incoming/A-OK', type: 'sfx', allowed_processing: 'TRANSCODE,LOUDNORM' },
    { asset_id: 'AST-A-NOLN-01', raw_path: 'assets/incoming/A-NOLN', type: 'sfx', allowed_processing: 'TRANSCODE' },
    { asset_id: 'AST-A-MODEL-01', raw_path: 'assets/incoming/A-MODEL', type: 'model', allowed_processing: 'TRANSCODE,LOUDNORM' },
    { asset_id: 'AST-A-PEND-01', raw_path: 'assets/incoming/A-PEND', type: 'sfx', allowed_processing: 'TRANSCODE,LOUDNORM', status: 'PENDING' },
  ]));
  const src = () => wavFile('g.wav', sine(0.3));

  it('OK sfx row passes; missing LOUDNORM, wrong type and non-OK status are refused', async () => {
    await expect(packAudio([{ logical_id: 'snd_g_ok', src: src(), cls: 'ui', asset_id: 'AST-A-OK-01' }], { ledger })).resolves.toBeDefined();
    await expect(packAudio([{ logical_id: 'snd_g_a', src: src(), cls: 'ui', asset_id: 'AST-A-NOLN-01' }], { ledger })).rejects.toMatchObject({ code: ExitCode.ProcessingNotAllowed });
    await expect(packAudio([{ logical_id: 'snd_g_b', src: src(), cls: 'ui', asset_id: 'AST-A-MODEL-01' }], { ledger })).rejects.toMatchObject({ code: ExitCode.NotOk });
    await expect(packAudio([{ logical_id: 'snd_g_c', src: src(), cls: 'ui', asset_id: 'AST-A-PEND-01' }], { ledger })).rejects.toMatchObject({ code: ExitCode.NotOk });
    expect(AUDIO_TYPES).toEqual(['sfx', 'music', 'voice', 'ir']);
  }, 60_000);
});

describe('probe protocol (acceptance 3)', () => {
  it('chooseCodec covers all four combinations', () => {
    expect(chooseCodec({ opus: true, aac: true })).toEqual({ codec: 'opus' });
    expect(chooseCodec({ opus: true, aac: false })).toEqual({ codec: 'opus' });
    expect(chooseCodec({ opus: false, aac: true })).toEqual({ codec: 'aac' });
    const none = chooseCodec({ opus: false, aac: false });
    expect(none.codec).toBe('none');
    expect(none.error).toMatch(/neither/);
  });

  it('probe samples are tiny (< 2 KB) and decodable', async () => {
    const p = await buildProbe();
    expect(p.opus.bytes).toBeLessThan(2048);
    expect(p.aac.bytes).toBeLessThan(2048);
    expect(p.timeout_ms).toBe(500);
    writeFileSync(join(tmp, 'probe.webm'), Buffer.from(p.opus.base64, 'base64'));
    writeFileSync(join(tmp, 'probe.m4a'), Buffer.from(p.aac.base64, 'base64'));
    await decodeToWav(join(tmp, 'probe.webm'), join(tmp, 'probe1.wav'));
    await decodeToWav(join(tmp, 'probe.m4a'), join(tmp, 'probe2.wav'));
    expect(forbiddenChunks(Buffer.from(p.opus.base64, 'base64'))).toEqual([]);
  }, 60_000);
});

describe('chunk scan', () => {
  it('finds iXML / bext / SMED / LIST-INFO', () => {
    const b = (s: string) => new TextEncoder().encode(s);
    expect(forbiddenChunks(b('xxbextyy'))).toEqual(['bext']);
    expect(forbiddenChunks(b('iXML ... SMED'))).toEqual(['iXML', 'SMED']);
    expect(forbiddenChunks(b('LIST\u0001\u0000\u0000\u0000INFOISFT'))).toEqual(['LIST/INFO']);
    expect(forbiddenChunks(b('clean data'))).toEqual([]);
  });
});

describe('missing audio (acceptance 7)', () => {
  it('a requirement without audio input reports missing while packing the others is not blocked', async () => {
    const rep = buildReport({
      requirements: { schema: 1, source: 'test', requirements: [{ id: 'A-NONE-001', milestone: 'M1', kind: 'audio' }] },
      ledger: (() => { writeFileSync(join(tmp, 'none.md'), ledgerText([])); return join(tmp, 'none.md'); })(), work: join(tmp, 'work'),
    }).requirements.find((r) => r.id === 'A-NONE-001');
    expect(rep?.status).toBe('missing');
    await expect(packAudio([])).resolves.toMatchObject({ report: { items: [] } });
  });
});
