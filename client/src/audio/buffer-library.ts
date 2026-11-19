import { AudioBank } from './bank';

/** Resident short clips. The asset loader supplies encoded bytes, not pack headers. */
export class AudioBufferLibrary extends AudioBank<AudioBuffer> {
  private readonly variantsByPrefix = new Map<string, number[]>();

  constructor(private readonly context: BaseAudioContext, log?: (message: string) => void, dev?: boolean) {
    super((seconds) => context.createBuffer(1, Math.round(seconds * context.sampleRate), context.sampleRate), log, dev);
  }

  /** Read and decode away from the event path. An unavailable or invalid clip remains silent. */
  async load(id: string, read: () => Promise<ArrayBuffer>): Promise<boolean> {
    if (this.has(id)) return true;
    try {
      // Give the decoder ownership of a copy; decodeAudioData detaches its input.
      this.add(id, await this.context.decodeAudioData((await read()).slice(0)));
      return true;
    } catch {
      this.noteMissing(id, 'load');
      return false;
    }
  }

  override add(id: string, buffer: AudioBuffer): void {
    const match = /^(.+)\.(0|[1-9]\d*)$/.exec(id);
    if (match && !this.has(id)) {
      const prefix = match[1]!;
      const variants = this.variantsByPrefix.get(prefix) ?? [];
      variants.push(Number(match[2]));
      variants.sort((a, b) => a - b);
      this.variantsByPrefix.set(prefix, variants);
    }
    super.add(id, buffer);
  }

  variantCount(prefix: string): number {
    return this.variantsByPrefix.get(prefix)?.length ?? 0;
  }

  variantIndex(prefix: string, slot: number): number {
    return this.variantsByPrefix.get(prefix)?.[slot] ?? slot;
  }
}
