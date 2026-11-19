/** Audio bank (M1-062a): id → buffer lookup with silent placeholders for missing assets (ASSET_GAPS §3.3). */
export const SILENCE_S = 0.1;
export const MISSING_PREFIX = '[asset-missing] audio';

export interface BufferLike {
  readonly duration: number;
}
export type SilenceFactory<B extends BufferLike> = (seconds: number) => B;

export class AudioBank<B extends BufferLike> {
  private readonly buffers = new Map<string, B>();
  private readonly missing = new Set<string>();
  private silence: B | undefined;
  /** Number of distinct ids reported missing this session. */
  get missingCount(): number {
    return this.missing.size;
  }

  constructor(private readonly makeSilence: SilenceFactory<B>, private readonly log: (msg: string) => void = () => {}, readonly dev = true) {}

  add(id: string, buf: B): void {
    this.buffers.set(id, buf);
  }
  has(id: string): boolean {
    return this.buffers.has(id);
  }

  /** `requestedBy` is the event name (for the one log line). Missing → 0.1 s silence; nothing substitutes for it. */
  get(id: string, requestedBy: string): B {
    const b = this.buffers.get(id);
    if (b) return b;
    this.noteMissing(id, requestedBy);
    return (this.silence ??= this.makeSilence(SILENCE_S));
  }

  /** Report absent assets even when the caller can skip allocating a voice. */
  noteMissing(id: string, requestedBy: string): void {
    if (!this.missing.has(id)) {
      this.missing.add(id);
      if (this.dev) this.log(`${MISSING_PREFIX} ${id} requested by ${requestedBy}`);
    }
  }
}
