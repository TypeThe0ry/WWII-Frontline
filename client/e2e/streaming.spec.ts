import { expect, test } from '@playwright/test';

type Fixture = typeof import('./streaming-fixture');
const fixture = '/e2e/streaming-fixture.ts';

for (const speed of [8, 10]) {
  test(`stream_fast_traverse ${speed} m/s covers all fixture tiles across 1,000 m`, async ({ page }) => {
    await page.goto('/e2e/streaming-fixture.html');
    const result = await page.evaluate(async ({ path, velocity }) => (await import(path) as Fixture).traverse(velocity), { path: fixture, velocity: speed });
    expect(result).toMatchObject({ holes: 0, uncoveredPixels: 0, resident: 81, failedAttempts: 4 });
    expect(result.maxUploadBytes).toBeLessThanOrEqual(2 * 1024 * 1024);
    console.log(`Streaming fixture ${speed} m/s`, result);
  });
}

test('baked fpak Range reads use browser workers and repeat from persistent cache', async ({ page }) => {
  const workers: string[] = [];
  page.on('worker', (worker) => workers.push(worker.url()));
  await page.goto('/e2e/streaming-fixture.html');
  const result = await page.evaluate(async (path) => (await import(path) as Fixture).packedCache(), fixture);
  expect(result.cold).toMatchObject({ heights: [65535, 1, 6], splat: 12, uploadBytes: 41222, collisionBytes: 12562, requests: 6, cacheMode: 'opfs' });
  expect(result.warm).toEqual(result.cold);
  expect(workers.filter((url) => url.includes('chunk-decode-worker'))).toHaveLength(4);
});
