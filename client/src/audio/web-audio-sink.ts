import { AudioGraph, setPosition } from './graph';
import type { Bus, Vec3, VoiceHandle, VoiceSink } from './voices';

export type SpatialParams = Pick<PannerOptions, 'distanceModel' | 'refDistance' | 'maxDistance' | 'rolloffFactor'>;
export interface VoiceBuffer {
  buffer: AudioBuffer;
  /** Apply the grouped-voice fallback gain only when there is no dedicated group sample. */
  gainScale: number;
  silent?: boolean;
}
interface Playing {
  source: AudioBufferSourceNode;
  gain: GainNode;
  panner?: PannerNode;
}

/** One-shot sources; one panner per concurrent spatial voice, recycled on end/steal/stop. */
export class WebAudioVoiceSink implements VoiceSink {
  private readonly playing = new Map<VoiceHandle, Playing>();
  private readonly idlePanners: PannerNode[] = [];

  constructor(
    private readonly context: BaseAudioContext,
    private readonly graph: AudioGraph,
    private readonly lookup: (h: VoiceHandle) => VoiceBuffer,
    private readonly release: (h: VoiceHandle) => void,
    private readonly spatial: Readonly<Partial<Record<Bus, SpatialParams>>> = {},
  ) {}

  start(h: VoiceHandle): void {
    try {
      const { buffer, gainScale, silent } = this.lookup(h);
      const { bus, pos, gain = 1, rate = 1, loop = false } = h.req;
      const source = this.context.createBufferSource();
      const level = this.context.createGain();
      const voice: Playing = { source, gain: level };
      this.playing.set(h, voice);
      source.buffer = buffer;
      // Missing loop assets use the short placeholder once; they must not occupy a slot forever.
      source.loop = loop && !silent;
      source.playbackRate.value = rate;
      level.gain.value = gain * gainScale;
      source.connect(level);
      if (bus !== 'ui' && pos && !silent) {
        const panner = voice.panner = this.idlePanners.pop() ?? this.context.createPanner();
        const params = this.spatial[bus];
        panner.panningModel = 'HRTF';
        // Web Audio defaults when no category-specific configuration is injected.
        panner.distanceModel = params?.distanceModel ?? 'inverse';
        panner.refDistance = params?.refDistance ?? 1;
        panner.maxDistance = params?.maxDistance ?? 10_000;
        panner.rolloffFactor = params?.rolloffFactor ?? 1;
        setPosition(panner, pos);
        level.connect(panner).connect(this.graph.inputs[bus]);
      } else {
        level.connect(this.graph.inputs[bus]);
      }
      source.onended = () => {
        this.disconnect(h, voice);
        this.release(h);
      };
      source.start();
    } catch (err) {
      const voice = this.playing.get(h);
      if (voice) this.disconnect(h, voice);
      this.release(h);
      throw err;
    }
  }

  stop(h: VoiceHandle): void {
    const voice = this.playing.get(h);
    if (!voice) return;
    this.disconnect(h, voice);
    voice.source.stop();
  }

  setPosition(h: VoiceHandle, pos: Vec3): void {
    const panner = this.playing.get(h)?.panner;
    if (panner) setPosition(panner, pos);
  }

  dispose(): void {
    for (const h of this.playing.keys()) {
      this.stop(h);
      this.release(h);
    }
    this.idlePanners.length = 0;
  }

  private disconnect(h: VoiceHandle, voice: Playing): void {
    this.playing.delete(h);
    voice.source.onended = null;
    voice.source.disconnect();
    voice.gain.disconnect();
    if (voice.panner) {
      voice.panner.disconnect();
      this.idlePanners.push(voice.panner);
    }
  }
}
