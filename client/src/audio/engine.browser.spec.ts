import { expect, test } from '@playwright/test';
import type { SettingsStore } from '../game/settings';
import type { AudioApi } from './engine';
import type { VoiceHandle } from './voices';

interface Harness {
  engine: AudioApi;
  context: AudioContext;
  settings: SettingsStore;
  gains: GainNode[];
  panners: PannerNode[];
  loop?: VoiceHandle;
  logs: string[];
  resumeError?: string;
}
declare global { interface Window { audioHarness: Harness } }
const fixture = '/src/ui/settings/browser-fixture.html';

test('real AudioContext: gesture gate, bus settings, panner reuse, polyphony, silence, and disposal', async ({ page }) => {
  await page.goto(fixture);
  const before = await page.evaluate(async () => {
    const load = (path: string) => import(/* @vite-ignore */ path);
    const { createAudioEngine } = await load('/src/audio/engine.ts') as typeof import('./engine');
    const { SettingsStore } = await load('/src/game/settings.ts') as typeof import('../game/settings');
    const { Persist } = await load('/src/input/bindings.ts') as typeof import('../input/bindings');
    const settings = new SettingsStore(new Persist(() => undefined));
    settings.set('audio.master', 0.5);
    settings.set('quality.tier', 'high');
    const gains: GainNode[] = [];
    const panners: PannerNode[] = [];
    const logs: string[] = [];
    let context!: AudioContext;
    const engine = createAudioEngine({ settings, log: (line) => logs.push(line),
      spatial: { sfx: { refDistance: 3, rolloffFactor: 0.5 } },
      createContext: (options) => {
        context = new AudioContext(options);
        const gain = context.createGain.bind(context);
        const panner = context.createPanner.bind(context);
        context.createGain = () => { const node = gain(); gains.push(node); return node; };
        context.createPanner = () => { const node = panner(); panners.push(node); return node; };
        return context;
      },
    });
    window.audioHarness = { engine, context, settings, gains, panners, logs };
    const buffer = context.createBuffer(1, context.sampleRate / 20, context.sampleRate);
    buffer.getChannelData(0).fill(0.05);
    engine.addBuffer('tone', buffer);
    document.querySelector('#audio')!.addEventListener('click', () => {
      void engine.resume().then(() => { window.audioHarness.loop = engine.play({ id: 'tone', bus: 'sfx', priority: 1, loop: true, pos: { x: 1, y: 2, z: 3 } })!; })
        .catch((err) => { window.audioHarness.resumeError = String(err); });
    });
    return { before: engine.play({ id: 'tone', bus: 'sfx', priority: 1 }), master: gains[0]!.gain.value };
  });
  expect(before).toEqual({ before: null, master: 0.25 });
  await page.getByRole('button', { name: 'Start audio' }).click();
  await expect.poll(() => page.evaluate(() => window.audioHarness.resumeError ?? window.audioHarness.engine.stats().active)).toBe(1);
  const results = await page.evaluate(() => {
    const { context, engine, settings, gains, panners, loop, logs } = window.audioHarness;
    const first = panners[0]!;
    engine.setListener({ x: 1, y: 0, z: 2 }, { x: 0, y: 0, z: 1 }, { x: 0, y: 1, z: 0 });
    engine.setVoicePosition(loop!, { x: 4, y: 5, z: 6 });
    const spatial = [first.panningModel, first.refDistance, first.rolloffFactor, first.positionX.value,
      first.positionZ.value, context.listener.positionZ.value, context.listener.forwardZ.value];
    loop!.stop();
    const second = engine.play({ id: 'tone', bus: 'voice', loop: true, priority: 1, pos: { x: 1, y: 0, z: 0 } });
    const pannerReuse = [panners.length, first.refDistance, first.rolloffFactor];
    second?.stop();
    settings.set('audio.ui', 0.5);
    window.dispatchEvent(new Event('blur'));
    const muted = gains[5]!.gain.value;
    window.dispatchEvent(new Event('focus'));
    const restored = gains[5]!.gain.value;
    let peak = 0;
    for (let i = 0; i < 200; i++) {
      engine.play({ id: 'tone', bus: 'sfx', loop: true, priority: i });
      peak = Math.max(peak, engine.stats().active);
    }
    settings.set('quality.tier', 'low');
    const capped = engine.stats();
    engine.stopAll();
    engine.play({ id: 'absent', bus: 'ui', priority: 1, loop: true }, 'test.sound');
    engine.play({ id: 'absent', bus: 'ui', priority: 1 }, 'test.sound');
    return { spatial, pannerReuse, muted, restored, ui: gains[3]!.gain.value, peak, capped, missing: engine.stats(), logs };
  });
  expect(results.spatial).toEqual(['HRTF', 3, 0.5, 4, -6, -2, -1]);
  expect(results.pannerReuse).toEqual([1, 1, 1]);
  expect([results.muted, results.restored, results.ui]).toEqual([0, 1, 0.25]);
  expect(results.peak).toBe(64);
  expect(results.capped).toMatchObject({ active: 32, stolen: 168, baseLatency: expect.any(Number), outputLatency: expect.any(Number) });
  expect(results.missing).toMatchObject({ active: 2, missingIds: 1 });
  expect(results.logs).toEqual(['[asset-missing] audio absent requested by test.sound']);
  await expect.poll(() => page.evaluate(() => window.audioHarness.engine.stats().active)).toBe(0);
  const after = await page.evaluate(async () => {
    const { engine, context, settings, gains } = window.audioHarness;
    const h = engine.play({ id: 'tone', bus: 'sfx', loop: true, priority: 1 });
    const disposed = engine.dispose();
    const idempotent = engine.dispose() === disposed;
    await disposed;
    settings.set('audio.master', 1);
    window.dispatchEvent(new Event('blur'));
    h?.stop();
    return { idempotent, state: context.state, master: gains[0]!.gain.value, focus: gains[5]!.gain.value,
      active: engine.stats().active, play: engine.play({ id: 'tone', bus: 'sfx', priority: 1 }) };
  });
  expect(after).toEqual({ idempotent: true, state: 'closed', master: 0.25, focus: 1, active: 0, play: null });
});

test('OfflineAudioContext renders the injected buffer through UI and master gain; mute is silent', async ({ page }) => {
  await page.goto(fixture);
  const rendered = await page.evaluate(async () => {
    const load = (path: string) => import(/* @vite-ignore */ path);
    const { AudioGraph } = await load('/src/audio/graph.ts') as typeof import('./graph');
    const { WebAudioVoiceSink } = await load('/src/audio/web-audio-sink.ts') as typeof import('./web-audio-sink');
    const { VoicePool } = await load('/src/audio/voices.ts') as typeof import('./voices');
    async function render(mute: boolean, slider = 0.5) {
      const ctx = new OfflineAudioContext(2, 4_800, 48_000);
      const graph = new AudioGraph(ctx);
      graph.setVolume('master', slider);
      graph.setVolume('ui', slider);
      graph.setVolume('sfx', 0);
      graph.setMuted(mute);
      const buffer = ctx.createBuffer(1, 2_400, 48_000);
      buffer.getChannelData(0).fill(0.02);
      const sink = new WebAudioVoiceSink(ctx, graph, () => ({ buffer, gainScale: 1 }), (h) => pool.release(h));
      const pool = new VoicePool(sink);
      pool.play({ id: 'tone', bus: 'ui', priority: 1 }, 0);
      pool.play({ id: 'tone', bus: 'sfx', priority: 1 }, 0);
      const output = await ctx.startRendering();
      const peaks = [0, 1].map((ch) => Math.max(...output.getChannelData(ch)));
      sink.dispose();
      graph.dispose();
      return peaks;
    }
    return { reference: await render(false, 1), sound: await render(false), mute: await render(true) };
  });
  for (const [ch, peak] of rendered.sound.entries()) {
    expect(peak).toBeGreaterThan(0);
    // Compare against the same browser's non-unity compressor response rather than assuming no makeup gain.
    expect(peak / rendered.reference[ch]!).toBeCloseTo(0.25 * 0.25, 3);
  }
  expect(rendered.mute).toEqual([0, 0]);
});
