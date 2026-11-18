import { describe, it, expect } from 'vitest';
import { VoicePool, POLYPHONY, voiceScore, type PlayRequest, type VoiceHandle, type VoiceSink } from './voices';
import { probeCodec, resetCodecProbe } from './codec-probe';
import { AudioBank, SILENCE_S } from './bank';
import { sliderToGain, effectiveGain, canPlay } from './volumes';

const sink = (): VoiceSink & { started: number; stopped: number } => {
  const s = { started: 0, stopped: 0, start: () => void s.started++, stop: () => void s.stopped++ };
  return s;
};
const req = (o: Partial<PlayRequest> = {}): PlayRequest => ({ id: 'x', bus: 'sfx', priority: 1, ...o });

describe('polyphony cap and stealing (acceptance 3)', () => {
  for (const [tier, cap] of [['high', 64], ['medium', 48], ['low', 32]] as const) {
    it(`${tier}: 200 requests never exceed ${cap}, steals > 0`, () => {
      const p = new VoicePool(sink(), POLYPHONY[tier]);
      let max = 0;
      for (let i = 0; i < 200; i++) {
        p.play(req({ priority: 1 + (i % 7), id: `v${i}` }), i * 100);
        max = Math.max(max, p.stats.active);
      }
      expect(max).toBeLessThanOrEqual(cap);
      expect(p.stats.stolen + p.stats.rejected).toBeGreaterThan(0);
      expect(p.stats.stolen).toBeGreaterThan(0);
    });
  }
  it('a louder/closer sound steals the quietest; a quieter one is rejected when full', () => {
    const p = new VoicePool(sink(), 2);
    p.listener = { x: 0, y: 0, z: 0 };
    const far = p.play(req({ pos: { x: 100, y: 0, z: 0 } }), 0)!;
    p.play(req({ pos: { x: 10, y: 0, z: 0 } }), 1);
    const near = p.play(req({ pos: { x: 2, y: 0, z: 0 } }), 2);
    expect(near).not.toBeNull();
    expect(p.stats.stolen).toBe(1);
    expect(far.score).toBeLessThan(voiceScore(req({ pos: { x: 2, y: 0, z: 0 } }), p.listener));
    expect(p.play(req({ pos: { x: 500, y: 0, z: 0 } }), 3)).toBeNull();
    expect(p.stats.rejected).toBe(1);
  });
  it('lowering the tier steals down to the new cap; stopAll empties', () => {
    const p = new VoicePool(sink(), 64);
    for (let i = 0; i < 64; i++) p.play(req({ priority: i + 1 }), i);
    p.setTier('low');
    expect(p.stats.active).toBeLessThanOrEqual(32);
    p.stopAll();
    expect(p.stats.active).toBe(0);
  });
});

describe('same-group merge (acceptance 4)', () => {
  it('10 triggers of one weapon within 50 ms play ≤ 4 individual + 1 group voice', () => {
    const sk = sink();
    const p = new VoicePool(sk, 64);
    const handles: (VoiceHandle | null)[] = [];
    for (let i = 0; i < 10; i++) handles.push(p.play(req({ groupKey: 'm1_garand' }), i * 4));
    expect(sk.started).toBe(5);
    expect(new Set(handles.map((h) => h!.serial)).size).toBe(5);
    expect(handles.filter((h) => h!.grouped).length).toBeGreaterThan(0);
    expect(p.stats.merged).toBe(6);
  });
  it('a new window starts counting afresh', () => {
    const sk = sink();
    const p = new VoicePool(sk, 64);
    for (let i = 0; i < 4; i++) p.play(req({ groupKey: 'g' }), 0);
    for (let i = 0; i < 4; i++) p.play(req({ groupKey: 'g' }), 60);
    expect(sk.started).toBe(8);
  });
});

describe('codec probe (acceptance 5)', () => {
  const buf = () => new ArrayBuffer(4);
  const probes = { opus: buf(), aac: buf() };
  it('prefers opus; falls back to aac; both failing → none without throwing', async () => {
    resetCodecProbe();
    expect(await probeCodec(async () => 1, probes)).toBe('opus');
    expect(await probeCodec(async () => { throw new Error('x'); }, probes)).toBe('opus'); // cached
    resetCodecProbe();
    let n = 0;
    expect(await probeCodec(async () => { if (n++ === 0) throw new Error('no opus'); return 1; }, probes)).toBe('aac');
    resetCodecProbe();
    expect(await probeCodec(async () => { throw new Error('none'); }, probes)).toBe('none');
    resetCodecProbe();
  });
});

describe('missing asset logging (acceptance 2)', () => {
  it('1000 requests of one missing id log exactly once and return 0.1 s silence', () => {
    const logs: string[] = [];
    const bank = new AudioBank((s) => ({ duration: s }), (m) => logs.push(m));
    for (let i = 0; i < 1000; i++) expect(bank.get('A-SFX-404', 'fire.local').duration).toBe(SILENCE_S);
    expect(logs).toEqual(['[asset-missing] audio A-SFX-404 requested by fire.local']);
    bank.get('A-SFX-405', 'hit.confirm');
    expect(logs).toHaveLength(2);
    expect(bank.missingCount).toBe(2);
  });
  it('present assets are returned as-is; production builds count but do not log', () => {
    const logs: string[] = [];
    const bank = new AudioBank((s) => ({ duration: s }), (m) => logs.push(m), false);
    bank.add('ok', { duration: 2 });
    expect(bank.get('ok', 'e').duration).toBe(2);
    bank.get('gone', 'e');
    expect(logs).toEqual([]);
    expect(bank.missingCount).toBe(1);
  });
});

describe('volumes and context gating', () => {
  it('squared curve, clamp, master × bus, mute on blur', () => {
    expect(sliderToGain(0.5)).toBe(0.25);
    expect(sliderToGain(2)).toBe(1);
    expect(sliderToGain(NaN)).toBe(0);
    const s = { master: 0.5, sfx: 1, ui: 0.5, voice: 1, music: 1 };
    expect(effectiveGain('sfx', s, false, true)).toBe(0.25);
    expect(effectiveGain('ui', s, false, true)).toBeCloseTo(0.0625);
    expect(effectiveGain('sfx', s, true, false)).toBe(0);
    expect(effectiveGain('sfx', s, false, false)).toBe(0.25);
  });
  it('nothing sounds before the first gesture', () => {
    expect(canPlay('running', false)).toBe(false);
    expect(canPlay('suspended', true)).toBe(false);
    expect(canPlay('running', true)).toBe(true);
  });
});
