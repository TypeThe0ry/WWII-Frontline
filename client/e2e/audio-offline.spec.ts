import { expect, test } from '@playwright/test';

const probe = {
  sampleRate: 48_000,
  bankPath: '/src/audio/bank.ts',
  bindPath: '/src/audio/bind.ts',
  dispatchPath: '/src/audio/dispatch.ts',
  schedulerPath: '/src/audio/web-audio.ts',
  busPath: '/src/game/event-bus.ts',
};

test.describe.configure({ timeout: 15_000 });

test.beforeEach(async ({ page }) => {
  // Keep Vite's origin for module imports without booting the game or loading assets.
  await page.route('**/audio-offline-probe', (route) => route.fulfill({
    contentType: 'text/html',
    body: '<!doctype html><title>Offline audio probe</title>',
  }));
  await page.goto('/audio-offline-probe');
});

test('M1-063b: local fire starts inline and remote shot onsets follow the sound delay', async ({ page }) => {
  const shots = [
    { kind: 'local', distanceM: 1, delayS: 0 },
    { kind: 'remote', distanceM: 150, delayS: 0 },
    { kind: 'remote', distanceM: 200, delayS: 200 / 343 },
    { kind: 'remote', distanceM: 343, delayS: 1 },
    { kind: 'remote', distanceM: 686, delayS: 2 },
    { kind: 'remote', distanceM: 1029, delayS: 3 },
  ];
  const results = await page.evaluate(async ({ shots, sampleRate, bankPath, bindPath, dispatchPath, schedulerPath, busPath }) => {
    const { AudioBank }: typeof import('../src/audio/bank') = await import(bankPath);
    const { bindAudioEvents }: typeof import('../src/audio/bind') = await import(bindPath);
    const { AudioDirector, shotId }: typeof import('../src/audio/dispatch') = await import(dispatchPath);
    const { WebAudioScheduler }: typeof import('../src/audio/web-audio') = await import(schedulerPath);
    const { EventBus }: typeof import('../src/game/event-bus') = await import(busPath);

    const emitAtS = 0.125;
    const weapon = 'us_m1_garand';
    const at = (x: number) => ({ x, y: 0, z: 0 });
    const results = [];
    for (const shot of shots) {
      const context = new OfflineAudioContext(1, Math.ceil((emitAtS + shot.delayS + 0.05) * sampleRate), sampleRate);
      const bank = new AudioBank((seconds) => context.createBuffer(1, Math.ceil(seconds * sampleRate), sampleRate));
      const pulse = context.createBuffer(1, 512, sampleRate);
      pulse.getChannelData(0).fill(0.5);
      for (const layer of ['near', 'mid', 'far'] as const) bank.add(shotId(weapon, layer, 0), pulse);
      const scheduler = new WebAudioScheduler(context, bank);
      const director = new AudioDirector(scheduler, scheduler.pool, { variantCount: () => 1 });
      const bus = new EventBus();
      bus.onError = (error) => { throw error; };
      const off = bindAudioEvents(bus, director, { now: () => emitAtS, weaponId: () => weapon, listener: () => at(0) });
      const event = { weapon: 0, muzzlePos: at(shot.distanceM), dir: at(1) };
      let inlineStarts = 0;
      try {
        if (shot.kind === 'local') {
          // Observe the real source call only during emit; PCM alone cannot distinguish inline from microtask scheduling.
          const proto = AudioBufferSourceNode.prototype;
          const start = proto.start;
          proto.start = function (this: AudioBufferSourceNode, ...args: Parameters<typeof start>) {
            inlineStarts++;
            return start.apply(this, args);
          };
          try {
            bus.emit('fire.local', event);
          } finally {
            proto.start = start;
          }
        } else {
          bus.emit('fire.remote', { ...event, shooter: 1, distance: shot.distanceM });
        }
        const pcm = (await context.startRendering()).getChannelData(0);
        const onsetFrame = pcm.findIndex((sample) => Math.abs(sample) > 1e-5);
        results.push({
          ...shot, inlineStarts, onsetFrame,
          measuredDelayMs: onsetFrame < 0 ? null : 1000 * (onsetFrame / sampleRate - emitAtS),
        });
      } finally {
        off();
        scheduler.dispose();
      }
    }
    return results;
  }, { ...probe, shots });

  console.log('[audio-offline] shot onsets', JSON.stringify(results));
  for (const shot of results) {
    const label = `${shot.kind} shot at ${shot.distanceM} m`;
    if (shot.kind === 'local') expect(shot.inlineStarts, 'native start calls during fire.local emit').toBe(1);
    expect(shot.onsetFrame, `${label} rendered onset sample`).toBeGreaterThanOrEqual(0);
    // Allow one Web Audio render quantum, tighter than the 16.7 ms game-frame budget.
    expect(Math.abs((shot.measuredDelayMs ?? Infinity) - shot.delayS * 1000), label).toBeLessThanOrEqual(128_000 / probe.sampleRate);
  }
});

test('M1-063b: scheduler gain and playback rate change rendered amplitude and pitch', async ({ page }) => {
  const result = await page.evaluate(async ({ sampleRate, bankPath, schedulerPath }) => {
    const { AudioBank }: typeof import('../src/audio/bank') = await import(bankPath);
    const { WebAudioScheduler }: typeof import('../src/audio/web-audio') = await import(schedulerPath);
    const context = new OfflineAudioContext(1, 0.4 * sampleRate, sampleRate);
    const bank = new AudioBank((seconds) => context.createBuffer(1, Math.ceil(seconds * sampleRate), sampleRate));
    const tone = context.createBuffer(1, 0.16 * sampleRate, sampleRate);
    const samples = tone.getChannelData(0);
    for (let i = 0; i < samples.length; i++) samples[i] = 0.5 * Math.sin(2 * Math.PI * 500 * i / sampleRate);
    bank.add('tone', tone);

    const scheduler = new WebAudioScheduler(context, bank);
    try {
      scheduler.schedule('tone', 0.02, 1, 1);
      scheduler.schedule('tone', 0.22, 0.25, 2);
      const pcm = (await context.startRendering()).getChannelData(0);
      const measure = (atS: number) => {
        // Measure steady state, strictly inside both the 1x and 2x bursts.
        const window = pcm.subarray(Math.round((atS + 0.01) * sampleRate), Math.round((atS + 0.05) * sampleRate));
        const rms = Math.sqrt(window.reduce((sum, x) => sum + x * x, 0) / window.length);
        let crossings = 0;
        for (let i = 1; i < window.length; i++) if (window[i - 1]! <= 0 && window[i]! > 0) crossings++;
        return { rms, hz: crossings * sampleRate / window.length };
      };
      const reference = measure(0.02);
      const adjusted = measure(0.22);
      return { reference, adjusted, gainRatio: adjusted.rms / reference.rms };
    } finally {
      scheduler.dispose();
    }
  }, probe);

  console.log('[audio-offline] gain/rate', JSON.stringify(result));
  expect(result.reference.rms).toBeCloseTo(0.5 / Math.SQRT2, 2);
  expect(Math.abs(result.gainRatio - 0.25), 'RMS gain ratio').toBeLessThan(0.015);
  // A 40 ms counting window has 25 Hz resolution.
  expect(Math.abs(result.reference.hz - 500)).toBeLessThanOrEqual(25);
  expect(Math.abs(result.adjusted.hz - 1000)).toBeLessThanOrEqual(25);
});

test('M1-063b: suppression ducks and low-passes, retriggers during recovery, then recovers', async ({ page }) => {
  const result = await page.evaluate(async ({ sampleRate, bankPath, bindPath, dispatchPath, schedulerPath, busPath }) => {
    const { AudioBank }: typeof import('../src/audio/bank') = await import(bankPath);
    const { bindAudioEvents }: typeof import('../src/audio/bind') = await import(bindPath);
    const { AudioDirector }: typeof import('../src/audio/dispatch') = await import(dispatchPath);
    const { WebAudioScheduler }: typeof import('../src/audio/web-audio') = await import(schedulerPath);
    const { EventBus }: typeof import('../src/game/event-bus') = await import(busPath);
    const context = new OfflineAudioContext(1, 0.82 * sampleRate, sampleRate);
    const bank = new AudioBank((seconds) => context.createBuffer(1, Math.ceil(seconds * sampleRate), sampleRate));
    const tone = context.createBuffer(1, context.length, sampleRate);
    const samples = tone.getChannelData(0);
    const lowHz = 200;
    const highHz = 20_000; // Near the open cutoff: sensitive to incomplete cutoff recovery.
    for (let i = 0; i < samples.length; i++) {
      samples[i] = 0.2 * (Math.sin(2 * Math.PI * lowHz * i / sampleRate) + Math.sin(2 * Math.PI * highHz * i / sampleRate));
    }
    bank.add('probe', tone);

    const scheduler = new WebAudioScheduler(context, bank);
    const director = new AudioDirector(scheduler, scheduler.pool, { variantCount: () => 0 });
    const bus = new EventBus();
    bus.onError = (error) => { throw error; };
    const off = bindAudioEvents(bus, director, {
      now: () => context.currentTime,
      weaponId: () => undefined,
      listener: () => ({ x: 0, y: 0, z: 0 }),
      onSuppress: (atS) => scheduler.applySuppression(director, atS),
    });
    try {
      scheduler.schedule('probe', 0, 1, 1);
      // Hit the graph during rendering, including a second hit before the first recovery deadline.
      const hitAt = (atS: number) => context.suspend(atS).then(async () => {
        const actualTime = context.currentTime;
        bus.emit('suppress', { intensity: 1, angle: 0 });
        await context.resume();
        return actualTime;
      });
      const first = hitAt(0.10);
      const retrigger = hitAt(0.34);
      const [rendered, firstAtS, retriggerAtS] = await Promise.all([context.startRendering(), first, retrigger]);
      const pcm = rendered.getChannelData(0);
      const amplitude = (atS: number, hz: number) => {
        // Quadrature projection isolates each band despite the filter's phase shift; 10 ms is whole cycles for both probes.
        const start = Math.round(atS * sampleRate);
        const length = sampleRate / 100;
        let re = 0, im = 0;
        for (let i = 0; i < length; i++) {
          const phase = 2 * Math.PI * hz * i / sampleRate;
          const sample = pcm[start + i]!;
          re += sample * Math.cos(phase);
          im += sample * Math.sin(phase);
        }
        return 2 * Math.hypot(re, im) / length;
      };
      const baseline = { low: amplitude(0.05, lowHz), high: amplitude(0.05, highHz) };
      const bands = (atS: number) => {
        const low = amplitude(atS, lowHz) / baseline.low;
        const high = amplitude(atS, highHz) / baseline.high;
        return { atS, low, highOverLow: high / low };
      };
      return {
        baseline, firstAtS, retriggerAtS,
        firstHit: bands(firstAtS + 0.01),
        beforeRetrigger: bands(retriggerAtS - 0.02),
        secondHit: bands(retriggerAtS + 0.01),
        firstDeadline: bands(firstAtS + 0.4),
        recovered: bands(retriggerAtS + 0.4),
      };
    } finally {
      off();
      scheduler.dispose();
    }
  }, probe);

  console.log('[audio-offline] suppression band ratios', JSON.stringify(result));
  expect(result.baseline.low).toBeGreaterThan(0.15);
  expect(result.baseline.high).toBeGreaterThan(0.05);
  for (const [label, hit] of [['first hit', result.firstHit], ['retrigger', result.secondHit]] as const) {
    expect(hit.low, `${label}: low-band duck (linear amplitude)`).toBeGreaterThan(0.25);
    expect(hit.low, `${label}: low-band duck (linear amplitude)`).toBeLessThan(0.55);
    expect(hit.highOverLow, `${label}: high-band loss beyond the duck`).toBeLessThan(0.15);
  }
  expect(result.beforeRetrigger.low - result.secondHit.low, 'retrigger deepens a partially recovered duck').toBeGreaterThan(0.12);
  expect(result.firstDeadline.low, 'first deadline does not clear the second duck').toBeLessThan(0.85);
  expect(result.firstDeadline.highOverLow, 'first deadline does not clear the second low-pass').toBeLessThan(0.5);
  expect(Math.abs(result.recovered.low - 1), 'duck recovers by 400 ms after retrigger').toBeLessThan(0.05);
  expect(Math.abs(result.recovered.highOverLow - 1), 'high/low transfer recovers by 400 ms after retrigger').toBeLessThan(0.05);
});
