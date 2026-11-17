import { mkdirSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';

// M0-025 grey-box actors, first-person weapon and shot feedback in the real app (software-rendered WebGL2 Chromium).
// `?actors=demo` feeds the actor path from a synthetic game state (no server). GPU frame times cannot be measured here (H-021).
const SHOT_DIR = '../docs/measurements/M0-025';
const SAVE_SHOTS = process.env.SAVE_SHOTS === '1';
test.use({ viewport: { width: 480, height: 270 } });

function watchErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`console.error: ${m.text()}`);
  });
  return errors;
}

async function open(page: Page, n: number, extra = ''): Promise<void> {
  await page.goto(`/?renderer=webgl2&scene=vegetation&actors=demo&entities=${n}${extra}`);
  await page.waitForFunction(() => window.__actors !== undefined && window.__vegetation !== undefined, undefined, { timeout: 120_000 });
  await frames(page, 4);
}

async function frames(page: Page, n: number): Promise<void> {
  const f0 = await page.evaluate(() => window.__perf?.frames() ?? 0);
  await page.waitForFunction((k) => (window.__perf?.frames() ?? 0) >= k, f0 + n, { timeout: 90_000 });
}

test('demo actors render: placeholder report, screenshot, no console errors', async ({ page }) => {
  const errors = watchErrors(page);
  await open(page, 10);
  const rep = await page.evaluate(() => window.__placeholders);
  expect(rep!.length).toBeGreaterThanOrEqual(6);
  for (const id of ['V-CHR-US-001', 'V-CHR-GE-001', 'V-WPN-US-001', 'V-WPN-GE-001', 'V-FX-001', 'V-FX-006']) expect(rep!.map((r) => r.id)).toContain(id);
  expect(rep!.every((r) => r.placeholder)).toBe(true);
  expect(rep!.find((r) => r.id === 'V-CHR-US-001')!.instances + rep!.find((r) => r.id === 'V-CHR-GE-001')!.instances).toBe(10);
  // every placeholder object in the live scene follows the naming rule
  const bad = await page.evaluate(() => {
    const re = /^PH_V-[A-Z]+-[A-Z0-9-]+(__.+)?$/;
    const names = (window as unknown as { __actorsNames?: () => string[] }).__actorsNames?.() ?? [];
    return names.filter((n) => n.startsWith('PH_V') && !re.test(n));
  });
  expect(bad).toEqual([]);
  const shot = await page.screenshot();
  if (SAVE_SHOTS) {
    mkdirSync(SHOT_DIR, { recursive: true });
    await page.screenshot({ path: `${SHOT_DIR}/demo-10.png` });
  }
  expect(shot.length).toBeGreaterThan(2000);
  // the picture has variation (not a flat colour)
  const variance = await page.evaluate(async (b64) => {
    const img = new Image();
    img.src = `data:image/png;base64,${b64}`;
    await img.decode();
    const t = document.createElement('canvas');
    t.width = 48;
    t.height = 27;
    const g = t.getContext('2d')!;
    g.drawImage(img, 0, 0, 48, 27);
    const d = g.getImageData(0, 0, 48, 27).data;
    const vals = new Set<number>();
    for (let i = 0; i < d.length; i += 4) vals.add((d[i]! >> 3) * 1024 + (d[i + 1]! >> 3) * 32 + (d[i + 2]! >> 3));
    return vals.size;
  }, shot.toString('base64'));
  expect(variance).toBeGreaterThan(20);
  expect(errors).toEqual([]);
});

test('acceptance 1: 30 consecutive local shots, fire event frame == muzzle flash visible frame', async ({ page }) => {
  const errors = watchErrors(page);
  await open(page, 10);
  const r = await page.evaluate(async () => {
    const a = window.__actors!;
    for (let i = 0; i < 30; i++) {
      a.bus.emit('fire.local', { weapon: 0, muzzlePos: { x: 128, y: 5, z: 112 }, dir: { x: 0, y: 0, z: 1 } });
      await new Promise<void>((res) => requestAnimationFrame(() => requestAnimationFrame(() => res())));
    }
    return { fire: window.__perf!.fireEventFrame!.slice(-30), flash: window.__perf!.muzzleFlashVisibleFrame!.slice(-30) };
  });
  expect(r.fire.length).toBe(30);
  expect(r.flash.length).toBe(30);
  for (let i = 0; i < 30; i++) expect(r.flash[i]! - r.fire[i]!, `shot ${i}`).toBe(0);
  expect(new Set(r.fire).size).toBe(30);
  expect(errors).toEqual([]);
});

test('acceptance 7: hit.confirm -> hit marker visible within one frame, gone after ~120 ms; damage bar on the right edge', async ({ page }) => {
  await open(page, 3);
  const r = await page.evaluate(async () => {
    const a = window.__actors!;
    const m = document.getElementById('hit-marker')!;
    const before = getComputedStyle(m).opacity;
    a.bus.emit('hit.confirm', { target: 1, zone: 0, damage: 0.5, killed: false });
    const sync = getComputedStyle(m).opacity; // right after the emit, before any frame
    await new Promise<void>((res) => requestAnimationFrame(() => res()));
    const afterOneFrame = getComputedStyle(m).opacity;
    await new Promise((res) => setTimeout(res, 250));
    const later = getComputedStyle(m).opacity;
    a.bus.emit('damage.taken', { angle: Math.PI / 2, amount: 0.5 });
    const bar = getComputedStyle(document.getElementById('damage-bar-1')!).opacity;
    return { before, sync, afterOneFrame, later, bar };
  });
  expect(r.before).toBe('0');
  expect(r.sync).toBe('1');
  expect(r.afterOneFrame).toBe('1');
  expect(r.later).toBe('0');
  expect(r.bar).toBe('1');
});

test('draw calls and triangles of the actors (instanced): 0 / 10 / 100 soldiers', async ({ page }) => {
  const rows: { n: number; draw: number; tris: number; active: number }[] = [];
  for (const n of [0, 10, 100]) {
    await open(page, n);
    await frames(page, 6);
    const d = await page.evaluate(() => window.__perf!.dump());
    rows.push({ n, draw: d.draw_calls, tris: d.triangles, active: d.active_meshes });
  }
  const base = rows[0]!;
  const out = rows.map((r) => `${r.n} soldiers: draw calls ${r.draw} (+${r.draw - base.draw}), triangles ${r.tris} (+${r.tris - base.tris}), active meshes ${r.active} (+${r.active - base.active})`);
  console.log(`[actors cost vs 0 soldiers (first-person weapon only)]\n${out.join('\n')}`);
  expect(rows[1]!.draw - base.draw).toBeLessThanOrEqual(40);
  expect(rows[2]!.draw - base.draw).toBeLessThanOrEqual(40);
  expect(rows[1]!.tris).toBeGreaterThan(base.tris);
  expect(rows[2]!.tris).toBeGreaterThan(rows[1]!.tris);
});

test('ADS view and a muzzle flash render without errors (screenshots with SAVE_SHOTS=1)', async ({ page }) => {
  const errors = watchErrors(page);
  await open(page, 10, '&ads=1');
  await page.waitForTimeout(1200); // > ads_ms of simulated render time at software-rendering frame rates
  await page.evaluate(() => window.__actors!.bus.emit('fire.local', { weapon: 0, muzzlePos: { x: 128, y: 5, z: 112 }, dir: { x: 0, y: 0, z: 1 } }));
  const shot = await page.screenshot();
  expect(shot.length).toBeGreaterThan(2000);
  if (SAVE_SHOTS) {
    mkdirSync(SHOT_DIR, { recursive: true });
    await page.screenshot({ path: `${SHOT_DIR}/ads-flash.png` });
  }
  expect(errors).toEqual([]);
});
