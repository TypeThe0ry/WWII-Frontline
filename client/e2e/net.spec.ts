import { mkdirSync, writeFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import {
  type DevCert,
  freePorts,
  type Harness,
  harnessQuery,
  HAVE_BINARIES,
  makeDevCert,
  removeDir,
  REPO_ROOT,
  type ServerProc,
  SKIP_REASON,
  startHarness,
  startServer,
  startUdpBlackhole,
} from './net-support';

// M0-021: the real client net module (WebTransport / WebSocket / fallback / handshake / disconnect) in headless Chromium
// against a real frontline-server on loopback. The page is served by a real localhost HTTP server (a Playwright
// route-faked page is refused by the Local Network Access check when it opens a WebTransport session).
const required = process.env.FRONTLINE_REQUIRE_SERVER === '1';
test.skip(!HAVE_BINARIES && !required, SKIP_REASON);
test.beforeAll(() => {
  if (!HAVE_BINARIES && required) throw new Error(SKIP_REASON);
});
test.describe.configure({ mode: 'serial' });

interface Info {
  status: string;
  info: Record<string, string | number> | undefined;
  error: { code: string; reason: string | undefined; message: string } | undefined;
  events: string[];
  log: string[];
  snapshots: number;
}

let cert: DevCert;
let harness: Harness;
const servers: ServerProc[] = [];
const measurements: Record<string, unknown> = {};

test.beforeAll(async () => {
  if (!HAVE_BINARIES) return;
  cert = makeDevCert();
  harness = await startHarness();
});
test.afterAll(async () => {
  for (const s of servers.splice(0)) await s.stop();
  await harness?.close();
  if (cert) removeDir(cert.dir);
  if (Object.keys(measurements).length > 0) {
    const dir = `${REPO_ROOT}/reports/m0`;
    mkdirSync(dir, { recursive: true });
    writeFileSync(`${dir}/M0-021-net-e2e.json`, JSON.stringify(measurements, null, 2) + '\n');
  }
});

async function server(o: Parameters<typeof startServer>[0] = { cert }): Promise<ServerProc> {
  const s = await startServer(o);
  servers.push(s);
  return s;
}

async function stopServer(s: ServerProc): Promise<void> {
  await s.stop();
  servers.splice(servers.indexOf(s), 1);
}

async function open(page: Page, srv: ServerProc, extra: Record<string, string> = {}): Promise<void> {
  await page.goto(`${harness.url}${harnessQuery(srv, extra)}`);
}

async function settled(page: Page, timeout = 15_000): Promise<Info> {
  await page.waitForFunction(() => ['connected', 'failed'].includes(window.__net?.status ?? ''), undefined, { timeout });
  return snapshot(page);
}

function snapshot(page: Page): Promise<Info> {
  return page.evaluate(() => {
    const n = window.__net;
    return { status: n.status, info: n.info as Info['info'], error: n.error, events: [...n.events], log: [...n.log], snapshots: n.snapshots };
  });
}

/** Sends inputs for a while and returns the smoothed RTT, jitter and the number of C2 datagrams that came back. */
async function exercise(page: Page, ms: number): Promise<{ rtt: number; jitter: number; echoes: number; stats: Record<string, unknown> }> {
  await page.evaluate(async (dur) => {
    const t0 = performance.now();
    while (performance.now() - t0 < dur) {
      window.__net.sendInput();
      await new Promise((r) => setTimeout(r, 33));
    }
  }, ms);
  return page.evaluate(() => {
    const s = window.__net.session!;
    return { rtt: s.clock.rtt(), jitter: s.clock.jitter(), echoes: window.__net.snapshots, stats: { ...s.stats() } };
  });
}

for (const path of ['wt', 'ws'] as const) {
  test(`connects over ${path === 'wt' ? 'WebTransport' : 'WebSocket'} (transport=${path}), Welcome fields, C1 up / C2 down, clock`, async ({ page }) => {
    const srv = await server();
    expect(srv.wtPort).toBeDefined();
    await open(page, srv, path === 'wt' ? { certhash: cert.hashHex } : { transport: 'ws' });
    const r = await settled(page);
    expect(r.error).toBeUndefined();
    expect(r.status).toBe('connected');
    expect(r.info?.transport).toBe(path);
    expect(r.info?.fallback_reason).toBe('none');
    expect(r.info?.tick_hz).toBe(30);
    expect(BigInt(r.info?.match_seed ?? 0)).not.toBe(0n);
    expect(r.events).toHaveLength(1);
    expect(r.events[0]).toMatch(new RegExp(`^connected:${path}:`));
    await expect(page.locator('#status')).toContainText(`transport=${path}`);

    const x = await exercise(page, 3500);
    expect(x.rtt).toBeGreaterThan(0);
    expect(x.rtt).toBeLessThan(250); // loopback; generous bound for a loaded VM
    if (srv.echo) expect(x.echoes).toBeGreaterThan(20); // --dev-echo answers each C1 bundle with a C2 packet
    const est1 = await page.evaluate(() => window.__net.session!.clock.estimatedServerTick());
    await page.waitForTimeout(1000);
    const est2 = await page.evaluate(() => window.__net.session!.clock.estimatedServerTick());
    expect(est2 - est1).toBeGreaterThanOrEqual(28); // ~30 ticks per second (loaded VM: not exact)
    expect(est2 - est1).toBeLessThanOrEqual(33);
    measurements[path] = {
      connect_ms: Number(r.info?.connect_ms),
      handshake_rtt_ms: Number(r.info?.handshake_rtt_ms),
      loopback_rtt_ewma_ms: Number(x.rtt.toFixed(3)),
      jitter_ms: Number(x.jitter.toFixed(3)),
      c2_received: x.echoes,
      stats: x.stats,
    };
    await page.evaluate(() => window.__net.close());
  });
}

test('auto: WebTransport disabled in the browser -> WebSocket, no WT attempt', async ({ page }) => {
  await page.addInitScript(() => {
    delete (window as { WebTransport?: unknown }).WebTransport;
  });
  const srv = await server();
  await open(page, srv, { certhash: cert.hashHex });
  const r = await settled(page);
  expect(r.info?.transport).toBe('ws');
  expect(r.info?.fallback_reason).toBe('WebTransport undefined');
  expect(r.log.some((l) => l.startsWith('wt:'))).toBe(true); // 'wt: skipped'
  expect(r.log.some((l) => l.startsWith('wt: connecting'))).toBe(false);
  await expect(page.locator('#status')).toContainText('transport=ws');
});

test('auto: WebTransport ready() rejected (self-signed certificate without certhash) -> immediate WebSocket fallback', async ({ page }) => {
  const srv = await server();
  await open(page, srv); // no certhash: the browser cannot validate the dev certificate
  const r = await settled(page);
  expect(r.info?.transport).toBe('ws');
  expect(String(r.info?.fallback_reason)).not.toBe('none');
  expect(Number(r.info?.connect_ms)).toBeLessThan(3000); // failed fast, no 3 s wait
  measurements['fallback_ready_rejected'] = { connect_ms: Number(r.info?.connect_ms), reason: r.info?.fallback_reason };
});

test('auto: wrong certificate hash -> fallback to WebSocket', async ({ page }) => {
  const srv = await server();
  await open(page, srv, { certhash: '00'.repeat(32) });
  const r = await settled(page);
  expect(r.info?.transport).toBe('ws');
  expect(Number(r.info?.connect_ms)).toBeLessThan(5000);
});

test('auto: WebTransport port is silent (UDP blackhole, no ICMP) -> fallback after 3 s, WebSocket handshake done within 5 s of starting', async ({ page }) => {
  const srv = await server();
  const hole = await startUdpBlackhole();
  try {
    await open(page, srv, { certhash: cert.hashHex, wtport: String(hole.port) });
    const r = await settled(page, 15_000);
    expect(r.info?.transport).toBe('ws');
    const ms = Number(r.info?.connect_ms);
    measurements['fallback_wt_blackhole'] = { connect_ms: ms, reason: r.info?.fallback_reason };
    expect(ms).toBeGreaterThanOrEqual(2900); // waited for the WT session until the 3 s deadline
    expect(ms).toBeLessThanOrEqual(5000); // E0-7: from starting to connect until the WS handshake is done
    expect(String(r.info?.fallback_reason)).toMatch(/not ready after 3000 ms/);
  } finally {
    await hole.close();
  }
});

// A port nobody listens on: Linux answers the QUIC packets with ICMP port unreachable, Chromium fails the session at once.
test('auto: WebTransport port closed (ICMP unreachable) -> immediate fallback', async ({ page }) => {
  const srv = await server();
  const dead = (await freePorts()).wt;
  await open(page, srv, { certhash: cert.hashHex, wtport: String(dead) });
  const r = await settled(page);
  expect(r.info?.transport).toBe('ws');
  measurements['fallback_wt_port_closed'] = { connect_ms: Number(r.info?.connect_ms), reason: r.info?.fallback_reason };
  expect(Number(r.info?.connect_ms)).toBeLessThanOrEqual(5000);
});

for (const path of ['wt', 'ws'] as const) {
  test(`handshake rejection is shown, not silent (${path})`, async ({ page }) => {
    const srv = await server();
    const base: Record<string, string> = path === 'wt' ? { certhash: cert.hashHex } : { transport: 'ws' };

    await open(page, srv, { ...base, badproto: '1' });
    let r = await settled(page);
    expect(r.status).toBe('failed');
    expect(r.error).toMatchObject({ code: 'rejected', reason: 'version_mismatch' });
    await expect(page.locator('#status')).toContainText('error=rejected:version_mismatch Server rejected the connection: version_mismatch');
    expect(r.events).toEqual([]); // no net.connected
    expect(r.log.some((l) => l.startsWith('ws:'))).toBe(path === 'ws'); // a Reject over WT must not fall back to WS

    await open(page, srv, { ...base, baddata: '1' });
    r = await settled(page);
    expect(r.error).toMatchObject({ code: 'rejected', reason: 'data_hash_mismatch' });
    await expect(page.locator('#status')).toContainText('game data differ');
  });
}

for (const path of ['wt', 'ws'] as const) {
  test(`server goes away -> net.disconnected; the client can connect again to the restarted server (${path})`, async ({ page }) => {
    const ports = await freePorts();
    const srv = await server({ cert, wsPort: ports.ws, wtPort: ports.wt });
    await open(page, srv, path === 'wt' ? { certhash: cert.hashHex } : { transport: 'ws' });
    const first = await settled(page);
    expect(first.info?.transport).toBe(path);
    const player1 = first.info?.player_id;

    await stopServer(srv); // SIGTERM: graceful shutdown closes the connections
    await page.waitForFunction(() => window.__net.status === 'disconnected', undefined, { timeout: 20_000 });
    const gone = await snapshot(page);
    expect(gone.events.at(-1)).toMatch(/^disconnected:.+/);
    await expect(page.locator('#status')).toContainText('disconnect=');

    // while the server is down, a new attempt fails visibly (no hang)
    await page.evaluate(() => void window.__net.connect());
    const down = await settled(page, 20_000);
    expect(down.status).toBe('failed');
    expect(down.error?.code).toBe('transport_failed');
    await expect(page.locator('#status')).toContainText('error=transport_failed');

    await server({ cert, wsPort: ports.ws, wtPort: ports.wt });
    await page.evaluate(() => void window.__net.connect());
    const again = await settled(page);
    expect(again.status).toBe('connected');
    expect(again.info?.transport).toBe(path);
    expect(again.events.at(-1)).toMatch(new RegExp(`^connected:${path}:`));
    expect(player1).toBeDefined();
    const x = await exercise(page, 600);
    expect(x.rtt).toBeGreaterThan(0);
    await page.evaluate(() => window.__net.close());
  });
}
