import type { Bus, Vec3 } from './voices';
import { sliderToGain } from './volumes';

export type GraphBus = Bus | 'master';

/** The master compressor and hard ceiling protect the output when many voices sum together. */
export class AudioGraph {
  readonly inputs: Readonly<Record<GraphBus, GainNode>>;
  private readonly compressor: DynamicsCompressorNode;
  private readonly ceiling: WaveShaperNode;
  private readonly focusGate: GainNode;

  constructor(context: BaseAudioContext) {
    const master = context.createGain();
    this.inputs = {
      master, sfx: context.createGain(), amb: context.createGain(), ui: context.createGain(), voice: context.createGain(),
    };
    for (const bus of ['sfx', 'amb', 'ui', 'voice'] as const) this.inputs[bus].connect(master);
    this.compressor = context.createDynamicsCompressor();
    this.compressor.threshold.value = -12;
    this.compressor.ratio.value = 12;
    this.compressor.attack.value = 0.003;
    this.ceiling = context.createWaveShaper();
    this.ceiling.curve = new Float32Array([-1, 0, 1]);
    this.focusGate = context.createGain();
    master.connect(this.compressor).connect(this.ceiling).connect(this.focusGate).connect(context.destination);
  }

  setVolume(bus: GraphBus, slider: number): void {
    this.inputs[bus].gain.value = sliderToGain(slider);
  }

  setMuted(muted: boolean): void {
    this.focusGate.gain.value = muted ? 0 : 1;
  }

  dispose(): void {
    for (const bus of Object.values(this.inputs)) bus.disconnect();
    this.compressor.disconnect();
    this.ceiling.disconnect();
    this.focusGate.disconnect();
  }
}

// Game space (+z forward, +x right) is left handed; Web Audio is right handed.
export function setPosition(node: PannerNode | AudioListener, pos: Vec3): void {
  node.positionX.value = pos.x;
  node.positionY.value = pos.y;
  node.positionZ.value = -pos.z;
}

export function setListenerPose(listener: AudioListener, pos: Vec3, fwd: Vec3, up: Vec3): void {
  setPosition(listener, pos);
  listener.forwardX.value = fwd.x;
  listener.forwardY.value = fwd.y;
  listener.forwardZ.value = -fwd.z;
  listener.upX.value = up.x;
  listener.upY.value = up.y;
  listener.upZ.value = -up.z;
}
