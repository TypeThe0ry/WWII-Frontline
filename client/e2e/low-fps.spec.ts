import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';
import { createServer, type ViteDevServer } from 'vite';
import { HAVE_BINARIES, removeDir, REPO_ROOT, type ServerProc, SKIP_REASON, startServer } from './net-support';

// M0-030: a client that renders at <= 5 fps (software rendering of the vegetation scene gave ~3 fps on the demo machine) must
// keep its connection. The page's main thread is blocked by a busy loop in requestAnimationFrame (about 280 ms per frame, so
// Pongs and snapshots are processed late exactly as on a slow machine) while W is held, against a real frontline-server.
// The server must not log `session_kicked` and must keep accepting the client's inputs. LOWFPS_SECONDS overrides the 60 s.
const required = process.env.FRONTLINE_REQUIRE_SERVER === '1';
test.skip(!HAVE_BINARIES && !required, SKIP_REASON);
test.beforeAll(() => {
  if (!HAVE_BINARIES && required) throw new Error(SKIP_REASON);
});

const SECONDS = Number(process.env.LOWFPS_SECONDS ?? 60);
const BLOCK_MS = Number(process.env.LOWFPS_BLOCK_MS ?? 280);
const CLIENT_DIR = fileURLToPath(new URL('../', import.meta.url));
let vite: ViteDevServer | undefined;
let base = '';
let server: ServerProc | undefined;
let cacheDir = '';

test.beforeAll(async () => {
  if (!HAVE_BINARIES) return;
  cacheDir = mkdtempSync(join(tmpdir(), 'fl-lowfps-vite-'));
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

test(`a client at <= 5 fps is not kicked for ${SECONDS} s`, async ({ page }) => {
  test.setTimeout((SECONDS + 60) * 1000);
  const srv = server as ServerProc;
  const scene = process.env.LOWFPS_SCENE ? `scene=${process.env.LOWFPS_SCENE}` : 'empty=1';
  await page.goto(`${base}?renderer=webgl2&${scene}&transport=ws&server=127.0.0.1:${srv.wsPort}`);
  await page.waitForFunction(() => window.__netGame !== undefined, undefined, { timeout: 30_000 });
  await page.evaluate((blockMs) => {
    window.__netGame?.sampler.press('KeyW');
    const w = window as unknown as { __disconnect?: string; __frames: number };
    w.__frames = 0;
    window.__netGame?.session.onDisconnect((r) => (w.__disconnect = r));
    const loop = (): void => {
      const t = performance.now();
      while (performance.now() - t < blockMs) {
        /* the frame is slow: nothing else runs on the main thread */
      }
      w.__frames++;
      requestAnimationFrame(loop);
    };
    if (blockMs > 0) requestAnimationFrame(loop);
  }, BLOCK_MS);

  const sample = async (): Promise<{ frames: number; steps: number; lead: number; rtt: number; disconnect?: string; snapshots: number }> =>
    page.evaluate(() => {
      const g = window.__netGame!;
      const w = window as unknown as { __disconnect?: string; __frames: number };
      return { frames: w.__frames, steps: g.game.stats.steps, lead: g.session.clock.inputLeadTicks(), rtt: g.session.clock.rtt(), disconnect: w.__disconnect, snapshots: g.game.stats.snapshots };
    });
  const t0 = Date.now();
  const first = await sample();
  const samples = [first];
  while (Date.now() - t0 < SECONDS * 1000) {
    await page.waitForTimeout(5000);
    const s = await sample();
    samples.push(s);
    if (s.disconnect) break;
  }
  const last = samples[samples.length - 1]!;
  const fps = (last.frames - first.frames) / ((Date.now() - t0) / 1000);
  console.log(`[low-fps] ${JSON.stringify({ seconds: SECONDS, fps: +fps.toFixed(2), last, leads: samples.map((s) => s.lead), rtts: samples.map((s) => Math.round(s.rtt)) })}`);
  const hist: Record<string, number> = {};
  for (const l of srv.lines) hist[String(l.event)] = (hist[String(l.event)] ?? 0) + 1;
  console.log(`[low-fps] server log events ${JSON.stringify(hist)}`);
  const mport = srv.lines.find((l) => l.event === 'metrics_listening')?.port;
  if (mport) {
    const txt = await (await fetch(`http://127.0.0.1:${String(mport)}/metrics`)).text();
    console.log(`[low-fps] metrics ${txt.split('\n').filter((l) => /^(input_rejected_total|input_clamped|server_input|session_)/.test(l)).join(' | ')}`);
  }
  const kicked = srv.lines.filter((l) => l.event === 'session_kicked');
  expect(kicked, JSON.stringify(kicked)).toEqual([]);
  expect(last.disconnect).toBeUndefined();
  if (BLOCK_MS > 0) expect(fps).toBeLessThanOrEqual(5);
  expect(last.snapshots).toBeGreaterThan(SECONDS * 5);
});

// The server side of the same fix, deterministically: a client whose tick estimate runs 25 ticks ahead (beyond the server's
// now + 10 window) had every command refused as `too_far`, and 30 in a row used to kick it (the demo-server incident).
// Now such commands are dropped without a strike; only ticks absurdly far ahead (> 300) or other malformed values count.
test('commands stamped beyond the server window are dropped, not punished', async ({ page }) => {
  test.setTimeout(90_000);
  const srv = server as ServerProc;
  const kicksBefore = srv.lines.filter((l) => l.event === 'session_kicked').length;
  await page.goto(`${base}?renderer=webgl2&empty=1&transport=ws&server=127.0.0.1:${srv.wsPort}`);
  await page.waitForFunction(() => window.__netGame !== undefined, undefined, { timeout: 30_000 });
  await page.evaluate(() => {
    const g = window.__netGame!;
    g.sampler.press('KeyW');
    const w = window as unknown as { __disconnect?: string };
    g.session.onDisconnect((r) => (w.__disconnect = r));
    const clock = g.session.clock;
    const real = clock.inputTargetTick.bind(clock);
    clock.inputTargetTick = (at?: number) => real(at) + 25; // 25 ticks ahead: every command lands outside the window
  });
  await page.waitForTimeout(12_000); // 30 commands per second: 360 refused commands, 12x the kick threshold
  const r = await page.evaluate(() => ({ disconnect: (window as unknown as { __disconnect?: string }).__disconnect, steps: window.__netGame!.game.stats.steps }));
  const mport = srv.lines.find((l) => l.event === 'metrics_listening')?.port;
  const txt = await (await fetch(`http://127.0.0.1:${String(mport)}/metrics`)).text();
  const tooFar = Number(/input_rejected_total\{reason="too_far"\} (\d+)/.exec(txt)?.[1] ?? 0);
  console.log(`[low-fps] skewed clock: ${JSON.stringify({ ...r, tooFar })}`);
  expect(srv.lines.filter((l) => l.event === 'session_kicked').length).toBe(kicksBefore);
  expect(r.disconnect).toBeUndefined();
  expect(tooFar).toBeGreaterThan(100);
});
