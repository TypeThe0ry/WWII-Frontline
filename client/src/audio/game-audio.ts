import type { EventBus, GameEvents } from '../game/event-bus';
import { bindAudioEvents, type AudioBindDeps } from './bind';
import { AudioBufferLibrary } from './buffer-library';
import { AudioDirector } from './dispatch';
import { WebAudioScheduler } from './web-audio';

/** Optional route supplied by the lobby engine. That engine owns the shared context. */
export interface GameAudioOutput {
  context: AudioContext;
  sfxInput: AudioNode;
  resume(): Promise<void>;
  canPlay(): boolean;
}

export interface GameAudioHandle {
  readonly library: AudioBufferLibrary;
  flush(): void;
  dispose(): void;
}

/** Realtime owner. The licensed clip catalog is populated by M1-065; an empty library is valid. */
export function attachGameAudio(bus: EventBus<GameEvents>, deps: Pick<AudioBindDeps, 'weaponId' | 'listener' | 'surfaceAt'>, output?: GameAudioOutput): GameAudioHandle | null {
  if (typeof AudioContext === 'undefined') return null;
  let context: AudioContext;
  try {
    context = output?.context ?? new AudioContext({ latencyHint: 'interactive' });
  } catch {
    return null;
  }
  const library = new AudioBufferLibrary(context, (message) => console.warn(message), import.meta.env.DEV);
  const scheduler = new WebAudioScheduler(context, library, output?.sfxInput);
  const director = new AudioDirector(scheduler, scheduler.pool, {
    variantCount: (weapon, layer) => library.variantCount(`wpn.${weapon}.${layer}`),
    footstepVariants: (surface) => library.variantCount(`fs.${surface}`),
    variantIndex: (prefix, slot) => library.variantIndex(prefix, slot),
    missing: (id, event) => library.noteMissing(id, event),
  });

  let gestureSeen = false;
  let resuming = false;
  const resume = (event: Event): void => {
    if (!event.isTrusted) return;
    gestureSeen = true;
    if ((output ? output.canPlay() : context.state === 'running') || resuming) return;
    resuming = true;
    void (output ? output.resume() : context.resume()).catch(() => { /* a later trusted gesture can retry */ }).finally(() => { resuming = false; });
  };
  document.addEventListener('pointerdown', resume, true);
  document.addEventListener('pointerup', resume, true);
  document.addEventListener('keydown', resume, true);
  const off = bindAudioEvents(bus, director, {
    ...deps,
    now: () => context.currentTime,
    canPlay: () => output?.canPlay() ?? (gestureSeen && context.state === 'running'),
    onSuppress: (at) => scheduler.applySuppression(director, at),
  });
  return {
    library,
    flush: () => { director.flush(context.currentTime); },
    dispose: () => {
      off();
      document.removeEventListener('pointerdown', resume, true);
      document.removeEventListener('pointerup', resume, true);
      document.removeEventListener('keydown', resume, true);
      scheduler.dispose();
      if (!output && context.state !== 'closed') void context.close().catch(() => {});
    },
  };
}
