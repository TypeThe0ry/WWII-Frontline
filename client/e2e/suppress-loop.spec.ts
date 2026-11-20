import { mkdirSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { expect, test, type Page } from '@playwright/test';
import { createServer, type ViteDevServer } from 'vite';
import { ROSTER_ALIVE } from '../src/game/state/c4';
import { removeDir, REPO_ROOT, startServer, WASM_JS } from './net-support';

const link = process.env.FRONTLINE_SUPPRESS_LINK ?? 'plain';
const usHq = [30, 60];
const gerHq = [22.5, 60];

function arena(dir: string) {
  const serverMap = join(dir, 'maps/carentan/m1');
  const clientMap = join(dir, 'public/maps/m0');
  for (const path of [serverMap, clientMap]) mkdirSync(join(path, 'chunks'), { recursive: true });
  writeFileSync(join(serverMap, 'meta.json'), JSON.stringify({ crs: { size_m: 256 } }));
  writeFileSync(join(serverMap, 'spawns.json'), JSON.stringify({ hq: { us: { pos: usHq }, ger: { pos: gerHq } } }));
  writeFileSync(join(serverMap, 'zones.json'), JSON.stringify({ lines: [{ points: [{ pos: [128, 128], radius_m: 10 }] }] }));
  // Flat, dry collision shared by the real server and prediction WASM. FCOL v1, 65×65 flat vertices.
  const chunk = new Uint8Array(16 + 65 * 65 * 2 + 64 * 64);
  chunk.set([0x46, 0x43, 0x4f, 0x4c, 1, 0, 65, 0]);
  new DataView(chunk.buffer).setFloat32(8, 5, true);
  new DataView(chunk.buffer).setFloat32(12, 6, true);
  for (const path of [serverMap, clientMap]) {
    for (let z = 0; z < 4; z++) for (let x = 0; x < 4; x++) writeFileSync(join(path, `chunks/c_${x}_${z}.bin`), chunk);
  }
  const wasmDir = join(dir, 'public/wasm');
  mkdirSync(wasmDir, { recursive: true });
  for (const file of ['frontline_sim_wasm.js', 'frontline_sim_wasm_bg.wasm']) {
    writeFileSync(join(wasmDir, file), readFileSync(join(dirname(WASM_JS), file)));
  }
}

function dataIndex(dir: string, id: string) {
  const index = readdirSync(join(REPO_ROOT, 'data', dir)).filter((name) => name.endsWith('.json')).sort().indexOf(`${id}.json`);
  if (index < 0) throw new Error(`Missing ${dir}/${id}.json`);
  return index;
}

const health = (page: Page) => page.evaluate(() => window.suppressLoop.health());
const pos = (page: Page) => page.evaluate(() => window.suppressLoop.pose());
const audio = (page: Page) => page.evaluate(() => window.suppressLoop.audio());
const events = (page: Page) => page.evaluate(() => window.suppressLoop.c4());
const squad = (page: Page, id: number) => page.evaluate((p) => window.suppressLoop.roster().find((row) => row.id === p)?.squad ?? 0, id);
const release = (page: Page, code: string) => page.evaluate((key) => window.suppressLoop.release(key), code);

async function deploy(page: Page, role: string, loadout: string, squadId: number) {
  const request = { role: dataIndex('roles', role), loadout: dataIndex('loadouts', loadout), squadId };
  const result = await page.evaluate((req) => window.suppressLoop.deploy(req), request);
  expect(result, JSON.stringify(result)).toMatchObject({ ok: true, spawnTick: expect.any(Number) });
  await expect.poll(() => health(page)).toBeGreaterThan(0);
}

async function pointAt(shooter: Page, victim: Page, height: number) {
  const [from, to] = await Promise.all([pos(shooter), pos(victim)]);
  const dx = to.x - from.x, dz = to.z - from.z;
  const range = Math.hypot(dx, dz);
  const yaw = Math.atan2(dx, dz);
  const pitch = Math.atan2(to.y + height - (from.y + 1.65), range);
  await shooter.evaluate(([y, p]) => window.suppressLoop.aim(y!, p!), [yaw, pitch]);
}

async function hold<T>(page: Page, key: string, check: () => Promise<T>): Promise<T> {
  await page.evaluate((k) => window.suppressLoop.press(k), key);
  try { return await check(); } finally { await release(page, key); }
}

test(`authoritative suppression and life cycle over WS (${link})`, async ({ context }, testInfo) => {
  const dir = mkdtempSync(join(tmpdir(), 'fl-suppress-'));
  let server: Awaited<ReturnType<typeof startServer>> | undefined;
  let vite: ViteDevServer | undefined;
  const errors: string[] = [];
  const evidence: Record<string, unknown> = { link };
  try {
    arena(dir);
    server = await startServer({ args: ['--dev-no-gateway', '--data-dir', dir, '--map', 'carentan/m1', '--mode', 'assault',
      '--match-id', '00000000-0000-4000-8000-000000000013'] });
    vite = await createServer({ root: join(REPO_ROOT, 'client'), publicDir: join(dir, 'public'), cacheDir: join(dir, 'vite'),
      configFile: false, logLevel: 'error', optimizeDeps: { noDiscovery: true, include: [] },
      server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await vite.listen();
    const address = vite.httpServer?.address();
    if (!address || typeof address === 'string') throw new Error('Vite did not bind a TCP port');
    const url = `http://127.0.0.1:${address.port}/e2e/suppress-loop.html?server=127.0.0.1:${server.wsPort}&transport=ws`;
    const open = async (withHud: boolean) => {
      const page = await context.newPage();
      page.on('pageerror', (e) => errors.push(e.message));
      await page.goto(url);
      const info = await page.evaluate(async (hud) => {
        const f = window.suppressLoop = await window.joinSuppressLoop(hud);
        return { id: f.id, team: f.team, transport: f.transport, renderer: f.renderer };
      }, withHud);
      expect(info.transport).toBe('ws');
      return { page, ...info };
    };
    const victim = await open(true);
    const shooter = await open(false);
    const medic = await open(false);
    const secondShooter = await open(false);
    expect([victim.team, shooter.team, medic.team, secondShooter.team]).toEqual([0, 1, 0, 1]);
    expect(victim.renderer).toContain('SwiftShader');
    evidence.renderer = victim.renderer;
    const v = victim.page, s = shooter.page, m = medic.page, r = secondShooter.page;
    const fx = v.locator('[data-hud-part="fx"]');
    const deathPanel = v.locator('[data-downed-death]');
    await v.bringToFront();
    await v.locator('#unlock-audio').click();
    await expect.poll(() => audio(v)).toEqual({ context: 'running', cutoff: 20000, gain: 1 });
    await expect(fx).toHaveAttribute('data-state', 'clear');

    await v.evaluate(() => window.suppressLoop.createSquad());
    await s.evaluate(() => window.suppressLoop.createSquad());
    await expect.poll(() => squad(v, victim.id)).toBeGreaterThan(0);
    await expect.poll(() => squad(s, shooter.id)).toBeGreaterThan(0);
    const [usSquad, gerSquad] = await Promise.all([squad(v, victim.id), squad(s, shooter.id)]);
    await m.evaluate((id) => window.suppressLoop.joinSquad(id), usSquad);
    await r.evaluate((id) => window.suppressLoop.joinSquad(id), gerSquad);
    await expect.poll(() => squad(v, medic.id)).toBe(usSquad);
    await expect.poll(() => squad(s, secondShooter.id)).toBe(gerSquad);
    await deploy(v, 'rifleman', 'us_rifleman_1', usSquad);
    await deploy(m, 'medic', 'us_medic_1', usSquad);
    await deploy(r, 'rifleman', 'ger_rifleman_1', gerSquad);
    await deploy(s, 'rifleman', 'ger_rifleman_1', gerSquad);
    const fullHp = await health(v);
    const to = await pos(v);
    // Separate the two throwers so the first blast doesn't kill the second during the fuse.
    await r.evaluate(({ x, z }) => window.suppressLoop.walkTo(x, z + 7.5), to);
    evidence.initial = { health: fullHp, victim: to, medic: await pos(m), shooter: await pos(s), secondShooter: await pos(r) };
    await expect(fx).toHaveAttribute('data-state', 'clear');

    const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
    await Promise.all([
      s.evaluate(() => window.suppressLoop.aim(0, -Math.PI / 2)),
      r.evaluate(() => window.suppressLoop.aim(Math.PI / 2, -Math.PI / 2)),
    ]);
    await pause(400);
    // Two overlapping in-game frag events raise suppression above the HUD threshold while
    // leaving the victim alive and above the HUD's low-health priority threshold.
    const bothBlasts = () => expect.poll(() => v.evaluate(() => window.suppressLoop.blasts().length),
      { timeout: 9_000, intervals: [20, 50] }).toBe(2);
    // A one-command pulse avoids multiple throws if TCP loss interrupts a held GADGET bit.
    // Pitch down and stagger the pulses so both frags come to rest before they detonate.
    await s.evaluate(() => window.suppressLoop.throwFrag());
    await pause(200);
    await r.evaluate(() => window.suppressLoop.throwFrag());
    try { await bothBlasts(); } finally {
      evidence.blastProbe = await Promise.all([v, s, r].map((p) => p.evaluate(() => ({
        blasts: window.suppressLoop.blasts(), suppress: window.suppressLoop.suppress(), diag: window.suppressLoop.diag(),
      }))));
    }
    const blasts = await v.evaluate(() => window.suppressLoop.blasts());
    evidence.blasts = { positions: blasts, health: await health(v), events: await v.evaluate(() => window.suppressLoop.suppress()) };
    await expect(fx).toHaveAttribute('data-state', 'suppressed');
    const intensity = Number(await fx.getAttribute('data-intensity'));
    const suppressed = await v.evaluate(() => Math.max(0, ...window.suppressLoop.suppress()));
    expect(intensity).toBeCloseTo(suppressed * 0.8);
    expect(suppressed).toBeGreaterThan(0.15);
    let activeAudio: Awaited<ReturnType<typeof audio>> | undefined;
    await expect.poll(async () => {
      activeAudio = await audio(v);
      return activeAudio.cutoff! < 20000 && activeAudio.gain! < 1;
    }, { intervals: [20, 50] }).toBe(true);
    const suppressedHp = await health(v);
    expect(suppressedHp).toBeGreaterThan(fullHp * 0.25);
    expect(suppressedHp).toBeLessThan(fullHp);
    evidence.suppression = { blasts, health: suppressedHp, busIntensity: suppressed, domIntensity: intensity, ...activeAudio };
    await expect.poll(() => audio(v)).toEqual({ context: 'running', cutoff: 20000, gain: 1 });
    await expect(fx).toHaveAttribute('data-state', 'clear', { timeout: 6_000 });
    await expect.poll(() => health(s), { timeout: 30_000 }).toBe(fullHp); // The frag thrower respawns.
    await s.evaluate(({ x, z }) => window.suppressLoop.walkTo(x - 3, z), to);
    await pause(300); // Let movement and prediction settle before aiming.
    const medicStart = await pos(m);
    await m.evaluate(() => window.suppressLoop.aim(Math.PI, 0));
    await hold(m, 'KeyW', () => expect.poll(async () => medicStart.z - (await pos(m)).z, { intervals: [20] }).toBeGreaterThan(0.8));
    const medicAt = await pos(m);
    expect(Math.hypot(medicAt.x - to.x, medicAt.z - to.z)).toBeLessThan(1.5);
    // Restore the frag damage and stop bleed with the real medic interaction. Two limb hits
    // then enter Downed even at the upper damage bound, without crossing the overkill limit.
    await hold(m, 'KeyJ', () => expect.poll(() => health(v), { timeout: 16_000 }).toBe(fullHp));

    const lifeEvents = async () => (await events(v)).filter((e) => e.type === 'death' || e.type === 'revive');
    const down = { type: 'death', victim: victim.id, canBeRevived: true, respawnAfter: 0 };
    const revive = { type: 'revive', medic: medic.id, target: victim.id };
    const downVictim = async () => {
      await pointAt(s, v, 0.49);
      await hold(s, 'Mouse2', async () => {
        await pause(600); // Allow the production ADS transition and TCP retransmission to finish.
        for (let shot = 0; shot < 2 && await deathPanel.count() === 0; shot++) {
          await hold(s, 'Mouse0', () => pause(220));
          await pause(1700); // Bolt cycle and any queued snapshot for this shot.
        }
      });
      evidence.lastDown = { health: await health(v), messages: await lifeEvents(), attacker: await s.evaluate(() => window.suppressLoop.diag()) };
      await expect(deathPanel).toHaveAttribute('data-phase', 'downed');
      await expect(fx).toHaveAttribute('data-state', 'downed');
      await expect.poll(async () => (await health(v)) < fullHp && (await health(v)) > 0).toBe(true);
      await expect.poll(async () => (await events(m)).some((e) => e.type === 'roster' && e.player === victim.id && (e.flags & 1) !== 0)).toBe(true);
      return health(v);
    };
    const firstDownHp = await downVictim();
    expect(await lifeEvents()).toEqual([down]);
    await hold(m, 'KeyJ', () => expect.poll(lifeEvents, { timeout: 10_000 }).toEqual([down, revive]));
    await expect(deathPanel).toHaveCount(0);
    await expect(fx).not.toHaveAttribute('data-state', 'downed');
    const secondDownHp = await downVictim();
    expect(await lifeEvents()).toEqual([down, revive, down]);
    await hold(v, 'KeyR', () => expect(deathPanel).toHaveAttribute('data-phase', 'dead', { timeout: 25_000 }));
    await expect.poll(() => health(v)).toBe(0);
    await expect(v.locator('[data-hud]')).toHaveCount(0);
    const kia = { type: 'death', victim: victim.id, canBeRevived: false, respawnAfter: expect.any(Number) };
    expect(await lifeEvents()).toEqual([down, revive, down, kia]);
    const deathTick = await v.evaluate(() => window.suppressLoop.tick());

    await expect.poll(() => health(v), { timeout: 45_000, intervals: [100, 250] }).toBe(fullHp);
    await expect(deathPanel).toHaveCount(0);
    await expect(fx).toHaveAttribute('data-state', 'clear');
    const respawnTick = await v.evaluate(() => window.suppressLoop.tick());
    const respawnPos = await pos(v);
    expect(respawnTick).toBeGreaterThan(deathTick);
    expect(Math.hypot(respawnPos.x - usHq[0]!, respawnPos.z - usHq[1]!)).toBeLessThan(5);
    const rosterLife = (await events(v)).filter((e) => e.type === 'roster' && e.player === victim.id).map((e) => e.type === 'roster' ? e.flags & (ROSTER_ALIVE | 1) : 0);
    // Collapse non-life changes (e.g. squad assignment) from the authoritative roster.
    const states = rosterLife.filter((state, i) => state !== rosterLife[i - 1]);
    expect(states.slice(-6)).toEqual([ROSTER_ALIVE, 1, ROSTER_ALIVE, 1, 0, ROSTER_ALIVE]);
    expect((await events(m)).filter((e) => e.type === 'death')).toHaveLength(0);
    expect((await events(s)).filter((e) => (e.type === 'death' && e.victim === victim.id) || e.type === 'revive')).toHaveLength(0);
    expect(errors).toEqual([]);
    evidence.life = { messages: await lifeEvents(), rosterStates: states, firstDownHp, secondDownHp, deathTick, respawnTick, respawnPos };
  } finally {
    console.log(`[suppress-loop] ${JSON.stringify(evidence)}`);
    await testInfo.attach('evidence', { body: JSON.stringify({ ...evidence, errors }, null, 2), contentType: 'application/json' });
    await context.close();
    await vite?.close();
    await server?.stop();
    removeDir(dir);
  }
});
