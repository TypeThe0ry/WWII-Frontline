// Process helpers for net.spec.ts: a real frontline-server, a dev certificate and a localhost HTTP server for the harness page.
import { type ChildProcess, execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { createSocket } from 'node:dgram';
import { createServer as createTcpServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer, type ViteDevServer } from 'vite';

const CLIENT_DIR = fileURLToPath(new URL('../', import.meta.url));
export const REPO_ROOT = resolve(CLIENT_DIR, '..');
export const HARNESS_DIR = resolve(CLIENT_DIR, 'e2e/net-harness');
export const WASM_JS = `${REPO_ROOT}/crates/sim-wasm/pkg/frontline_sim_wasm.js`;
const DEBUG_BIN = `${REPO_ROOT}/target/debug`;

/** `FRONTLINE_SERVER_BIN` / `FRONTLINE_GEN_CERT_BIN` override the cargo debug build outputs. */
export const SERVER_BIN = process.env.FRONTLINE_SERVER_BIN ?? `${DEBUG_BIN}/frontline-server`;
export const GEN_CERT_BIN = process.env.FRONTLINE_GEN_CERT_BIN ?? `${DEBUG_BIN}/gen_dev_cert`;

export const HAVE_BINARIES = existsSync(SERVER_BIN) && existsSync(GEN_CERT_BIN) && existsSync(WASM_JS);
export const SKIP_REASON =
  'needs target/debug/{frontline-server,gen_dev_cert} (cargo build -p frontline-server) and the sim-wasm package ' +
  '(wasm-pack build crates/sim-wasm --target web --profile wasm-release); FRONTLINE_SERVER_BIN / FRONTLINE_GEN_CERT_BIN override the paths';

export interface DevCert {
  dir: string;
  cert: string;
  key: string;
  hashHex: string;
}

export function makeDevCert(): DevCert {
  const dir = mkdtempSync(join(tmpdir(), 'fl-net-cert-'));
  execFileSync(GEN_CERT_BIN, ['--out', dir], { stdio: 'pipe' });
  return { dir, cert: `${dir}/dev-cert.pem`, key: `${dir}/dev-key.pem`, hashHex: /hex=([0-9a-f]{64})/.exec(readFileSync(`${dir}/dev-cert.sha256`, 'utf8'))?.[1] ?? '' };
}

export function removeDir(dir: string): void {
  rmSync(dir, { recursive: true, force: true });
}

/** A free TCP port (WebSocket) and a free UDP port (QUIC), found by binding to port 0 and releasing. */
export async function freePorts(): Promise<{ ws: number; wt: number }> {
  const tcp = await new Promise<number>((res, rej) => {
    const s = createTcpServer();
    s.once('error', rej);
    s.listen(0, '127.0.0.1', () => {
      const p = (s.address() as { port: number }).port;
      s.close(() => res(p));
    });
  });
  const udp = await new Promise<number>((res, rej) => {
    const s = createSocket('udp4');
    s.once('error', rej);
    s.bind(0, '127.0.0.1', () => {
      const p = s.address().port;
      s.close(() => res(p));
    });
  });
  return { ws: tcp, wt: udp };
}

/** A UDP socket that swallows everything: a QUIC handshake towards it never completes and never errors (no ICMP). */
export async function startUdpBlackhole(): Promise<{ port: number; close(): Promise<void> }> {
  const sock = createSocket('udp4');
  sock.on('message', () => undefined);
  const port = await new Promise<number>((res, rej) => {
    sock.once('error', rej);
    sock.bind(0, '127.0.0.1', () => res(sock.address().port));
  });
  return { port, close: () => new Promise<void>((r) => sock.close(() => r())) };
}

export interface ServerProc {
  proc: ChildProcess;
  wsPort: number;
  wtPort: number | undefined;
  /** JSON log lines seen so far. */
  lines: Record<string, unknown>[];
  /** Whether `--dev-echo` was accepted (C1 bundle -> C2 echo). */
  echo: boolean;
  stop(): Promise<void>;
}

async function spawnServer(args: string[]): Promise<ServerProc> {
  const proc = spawn(SERVER_BIN, args, { stdio: ['ignore', 'pipe', 'pipe'], cwd: REPO_ROOT });
  const lines: Record<string, unknown>[] = [];
  let stderr = '';
  proc.stderr?.on('data', (d) => (stderr += d.toString()));
  let buf = '';
  let exited = false;
  proc.once('exit', () => (exited = true));
  proc.stdout?.on('data', (d) => {
    buf += d.toString();
    const parts = buf.split('\n');
    buf = parts.pop() ?? '';
    for (const l of parts) {
      try {
        lines.push(JSON.parse(l) as Record<string, unknown>);
      } catch {
        /* not a log line */
      }
    }
  });
  const find = (event: string): Record<string, unknown> | undefined => lines.find((l) => l.event === event);
  const wantWt = args.includes('--cert') || args.includes('--dev-cert');
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (exited) throw new Error(`frontline-server exited early: ${stderr.slice(0, 400)} ${JSON.stringify(lines.slice(-2))}`);
    if (find('ws_listening') && (!wantWt || find('wt_listening'))) break;
    await new Promise((r) => setTimeout(r, 25));
  }
  const ws = find('ws_listening');
  if (!ws) {
    proc.kill('SIGKILL');
    throw new Error('frontline-server did not report ws_listening');
  }
  const wt = find('wt_listening');
  return {
    proc,
    wsPort: ws.port as number,
    wtPort: wt?.port as number | undefined,
    lines,
    echo: args.includes('--dev-echo'),
    async stop() {
      if (exited) return;
      const done = new Promise<void>((r) => proc.once('exit', () => r()));
      proc.kill('SIGTERM');
      const t = setTimeout(() => proc.kill('SIGKILL'), 5000);
      await done;
      clearTimeout(t);
    },
  };
}

export interface StartOptions {
  cert?: DevCert;
  wsPort?: number;
  wtPort?: number;
}

/** Starts the server (ephemeral ports unless given). Tries `--dev-echo` first and falls back without it. */
export async function startServer(o: StartOptions = {}): Promise<ServerProc> {
  const base = ['--ws-port', String(o.wsPort ?? 0), '--wt-port', String(o.wtPort ?? 0), '--metrics-port', '0'];
  if (o.cert) base.push('--cert', o.cert.cert, '--key', o.cert.key);
  try {
    return await spawnServer([...base, '--dev-echo']);
  } catch (e) {
    if (!/exited early/.test(String(e))) throw e;
    return spawnServer(base);
  }
}

export interface Harness {
  url: string;
  close(): Promise<void>;
}

/** Vite dev server for the harness page on a real localhost HTTP port (WebTransport refuses faked origins). */
export async function startHarness(): Promise<Harness> {
  const cacheDir = mkdtempSync(join(tmpdir(), 'fl-net-vite-'));
  const vite: ViteDevServer = await createServer({
    root: HARNESS_DIR,
    configFile: false,
    logLevel: 'error',
    cacheDir,
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, strictPort: false, fs: { allow: [REPO_ROOT] }, hmr: false, watch: null },
  });
  await vite.listen();
  const addr = vite.httpServer?.address();
  if (!addr || typeof addr === 'string') throw new Error('harness server has no address');
  return {
    url: `http://localhost:${addr.port}/`,
    async close() {
      await vite.close();
      removeDir(cacheDir);
    },
  };
}

/** Query string for the harness page. */
export function harnessQuery(srv: ServerProc, extra: Record<string, string> = {}): string {
  const q = new URLSearchParams({ server: `127.0.0.1:${srv.wsPort}`, wasm: `/@fs${WASM_JS}`, ...extra });
  if (srv.wtPort !== undefined && !('wtport' in extra)) q.set('wtport', String(srv.wtPort));
  return `?${q.toString()}`;
}
