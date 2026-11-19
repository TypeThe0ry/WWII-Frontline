import { chromium, expect, test, type Page } from '@playwright/test';

async function verifyBatches(page: Page, renderer: 'webgl2' | 'webgpu'): Promise<void> {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  if (renderer === 'webgpu') await page.addInitScript({ path: 'e2e/webgpu-swiftshader-shim.js' });
  await page.goto(`/?renderer=${renderer}&scene=terrain&actors=demo&entities=0&norender=1`);
  await expect.poll(async () => {
    expect(errors).toEqual([]);
    return page.evaluate(() => Boolean(window.__actors && window.__terrain));
  }, { timeout: 30_000 }).toBe(true);
  const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null } | undefined);
  test.skip(renderer === 'webgpu' && caps?.kind !== renderer, `WebGPU unavailable: ${caps?.fallbackReason}`);
  expect(caps?.kind).toBe(renderer);

  const result = await page.evaluate(async () => {
    const actors = window.__actors!;
    const fx = actors.manager.fx;
    const scene = fx.particles!.mesh.getScene();
    scene.activeCamera!.getViewMatrix(true);
    const { x, y, z } = scene.activeCamera!.globalPosition;
    const impact = { pos: { x, y: window.__terrain!.heightAt(x, z + 2), z: z + 2 }, normal: (32 << 6) | 32, material: 0 };
    const explosion = { pos: { x: x + 2, y, z: z + 6 }, radius: 8, own: false };
    actors.bus.emit('impact', impact);
    const decals = fx.decals!.mesh;
    const particleMesh = fx.particles!.mesh;
    let decalDraws = 0;
    let particleDraws = 0;
    const decalObserver = decals.onBeforeDrawObservable.add(() => decalDraws++);
    const particleObserver = particleMesh.onBeforeDrawObservable.add(() => particleDraws++);
    try {
      for (let i = 0; i < 30 && !(decalDraws && particleDraws); i++) {
        actors.bus.emit('explosion', explosion);
        window.__terrain!.renderOnce();
        await new Promise<void>((resolve) => setTimeout(resolve, 16));
      }
      return { decalDraws, particleDraws, activeDecals: fx.decals!.marks.active };
    } finally {
      decals.onBeforeDrawObservable.remove(decalObserver);
      particleMesh.onBeforeDrawObservable.remove(particleObserver);
    }
  });
  expect(result.activeDecals).toBe(1);
  expect(result.decalDraws).toBeGreaterThan(0);
  expect(result.particleDraws).toBeGreaterThan(0);
  expect(errors).toEqual([]);
}

for (const renderer of ['webgl2', 'webgpu'] as const) {
  test(`${renderer} actor entry renders particle and decal batches`, async ({ baseURL, launchOptions }) => {
    // A fresh browser keeps SwiftShader device state local to this test.
    const args = renderer === 'webgl2'
      ? ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-vulkan']
      : launchOptions.args;
    const browser = await chromium.launch({ ...launchOptions, args });
    try {
      const page = await browser.newPage({ baseURL, viewport: { width: 480, height: 270 } });
      await verifyBatches(page, renderer);
    } finally {
      await browser.close();
    }
  });
}
