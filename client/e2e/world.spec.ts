import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';

test.use({ launchOptions: { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-vulkan'] } });

const map = new URL('../../data/maps/carentan/', import.meta.url);
const previewFiles = new Set(['meta.json', 'heightmap_1m.r16', 'heightmap_far_8m.r16',
  'hedgerows.geojson', 'buildings.geojson', 'roads.geojson', 'water.geojson', 'forests.geojson']);

test('Carentan world registers the custom CSM and velocity shaders and updates resident hedge meshes', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.route('**/maps/carentan/*', async (route) => {
    const name = new URL(route.request().url()).pathname.split('/').pop() ?? '';
    if (previewFiles.has(name)) await route.fulfill({ path: fileURLToPath(new URL(name, map)) });
    else await route.continue();
  });
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
