import { describe, expect, it } from 'vitest';
import { AUDIO_PARAMS } from './crossfade';
import { AudioDirector, type DirectorOpts, type Scheduler } from './dispatch';
import { VoicePool, type VoiceSink } from './voices';

const P = AUDIO_PARAMS as NonNullable<DirectorOpts['params']>;
function make(counts: Record<string, number> = { near: 3, mid: 3, far: 3 }) {
  const played: { id: string; at: number; gain: number }[] = [];
  const sched: Scheduler = { schedule: (id, at, gain) => played.push({ id, at, gain }) };
  const sink: VoiceSink = { start: () => {}, stop: () => {} };
  let r = 0;
  const d = new AudioDirector(sched, new VoicePool(sink), { variantCount: (_w, l) => counts[l] ?? 0, footstepVariants: () => 2, rnd: () => ((r = (r + 0.37) % 1), r) });
  return { d, played };
}
const at = (x: number) => ({ x, y: 0, z: 0 });

describe('shot dispatch', () => {
  it('close shot: near layer only, no extra delay', () => {
    const { d, played } = make();
    d.handle({ kind: 'shot', weapon: 'garand', pos: at(10) }, 5);
    expect(played.map((p) => p.id.split('.')[2])).toEqual(['near']);
    expect(played[0]!.at).toBe(5);
  });
  it('600 m shot: far layer, delayed by distance / speed of sound', () => {
    const { d, played } = make();
    d.handle({ kind: 'shot', weapon: 'garand', pos: at(600) }, 2);
    expect(played.map((p) => p.id.split('.')[2])).toEqual(['far']);
    expect(played[0]!.at).toBeCloseTo(2 + 600 / P.speed_of_sound_mps, 6);
  });
  it('missing layer is silent and nothing substitutes', () => {
    const { d, played } = make({ near: 0, mid: 0, far: 3 });
    d.handle({ kind: 'shot', weapon: 'garand', pos: at(10) }, 0);
    expect(played).toEqual([]);
  });
  it('never repeats the previous variant of a layer', () => {
    const { d, played } = make();
    for (let i = 0; i < 20; i++) d.handle({ kind: 'shot', weapon: 'garand', pos: at(5) }, i);
    for (let i = 1; i < played.length; i++) expect(played[i]!.id).not.toBe(played[i - 1]!.id);
  });
  it('schedules only four individual shots and one group voice in the same window', () => {
    const { d, played } = make();
    const started = Array.from({ length: 10 }, () => d.handle({ kind: 'shot', weapon: 'garand', pos: at(0) }, 0));
    expect(started.reduce((sum, n) => sum + n, 0)).toBe(5);
    expect(played).toHaveLength(5);
  });
});

describe('footsteps and suppression', () => {
  it('keeps only the nearest slots, drops inaudible ones', () => {
    const { d, played } = make();
    for (let i = 0; i < 12; i++) d.handle({ kind: 'footstep', pos: at(2 + i), gait: 'run', surface: 'grass', actor: i }, 0);
    d.handle({ kind: 'footstep', pos: at(100), gait: 'run', surface: 'grass', actor: 99 }, 0); // beyond 35 m
    expect(d.flush(0)).toBe(P.footstep_max_slots);
    expect(played.length).toBe(P.footstep_max_slots);
    expect(Math.max(...played.map((p) => 1 / p.gain))).toBeLessThanOrEqual(2 + P.footstep_max_slots);
  });
  it('crouch footsteps are audible only within the crouch radius', () => {
    const { d } = make();
    d.handle({ kind: 'footstep', pos: at(P.footstep_audible_crouch_m + 1), gait: 'crouch', surface: 'mud', actor: 1 }, 0);
    expect(d.flush(0)).toBe(0);
  });
  it('suppression filter drops then recovers within the recover window', () => {
    const { d } = make();
    d.handle({ kind: 'suppress', intensity: 255 }, 10);
    expect(d.filter(10).cutoffHz).toBeCloseTo(1200, 0);
    expect(d.filter(10 + P.suppress_lowpass_recover_ms / 1000).cutoffHz).toBeCloseTo(20000, 0);
  });
});
