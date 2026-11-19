import type { AudioBank } from './bank';
import { suppressFilter } from './crossfade';
import type { AudioDirector, Scheduler } from './dispatch';
import { VoicePool, type VoiceHandle } from './voices';

const linearGain = (db: number): number => 10 ** (db / 20);

/** One-shot SFX graph. The owner retains and closes the realtime context. */
export class WebAudioScheduler implements Scheduler {
  private readonly filter: BiquadFilterNode;
  private readonly duck: GainNode;
  private readonly sources = new Map<VoiceHandle | AudioBufferSourceNode, (stop: boolean) => void>();
  private disposed = false;
  readonly pool = new VoicePool({ start: () => {}, stop: (voice) => this.sources.get(voice)?.(true) });

  constructor(private readonly context: BaseAudioContext, private readonly bank: AudioBank<AudioBuffer>) {
    this.filter = context.createBiquadFilter();
    this.filter.type = 'lowpass';
    this.filter.Q.value = 0;
    this.filter.frequency.value = suppressFilter(0, 0).cutoffHz;
    this.duck = context.createGain();
    this.filter.connect(this.duck).connect(context.destination);
  }

  schedule(id: string, atS: number, gain: number, rate: number, voice?: VoiceHandle): void {
    if (this.disposed) {
      if (voice) this.pool.release(voice);
      return;
    }
    const source = this.context.createBufferSource();
    const level = this.context.createGain();
    source.buffer = this.bank.get(id, 'schedule');
    source.playbackRate.value = rate;
    level.gain.value = gain;
    source.connect(level).connect(this.filter);

    const key = voice ?? source;
    const cleanup = (stop: boolean): void => {
      this.sources.delete(key);
      source.onended = null;
      if (stop) source.stop();
      source.disconnect();
      level.disconnect();
      if (voice) this.pool.release(voice);
    };
    this.sources.set(key, cleanup);
    source.onended = () => cleanup(false);
    try {
      source.start(atS);
    } catch (err) {
      cleanup(false);
      throw err;
    }
  }

  applySuppression(director: AudioDirector, atS: number): void {
    if (this.disposed) return;
    const end = atS + director.filterRecoveryS;
    const hit = director.filter(atS);
    const open = suppressFilter(0, 0);
    // Both cutoff (Hz) and converted gain (dB) recover geometrically, independent of render frames.
    this.ramp(this.filter.frequency, hit.cutoffHz, open.cutoffHz, atS, end);
    this.ramp(this.duck.gain, linearGain(hit.gainDb), linearGain(open.gainDb), atS, end);
  }

  private ramp(param: AudioParam, hit: number, open: number, at: number, end: number): void {
    param.cancelScheduledValues(at);
    param.setValueAtTime(hit, at);
    param.exponentialRampToValueAtTime(open, end);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const cleanup of this.sources.values()) cleanup(true);
    this.filter.disconnect();
    this.duck.disconnect();
  }
}
