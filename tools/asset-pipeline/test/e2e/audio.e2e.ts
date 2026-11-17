import { mkdirSync, readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import { BufferSource, ExitCode, FpakReader, assertNoStandalone, decodeAudioEntry, ingest, packAudio, writeWav } from '../../src/index.ts';
import { main } from '../../src/cli.ts';
import { ledgerText, tmpRepo, type TmpRepo } from '../helpers.ts';

const repos: TmpRepo[] = [];
afterAll(() => repos.forEach((r) => r.cleanup()));
const sine = (s: number, hz: number) => Float32Array.from({ length: Math.round(s * 48000) }, (_, i) => 0.1 * Math.sin((2 * Math.PI * hz * i) / 48000));

describe('audio e2e', () => {
  it('ingest -> audio-pack (CLI) -> fpak read; outputs clean; report and probe written', async () => {
    const r = tmpRepo(ledgerText([{ asset_id: 'AST-A-TEST-001-01', req_id: 'A-TEST-001', type: 'sfx', raw_path: 'assets/incoming/A-TEST-001/Src-FREE', allowed_processing: 'TRANSCODE,LOUDNORM' }]));
    repos.push(r);
    const dir = join(r.root, 'assets/incoming/A-TEST-001/Src-FREE');
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, 'fire.wav'), writeWav({ sampleRate: 44100, samples: sine(1.5, 600), extraChunks: [{ id: 'bext', data: new Uint8Array(100) }] }));
    ingest({ incoming: r.incoming, ledger: r.ledger, work: r.work });
    const items = [{ logical_id: 'snd_wpn_test_fire_near', src: 'work/AST-A-TEST-001-01/raw/fire.wav', cls: 'gun_near', asset_id: 'AST-A-TEST-001-01' }];
    r.put('items.json', JSON.stringify(items));
    const lines: string[] = [];
    expect(await main(['audio-pack', '--root', r.root, '--items', 'items.json'], (l) => lines.push(l))).toBe(ExitCode.Ok);
    const out = join(r.root, 'assets/processed');
    assertNoStandalone(out);
    const pak = readdirSync(join(out, 'a'), { recursive: true, withFileTypes: true }).find((e) => e.isFile() && e.name.endsWith('.fpak'))!;
    const reader = await FpakReader.open(new BufferSource(new Uint8Array(readFileSync(join(pak.parentPath, pak.name)))));
    expect(reader.entries().map((e) => e.logical_id).sort()).toEqual(['snd_wpn_test_fire_near@aac', 'snd_wpn_test_fire_near@opus']);
    const a = decodeAudioEntry(await reader.read('snd_wpn_test_fire_near@opus'));
    expect(a).toMatchObject({ container: 'webm-opus', preload: true, channels: 1, sampleRate: 48000 });
    const rep = JSON.parse(readFileSync(join(r.work, 'audio-report.json'), 'utf8'));
    expect(rep.items[0]).toMatchObject({ cls: 'gun_near', bitrate_kbps: 96, target_lufs: -18, loudness_measured: true });
    expect(existsSync(join(out, 'probe.json'))).toBe(true);
    // The source WAV never lands in the output tree.
    expect(readdirSync(out, { recursive: true }).some((f) => String(f).endsWith('.wav'))).toBe(false);
  }, 120_000);

  it('performance: 30 clips x 5 s pack in <= 60 s', async () => {
    const dir = join(tmpRepo(ledgerText([])).root);
    const items = Array.from({ length: 30 }, (_, i) => {
      const p = join(dir, `c${i}.wav`);
      writeFileSync(p, writeWav({ sampleRate: 48000, samples: sine(5, 200 + i * 37) }));
      return { logical_id: `snd_perf_${i}`, src: p, cls: 'foley' as const };
    });
    const t0 = performance.now();
    const { report } = await packAudio(items);
    const s = (performance.now() - t0) / 1000;
    writeFileSync(join(tmpdir(), 'audio-perf.txt'), `30 x 5 s clips packed in ${s.toFixed(1)} s (4 parallel ffmpeg jobs)\n`);
    expect(report.items).toHaveLength(30);
    expect(s).toBeLessThanOrEqual(60);
  }, 180_000);
});
