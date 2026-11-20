import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { Agent } from 'node:http';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';
import { createServer, type ViteDevServer } from 'vite';
import { HQ_SPAWN_ID } from '../src/game/deploy-actions';
import { removeDir, REPO_ROOT, SERVER_BIN, WASM_JS } from './net-support';

const CLIENT = fileURLToPath(new URL('..', import.meta.url));
const GATEWAY = process.env.FRONTLINE_GATEWAY_BIN ?? join(REPO_ROOT, 'target/debug/frontline-gateway');
const DEPOT = { x: 64, z: 24 };
const SITE = { x: 64, z: 237 };
const MATCH_ID = '00000000-0000-4000-8000-000000000011';

function demoPass(step: number, evidence: string) {
  const dir = process.env.FRONTLINE_M1_DEMO_DIR;
  if (dir) writeFileSync(join(dir, `step-${step}`), evidence);
}

function fixtureMap(dir: string, publicDir: string) {
  const map = join(dir, 'maps/carentan/m1');
  // startNetGame currently loads its collision from the M0 URL. Serve the exact same bytes there.
  const clientMap = join(publicDir, 'maps/m0');
  for (const path of [map, clientMap]) mkdirSync(join(path, 'chunks'), { recursive: true });
  writeFileSync(join(map, 'meta.json'), JSON.stringify({ crs: { size_m: 512 } }));
  writeFileSync(join(map, 'spawns.json'), JSON.stringify({ hq: { us: { pos: [DEPOT.x, DEPOT.z] }, ger: { pos: [440, 24] } } }));
  writeFileSync(join(map, 'zones.json'), JSON.stringify({ lines: [{ points: [{ pos: [384, 390], radius_m: 10 }] }] }));
  // Flat, dry build ground (>200 m from both HQs) and a cliff south of it for real fall damage.
  for (let bz = 0; bz < 7; bz++) {
    const chunk = new Uint8Array(16 + 65 * 65 * 2 + 64 * 64);
    chunk.set([0x46, 0x43, 0x4f, 0x4c, 1, 0, 65, 0]); // FCOL v1, 65 vertices per edge
    const view = new DataView(chunk.buffer);
    view.setFloat32(8, 5, true);
    view.setFloat32(12, 45, true);
    for (let z = 0; z <= 64; z++) {
      for (let x = 0; x <= 64; x++) view.setUint16(16 + 2 * (z * 65 + x), bz * 64 + z <= 256 ? 65535 : 0, true);
    }
    for (let bx = 0; bx < 7; bx++) {
      const name = `chunks/c_${bx}_${bz}.bin`;
      writeFileSync(join(map, name), chunk);
      if (bx <= 4 && bz <= 4) writeFileSync(join(clientMap, name), chunk);
    }
  }
}

function launch(bin: string, args: string[], dir: string, env: Record<string, string>) {
  const child = spawn(bin, args, { cwd: dir, env: { PATH: process.env.PATH, ...env }, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = '';
  let stderr = '';
  let exited = false;
  let exitCode: number | null = null;
  console.log(`[garrison-loop] started ${bin} pid=${child.pid}`);
  child.stdout?.on('data', (d) => { stdout += d.toString(); });
  child.stderr?.on('data', (d) => { stderr += d.toString(); });
  const closed = new Promise<void>((resolve) => {
    const done = () => { exited = true; resolve(); };
    child.once('exit', (code) => { exitCode = code; done(); });
    child.once('error', (err) => { stderr += err.message; done(); });
  });
  return {
    output: () => stdout,
    error: () => stderr,
    async ready<T>(read: () => T | undefined): Promise<T> {
      const deadline = Date.now() + 10_000;
      while (!exited && Date.now() < deadline) {
        const value = read();
        if (value !== undefined) return value;
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
      throw new Error(`${bin} not ready: ${stderr}\n${stdout.slice(-2500)}`);
    },
    async stop() {
      if (!exited && child.pid) child.kill('SIGTERM');
      await closed;
      return exitCode;
    },
  };
}

function event(log: ReturnType<typeof launch>, name: string) {
  return log.output().split('\n').filter((line) => line.startsWith('{'))
    .map((line): Record<string, unknown> | undefined => {
      try { return JSON.parse(line) as Record<string, unknown>; } catch { return undefined; }
    }).find((row) => row?.event === name);
}

function index(dir: string, id: string) {
  const n = readdirSync(join(REPO_ROOT, 'data', dir)).filter((file) => file.endsWith('.json')).sort().indexOf(`${id}.json`);
  if (n < 0) throw new Error(`Missing ${dir}/${id}.json`);
  return n;
}

async function walk(page: Page, target: { x: number; z: number }, sprint: boolean, tolerance = 0.9) {
  try {
    await expect.poll(() => page.evaluate(({ x, z, sprint, tolerance }) => {
        const f = window.garrisonFixture;
        const [px, , pz] = f.status().pos;
        const d = Math.hypot(px! - x, pz! - z);
        if (d > tolerance) f.move(x, z, sprint);
        else f.stopMove();
        return d;
    }, { ...target, sprint, tolerance }), { message: `reach (${target.x},${target.z})`, timeout: 70_000, intervals: [100] }).toBeLessThanOrEqual(tolerance);
  } finally {
    await page.evaluate(() => window.garrisonFixture.stopMove());
  }
}

const status = (page: Page) => page.evaluate(() => window.garrisonFixture.status());

for (const transport of ['ws', 'wt'] as const) {
  test(`two allied browsers complete the authoritative Garrison and ammo loop over ${transport}`, async ({ browser }, testInfo) => {
    const context = await browser.newContext();
    const support = await context.newPage();
    const officer = await context.newPage();
    const dir = mkdtempSync(join(tmpdir(), 'frontline-garrison-'));
    const publicDir = join(dir, 'public');
    let gateway: ReturnType<typeof launch> | undefined;
    let server: ReturnType<typeof launch> | undefined;
    let vite: ViteDevServer | undefined;
    const demo = transport === 'ws' && Boolean(process.env.FRONTLINE_M1_DEMO_DIR);
    // Match the native load tester's loopback source spread without changing the real per-IP cap.
    const reserveAgent = new Agent({ localAddress: '127.0.0.2' });
    try {
      fixtureMap(dir, publicDir);
      mkdirSync(join(publicDir, 'wasm'));
      for (const file of ['frontline_sim_wasm.js', 'frontline_sim_wasm_bg.wasm']) {
        writeFileSync(join(publicDir, 'wasm', file), readFileSync(join(dirname(WASM_JS), file)));
      }
      const token = 'garrison-loop-local-only';
      const db = join(dir, 'gateway.sqlite3');
      gateway = launch(GATEWAY, [], dir, { GATEWAY_BIND: '127.0.0.1:0', INTERNAL_API_TOKEN: token,
        TOKEN_SECRET: 'garrison-loop-disposable-guest-secret', DATABASE_URL: demo ? `sqlite://${db}` : 'sqlite::memory:', RUST_LOG: 'info', NO_COLOR: '1' });
      const addr = await gateway.ready(() => /addr=(127\.0\.0\.1:\d+)/.exec(gateway!.output())?.[1]);
      server = launch(SERVER_BIN, ['--data-dir', dir, '--map', 'carentan/m1', '--mode', 'assault',
        '--ws-port', '0', '--wt-port', '0', '--metrics-port', '0', '--log-level', 'debug',
        '--gateway-url', `http://${addr}`, '--match-id', MATCH_ID, ...(transport === 'wt' ? ['--dev-cert'] : [])], dir,
      { INTERNAL_API_TOKEN: token });
      const registration = await server.ready(() => event(server!, 'gateway_registered'));
      const wsPort = event(server, 'ws_listening')?.port;
      if (typeof wsPort !== 'number') throw new Error('missing WS port');
      const hash = transport === 'wt' ? event(server, 'wt_listening')?.cert_sha256_hex : '';
      if (typeof hash !== 'string' || (transport === 'wt' && !/^[a-f0-9]{64}$/.test(hash))) throw new Error('missing WT pin');
      vite = await createServer({ root: CLIENT, publicDir, cacheDir: join(dir, 'vite-cache'), configFile: false, logLevel: 'error',
        optimizeDeps: { noDiscovery: true, include: [] },
        server: { host: '127.0.0.1', port: 0, hmr: false, watch: null, proxy: {
          '/v1': `http://${addr}`,
          '/game-ws': { target: `ws://127.0.0.1:${wsPort}`, ws: true, agent: reserveAgent, rewrite: () => '/' },
        } } });
      await vite.listen();
      const http = vite.httpServer?.address();
      if (!http || typeof http === 'string') throw new Error('Vite has no address');
      const origin = `http://127.0.0.1:${http.port}`;
      if (demo) {
        // Real browser capability detection, public menu and registered server list, with no mocked routes.
        const lobby = await context.newPage();
        await lobby.goto(origin);
        await expect(lobby.getByText('Compatibility mode (WebGL2)')).toBeVisible();
        await lobby.getByRole('button', { name: 'Click to continue' }).click();
        const browserButton = lobby.getByRole('button', { name: 'Server browser' });
        await expect(browserButton).toBeVisible();
        demoPass(1, `Chromium ${browser.version()} 实际探测软件 WebGL2，启动页进入主菜单。`);
        await browserButton.click();
        await expect(lobby.getByRole('row', { name: /Frontline/ }).getByRole('button', { name: 'Join' })).toBeEnabled();
        await lobby.close();
      }
      const entry = `${origin}/e2e/garrison-loop.html`;
      const errors: string[] = [];
      for (const page of [support, officer]) page.on('pageerror', (e) => errors.push(e.message));
      await Promise.all([support.goto(entry), officer.goto(entry)]);
      test.skip(transport === 'wt' && !(await support.evaluate(() => typeof WebTransport === 'function')), 'Chromium has no WebTransport API on loopback');
      const joinPage = (page: Page, nick: string) => page.evaluate(async (p) => {
        const f = window.garrisonFixture = await window.joinGarrisonFixture(p.nick, p.transport, p.hash);
        return { id: f.id, team: f.team, transport: f.transport };
      }, { nick, transport, hash });
      const lead = await joinPage(officer, 'Loop-Officer');
      await officer.evaluate(() => window.garrisonFixture.addReserve('Idle-Reserve-1', 1));
      const helper = await joinPage(support, 'Loop-Support');
      expect(lead.transport).toBe(transport);
      expect(helper.transport).toBe(transport);
      expect(lead.team).toBe(0);
      expect(helper.team).toBe(lead.team);
      expect(helper.id).not.toBe(lead.id);

      await officer.evaluate(() => window.garrisonFixture.createSquad());
      const roster = (page: Page, id: number) => page.evaluate((id) => window.garrisonFixture.roster().find((p) => p.id === id), id);
      await expect.poll(async () => (await roster(officer, lead.id))?.squad ?? 0).toBeGreaterThan(0);
      const squadId = (await roster(officer, lead.id))!.squad;
      await support.evaluate((id) => window.garrisonFixture.joinSquad(id), squadId);
      await expect.poll(async () => (await roster(officer, helper.id))?.squad).toBe(squadId);
      const reserveId = await officer.evaluate(async (id) => {
        const f = window.garrisonFixture;
        await f.addReserve('Idle-Reserve-2', 1);
        return f.addReserve('Idle-Reserve-3', 0, id);
      }, squadId);
      await expect.poll(async () => (await roster(officer, reserveId))?.squad).toBe(squadId);

      const officerReq = { spawnId: HQ_SPAWN_ID, role: index('roles', 'officer'), loadout: index('loadouts', 'us_officer_1'), squadId };
      const supportReq = { spawnId: HQ_SPAWN_ID, role: index('roles', 'support'), loadout: index('loadouts', 'us_support_1'), squadId };
      for (const [page, req] of [[officer, officerReq], [support, supportReq]] as const) {
        const result = await page.evaluate((req) => window.garrisonFixture.deploy(req), req);
        expect(result, JSON.stringify(result)).toMatchObject({ ok: true });
      }
      await Promise.all([officer, support].map((p) => expect.poll(async () => (await status(p)).health).toBeGreaterThan(0)));
      if (demo) demoPass(2, '服务器列表显示已注册对局。2 个 Chromium 游客取得真实签票并通过 WS 入场，US 小队 Officer/Support 的 HQ 部署获 C0 确认和存活快照。');

      // The Depot starts empty and generates the first crate at tick 1350. Approach the site with
      // the officer concurrently; start the Support's hold only after stock can exist.
      await Promise.all([
        walk(officer, SITE, true),
        expect.poll(async () => (await status(support)).tick, { timeout: 52_000, intervals: [250] }).toBeGreaterThanOrEqual(1355),
      ]);
      expect(await officer.evaluate(() => window.garrisonFixture.place('garrison', false)))
        .toEqual({ reason: 'ui.officer.err.noSupplies' });
      expect(await support.evaluate(() => window.garrisonFixture.place('supply_crate', false)))
        .toEqual({ reason: 'ui.officer.err.notCarrying' });
      await support.evaluate(() => window.garrisonFixture.hold('KeyJ'));
      const takeAt = (await status(support)).tick;
      await expect.poll(async () => (await status(support)).tick).toBeGreaterThanOrEqual(takeAt + 52);
      await support.evaluate(() => window.garrisonFixture.release('KeyJ'));
      expect(await support.evaluate(() => window.garrisonFixture.place('supply_crate', false))).toEqual({ cost: 0 });
      await walk(support, SITE, true);
      expect(await support.evaluate(() => window.garrisonFixture.place('supply_crate'))).toMatchObject({ cost: 0, confirmToAckMs: expect.any(Number) });
      expect(await support.evaluate(() => window.garrisonFixture.place('garrison', false)))
        .toEqual({ reason: 'ui.officer.err.wrongRole' });
      const timing = await officer.evaluate(() => window.garrisonFixture.place('garrison'));
      expect(timing).toMatchObject({ cost: 50, selectToAckMs: expect.any(Number), confirmToAckMs: expect.any(Number) });

      const point = () => support.evaluate(() => window.garrisonFixture.points()[0]);
      await expect.poll(point).toMatchObject({ kind: 0, state: 0, team: lead.team }); // C4 SpawnKind::Garrison
      const garrison = (await point())!;
      await expect.poll(() => event(server!, 'garrison_placed')).toMatchObject({
        player: lead.id, garrison: garrison.id, supplies_before: 100, supplies_after: 50,
      });
      await officer.evaluate(() => window.garrisonFixture.hold('KeyJ'));
      await expect.poll(point, { timeout: 28_000, intervals: [100] }).toMatchObject({ id: garrison.id, state: 1 });
      await officer.evaluate(() => window.garrisonFixture.release('KeyJ'));
      const stateTimes = await support.evaluate(() => window.garrisonFixture.spawnEvents);
      expect(stateTimes.map(({ id, state }) => [id, state])).toEqual([[garrison.id, 0], [garrison.id, 1]]);

      // Fall off the test cliff, then choose the active Garrison rather than waiting for HQ.
      await officer.evaluate(() => window.garrisonFixture.move(64, 275, true));
      await expect.poll(async () => (await status(officer)).health, { timeout: 15_000 }).toBe(0);
      await officer.evaluate(() => window.garrisonFixture.stop());
      const deadTick = (await status(officer)).tick;
      const respawn = await officer.evaluate((req) => window.garrisonFixture.deploy(req), { ...officerReq, spawnId: garrison.id });
      expect(respawn).toMatchObject({ ok: true });
      await expect.poll(async () => (await status(officer)).health, { timeout: 14_000 }).toBeGreaterThan(0);
      const peer = () => support.evaluate((id) => window.garrisonFixture.peers(id), lead.id);
      const spawnDistance = async () => {
        const p = await peer();
        // C4 coordinates are 1/32 m codes; SnapshotDecoder returns metres.
        return p && p.tick > deadTick && p.health > 0 ? Math.hypot(p.x - garrison.pos[0] / 32, p.z - garrison.pos[1] / 32) : Infinity;
      };
      await expect.poll(spawnDistance).toBeLessThanOrEqual(8.1);

      const garrisonDistance = await spawnDistance();
      const atSpawn = (await peer())!;
      await walk(support, atSpawn, false);
      expect(await support.evaluate(() => window.garrisonFixture.place('ammo_crate'))).toMatchObject({ cost: 0, confirmToAckMs: expect.any(Number) });
      const full = await status(officer);
      expect(full.ammo).toBeGreaterThan(0);
      expect(full.reserve).toBeGreaterThan(0);
      await officer.evaluate(() => window.garrisonFixture.hold('Mouse0'));
      await expect.poll(async () => (await status(officer)).ammo, { timeout: 9_000 }).toBe(0);
      if (demo) demoPass(3, `步行到达放置点，射击消耗 ${full.ammo} 发，Garrison 扣除 50 补给并激活，重生距驻点 ${garrisonDistance.toFixed(2)} m。`);
      await officer.evaluate(() => {
        window.garrisonFixture.release('Mouse0');
        window.garrisonFixture.hold('KeyR');
      });
      await expect.poll(async () => (await status(officer)).ammo, { timeout: 8_000 }).toBe(full.ammo);
      await officer.evaluate(() => window.garrisonFixture.release('KeyR'));
      const depleted = (await status(officer)).reserve;
      expect(depleted).toBe(full.reserve - full.ammo);
      await officer.evaluate(() => window.garrisonFixture.hold('KeyJ'));
      await expect.poll(async () => (await status(officer)).reserve).toBe(full.reserve);
      await officer.evaluate(() => window.garrisonFixture.release('KeyJ'));
      expect(errors).toEqual([]);

      const result = { transport, ...timing, buildingToActiveMs: stateTimes[1]!.at - stateTimes[0]!.at,
        respawnDistanceM: garrisonDistance, reserve: [full.reserve, depleted, (await status(officer)).reserve] };
      console.log(`[garrison-loop] ${JSON.stringify(result)}`);
      await testInfo.attach('garrison timings', { body: JSON.stringify(result, null, 2), contentType: 'application/json' });
      if (demo) {
        demoPass(4, `Support 在 HQ 取箱并搬运，放置补给箱和弹药箱。Officer 备弹 ${result.reserve.join(' → ')}。`);
        const port = event(server, 'metrics_listening')?.port;
        expect(typeof port).toBe('number');
        const metrics = await fetch(`http://127.0.0.1:${port}/metrics`, { signal: AbortSignal.timeout(5000) });
        expect(metrics.ok).toBe(true);
        const tickCount = Number(/^server_tick_ms_count (\d+)$/m.exec(await metrics.text())?.[1]);
        expect(tickCount).toBeGreaterThan(0);

        // A controlled finish checks the real gateway result handoff, terminal flag and persistence.
        expect(await server.stop()).toBe(0);
        const summary = event(server, 'summary');
        expect(Number(summary?.ticks)).toBeGreaterThan(0);
        const gatewayMatchId = registration.gateway_match_id;
        if (typeof gatewayMatchId !== 'string') throw new Error('missing gateway match ID');
        const persisted = execFileSync('python3', ['-c', `
import json, sqlite3, sys
with sqlite3.connect(sys.argv[1]) as db:
    ended, payload = db.execute('SELECT ended, payload FROM matches JOIN results ON matches.id = results.match_id WHERE matches.id = ?', (sys.argv[2],)).fetchone()
    print(json.dumps([ended, json.loads(payload)]))
`, db, gatewayMatchId], { encoding: 'utf8', timeout: 5000 });
        expect(JSON.parse(persisted)).toMatchObject([1, { objectives: 1, ticks: summary?.ticks, rows: [] }]);
        demoPass(8, `HTTP /metrics 返回 ${tickCount} 个 tick 样本。主动停服触发结果上报，gateway SQLite 确认 ended=1、${summary?.ticks} ticks、1 个据点。`);
      }
    } finally {
      if (server) await testInfo.attach('match diagnostics', { body: server.output().slice(-8000) + server.error(), contentType: 'text/plain' });
      await context.close();
      await vite?.close();
      reserveAgent.destroy();
      await server?.stop();
      await gateway?.stop();
      removeDir(dir);
    }
  });
}
