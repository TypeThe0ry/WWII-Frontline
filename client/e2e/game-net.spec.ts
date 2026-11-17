import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';
import { createServer, type ViteDevServer } from 'vite';
import { HAVE_BINARIES, removeDir, REPO_ROOT, type ServerProc, SKIP_REASON, startServer } from './net-support';

// M0-022: the real client (wasm loaded in the browser, collision chunks fetched, session over WebSocket, GameLoop driven by
// the render loop's fixed stepper) against a real frontline-server. What the snapshots contain depends on the server
// build of the day (M0-018/019): this spec only asserts what holds either way (connection, stepping, input upstream);
// the reconciliation numbers come from the in-process tests and, with a complete server, from M0-025 / M0-027.
const required = process.env.FRONTLINE_REQUIRE_SERVER === '1';
test.skip(!HAVE_BINARIES && !required, SKIP_REASON);
test.beforeAll(() => {
  if (!HAVE_BINARIES && required) throw new Error(SKIP_REASON);
});

const CLIENT_DIR = fileURLToPath(new URL('../', import.meta.url));
let vite: ViteDevServer | undefined;
let base = '';
let server: ServerProc | undefined;
let cacheDir = '';

test.beforeAll(async () => {
  if (!HAVE_BINARIES) return;
  cacheDir = mkdtempSync(join(tmpdir(), 'fl-game-vite-'));
  // sync-map output (client/public/{maps,wasm}) must exist: `pnpm -C client sync-map`
  vite = await createServer({
    root: CLIENT_DIR,
    configFile: false,
    logLevel: 'error',
    cacheDir,
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, strictPort: false, fs: { allow: [REPO_ROOT] }, hmr: false, watch: null },
  });
  await vite.listen();
  const addr = vite.httpServer?.address();
  if (!addr || typeof addr === 'string') throw new Error('vite has no address');
  base = `http://localhost:${addr.port}/`;
  server = await startServer({});
});
test.afterAll(async () => {
  await server?.stop();
  await vite?.close();
  if (cacheDir) removeDir(cacheDir);
});

test('networked game boots, steps at 30 Hz, sends InputBundles and survives whatever the server answers', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`console.error: ${m.text()}`);
  });
  await page.goto(`${base}?renderer=webgl2&empty=1&debug=1&transport=ws&server=127.0.0.1:${(server as ServerProc).wsPort}`);
  await page.waitForFunction(() => window.__netGame !== undefined, undefined, { timeout: 30_000 });
  // hold W through the sampler (the keyboard path needs Pointer Lock, which headless Chromium does not grant)
  await page.evaluate(() => window.__netGame?.sampler.press('KeyW'));
  const t0 = Date.now();
  await page.waitForFunction(() => (window.__netGame?.game.stats.steps ?? 0) >= 90, undefined, { timeout: 30_000 });
  const seconds = (Date.now() - t0) / 1000;
  const r = await page.evaluate(() => {
    const g = window.__netGame!.game;
    return { steps: g.stats.steps, bundles: g.stats.bundles, avgBundle: g.stats.bundleBytes / Math.max(1, g.stats.bundles), snapshots: g.stats.snapshots, decodeErrors: g.stats.decodeErrors, ring: g.ring.length, kind: window.__netGame!.session.kind, overlay: document.getElementById('debug-overlay')?.textContent ?? '' };
  });
  console.log(`[game-net] ${JSON.stringify({ ...r, overlay: undefined, waitedS: +seconds.toFixed(2), echo: (server as ServerProc).echo })}`);
  expect(r.kind).toBe('ws');
  expect(r.bundles).toBeGreaterThanOrEqual(r.steps * 0.9);
  expect(r.avgBundle).toBeGreaterThan(10);
  expect(r.avgBundle).toBeLessThanOrEqual(32);
  expect(r.overlay).toContain('reconcile corrections');
  // no crash on whatever came back: decode failures are counted, never thrown
  const ours = errors.filter((e) => !/WebGPU|GPU stall|swiftshader/i.test(e));
  expect(ours).toEqual([]);
});
