import { expect, test, type Page } from '@playwright/test';

// Relative to the client/ directory (Playwright's cwd).
const SHIM = 'e2e/webgpu-swiftshader-shim.js';

// Clear colour of the test scene (src/render/test-scene.ts CLEAR_COLOR) in 8-bit.
const CLEAR = [Math.round(0.04 * 255), Math.round(0.06 * 255), Math.round(0.1 * 255)];

function watchErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`console.error: ${m.text()}`);
  });
  return errors;
}

async function framesAtLeast(page: Page, n: number): Promise<void> {
  await page.waitForFunction((k) => (window.__perf?.frames() ?? 0) >= k, n, { timeout: 30_000 });
}

/** Screenshot the page and sample the centre + 4 corners (inset) of the canvas. */
async function samplePoints(page: Page): Promise<number[][]> {
  const png = (await page.screenshot()).toString('base64');
  return page.evaluate(async (b64) => {
    const img = new Image();
    img.src = `data:image/png;base64,${b64}`;
    await img.decode();
    const c = document.createElement('canvas');
    c.width = img.width;
    c.height = img.height;
    const g = c.getContext('2d')!;
    g.drawImage(img, 0, 0);
    const w = img.width;
    const h = img.height;
    const pts: [number, number][] = [
      [w >> 1, h >> 1],
      [8, 8],
      [w - 9, 8],
      [8, h - 9],
      [w - 9, h - 9],
    ];
    return pts.map(([x, y]) => Array.from(g.getImageData(x, y, 1, 1).data.subarray(0, 3)));
  }, png);
}

const dist = (a: number[], b: number[]) => Math.abs(a[0]! - b[0]!) + Math.abs(a[1]! - b[1]!) + Math.abs(a[2]! - b[2]!);

test('webgl2 path renders the test scene, stable for >= 60 frames (acceptance 1)', async ({ page }, info) => {
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2');
  await framesAtLeast(page, 10);
  const first = await samplePoints(page);
  const f0 = await page.evaluate(() => window.__perf!.frames());
  await framesAtLeast(page, f0 + 60);
  const second = await samplePoints(page);
  await page.screenshot({ path: info.outputPath('webgl2.png') });

  const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null });
  expect(caps.kind).toBe('webgl2');
  expect(caps.fallbackReason).toBeNull();
  for (let i = 0; i < 5; i++) {
    expect(dist(first[i]!, CLEAR), `sample ${i} differs from clear colour: ${first[i]}`).toBeGreaterThan(30);
    expect(dist(first[i]!, second[i]!), `sample ${i} stable across 60 frames`).toBeLessThanOrEqual(6);
  }
  expect(errors).toEqual([]);
});

test('navigator.gpu = undefined -> auto falls back to WebGL2 within 2 s and says why (acceptance 5)', async ({ page }, info) => {
  const errors = watchErrors(page);
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'gpu', { value: undefined, configurable: true });
  });
  await page.goto('/?debug=1');
  await framesAtLeast(page, 30);
  const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null; readyMs: number });
  expect(caps.kind).toBe('webgl2');
  expect(caps.fallbackReason).toContain('navigator.gpu is undefined');
  expect(caps.readyMs).toBeLessThanOrEqual(2000); // ms since navigation start
  await expect(page.locator('#debug-overlay')).toContainText('fallback: navigator.gpu is undefined');
  await expect(page.locator('#debug-overlay')).toContainText('WebGL2');
  await page.screenshot({ path: info.outputPath('fallback.png') });
  const pts = await samplePoints(page);
  expect(dist(pts[0]!, CLEAR)).toBeGreaterThan(30);
  expect(errors).toEqual([]);
  console.log(`fallback ready at ${caps.readyMs.toFixed(0)} ms after navigation start`);
});

test('window.__perf.dump() has the fixed schema (acceptance 2)', async ({ page }) => {
  await page.goto('/?renderer=webgl2&debug=1');
  await framesAtLeast(page, 30);
  const d = await page.evaluate(() => window.__perf!.dump() as unknown as Record<string, unknown>);
  expect(Object.keys(d).sort()).toEqual(
    ['active_meshes', 'cpu_ms', 'draw_calls', 'fallback_reason', 'fps', 'frames', 'gpu_ms', 'interval_ms', 'render_scale', 'renderer', 'resolution', 'schema', 'sim', 'slots', 'timestamp_ms', 'triangles'].sort(),
  );
  expect(Object.keys(d.cpu_ms as object).sort()).toEqual(['count', 'max', 'mean', 'p50', 'p99']);
  expect(d.draw_calls as number).toBeGreaterThan(0);
  expect(d.triangles as number).toBeGreaterThan(0);
  await expect(page.locator('#debug-overlay')).toContainText('GPU ms');
  await page.keyboard.press('F3');
  await expect(page.locator('#debug-overlay')).toBeHidden();
});

test('WebGPU path on the SwiftShader software adapter: engine, timestamp-query, compute smoke (acceptance 2/3, software only)', async ({ page }) => {
  const errors = watchErrors(page);
  await page.addInitScript({ path: SHIM });
  await page.goto('/?renderer=webgpu&debug=1&smoke=compute');
  await page.waitForFunction(() => window.__caps !== undefined, undefined, { timeout: 15_000 });
  const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null });
  test.skip(caps.kind !== 'webgpu', `no WebGPU adapter in this browser (${caps.fallbackReason})`);
  await framesAtLeast(page, 60);
  await page.waitForFunction(() => window.__smoke !== undefined, undefined, { timeout: 30_000 });
  const smoke = await page.evaluate(() => window.__smoke as Record<string, { ok: boolean; mismatches: number; ms: number; detail: string } | boolean | number | string>);
  console.log('smoke', JSON.stringify(smoke));
  expect(smoke.ok).toBe(true);
  expect(smoke.mismatches).toBe(0);
  for (const k of ['buffer', 'texture', 'hiz', 'froxel', 'taau']) {
    expect((smoke[k] as { ok: boolean }).ok, `trial ${k}`).toBe(true);
  }
  // GPU time comes from timestamp-query; the number itself is software-adapter time (not a GPU figure).
  await expect.poll(() => page.evaluate(() => window.__perf!.dump().gpu_ms.source)).toBe('timestamp-query');
  const gpu = await page.evaluate(() => window.__perf!.dump().gpu_ms);
  expect(gpu.value).not.toBeNull();
  expect(errors).toEqual([]);
});

test('?smoke=compute on WebGL2 reports unsupported (expected)', async ({ page }) => {
  await page.goto('/?renderer=webgl2&smoke=compute');
  await page.waitForFunction(() => window.__smoke !== undefined, undefined, { timeout: 15_000 });
  const smoke = await page.evaluate(() => window.__smoke as { ok: boolean; status: string });
  expect(smoke.status).toBe('unsupported (expected)');
});
