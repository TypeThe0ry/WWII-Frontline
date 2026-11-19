import { expect, test, type Page } from '@playwright/test';

const mapId = '0123456789abcdef0123456789abcdef';
const server = { match_id: mapId, name: 'Test match', map: 'm0', mode: 'sandbox', phase: 'live', time_left_s: 500, humans: 7, bots: 2, cap: 100, region: 'local', tags: [], version: 'dev', compatible: true };

async function gpu(page: Page, backend: 'webgpu' | 'webgl2' | 'none') {
  await page.addInitScript((kind) => {
    Object.defineProperty(navigator, 'gpu', { value: kind === 'webgpu' ? { requestAdapter: async () => ({ info: {} }) } : undefined });
    if (kind === 'none') {
      const orig = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement, type: string, ...args: unknown[]) {
        return type === 'webgl2' ? null : Reflect.apply(orig, this, [type, ...args]);
      } as typeof orig;
    }
  }, backend);
}
async function enter(page: Page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Click to continue' }).click();
}

test('WebGPU capability path gates the menu on a gesture', async ({ page }) => {
  await gpu(page, 'webgpu');
  await page.route('**/v1/servers', (route) => route.fulfill({ json: [server] }));
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Server browser' })).toHaveCount(0);
  await expect(page.getByText('Compatibility mode (WebGL2)')).toHaveCount(0);
  await page.getByRole('button', { name: 'Click to continue' }).click();
  await expect(page.getByRole('button', { name: 'Quick join' })).toBeEnabled();
});

test('unsupported browser has diagnostics but no continue path', async ({ page }) => {
  await gpu(page, 'none');
  await page.goto('/');
  await expect(page.getByRole('alert')).toContainText('does not support WebGPU or WebGL2');
  await expect(page.getByRole('button', { name: 'Copy diagnostics' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Click to continue' })).toHaveCount(0);
});

test('WebGL2 fallback, server refresh, filters persist across reload and join errors are recoverable', async ({ page }) => {
  await gpu(page, 'webgl2');
  const incompatible = { ...server, match_id: '1123456789abcdef0123456789abcdef', name: 'Old match', compatible: false };
  let requests = 0;
  await page.route('**/v1/servers', (route) => route.fulfill({ json: [server, { ...incompatible, humans: ++requests }] }));
  await page.route('**/v1/guest', (route) => route.fulfill({ json: { token: 'fixture-guest', nick_suggestion: 'Guest-fixture' } }));
  let tickets = 0;
  await page.route('**/v1/servers/*/ticket', (route) => {
    tickets++;
    expect(route.request().headers()['authorization']).toBe('Bearer fixture-guest');
    expect(route.request().postDataJSON()).toEqual({ nick: 'Guest-fixture' });
    return route.fulfill({ status: 409, json: { error: 'match_full' } });
  });
  await page.clock.install();
  await page.goto('/');
  await expect(page.getByText('Compatibility mode (WebGL2)')).toBeVisible();
  await expect(page.getByRole('listitem')).toHaveCount(4);
  await page.getByRole('button', { name: 'Click to continue' }).click();
  await page.getByRole('button', { name: 'Server browser' }).click();
  await expect(page.getByRole('row', { name: /Old match/ }).getByRole('button')).toBeDisabled();
  await expect(page.getByRole('row', { name: /Test match/ })).toContainText('7+2/100');
  await page.clock.fastForward(10_100);
  await expect.poll(() => requests).toBeGreaterThanOrEqual(2);
  await page.getByRole('textbox', { name: 'Server name contains' }).fill('Test');
  await expect(page.getByRole('row', { name: /Old match/ })).toHaveCount(0);
  await enter(page);
  await page.getByRole('button', { name: 'Server browser' }).click();
  await expect(page.getByRole('textbox', { name: 'Server name contains' })).toHaveValue('Test');
  await page.getByRole('row', { name: /Test match/ }).getByRole('button', { name: 'Join' }).click();
  await expect(page.getByText(/server assigns your team/)).toBeVisible();
  await page.getByRole('button', { name: 'Join with automatic assignment' }).click();
  await expect(page.getByRole('alert')).toContainText('Server is full');
  await expect(page.getByRole('button', { name: 'Join with automatic assignment' })).toBeEnabled();
  expect(tickets).toBe(1);
});

test('real local gateway issues a ticket, the match accepts it, map leaves C2 flowing, and leave tears down', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));
  test.skip(process.env.FRONTLINE_UI_LIVE !== '1', 'requires a local gateway and registered m0 match; see DEVELOPING.md');
  await gpu(page, 'webgl2');
  await enter(page);
  await expect(page.getByRole('button', { name: 'Quick join' })).toBeEnabled();
  await page.getByRole('button', { name: 'Quick join' }).click();
  const ticketResponse = page.waitForResponse((response) => /\/v1\/servers\/[^/]+\/ticket$/.test(response.url()));
  await page.getByRole('button', { name: 'Join with automatic assignment' }).click();
  expect((await ticketResponse).ok()).toBe(true);
  await expect(page.getByLabel('Combat HUD')).toBeVisible({ timeout: 25_000 });
  await page.getByRole('button', { name: 'Tactical map' }).click();
  await expect(page.getByRole('region', { name: 'Tactical map' })).toBeVisible();
  const before = await page.evaluate(() => window.__netGame!.game.stats.snapshots);
  await expect.poll(() => page.evaluate(() => window.__netGame!.game.stats.snapshots)).toBeGreaterThan(before + 2);
  const after = await page.evaluate(() => window.__netGame!.game.stats.snapshots);
  console.log(`[local M0] C2 snapshots while the map was open: ${before} → ${after}`);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('region', { name: 'Tactical map' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Pause', exact: true }).click();
  await page.getByRole('button', { name: 'Leave match', exact: true }).click();
  await page.getByRole('button', { name: 'Confirm', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Server browser' })).toBeVisible();
  expect(await page.evaluate(() => window.__netGame)).toBeUndefined();
  expect(errors).toEqual([]);
});
