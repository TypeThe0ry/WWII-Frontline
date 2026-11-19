import type { SettingsStore, SettingValue } from '../game/settings';
import { AudioBank } from './bank';
import { probeCodec } from './codec-probe';
import type { GameAudioOutput } from './game-audio';
import { AudioGraph, setListenerPose, type GraphBus } from './graph';
import { VoicePool, type Bus, type PlayRequest, type Tier, type Vec3, type VoiceHandle } from './voices';
import { SETTING_TO_BUS } from './volumes';
import { WebAudioVoiceSink, type SpatialParams } from './web-audio-sink';

export interface AudioEngineOptions {
  settings: Pick<SettingsStore, 'get' | 'onChange'>;
  /** The engine owns the context returned by this factory and will close it on dispose. */
  createContext?: (options: AudioContextOptions) => AudioContext;
  document?: Document;
  muteOnBlur?: boolean;
  spatial?: Readonly<Partial<Record<Bus, SpatialParams>>>;
  groupVariant?: (id: string) => string | undefined;
  dev?: boolean;
  log?: (message: string) => void;
}

export interface AudioApi {
  readonly gameOutput: GameAudioOutput;
  /** Invoke directly in a user gesture. A failed attempt can be retried on the next gesture. */
  resume(): Promise<void>;
  addBuffer(id: string, buffer: AudioBuffer): void;
  probe(probes: { opus: ArrayBuffer; aac: ArrayBuffer }): ReturnType<typeof probeCodec>;
  play(r: PlayRequest, requestedBy?: string): VoiceHandle | null;
  setListener(pos: Vec3, fwd: Vec3, up: Vec3): void;
  setVoicePosition(h: VoiceHandle, pos: Vec3): void;
  setVolume(bus: GraphBus, slider: number): void;
  setTier(tier: Tier): void;
  setMuteOnBlur(enabled: boolean): void;
  stopAll(): void;
  stats(): { active: number; stolen: number; missingIds: number; baseLatency: number; outputLatency?: number };
  dispose(): Promise<void>;
}

const finitePosition = ({ x, y, z }: Vec3): boolean => Number.isFinite(x) && Number.isFinite(y) && Number.isFinite(z);

export function createAudioEngine(options: AudioEngineOptions): AudioApi {
  const { settings, spatial, groupVariant, dev = import.meta.env.DEV, log = console.warn } = options;
  const doc = options.document ?? globalThis.document;
  const win = doc?.defaultView;
  const context = (options.createContext ?? ((opts) => new AudioContext(opts)))({ latencyHint: 'interactive' });
  const graph = new AudioGraph(context);
  let bank: AudioBank<AudioBuffer> | undefined = new AudioBank(
    (s) => context.createBuffer(1, Math.round(s * context.sampleRate), context.sampleRate), log, dev,
  );
  const requestNames = new WeakMap<PlayRequest, string>();
  const sink = new WebAudioVoiceSink(context, graph, (h) => {
    const variant = h.grouped ? groupVariant?.(h.req.id) : undefined;
    const hasVariant = variant !== undefined && bank!.has(variant);
    const id = hasVariant ? variant : h.req.id;
    return {
      buffer: bank!.get(id, requestNames.get(h.req) ?? h.req.id),
      gainScale: h.grouped && !hasVariant ? 0.6 : 1,
      silent: !bank!.has(id),
    };
  }, (h) => pool.release(h), spatial);
  const pool = new VoicePool(sink);
  let disposed: Promise<void> | undefined;
  let gestureResumed = false;
  let muteOnBlur = options.muteOnBlur ?? true;
  let focused = doc?.hasFocus() ?? true;
  let missingIds = 0;

  const updateMute = (): void => graph.setMuted(muteOnBlur && (!focused || doc?.hidden === true));
  const blur = (): void => { focused = false; updateMute(); };
  const focus = (): void => { focused = true; updateMute(); };
  doc?.addEventListener('visibilitychange', updateMute);
  win?.addEventListener('blur', blur);
  win?.addEventListener('focus', focus);
  updateMute();

  const applySetting = (key: string, value: SettingValue): void => {
    if (key === 'quality.tier' && (value === 'low' || value === 'medium' || value === 'high')) pool.setTier(value);
    if (key === 'audio.muteOnBlur' && typeof value === 'boolean') { muteOnBlur = value; updateMute(); }
    const bus = SETTING_TO_BUS[key];
    // M1 has no separate soundtrack playback bus; the music slider (if supplied) controls ambient beds.
    if (bus && typeof value === 'number') graph.setVolume(bus === 'music' ? 'amb' : bus, value);
  };
  pool.setTier('medium');
  for (const key of [...Object.keys(SETTING_TO_BUS), 'quality.tier', 'audio.muteOnBlur']) applySetting(key, settings.get(key));
  const offSettings = settings.onChange(({ key, value }) => applySetting(key, value));

  async function resume() {
    if (disposed) return;
    await context.resume();
    if (!disposed && context.state === 'running') gestureResumed = true;
  }
  const canOutput = (): boolean => !disposed && gestureResumed && context.state === 'running';

  return {
    gameOutput: { context, sfxInput: graph.inputs.sfx, resume, canPlay: canOutput },
    resume,
    addBuffer(id, buffer) { bank?.add(id, buffer); },
    probe: (probes) => disposed ? Promise.resolve('none') : probeCodec((bytes) => context.decodeAudioData(bytes), probes),
    play(r, requestedBy = r.id) {
      if (!canOutput()) return null;
      if (!Number.isFinite(r.priority) || (r.pos && !finitePosition(r.pos))) return null;
      if (r.gain !== undefined && (!Number.isFinite(r.gain) || r.gain < 0)) return null;
      if (r.rate !== undefined && (!Number.isFinite(r.rate) || r.rate <= 0)) return null;
      requestNames.set(r, requestedBy);
      return pool.play(r, context.currentTime * 1000);
    },
    setListener(pos, fwd, up) {
      if (disposed || !finitePosition(pos) || !finitePosition(fwd) || !finitePosition(up)) return;
      pool.listener = { ...pos };
      setListenerPose(context.listener, pos, fwd, up);
    },
    setVoicePosition(h, pos) { if (!disposed && finitePosition(pos)) sink.setPosition(h, pos); },
    setVolume(bus, slider) { if (!disposed) graph.setVolume(bus, slider); },
    setTier(tier) { if (!disposed) pool.setTier(tier); },
    setMuteOnBlur(enabled) { if (!disposed) { muteOnBlur = enabled; updateMute(); } },
    stopAll() { pool.stopAll(); },
    stats: () => ({
      active: pool.stats.active, stolen: pool.stats.stolen, missingIds: bank?.missingCount ?? missingIds,
      baseLatency: context.baseLatency, outputLatency: context.outputLatency,
    }),
    dispose() {
      if (disposed) return disposed;
      offSettings();
      doc?.removeEventListener('visibilitychange', updateMute);
      win?.removeEventListener('blur', blur);
      win?.removeEventListener('focus', focus);
      sink.dispose();
      graph.dispose();
      missingIds = bank!.missingCount;
      bank = undefined;
      disposed = context.state === 'closed' ? Promise.resolve() : context.close();
      return disposed;
    },
  };
}
