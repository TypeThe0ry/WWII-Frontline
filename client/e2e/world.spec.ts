import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';

test.use({ launchOptions: { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-vulkan'] } });

const map = new URL('../../data/maps/carentan/', import.meta.url);
const previewFiles = new Set(['meta.json', 'heightmap_1m.r16', 'heightmap_far_8m.r16',
  'hedgerows.geojson', 'buildings.geojson', 'building-interiors.json', 'roads.geojson', 'water.geojson', 'forests.geojson']);

async function serveCarentan(page: Page): Promise<void> {
  await page.route('**/maps/carentan/*', async (route) => {
    const name = new URL(route.request().url()).pathname.split('/').pop() ?? '';
    if (previewFiles.has(name)) await route.fulfill({ path: fileURLToPath(new URL(name, map)) });
    else await route.continue();
  });
}

test('Carentan world registers the custom CSM and velocity shaders and updates resident hedge meshes', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await serveCarentan(page);
  await page.goto('/?scene=world&renderer=webgl2&cam=hedgerow&veg=0&csm=custom&post=high&taa_vel=1&post_guard=0');
  await expect.poll(() => page.evaluate(() => window.__post?.velocity?.drawn ?? 0), { timeout: 30_000 }).toBeGreaterThan(0);
  await expect.poll(() => page.evaluate(() => window.__world?.stream?.stream_queue_len ?? -1), { timeout: 20_000 }).toBe(0);
  const initial = await page.evaluate(() => ({
    hedgeMeshes: window.__world!.stats.hedge_meshes,
    segments: window.__world!.stats.hedge_segments,
    shaders: window.__world!.variants.stats().custom,
    holes: window.__world!.stream!.stream_holes,
  }));
  console.log('Carentan functional counters', initial);
  expect(initial.segments).toBeGreaterThan(0);
  expect(initial.hedgeMeshes).toBeGreaterThan(0);
  expect(initial.holes).toBe(0);
  expect(initial.shaders).toEqual(['csm:instanced', 'csm:plain', 'velocity:instanced', 'velocity:plain', 'velocity:terrain']);
  await page.evaluate(() => window.__terrain!.setPose([0, 120, 0], [10, 110, 10]));
  await expect.poll(() => page.evaluate(() => window.__world!.stats.hedge_meshes)).not.toBe(initial.hedgeMeshes);
  expect(errors).toEqual([]);
});

test('Carentan door admits the WASM KCC and camera; jamb and lintel remain occluders', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await serveCarentan(page);
  await page.goto('/?scene=world&renderer=webgl2&veg=0&splat=0');
  await expect.poll(() => page.evaluate(() => window.__world?.stats.interiors)).toBe(70);
  const layout = await page.evaluate(() => {
    const w = window.__world!;
    const b = w.buildings.find((room) => room.id === 'way/154019580')!;
    const [x, z] = b.door.center;
    const [nx, nz] = b.door.outward;
    const ray = (h: number, side: number) => w.buildingOcclusion(x + nx * 1.2 - nz * side, b.base_y + h, z + nz * 1.2 + nx * side, -nx, 0, -nz, 1.7);
    const roof = b.probes.at(-1)!;
    return {
      pick: [ray(1.2, 0), ray(1.2, 0.95), ray(2.45, 0), w.buildingOcclusion(...roof, 0, 1, 0, 3)],
      probes: w.stats.interior_probes, door: b.door, base_y: b.base_y, id: b.id,
    };
  });
  expect(layout.pick[0]).toBeNull();
  expect(layout.pick[1]).toBeCloseTo(1.05, 2);
  expect(layout.pick[2]).toBeCloseTo(1.05, 2);
  expect(layout.pick[3]).toBeCloseTo(1.8, 2);
  expect(layout.probes).toBe(146);
  const result = await page.evaluate(async ({ modulePath, b }) => {
    const fixture = await import(modulePath) as typeof import('./building-walk');
    return fixture.walkDoor(b);
  }, { modulePath: '/e2e/building-walk.ts', b: layout });
  expect(result.entered.distance).toBeLessThan(-0.8);
  expect(result.blocked.distance).toBeGreaterThan(0.25);
  expect(result.exited.distance).toBeGreaterThan(0.8);
  for (const [sample, id] of [[result.entered, layout.id], [result.exited, null]] as const) {
    await page.evaluate((eye) => window.__terrain!.setPose(eye, [eye[0], eye[1], eye[2] + 1]), sample.eye);
    await expect.poll(() => page.evaluate(() => window.__world!.indoorBuilding)).toBe(id);
  }
  const near = await page.evaluate(() => window.__world!.buildingMeshes.filter((m) => m.name.includes('_interior') && m.isVisible).length);
  expect(near).toBeGreaterThan(0);
  await page.evaluate(() => window.__terrain!.setPose([0, 100, 0], [10, 100, 10]));
  await expect.poll(() => page.evaluate(() => window.__world!.buildingMeshes.filter((m) => m.name.includes('_interior') && m.isVisible).length)).toBe(0);
  const culledFloor = await page.evaluate((id) => {
    const world = window.__world!;
    const [x, y, z] = world.buildings.find((b) => b.id === id)!.probes[0]!;
    return world.buildingOcclusion(x, y, z, 0, -1, 0, 2);
  }, layout.id);
  expect(culledFloor).toBeCloseTo(1.2, 2);
  console.log('Carentan doorway functional offsets (outside positive)', {
    enter: result.entered.distance, exit: result.exited.distance, blocked: result.blocked.distance,
  });
  expect(errors).toEqual([]);
});

test('Low Carentan renders at 75% of the world canvas size', async ({ page }) => {
  await serveCarentan(page);
  // Disable expensive vegetation generation here; the vegetation-scene tests cover Low selection.
  for (const [tier, size] of [['high', [960, 540]], ['low', [720, 405]]] as const) {
    await page.goto(`/?scene=world&renderer=webgl2&cam=open_field&veg=0&splat=0&debug=1&quality=${tier}`);
    await expect.poll(() => page.evaluate(() => window.__perf?.frames() ?? 0)).toBeGreaterThan(0);
    const rendered = await page.locator('#game-canvas').evaluate((canvas: HTMLCanvasElement) => ({
      pixels: [canvas.width, canvas.height],
      css: [canvas.clientWidth, canvas.clientHeight],
    }));
    expect(rendered.css).toEqual([960, 540]);
    expect(rendered.pixels).toEqual(size);
  }
});
