import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { type Browser, chromium, expect, firefox, type Page, test, webkit } from '@playwright/test';

const pw = { chromium, firefox, webkit };
import {
  type DevCert,
  type Harness,
  harnessQuery,
  HAVE_BINARIES,
  makeDevCert,
  removeDir,
  REPO_ROOT,
  SKIP_REASON,
  startHarness,
} from '../net-support';
import {
  BROWSERS,
  type BrowserName,
  type FallbackResult,
  FALLBACK_LIMIT_MS,
  type Matrix,
  median,
  PROBE_SECONDS,
  probePassed,
  type ProbeResult,
  renderTable,
  TRANSPORTS,
  type Untested,
} from './fixtures';
import { startInteropServer } from './server';

// M0-028: WT / WS interop per browser against a real frontline-server. Builds on the M0-021 net harness page
// (client/e2e/net-harness, which uses the real net module `connect()` and sim-wasm) instead of a second probe page.
// Browsers that are not installed are recorded as "untested" with the reason (never as passed). Output:
// reports/m0/browser-matrix.json + reports/m0/browser-matrix.md.
const required = process.env.FRONTLINE_REQUIRE_SERVER === '1';
test.skip(!HAVE_BINARIES && !required, SKIP_REASON);
test.beforeAll(() => {
  if (!HAVE_BINARIES && required) throw new Error(SKIP_REASON);
});


const wanted = (process.env.FRONTLINE_INTEROP_BROWSERS ?? BROWSERS.join(',')).split(',').filter((b): b is BrowserName => (BROWSERS as readonly string[]).includes(b));

let cert: DevCert;
let harness: Harness;
const MATRIX_DIR = `${REPO_ROOT}/reports/m0`;
const MATRIX_JSON = `${MATRIX_DIR}/browser-matrix.json`;

// A failing test restarts the Playwright worker (module state is lost), so every result is merged into the file at once.
// global-setup.ts deletes the file at the start of a run.
function record(update: (m: Matrix) => void): void {
  const m: Matrix = existsSync(MATRIX_JSON)
    ? (JSON.parse(readFileSync(MATRIX_JSON, 'utf8')) as Matrix)
    : {
        generatedAt: '',
        note: 'Playwright-provided browser builds, headless, loopback. WebKit build != Safari release. Untested = not run on this machine, not a failure.',
        probes: [],
        fallbackNoWebTransport: [],
        fallbackWtPortClosed: [],
        untested: [],
      };
  update(m);
  m.generatedAt = new Date().toISOString();
  mkdirSync(MATRIX_DIR, { recursive: true });
  writeFileSync(MATRIX_JSON, JSON.stringify(m, null, 2) + '\n');
  writeFileSync(`${MATRIX_DIR}/browser-matrix.md`, renderTable(m) + '\n');
}

test.beforeAll(async () => {
  if (!HAVE_BINARIES) return;
  cert = makeDevCert();
  harness = await startHarness();
});
test.afterAll(async () => {
  await harness?.close();
  if (cert) removeDir(cert.dir);
});

function installed(name: BrowserName): string | undefined {
  const path = pw[name].executablePath();
  return existsSync(path) ? undefined : `browser binary not installed (${path}); run \`npx playwright install ${name}\` on a machine that can`;
}

interface Settled {
  status: string;
  info: Record<string, string | number> | undefined;
  error: { code: string; reason: string | undefined; message: string } | undefined;
  log: string[];
}

for (const name of wanted) {
  test.describe(name, () => {
    let browser: Browser | undefined;
    let version = 'n/a';

    test.beforeAll(async () => {
      const missing = installed(name);
      if (missing) {
        record((m) => m.untested.push({ browser: name, reason: missing } satisfies Untested));
        return;
      }
      try {
        browser = await pw[name].launch();
        version = browser.version();
      } catch (e) {
        const reason = `launch failed: ${String(e instanceof Error ? e.message : e).split('\n')[0]}`;
        record((m) => m.untested.push({ browser: name, reason }));
      }
    });
    test.afterAll(async () => {
      await browser?.close();
    });

    async function newPage(opts: { noWebTransport?: boolean } = {}): Promise<Page> {
      const ctx = await browser!.newContext();
      const page = await ctx.newPage();
      if (opts.noWebTransport) {
        await page.addInitScript(() => {
          delete (window as { WebTransport?: unknown }).WebTransport;
        });
      }
      return page;
    }

    async function settled(page: Page, timeout = 20_000): Promise<Settled> {
      await page.waitForFunction(() => ['connected', 'failed'].includes(window.__net?.status ?? ''), undefined, { timeout });
      return page.evaluate(() => ({ status: window.__net.status, info: window.__net.info as Settled['info'], error: window.__net.error, log: [...window.__net.log] }));
    }

    for (const transport of TRANSPORTS) {
      test(`${transport}: handshake, ${PROBE_SECONDS} s of InputBundle -> snapshots, RTT`, async () => {
        test.skip(!browser, `${name} not available`);
        test.setTimeout(60_000);
        const srv = await startInteropServer(cert);
        const page = await newPage();
        const r: ProbeResult = {
          browser: name, version, transport, supported: false, serverCertificateHashesWorked: false,
          connected: false, snapshotsReceived: 0, disconnects: 0,
        };
        try {
          await page.goto(`${harness.url}${harnessQuery(srv, transport === 'wt' ? { certhash: cert.hashHex, transport: 'wt' } : { transport: 'ws' })}`);
          r.supported = await page.evaluate(() => typeof (window as { WebTransport?: unknown }).WebTransport !== 'undefined');
          const s = await settled(page);
          if (s.status === 'connected') {
            r.connected = true;
            r.usedTransport = String(s.info?.transport);
            r.serverCertificateHashesWorked = transport === 'wt' && r.usedTransport === 'wt';
            const out = await page.evaluate(async (ms) => {
              const rtts: number[] = [];
              const t0 = performance.now();
              let last = 0;
              const start = window.__net.snapshots;
              while (performance.now() - t0 < ms) {
                window.__net.sendInput();
                if (performance.now() - last >= 250) {
                  last = performance.now();
                  rtts.push(window.__net.session!.clock.rtt());
                }
                await new Promise((res) => setTimeout(res, 33));
              }
              return { snapshots: window.__net.snapshots - start, rtts, status: window.__net.status, events: [...window.__net.events] };
            }, PROBE_SECONDS * 1000);
            r.snapshotsReceived = out.snapshots;
            r.rttMsMedian = median(out.rtts.filter((x) => x > 0));
            r.disconnects = out.events.filter((e) => e.startsWith('disconnected')).length + (out.status === 'connected' ? 0 : 1);
          } else {
            r.error = `${s.error?.code ?? 'error'}${s.error?.reason ? ':' + s.error.reason : ''}: ${s.error?.message ?? ''} | log: ${s.log.join(' ; ')}`;
          }
        } catch (e) {
          r.error = String(e instanceof Error ? e.message : e);
        } finally {
          record((m) => m.probes.push(r));
          await page.context().close();
          await srv.stop();
        }
        // Chromium is the only browser with a hard requirement; the others are recorded as they are.
        if (name === 'chromium') {
          expect(r.error).toBeUndefined();
          expect(probePassed(r)).toBe(true);
          expect(r.rttMsMedian ?? Infinity).toBeLessThan(20);
        }
      });
    }

    test('fallback: delete WebTransport, auto -> WebSocket within 5 s', async () => {
      test.skip(!browser, `${name} not available`);
      const srv = await startInteropServer(cert);
      const page = await newPage({ noWebTransport: true });
      const f: FallbackResult = { browser: name, version, connected: false };
      try {
        const t0 = Date.now();
        await page.goto(`${harness.url}${harnessQuery(srv, { certhash: cert.hashHex })}`);
        const s = await settled(page);
        const wall = Date.now() - t0;
        f.connected = s.status === 'connected';
        f.transport = s.info?.transport === undefined ? undefined : String(s.info.transport);
        f.connectMs = Number(s.info?.connect_ms ?? wall); // client-side: from starting to connect until the handshake is done
        f.fallbackReason = s.info?.fallback_reason === undefined ? undefined : String(s.info.fallback_reason);
        if (!f.connected) f.error = `${s.error?.code}: ${s.error?.message}`;
      } catch (e) {
        f.error = String(e instanceof Error ? e.message : e);
      } finally {
        record((m) => m.fallbackNoWebTransport.push(f));
        await page.context().close();
        await srv.stop();
      }
      expect(f.error).toBeUndefined();
      expect(f.transport).toBe('ws');
      expect(f.connectMs ?? Infinity).toBeLessThanOrEqual(FALLBACK_LIMIT_MS);
    });

    if (name === 'chromium') {
      test('fallback: server serves no WebTransport port (WS only), auto -> WebSocket within 5 s', async () => {
        test.skip(!browser, `${name} not available`);
        const srv = await startInteropServer(); // no certificate: no WT listener at all
        expect(srv.wtPort).toBeUndefined();
        const page = await newPage();
        const f: FallbackResult = { browser: name, version, connected: false };
        try {
          await page.goto(`${harness.url}${harnessQuery(srv, { certhash: cert.hashHex })}`);
          const s = await settled(page);
          f.connected = s.status === 'connected';
          f.transport = s.info?.transport === undefined ? undefined : String(s.info.transport);
          f.connectMs = Number(s.info?.connect_ms);
          f.fallbackReason = s.info?.fallback_reason === undefined ? undefined : String(s.info.fallback_reason);
          if (!f.connected) f.error = `${s.error?.code}: ${s.error?.message}`;
        } catch (e) {
          f.error = String(e instanceof Error ? e.message : e);
        } finally {
          record((m) => m.fallbackWtPortClosed.push(f));
          await page.context().close();
          await srv.stop();
        }
        expect(f.error).toBeUndefined();
        expect(f.transport).toBe('ws');
        expect(f.connectMs ?? Infinity).toBeLessThanOrEqual(FALLBACK_LIMIT_MS);
      });
    }
  });
}
