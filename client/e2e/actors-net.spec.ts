import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';
import { createServer, type ViteDevServer } from 'vite';
import { HAVE_BINARIES, removeDir, REPO_ROOT, SERVER_BIN, SKIP_REASON } from './net-support';

// M0-025 end-to-end smoke: the real client (wasm, session over WebSocket, GameLoop) against a real frontline-server
// with bots; first-person camera on the predicted pose, remote bots as grey-box actors. What the server sends depends on
// the server build of the day (M0-019 snapshots, M0-026 bots): the test records what it sees and asserts only the
// invariants that hold either way (connection, camera follows the local state, one actor per remote entity).
const required = process.env.FRONTLINE_REQUIRE_SERVER === '1';
test.skip(!HAVE_BINARIES && !required, SKIP_REASON);
test.beforeAll(() => {
  if (!HAVE_BINARIES && required) throw new Error(SKIP_REASON);
});

const CLIENT_DIR = fileURLToPath(new URL('../', import.meta.url));
const BOTS = 8;
let vite: ViteDevServer | undefined;
let base = '';
let proc: ChildProcess | undefined;
let exited = false;
let wsPort = 0;
let cacheDir = '';

test.beforeAll(async () => {
  if (!HAVE_BINARIES) return;
  cacheDir = mkdtempSync(join(tmpdir(), 'fl-actors-vite-'));
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
  proc = spawn(SERVER_BIN, ['--ws-port', '0', '--wt-port', '0', '--metrics-port', '0', '--bots', String(BOTS)], { stdio: ['ignore', 'pipe', 'pipe'], cwd: REPO_ROOT });
  let buf = '';
  wsPort = await new Promise<number>((res, rej) => {
    const t = setTimeout(() => rej(new Error('server did not report ws_listening')), 20_000);
    proc!.once('exit', () => {
      exited = true;
      rej(new Error('server exited early'));
    });
    proc!.stdout?.on('data', (d) => {
      buf += d.toString();
      for (const l of buf.split('\n')) {
        try {
          const j = JSON.parse(l) as { event?: string; port?: number };
          if (j.event === 'ws_listening' && j.port) {
            clearTimeout(t);
            res(j.port);
          }
        } catch {
          /* partial line */
        }
      }
    });
  });
});
test.afterAll(async () => {
  if (proc && !exited) {
    const done = new Promise<void>((r) => proc!.once('exit', () => r()));
    proc.kill('SIGTERM');
    const t = setTimeout(() => proc!.kill('SIGKILL'), 5000);
    await done;
    clearTimeout(t);
  }
  await vite?.close();
  if (cacheDir) removeDir(cacheDir);
});

// `vegetation`: terrain + vegetation (~500 k triangles: about 1-4 frames/s in software rendering); `empty`: no scene meshes (fast frames).
for (const variant of [{ name: 'vegetation', q: 'scene=vegetation' }, { name: 'empty', q: 'empty=1' }]) {
test(`browser -> server -> walk -> see the other entities (first-person camera, grey-box actors), scene ${variant.name}`, async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`console.error: ${m.text()}`);
  });
  await page.goto(`${base}?renderer=webgl2&${variant.q}&transport=ws&server=127.0.0.1:${wsPort}`);
  await page.waitForFunction(() => window.__netGame !== undefined && window.__actors !== undefined, undefined, { timeout: 120_000 });
  const snap = () =>
    page.evaluate(() => {
      const g = window.__netGame!.game;
      const a = window.__actors!;
      const cam = a.manager.rig!.camera.position;
      return {
        steps: g.stats.steps,
        snapshots: g.stats.snapshots,
        local: [g.state.local.pos[0]!, g.state.local.pos[1]!, g.state.local.pos[2]!],
        cam: [cam.x, cam.y, cam.z],
        eye: a.manager.rig!.eye,
        entities: g.state.entities.count,
        actors: a.manager.lastUpdate.actors,
        entitiesAtUpdate: a.manager.lastUpdate.entities,
        corrections: g.state.net.corrections.value,
        snaps: g.state.net.snaps.value,
        fireLocal: g.stats.fireLocal,
        hp: g.state.net.health.value,
        ammo: g.state.local.ammo,
        weaponNodes: a.manager.rig!.weapon?.root.name ?? null,
      };
    });
  await page.waitForFunction(() => (window.__netGame?.game.stats.steps ?? 0) >= 60, undefined, { timeout: 60_000 });
  await page.waitForTimeout(1500);
  const s0 = await snap();
  await page.evaluate(() => window.__netGame!.sampler.press('KeyW'));
  // frame rate differs by orders of magnitude between the variants: wait for the effect, not for a fixed time
  await page.waitForFunction((z0) => Math.hypot(0, (window.__netGame!.game.state.local.pos[2] as number) - z0) > 2, s0.local[2]!, { timeout: 60_000 });
  await page.waitForTimeout(500);
  const s1 = await snap();
  await page.evaluate(() => {
    window.__netGame!.sampler.release('KeyW');
    window.__netGame!.sampler.press('Mouse0');
  });
  await page.waitForFunction(() => window.__netGame!.game.stats.fireLocal > 0, undefined, { timeout: 60_000 });
  await page.evaluate(() => window.__netGame!.sampler.release('Mouse0'));
  const s2 = await snap();
  if (process.env.SAVE_SHOTS === '1') await page.screenshot({ path: `../docs/measurements/M0-025/net-bots-${variant.name}.png` });
  console.log(`[actors-net smoke ${variant.name}] bots=${BOTS}\n s0 ${JSON.stringify(s0)}\n s1 ${JSON.stringify(s1)}\n s2 ${JSON.stringify(s2)}`);
  // invariants
  expect(s1.steps).toBeGreaterThan(s0.steps);
  for (const s of [s0, s1, s2]) {
    expect(s.cam[0]).toBeCloseTo(s.local[0]!, 3); // camera on the predicted pose
    expect(s.cam[2]).toBeCloseTo(s.local[2]!, 3);
    expect(s.cam[1]! - s.local[1]!).toBeCloseTo(s.eye, 3);
    expect(s.actors).toBe(s.entitiesAtUpdate); // after every update: one grey-box actor per remote entity
  }
  expect(s2.weaponNodes).toMatch(/^PH_V-WPN-(US|GE)-001__fp$/);
  expect(Math.hypot(s1.local[0]! - s0.local[0]!, s1.local[2]! - s0.local[2]!)).toBeGreaterThan(1); // really walked
  expect(s2.fireLocal).toBeGreaterThan(0);
  expect(errors).toEqual([]);
});
}

test('two players see each other as grey-box actors (3 pages: two of them share a team, friendlies are always visible)', async ({ browser }) => {
  const pages: Page[] = [];
  for (let i = 0; i < 3; i++) {
    const p = await (await browser.newContext({ viewport: { width: 320, height: 180 } })).newPage();
    await p.goto(`${base}?renderer=webgl2&empty=1&transport=ws&server=127.0.0.1:${wsPort}`);
    pages.push(p);
  }
  try {
    const info: { id: number; team: number }[] = [];
    for (const p of pages) {
      await p.waitForFunction(() => window.__netGame !== undefined && window.__actors !== undefined, undefined, { timeout: 120_000 });
      info.push(await p.evaluate(() => ({ id: window.__netGame!.session.welcome.playerId, team: window.__netGame!.session.welcome.team })));
    }
    const a = info.findIndex((x, i) => info.some((y, j) => j !== i && y.team === x.team));
    const b = info.findIndex((y, j) => j !== a && y.team === info[a]!.team);
    console.log(`[actors-net two players] welcome ${JSON.stringify(info)} -> pair ${a},${b}`);
    for (const [me, other] of [[a, b], [b, a]] as const) {
      await pages[me]!.waitForFunction(
        (id) => {
          const e = window.__netGame!.game.state.entities;
          for (let k = 0; k < e.count; k++) if (e.ids[k] === id) return window.__actors!.manager.actor(id) !== undefined;
          return false;
        },
        info[other]!.id,
        { timeout: 60_000 },
      );
    }
  } finally {
    for (const p of pages) await p.context().close();
  }
});
