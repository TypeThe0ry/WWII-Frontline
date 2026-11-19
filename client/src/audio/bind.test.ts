import { describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../game/event-bus';
import { bindAudioEvents } from './bind';
import { AudioDirector, type Scheduler } from './dispatch';
import { VoicePool } from './voices';

const rig = () => {
  const played: Array<{ id: string; at: number; gain: number }> = [];
  const sched: Scheduler = { schedule: (id, at, gain) => void played.push({ id, at, gain }) };
  const dir = new AudioDirector(sched, new VoicePool({ start: () => {}, stop: () => {} }), { variantCount: () => 2, footstepVariants: () => 2, rnd: () => 0 });
  const bus = new EventBus<GameEvents>();
  const off = bindAudioEvents(bus, dir, { now: () => 10, weaponId: (i) => (i === 3 ? 'us_m1_garand' : undefined), listener: () => ({ x: 0, y: 0, z: 0 }) });
  return { bus, dir, played, off };
};
const v = (x: number) => ({ x, y: 0, z: 0 });

describe('bindAudioEvents', () => {
  it('plays remote shots of known weapons only', () => {
    const { bus, played } = rig();
    bus.emit('fire.remote', { shooter: 1, weapon: 9, muzzlePos: v(20), dir: v(1), distance: 20 });
    expect(played).toHaveLength(0);
    bus.emit('fire.remote', { shooter: 1, weapon: 3, muzzlePos: v(20), dir: v(1), distance: 20 });
    expect(played.length).toBeGreaterThan(0);
    expect(played.every((p) => p.id.startsWith('wpn.us_m1_garand.'))).toBe(true);
  });

  it('queues footsteps until flush and applies suppression to the master filter', () => {
    const { bus, dir, played } = rig();
    bus.emit('entity.footstep', { entity: 5, foot: 'left', pos: v(4), stance: 0 });
    expect(played).toHaveLength(0);
    expect(dir.flush(10)).toBe(1);
    expect(played[0]?.id.startsWith('fs.grass.')).toBe(true);
    const open = dir.filter(10).cutoffHz;
    bus.emit('suppress', { angle: 0, intensity: 1 });
    expect(dir.filter(10).cutoffHz).toBeLessThan(open);
  });

  it('stops after unsubscribe', () => {
    const { bus, played, off } = rig();
    off();
    bus.emit('fire.local', { weapon: 3, muzzlePos: v(0), dir: v(1) });
    expect(played).toHaveLength(0);
  });
});
