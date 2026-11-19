/**
 * Audio event dispatch (M1-063b): turns game events (shots, footsteps, suppression) into voice requests.
 * Pure: the WebAudio side is the `Scheduler`; time is the caller's clock in seconds. Params come from `/data/audio/params.json`.
 */
import { AUDIO_PARAMS, footstepAudibleM, layerGains, limitFootsteps, pickVariant, shotDelaySeconds, suppressFilter } from './crossfade';
import type { AudioParams, Gait, Surface, SuppressFilter } from './crossfade';
import type { Vec3 } from './voices';
import type { PlayRequest, VoiceHandle, VoicePool } from './voices';

export type Layer = 'near' | 'mid' | 'far';

/** Starts a buffer `id` at audio-clock time `atS` with gain `gain` (a thin wrapper over BufferSource in the browser). */
export interface Scheduler {
  schedule(id: string, atS: number, gain: number, rate: number, voice?: VoiceHandle): void;
}

export type AudioEvent =
  | { kind: 'shot'; weapon: string; pos: Vec3 }
  | { kind: 'footstep'; pos: Vec3; gait: Gait; surface: Surface; actor: number }
  | { kind: 'suppress'; intensity: number };

export interface DirectorOpts {
  params?: AudioParams & { footstep_audible_walk_m: number; footstep_audible_run_m: number; footstep_audible_crouch_m: number; footstep_max_slots: number; suppress_lowpass_recover_ms: number };
  rnd?: () => number;
  /** How many recorded variants exist for `weapon`/`layer`; 0 = layer missing (silent, nothing substitutes). */
  variantCount: (weapon: string, layer: Layer) => number;
  footstepVariants?: (surface: Surface) => number;
  /** Translate a dense selection slot to a recorded variant number when asset IDs are sparse. */
  variantIndex?: (prefix: string, slot: number) => number;
  missing?: (id: string, event: 'shot' | 'footstep') => void;
}

const dist = (a: Vec3, b: Vec3): number => Math.hypot(a.x - b.x, a.y - b.y, a.z - b.z);
export const shotId = (weapon: string, layer: Layer, variant: number): string => `wpn.${weapon}.${layer}.${variant}`;
export const stepId = (surface: Surface, variant: number): string => `fs.${surface}.${variant}`;

export class AudioDirector {
  private readonly p: NonNullable<DirectorOpts['params']>;
  private readonly rnd: () => number;
  private readonly lastVariant = new Map<string, number>();
  private suppress: { intensity: number; at: number } | null = null;
  private stepBatch: { distM: number; ev: Extract<AudioEvent, { kind: 'footstep' }> }[] = [];
  listener: Vec3 = { x: 0, y: 0, z: 0 };

  constructor(private readonly sched: Scheduler, private readonly pool: VoicePool, private readonly o: DirectorOpts) {
    this.p = o.params ?? (AUDIO_PARAMS as NonNullable<DirectorOpts['params']>);
    this.rnd = o.rnd ?? Math.random;
  }

  private variant(key: string, count: number): number {
    const v = pickVariant(count, this.lastVariant.get(key), this.rnd);
    this.lastVariant.set(key, v);
    return this.o.variantIndex?.(key, v) ?? v;
  }

  private play(req: PlayRequest, now: number, at: number): number {
    this.pool.listener = this.listener;
    const voice = this.pool.play(req, now * 1000);
    // An already-playing merged voice is returned for the rest of its window.
    if (!voice || voice.req !== req) return 0;
    this.sched.schedule(req.id, at, req.gain ?? 1, req.rate ?? 1, voice);
    return 1;
  }

  /** Queue an event; footsteps are batched until `flush` so the nearest `footstep_max_slots` win. Returns voices started now. */
  handle(ev: AudioEvent, now: number): number {
    if (ev.kind === 'suppress') {
      this.suppress = { intensity: ev.intensity, at: now };
      return 0;
    }
    if (ev.kind === 'footstep') {
      const d = dist(ev.pos, this.listener);
      if (d <= footstepAudibleM(ev.gait, this.p)) this.stepBatch.push({ distM: d, ev });
      return 0;
    }
    return this.shot(ev, now);
  }

  private shot(ev: Extract<AudioEvent, { kind: 'shot' }>, now: number): number {
    const d = dist(ev.pos, this.listener);
    const g = layerGains(d, this.p);
    const at = now + shotDelaySeconds(d, this.p);
    let started = 0;
    for (const layer of ['near', 'mid', 'far'] as const) {
      const gain = g[layer];
      if (gain <= 0) continue;
      const count = this.o.variantCount(ev.weapon, layer);
      if (count <= 0) {
        this.o.missing?.(shotId(ev.weapon, layer, 0), 'shot');
        continue;
      }
      const id = shotId(ev.weapon, layer, this.variant(`wpn.${ev.weapon}.${layer}`, count));
      started += this.play({ id, bus: 'sfx', pos: ev.pos, gain, priority: layer === 'far' ? 1 : 3, groupKey: `shot.${ev.weapon}.${layer}` }, now, at);
    }
    return started;
  }

  /** End of tick: play the nearest footsteps (≤ `footstep_max_slots`), drop the rest. */
  flush(now: number): number {
    if (this.stepBatch.length === 0) return 0;
    const keep = limitFootsteps(this.stepBatch, this.p.footstep_max_slots);
    this.stepBatch = [];
    let n = 0;
    for (const s of keep) {
      const count = this.o.footstepVariants?.(s.ev.surface) ?? 1;
      if (count <= 0) {
        this.o.missing?.(stepId(s.ev.surface, 0), 'footstep');
        continue;
      }
      const id = stepId(s.ev.surface, this.variant(`fs.${s.ev.surface}`, count));
      const gain = 1 / Math.max(1, s.distM);
      n += this.play({ id, bus: 'sfx', pos: s.ev.pos, gain, priority: 2, groupKey: `fs.${s.ev.actor}` }, now, now);
    }
    return n;
  }

  /** Current master filter: recovers within `suppress_recover_ms`; null intensity → fully open. */
  filter(now: number): SuppressFilter {
    if (!this.suppress) return suppressFilter(0, 0);
    return suppressFilter(this.suppress.intensity, (now - this.suppress.at) * 1000, this.p.suppress_lowpass_recover_ms);
  }

  get filterRecoveryS(): number {
    return this.p.suppress_lowpass_recover_ms / 1000;
  }
}
