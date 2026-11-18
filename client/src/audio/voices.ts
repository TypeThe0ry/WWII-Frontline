/**
 * Voice pool logic (ARCHITECTURE §12.4): polyphony cap by quality tier, priority-based stealing, per-group merge window.
 * Pure bookkeeping — the WebAudio nodes live behind the `VoiceSink`, so the rules are testable without an AudioContext.
 */
export type Bus = 'sfx' | 'amb' | 'ui' | 'voice';
export type Tier = 'low' | 'medium' | 'high';
export const POLYPHONY: Readonly<Record<Tier, number>> = { high: 64, medium: 48, low: 32 };
export const MERGE_WINDOW_MS = 50;
export const MERGE_MAX = 4;

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}
export interface PlayRequest {
  id: string;
  bus: Bus;
  pos?: Vec3;
  gain?: number;
  rate?: number;
  priority: number; // higher = more important
  loop?: boolean;
  groupKey?: string;
}

export interface VoiceHandle {
  readonly serial: number;
  readonly req: PlayRequest;
  /** Score used for stealing (higher survives). */
  score: number;
  startedAt: number;
  /** True for the single merged "group" voice standing in for dropped repeats. */
  grouped: boolean;
  stop(): void;
}

/** Loudness estimate × importance ÷ distance (§12.4). Distance floor 1 m so near sources do not blow up. */
export function voiceScore(req: PlayRequest, listener: Vec3 | undefined): number {
  const loud = req.gain ?? 1;
  if (!req.pos || !listener) return loud * req.priority;
  const d = Math.max(1, Math.hypot(req.pos.x - listener.x, req.pos.y - listener.y, req.pos.z - listener.z));
  return (loud * req.priority) / d;
}

export interface VoiceSink {
  start(h: VoiceHandle): void;
  stop(h: VoiceHandle): void;
}
export interface VoiceStats {
  active: number;
  stolen: number;
  merged: number;
  rejected: number;
}

export class VoicePool {
  private readonly voices: VoiceHandle[] = [];
  private serial = 0;
  private readonly groups = new Map<string, { windowStart: number; count: number; grouped?: VoiceHandle }>();
  readonly stats: VoiceStats = { active: 0, stolen: 0, merged: 0, rejected: 0 };
  listener: Vec3 | undefined;

  constructor(private readonly sink: VoiceSink, private cap: number = POLYPHONY.high) {}

  setTier(t: Tier): void {
    this.cap = POLYPHONY[t];
    while (this.voices.length > this.cap) this.stealLowest();
  }
  get capacity(): number {
    return this.cap;
  }

  /** Voices that ended naturally call this (or `update` removes them). */
  release(h: VoiceHandle): void {
    const i = this.voices.indexOf(h);
    if (i >= 0) {
      this.voices.splice(i, 1);
      this.stats.active = this.voices.length;
    }
  }

  play(req: PlayRequest, now: number): VoiceHandle | null {
    // Same-group merge: ≤ 4 individual voices per 50 ms window, the rest collapse into one group voice.
    let merged = false;
    if (req.groupKey) {
      let g = this.groups.get(req.groupKey);
      if (!g || now - g.windowStart >= MERGE_WINDOW_MS) {
        g = { windowStart: now, count: 0 };
        this.groups.set(req.groupKey, g);
      }
      g.count++;
      if (g.count > MERGE_MAX) {
        this.stats.merged++;
        if (g.grouped && this.voices.includes(g.grouped)) return g.grouped; // already playing this window's group voice
        merged = true;
      }
    }
    const score = voiceScore(req, this.listener) * (merged ? 0.6 : 1);
    if (this.voices.length >= this.cap) {
      const low = this.lowest();
      if (low && low.score >= score) {
        this.stats.rejected++;
        return null;
      }
      this.stealLowest();
    }
    const h: VoiceHandle = { serial: ++this.serial, req, score, startedAt: now, grouped: merged, stop: () => { this.sink.stop(h); this.release(h); } };
    this.voices.push(h);
    this.stats.active = this.voices.length;
    if (merged && req.groupKey) this.groups.get(req.groupKey)!.grouped = h;
    this.sink.start(h);
    return h;
  }

  stopAll(): void {
    for (const v of [...this.voices]) v.stop();
  }

  private lowest(): VoiceHandle | undefined {
    let low: VoiceHandle | undefined;
    for (const v of this.voices) if (!low || v.score < low.score || (v.score === low.score && v.startedAt < low.startedAt)) low = v;
    return low;
  }
  private stealLowest(): void {
    const l = this.lowest();
    if (!l) return;
    this.stats.stolen++;
    l.stop();
  }
}
