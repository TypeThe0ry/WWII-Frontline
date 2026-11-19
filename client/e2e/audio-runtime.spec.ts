import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.route('**/audio-runtime-test.html', (route) => route.fulfill({ contentType: 'text/html', body: '<button>Start</button>' }));
  await page.goto('/audio-runtime-test.html');
});

test('loads an encoded clip; invalid or failed clips stay silent and missing logs deduplicate', async ({ page }) => {
  const result = await page.evaluate(async () => {
    const path = '/src/audio/buffer-library.ts';
    const { AudioBufferLibrary } = await import(path) as typeof import('../src/audio/buffer-library');
    const ctx = new OfflineAudioContext(1, 4800, 48000);
    const logs: string[] = [];
    const bank = new AudioBufferLibrary(ctx, (message) => logs.push(message));
    // In-memory WAV with two signed 16-bit PCM samples. No test assets are shipped.
    const wav = new ArrayBuffer(48);
    const view = new DataView(wav);
    for (const [at, text] of [[0, 'RIFF'], [8, 'WAVE'], [12, 'fmt '], [36, 'data']] as const) {
      new Uint8Array(wav, at, 4).set(new TextEncoder().encode(text));
    }
    for (const [at, n] of [[4, 40], [16, 16], [24, 48000], [28, 96000], [40, 4]] as const) view.setUint32(at, n, true);
    for (const [at, n] of [[20, 1], [22, 1], [32, 2], [34, 16], [44, 16384], [46, 32768]] as const) view.setUint16(at, n, true);
    const id = 'wpn.test.near.4';
    const decoded = await bank.load(id, async () => wav);
    const invalid = await bank.load('wpn.test.near.0', async () => new ArrayBuffer(1));
    const unavailable = await bank.load('fs.grass.0', async () => { throw new Error('unavailable'); });
    for (let i = 0; i < 200; i++) bank.noteMissing('fs.grass.0', 'footstep');
    const silence = bank.get('fs.grass.0', 'footstep');
    return {
      decoded, invalid, unavailable, encodedBytesAfterDecode: wav.byteLength,
      samples: [...bank.get(id, 'test').getChannelData(0)],
      variants: bank.variantCount('wpn.test.near'), selected: bank.variantIndex('wpn.test.near', 0),
      silenceSamples: silence.length, nonzero: silence.getChannelData(0).some(Boolean), logs,
    };
  });
  const { samples, ...metadata } = result;
  expect(metadata).toEqual({
    decoded: true, invalid: false, unavailable: false, encodedBytesAfterDecode: 48,
    variants: 1, selected: 4, silenceSamples: 4800, nonzero: false,
    logs: ['[asset-missing] audio wpn.test.near.0 requested by load', '[asset-missing] audio fs.grass.0 requested by load'],
  });
  expect(samples).toHaveLength(2);
  expect(samples[0]).toBeCloseTo(0.5, 4);
  expect(samples[1]).toBeCloseTo(-1, 4);
  console.log('[audio-library] decoded 2 PCM samples, fallback 4800 zero samples, 200 repeated misses: 1 log for the ID');
});

test('steals a delayed native source and releases the remaining voices when they end', async ({ page }) => {
  const counts = await page.evaluate(async () => {
    const bankPath = '/src/audio/buffer-library.ts';
    const schedulerPath = '/src/audio/web-audio.ts';
    const { AudioBufferLibrary } = await import(bankPath) as typeof import('../src/audio/buffer-library');
    const { WebAudioScheduler } = await import(schedulerPath) as typeof import('../src/audio/web-audio');
    const ctx = new OfflineAudioContext(1, 24000, 48000);
    const bank = new AudioBufferLibrary(ctx);
    const tone = ctx.createBuffer(1, 512, ctx.sampleRate);
    tone.getChannelData(0).fill(0.5);
    bank.add('test', tone);
    const scheduler = new WebAudioScheduler(ctx, bank);
    const { pool } = scheduler;
    pool.setTier('low');
    for (let i = 0; i < pool.capacity; i++) {
      const voice = pool.play({ id: 'test', bus: 'sfx', priority: 1 }, 0)!;
      scheduler.schedule('test', i === 0 ? 0.35 : 0.05, 0.01, 1, voice);
    }
    const winner = pool.play({ id: 'test', bus: 'sfx', priority: 2 }, 0)!;
    scheduler.schedule('test', 0.05, 0.01, 1, winner);
    const peak = pool.stats.active;
    // Observe the real onended callbacks before disposal and after the surviving sources finish.
    const afterEnds = ctx.suspend(0.25).then(async () => {
      const active = pool.stats.active;
      await ctx.resume();
      return active;
    });
    const [rendered, activeAfter] = await Promise.all([ctx.startRendering(), afterEnds]);
    const pcm = rendered.getChannelData(0);
    const result = {
      peak, stolen: pool.stats.stolen, activeAfter,
      audible: pcm.slice(2400, 3000).some((x) => Math.abs(x) > 1e-5),
      stolenPlayed: pcm.slice(16800).some((x) => Math.abs(x) > 1e-5),
    };
    scheduler.dispose();
    return result;
  });
  expect(counts).toEqual({ peak: 32, stolen: 1, activeAfter: 0, audible: true, stolenPlayed: false });
  console.log('[audio-voices] low tier: 32 active, 1 delayed source stolen before onset, 0 active after natural end');
});

test('gates the realtime event bus until a trusted gesture, deduplicates misses and unbinds on dispose', async ({ page }) => {
  const warnings: string[] = [];
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'warning' && message.text().startsWith('[asset-missing] audio')) warnings.push(message.text());
    if (message.type() === 'error') errors.push(message.text());
  });
  await page.evaluate(async () => {
    const audioPath = '/src/audio/game-audio.ts';
    const busPath = '/src/game/event-bus.ts';
    const { attachGameAudio } = await import(audioPath) as typeof import('../src/audio/game-audio');
    const { EventBus } = await import(busPath) as typeof import('../src/game/event-bus');
    const bus = new EventBus();
    const point = { x: 0, y: 0, z: 0 };
    const game = attachGameAudio(bus, { weaponId: (id) => id === 1 ? 'test' : 'other', listener: () => point });
    if (!game) throw new Error('AudioContext not available');
    const emit = (weapon = 1) => bus.emit('fire.local', { weapon, muzzlePos: point, dir: point });
    const hooks = { emit, dispose: game.dispose, missing: () => game.library.missingCount };
    Object.assign(window, { audioTest: hooks });
    document.querySelector('button')!.click(); // Untrusted, should not unlock.
    emit();
  });
  expect(warnings).toEqual([]);
  await page.getByRole('button', { name: 'Start' }).click();
  await expect.poll(() => page.evaluate(() => {
    const hooks = (window as unknown as { audioTest: { emit: () => void; missing: () => number } }).audioTest;
    hooks.emit();
    return hooks.missing();
  })).toBe(1);
  const after = await page.evaluate(() => {
    const hooks = (window as unknown as { audioTest: { emit: (weapon?: number) => void; missing: () => number; dispose: () => void } }).audioTest;
    for (let i = 0; i < 200; i++) hooks.emit();
    hooks.dispose();
    hooks.emit(2);
    return hooks.missing();
  });
  expect(after).toBe(1);
  expect(warnings).toEqual(['[asset-missing] audio wpn.test.near.0 requested by shot']);
  expect(errors).toEqual([]);
  console.log('[audio-runtime] 200 event repeats: 1 missing ID/log, 0 console errors');
});
