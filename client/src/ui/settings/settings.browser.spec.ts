import { expect, test, type Page } from '@playwright/test';
import type { SettingsStore } from '../../game/settings';
import type { InputSampler } from '../../input/sampler';
import type { PauseSettingsHandle } from './index';

interface Harness {
  sampler: InputSampler;
  settings: SettingsStore;
  handle: PauseSettingsHandle;
  leaves: number;
  redeploys: number;
  cooldown: number;
  mapOpen: boolean;
  mapPresses: number;
  closeMapCalls: number;
  requestedAtGesture: boolean[];
  pauseMs: number;
}
declare global { interface Window { pauseHarness: Harness } }

async function mount(page: Page, { refuseOnce = false, noStorage = false, toggleThroughAction = false } = {}) {
  await page.evaluate(async ({ refuseOnce, noStorage, toggleThroughAction }) => {
    const load = (path: string) => import(/* @vite-ignore */ path);
    const { mountPauseSettings } = await load('/src/ui/settings/index.ts') as typeof import('./index');
    const { SettingsStore } = await load('/src/game/settings.ts') as typeof import('../../game/settings');
    const { Bindings, Persist } = await load('/src/input/bindings.ts') as typeof import('../../input/bindings');
    const { InputSampler } = await load('/src/input/sampler.ts') as typeof import('../../input/sampler');
    const { PointerLockController } = await load('/src/input/pointer-lock.ts') as typeof import('../../input/pointer-lock');
    const canvas = document.querySelector('canvas')!;
    const nativeRequest = canvas.requestPointerLock.bind(canvas);
    const requestedAtGesture: boolean[] = [];
    canvas.requestPointerLock = (...args) => {
      requestedAtGesture.push(navigator.userActivation.isActive);
      if (refuseOnce) {
        refuseOnce = false;
        // Legacy void-returning refusal path.
        queueMicrotask(() => document.dispatchEvent(new Event('pointerlockerror')));
        return undefined as unknown as Promise<void>;
      }
      return nativeRequest(...args);
    };
    const persist = new Persist(() => { if (noStorage) throw new Error('denied'); return localStorage; });
    const settings = new SettingsStore(persist);
    const sampler = new InputSampler();
    const legacy = new PointerLockController(sampler, canvas, document, window);
    legacy.attach();
    let unlockStarted = 0;
    const recordUnlock = () => { if (!document.pointerLockElement) unlockStarted = performance.now(); };
    document.addEventListener('pointerlockchange', recordUnlock);
    const handle = mountPauseSettings({ root: document.querySelector('#ui')!, canvas, settings, bindings: new Bindings(), persist, sampler,
      onLeave: () => { window.pauseHarness.leaves++; legacy.detach(); },
      onRedeploy: () => { window.pauseHarness.redeploys++; },
      onTeamChange: () => {}, teamCooldownSeconds: () => window.pauseHarness?.cooldown ?? 0,
      isMapOpen: () => window.pauseHarness.mapOpen,
      onCloseMap: () => { window.pauseHarness.mapOpen = false; window.pauseHarness.closeMapCalls++; },
      onAction: toggleThroughAction ? (action, down) => {
        if (action !== 'map' || !down) return;
        const h = window.pauseHarness;
        h.mapPresses++;
        h.mapOpen = !h.mapOpen;
        if (h.mapOpen) handle.suspendForMap();
      } : undefined,
    });
    window.pauseHarness = { sampler, settings, handle, leaves: 0, redeploys: 0, cooldown: 0, mapOpen: false, mapPresses: 0, closeMapCalls: 0, requestedAtGesture, pauseMs: -1 };
    document.addEventListener('pointerlockchange', () => {
      if (!document.pointerLockElement && document.querySelector('[data-view="pause"]')) {
        window.pauseHarness.pauseMs = performance.now() - unlockStarted;
      }
    });
  }, { refuseOnce, noStorage, toggleThroughAction });
}

const fixture = '/src/ui/settings/browser-fixture.html';
const lock = (page: Page) => expect.poll(() => page.evaluate(() => document.pointerLockElement?.localName)).toBe('canvas');
const input = (page: Page) => page.evaluate(() => window.pauseHarness.sampler.build(1, 1, 1));

test('real pointer lock, legacy-safe rebinding, reload persistence and confirmation teardown', async ({ page }) => {
  await page.goto(fixture);
  await mount(page);
  await page.locator('canvas').click();
  await lock(page);
  await page.keyboard.down('KeyW');
  expect((await input(page)).moveZ).toBe(127);
  await page.evaluate(() => document.exitPointerLock()); // same real event as browser Esc, without synthesizing a lock-state change
  await expect(page.getByText(/Match in progress/)).toBeVisible();
  expect(await page.evaluate(() => window.pauseHarness.pauseMs)).toBeGreaterThanOrEqual(0);
  expect(await page.evaluate(() => window.pauseHarness.pauseMs)).toBeLessThan(100);
  expect((await input(page)).moveZ).toBe(0);
  await page.keyboard.up('KeyW');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  // Time the DOM construction synchronously in the page, without including Playwright's click transport.
  const metrics = await page.evaluate(() => {
    const panel = document.querySelector('[data-pause-settings]')!;
    (panel.querySelector('button') as HTMLButtonElement).click(); // Back
    const settings = [...panel.querySelectorAll('button')].find((b) => b.textContent === 'Settings')!;
    const start = performance.now();
    settings.click();
    return { ms: performance.now() - start, nodes: panel.querySelectorAll('*').length };
  });
  expect(metrics.nodes).toBeLessThanOrEqual(1500);
  expect(metrics.ms).toBeLessThan(100);
  await page.getByRole('button', { name: 'Controls', exact: true }).click();
  const reload = page.locator('[data-binding="reload.primary"]');
  await reload.click();
  await page.keyboard.press('KeyT');
  await expect(reload).toHaveText('KeyT');
  await page.getByRole('button', { name: 'Graphics', exact: true }).click();
  const fov = page.getByRole('slider', { name: 'Field of view' });
  await expect(fov).toHaveAttribute('min', '60');
  await expect(fov).toHaveAttribute('max', '100');
  await page.getByLabel(/Quality preset/).selectOption('high');
  expect(await page.evaluate(() => window.pauseHarness.settings.get('quality.tier'))).toBe('high');
  await page.evaluate(() => window.pauseHarness.handle.dispose());
  await page.reload();
  await mount(page);
  await page.locator('canvas').click();
  await lock(page);
  await page.keyboard.down('KeyR');
  expect(await page.evaluate(() => window.pauseHarness.sampler.isHeld('reload'))).toBe(false);
  await page.keyboard.up('KeyR');
  await page.keyboard.down('KeyT');
  expect(await page.evaluate(() => window.pauseHarness.sampler.isHeld('reload'))).toBe(true);
  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  await expect(page.getByRole('dialog')).toBeVisible();
  expect(await page.evaluate(() => window.pauseHarness.sampler.isHeld('reload'))).toBe(false);
  await page.keyboard.up('KeyT');
  await expect.poll(() => page.evaluate(() => document.pointerLockElement)).toBe(null);
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await lock(page);
  expect(await page.evaluate(() => window.pauseHarness.requestedAtGesture)).toEqual([true, true]);
  await page.evaluate(() => document.exitPointerLock());
  await page.getByRole('button', { name: 'Leave match' }).click();
  await expect(page.getByText('Leave the match?')).toBeVisible();
  expect(await page.evaluate(() => window.pauseHarness.leaves)).toBe(0);
  await page.getByRole('button', { name: 'Confirm', exact: true }).click();
  await expect(page.locator('[data-pause-settings]')).toHaveCount(0);
  expect(await page.evaluate(() => window.pauseHarness.leaves)).toBe(1);
});

test('refusal/retry, map-first, reserved key, conflict resolutions and in-memory fallback', async ({ page }) => {
  await page.goto(fixture);
  await mount(page, { refuseOnce: true, noStorage: true });
  await page.locator('canvas').click();
  await expect(page.getByRole('button', { name: 'Click to continue' })).toBeVisible();
  await page.getByRole('button', { name: 'Click to continue' }).click();
  await lock(page);
  await page.evaluate(() => { window.pauseHarness.mapOpen = true; document.exitPointerLock(); });
  await expect(page.getByRole('button', { name: 'Click to continue' })).toBeVisible();
  expect(await page.evaluate(() => window.pauseHarness.mapOpen)).toBe(false);
  await page.evaluate(() => window.pauseHarness.handle.open());
  await expect(page.getByText(/Local storage is unavailable/)).toHaveCount(1);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Controls', exact: true }).click();
  const fire = page.locator('[data-binding="fire.primary"]');
  await fire.click();
  await page.keyboard.press('F11');
  await expect(page.getByText(/reserved by the browser/)).toBeVisible();
  await expect(fire).toHaveText('Mouse0');
  await fire.click();
  await page.keyboard.press('Escape');
  await expect(fire).toBeVisible();
  await fire.click();
  await page.keyboard.press('Space');
  await expect(fire).toHaveAttribute('aria-invalid', 'true');
  await expect(page.getByText(/Already used by another action/)).toBeVisible();
  await page.getByRole('button', { name: 'Swap', exact: true }).click();
  await expect(fire).toHaveText('Space');
  await expect(page.locator('[data-binding="jump.primary"]')).toHaveText('Mouse0');
  await page.getByLabel('Binding scheme').selectOption('2');
  await page.getByRole('button', { name: 'Save scheme' }).click();
  await fire.click();
  await page.keyboard.press('KeyT');
  await expect(fire).toHaveText('KeyT');
  await page.getByRole('button', { name: 'Load scheme' }).click();
  await expect(fire).toHaveText('Space');
  await page.getByRole('button', { name: 'Export JSON' }).click();
  const json = page.getByLabel('Binding JSON');
  await expect(json).toHaveValue(/"fire":\{"primary":"Space"\}/);
  await json.fill('{bad');
  await page.getByRole('button', { name: 'Import JSON' }).click();
  await expect(page.getByText(/Invalid or conflicting/)).toBeVisible();
  await expect(fire).toHaveText('Space');
});


test('a bound map press deliberately unlocks without closing the map; Escape or the binding closes it', async ({ page }) => {
  await page.goto(fixture);
  await mount(page, { toggleThroughAction: true });
  await page.locator('canvas').click();
  await lock(page);
  await page.keyboard.down('KeyW');
  expect((await input(page)).moveZ).toBe(127);
  await page.keyboard.press('KeyM');
  await expect.poll(() => page.evaluate(() => document.pointerLockElement)).toBe(null);
  await expect(page.getByRole('dialog')).toBeHidden();
  expect(await page.evaluate(() => {
    const { mapOpen, mapPresses, closeMapCalls } = window.pauseHarness;
    return { mapOpen, mapPresses, closeMapCalls };
  })).toEqual({ mapOpen: true, mapPresses: 1, closeMapCalls: 0 });
  expect((await input(page)).moveZ).toBe(0);
  await page.keyboard.up('KeyW');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Click to continue' })).toBeVisible();
  expect(await page.evaluate(() => window.pauseHarness.closeMapCalls)).toBe(1);
  await page.getByRole('button', { name: 'Click to continue' }).click();
  await lock(page);
  await page.keyboard.press('KeyM');
  await expect.poll(() => page.evaluate(() => document.pointerLockElement)).toBe(null);
  await page.keyboard.press('KeyM');
  await expect(page.getByRole('button', { name: 'Click to continue' })).toBeVisible();
  expect(await page.evaluate(() => window.pauseHarness.mapPresses)).toBe(3);
  expect(await page.evaluate(() => window.pauseHarness.mapOpen)).toBe(false);
});

test('disposing during the lock request does not leave the canvas locked', async ({ page }) => {
  await page.goto(fixture);
  await mount(page);
  await page.evaluate(() => {
    const canvas = document.querySelector('canvas')!;
    const nativeRequest = canvas.requestPointerLock;
    canvas.requestPointerLock = function (...args) {
      const result = nativeRequest.apply(this, args);
      // Observe the real browser promise separately to know that the pending request settled.
      void result?.then(() => { canvas.dataset.requestDone = 'granted'; }, () => { canvas.dataset.requestDone = 'refused'; });
      return result;
    };
    canvas.addEventListener('click', (e) => e.stopImmediatePropagation(), { once: true, capture: true });
    canvas.addEventListener('pointerdown', () => {
      window.pauseHarness.handle.resume();
      window.pauseHarness.handle.dispose();
    }, { once: true });
  });
  await page.locator('canvas').click();
  await expect(page.locator('canvas')).toHaveAttribute('data-request-done', 'granted');
  await expect.poll(() => page.evaluate(() => document.pointerLockElement)).toBe(null);
  await expect(page.getByRole('dialog')).toHaveCount(0);
});
