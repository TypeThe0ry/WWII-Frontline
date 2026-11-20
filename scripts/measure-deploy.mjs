#!/usr/bin/env node
/**
 * E1-10 deployment & R_near prefetch timing harness (B6-deploytiming).
 *
 * Launches a real local frontline-gateway + frontline-server (Carentan M1 assault mode)
 * and a static/range/proxy HTTP server serving the production client/dist bundle plus the
 * real Carentan M1 terrain .fpak packs (380 chunks / 30 packs).
 *
 * Drives headless Playwright Chromium (SwiftShader WebGL2 software rendering) under CDP
 * network throttling (default: 20 Mbps / 50 ms RTT and 5 Mbps / 100 ms RTT) across cold
 * and warm OPFS caches (default: 5 runs each), measuring:
 *   - Boot -> Menu -> Join -> Combat HUD -> C0 DeployRequest/DeployResult -> C2 Alive (playable)
 *   - R_near (=4, 9x9=81 chunks) ChunkStreamer prefetch completion rate & timing via OPFS
 *
 * Non-reference workstation measurements must be annotated per PI-109.
 */
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { createServer as createHttpServer, request as httpRequest } from 'node:http';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { inflateSync } from 'node:zlib';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, '..');
const CLIENT_DIR = join(REPO_ROOT, 'client');
const DIST_DIR = join(CLIENT_DIR, 'dist');

const DEFAULT_PROFILES = [
  { name: '20mbps_50ms', label: '20 Mbps / 50 ms RTT', downMbps: 20, upMbps: 10, latencyMs: 50 },
  { name: '5mbps_100ms', label: '5 Mbps / 100 ms RTT', downMbps: 5, upMbps: 2, latencyMs: 100 },
];

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.geojson': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.fpak': 'application/octet-stream',
  '.bin': 'application/octet-stream',
  '.r16': 'application/octet-stream',
};

function parseArgs(argv) {
  const opts = {
    runs: 5,
    jsonOut: null,
    gatewayBin: process.env.FRONTLINE_GATEWAY_BIN ?? join(REPO_ROOT, 'target/debug/frontline-gateway'),
    serverBin: process.env.FRONTLINE_SERVER_BIN ?? join(REPO_ROOT, 'target/debug/frontline-server'),
    profiles: DEFAULT_PROFILES,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--runs' && argv[i + 1]) opts.runs = Number(argv[++i]);
    else if (a === '--json' && argv[i + 1]) opts.jsonOut = resolve(argv[++i]);
    else if (a === '--gateway-bin' && argv[i + 1]) opts.gatewayBin = resolve(argv[++i]);
    else if (a === '--server-bin' && argv[i + 1]) opts.serverBin = resolve(argv[++i]);
    else if (a === '--help' || a === '-h') {
      console.log('Usage: node scripts/measure-deploy.mjs [--runs 5] [--json reports/m1-deploy-timing.json] [--gateway-bin ...] [--server-bin ...]');
      process.exit(0);
    }
  }
  if (!Number.isInteger(opts.runs) || opts.runs < 1) throw new Error(`Invalid --runs: ${opts.runs}`);
  return opts;
}

function dataIndex(dir, id) {
  const files = readdirSync(join(REPO_ROOT, 'data', dir)).filter((f) => f.endsWith('.json')).sort();
  const idx = files.indexOf(`${id}.json`);
  if (idx < 0) throw new Error(`Missing data/${dir}/${id}.json`);
  return idx;
}

function runPython(args, env = {}) {
  const res = spawnSync('python3', args, {
    cwd: REPO_ROOT,
    env: { ...process.env, PYTHONPATH: join(REPO_ROOT, 'tools/map-pipeline'), ...env },
    encoding: 'utf8',
  });
  if (res.status !== 0) {
    throw new Error(`python3 ${args.join(' ')} failed (rc=${res.status}):\n${res.stdout}\n${res.stderr}`);
  }
  return res.stdout;
}

function prepareCarentanAssets(workDir) {
  const chunksDir = join(workDir, 'chunks');
  const fpakDir = join(workDir, 'fpak');
  const manifestPath = join(workDir, 'terrain_carentan.json');
  console.log('[measure-deploy] baking Carentan M1 chunks & .fpak into temp dir...');
  const pyOut = runPython([
    '-c',
    `
import json, sys
from pathlib import Path
from map_pipeline import chunker, pack_chunks
chunks_dir, fpak_dir, manifest_path = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
ctx, res, entries = chunker.bake(Path("data/maps/carentan"), chunks_dir, 4)
(chunks_dir / "index.json").write_text(json.dumps(chunker.index_doc(entries), indent=1, sort_keys=True) + "\\n", encoding="utf-8")
stats = pack_chunks.pack_chunks(chunks_dir, fpak_dir, manifest_path)
print(json.dumps({"chunks": len(entries), "fpak": stats}))
`,
    chunksDir,
    fpakDir,
    manifestPath,
  ]);
  const bakeReport = JSON.parse(pyOut.trim().split('\n').pop());

  // Prepare server data dir with decompressed FCOL chunks for --map carentan/m1 --mode assault.
  const dataDir = join(workDir, 'data');
  const srvMap = join(dataDir, 'maps/carentan/m1');
  const srvChunks = join(srvMap, 'chunks');
  mkdirSync(srvChunks, { recursive: true });
  writeFileSync(join(dataDir, 'maps/carentan/meta.json'), readFileSync(join(REPO_ROOT, 'data/maps/carentan/meta.json')));
  for (const f of ['spawns.json', 'zones.json']) {
    writeFileSync(join(srvMap, f), readFileSync(join(REPO_ROOT, 'data/maps/carentan/m1', f)));
  }
  let fcolCount = 0;
  for (const name of readdirSync(chunksDir)) {
    const m = /^c_(\d+)_(\d+)\.fcol$/.exec(name);
    if (!m) continue;
    const raw = inflateSync(readFileSync(join(chunksDir, name)));
    writeFileSync(join(srvChunks, `c_${m[1]}_${m[2]}.bin`), raw);
    fcolCount++;
  }
  console.log(`[measure-deploy] baked ${bakeReport.chunks} chunks (${bakeReport.fpak.packs} .fpak, ${bakeReport.fpak.total_fpak_bytes} B), decompressed ${fcolCount} FCOL`);
  return { chunksDir, fpakDir, manifestPath, dataDir, bakeReport };
}

async function buildProbeBundle(workDir) {
  const probeSrc = join(workDir, 'probe-entry.ts');
  const probeOut = join(workDir, 'probe-dist');
  const srcRel = (p) => join(CLIENT_DIR, 'src', p).replaceAll('\\', '/');
  writeFileSync(probeSrc, `
import { createDeployActions, HQ_SPAWN_ID } from '${srcRel('game/deploy-actions.ts')}';
import { AssetLoader, CacheLayer, opfsSyncBackend } from '${srcRel('render/assets/index.ts')}';
import { ChunkStreamer } from '${srcRel('render/world/chunk-streamer.ts')}';
import { ChunkIndex, fpakChunkSource } from '${srcRel('render/world/chunk-stream-source.ts')}';
import { R_NEAR } from '${srcRel('render/world/chunk-streamer-core.ts')}';

export interface ProbeOpts {
  manifestUrl: string;
  indexUrl: string;
  spawnPos: { x: number; z: number };
  ring?: number;
  roleIdx: number;
  loadoutByTeam: [number, number];
  clearOpfs: boolean;
  opfsDir: string;
}

export async function runDeployAndPrefetch(opts: ProbeOpts) {
  const net = (window as unknown as { __netGame?: any }).__netGame;
  if (!net) throw new Error('window.__netGame missing');

  // Initialize OPFS backend before timing the deploy click so clearing OPFS on cold runs is excluded.
  if (opts.clearOpfs) {
    const setupOpfs = await opfsSyncBackend({ dirName: opts.opfsDir });
    await setupOpfs.clear();
    setupOpfs.close?.();
  }
  const opfs = await opfsSyncBackend({ dirName: opts.opfsDir });
  // Small memory LRU ensures warm runs read .fpak entries from persistent OPFS rather than RAM.
  const cache = new CacheLayer({ backends: [opfs], memoryBytes: 2 * 1024 * 1024 });

  const [manifestJson, indexJson] = await Promise.all([
    fetch(opts.manifestUrl).then((r) => r.json()),
    fetch(opts.indexUrl).then((r) => r.json()),
  ]);
  const index = new ChunkIndex(indexJson);
  const loader = new AssetLoader({ base: '', manifest: manifestJson, cache, log: () => undefined });
  await loader.init({ tier: 'medium' });

  const source = fpakChunkSource(loader, index, (entry) => ({
    visual: \`terrain_carentan_b_\${entry.bx}_\${entry.bz}@chunk\`,
    collision: \`terrain_carentan_b_\${entry.bx}_\${entry.bz}@collision\`,
  }));

  let visualResident = 0;
  let collisionInstalled = 0;
  const ring = opts.ring ?? R_NEAR;
  const streamer = new ChunkStreamer({
    radius: ring,
    retainRadius: ring,
    hasChunk: source.hasChunk,
    load: source.load,
    retryLoad: false,
    upload: () => { visualResident++; },
    evict: () => { visualResident = Math.max(0, visualResident - 1); },
    hasFallback: () => true,
    collision: {
      load: source.loadCollision,
      install: () => { collisionInstalled++; },
      remove: () => { collisionInstalled = Math.max(0, collisionInstalled - 1); },
    },
  });

  const actions = createDeployActions(net.session.transport, net.wasm, 15_000);
  const team = net.session.welcome.team === 1 ? 1 : 0;
  const loadoutIdx = opts.loadoutByTeam[team];

  let pumping = true;
  const pump = () => {
    streamer.update(opts.spawnPos, { x: 0, z: 0 }, () => true, 1 / 60);
    if (pumping) requestAnimationFrame(pump);
  };
  requestAnimationFrame(pump);

  const tDeployStart = performance.now();
  const welcomeTick = net.game.state.net.snapshotTick.peek() || net.session.welcome.serverTick;

  // 1) Start R_near prefetch around the target spawn point in parallel with C0 DeployRequest.
  const prefetchPromise = streamer.prefetchAround(opts.spawnPos, { ring });

  // 2) Send real C0 DeployRequest and wait for C0 DeployResult + authoritative C2 spawn (health > 0).
  const c0Promise = (async () => {
    const res = await actions.deploy({
      spawnId: HQ_SPAWN_ID,
      role: opts.roleIdx,
      loadout: loadoutIdx,
      squadId: null,
    });
    const c0AckMs = performance.now() - tDeployStart;
    if (!res.ok) throw new Error(\`Deploy rejected: \${res.reasonKey} (\${res.message})\`);

    // Wait for authoritative C2 snapshot showing health > 0, plus one more C2 snapshot confirming live loop.
    const aliveInfo = await new Promise<{ spawnAliveMs: number; spawnTickActual: number; health: number }>((resolveAlive, rejectAlive) => {
      const deadline = setTimeout(() => rejectAlive(new Error('Timed out waiting for authoritative C2 spawn (health > 0)')), 25_000);
      let firstAliveTick = -1;
      const poll = () => {
        const hp = net.game.state.net.health.peek();
        const tick = net.game.state.net.snapshotTick.peek();
        if (hp > 0) {
          if (firstAliveTick < 0) firstAliveTick = tick;
          else if (tick > firstAliveTick) {
            clearTimeout(deadline);
            resolveAlive({ spawnAliveMs: performance.now() - tDeployStart, spawnTickActual: firstAliveTick, health: hp });
            return;
          }
        }
        requestAnimationFrame(poll);
      };
      requestAnimationFrame(poll);
    });

    return {
      c0Ok: res.ok,
      c0AckMs,
      spawnTickOrdered: res.spawnTick,
      welcomeTick,
      spawnWaitTicks: Math.max(0, res.spawnTick - welcomeTick),
      ...aliveInfo,
    };
  })();

  try {
    const [prefetch, c0] = await Promise.all([prefetchPromise, c0Promise]);
    const deployToPlayableMs = Math.max(prefetch.elapsedMs, c0.spawnAliveMs);
    return {
      team,
      cacheMode: cache.mode,
      opfsSync: opfs.sync,
      ring,
      prefetchTotal: prefetch.total,
      prefetchReady: prefetch.ready,
      prefetchFailed: prefetch.failed,
      prefetchRate: prefetch.total > 0 ? prefetch.ready / prefetch.total : 0,
      prefetchMs: prefetch.elapsedMs,
      visualResident,
      collisionInstalled,
      ...c0,
      deployToPlayableMs,
    };
  } finally {
    pumping = false;
    actions.dispose();
    streamer.dispose();
    source.dispose();
    cache.close();
  }
}

(window as unknown as { __deployProbe: unknown }).__deployProbe = { runDeployAndPrefetch };
`);

  const viteMod = await import(pathToFileURL(join(CLIENT_DIR, 'node_modules/vite/dist/node/index.js')).href);
  await viteMod.build({
    root: workDir,
    base: '/__probe/',
    logLevel: 'error',
    configFile: false,
    build: {
      outDir: probeOut,
      emptyOutDir: true,
      minify: false,
      target: 'esnext',
      lib: {
        entry: probeSrc,
        formats: ['es'],
        fileName: () => 'probe.js',
      },
    },
  });
  return probeOut;
}

function launchProc(bin, args, cwd, env = {}) {
  const child = spawn(bin, args, { cwd, env: { PATH: process.env.PATH, ...env }, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = '';
  let stderr = '';
  let exited = false;
  child.stdout?.on('data', (d) => { stdout += d.toString(); });
  child.stderr?.on('data', (d) => { stderr += d.toString(); });
  const closed = new Promise((res) => {
    const done = () => { exited = true; res(); };
    child.once('exit', done);
    child.once('error', (err) => { stderr += err.message; done(); });
  });
  return {
    pid: child.pid,
    output: () => stdout,
    error: () => stderr,
    async ready(read, timeoutMs = 15_000) {
      const deadline = Date.now() + timeoutMs;
      while (!exited && Date.now() < deadline) {
        const v = read();
        if (v !== undefined) return v;
        await new Promise((r) => setTimeout(r, 50));
      }
      throw new Error(`${bin} not ready:\n${stderr}\n${stdout.slice(-2000)}`);
    },
    async stop() {
      if (!exited && child.pid) child.kill('SIGTERM');
      await closed;
    },
  };
}

function serverEvent(proc, name) {
  for (const line of proc.output().split('\n')) {
    if (!line.startsWith('{')) continue;
    try {
      const obj = JSON.parse(line);
      if (obj.event === name) return obj;
    } catch { /* ignore */ }
  }
  return undefined;
}

function serveFileWithRange(req, res, filePath, onBytes) {
  if (!existsSync(filePath) || !statSync(filePath).isFile()) {
    res.writeHead(404);
    res.end('Not found');
    return;
  }
  const size = statSync(filePath).size;
  const contentType = MIME[extname(filePath)] ?? 'application/octet-stream';
  const range = req.headers.range;
  const data = readFileSync(filePath);
  if (range) {
    const suffix = /^bytes=-(\d+)$/.exec(range);
    const span = /^bytes=(\d+)-(\d*)$/.exec(range);
    let start = 0;
    let end = size - 1;
    if (suffix) {
      const n = Number(suffix[1]);
      start = Math.max(0, size - n);
    } else if (span) {
      start = Number(span[1]);
      end = span[2] ? Math.min(size - 1, Number(span[2])) : size - 1;
    } else {
      res.writeHead(416, { 'Content-Range': `bytes */${size}` });
      res.end();
      return;
    }
    if (start > end || start >= size) {
      res.writeHead(416, { 'Content-Range': `bytes */${size}` });
      res.end();
      return;
    }
    const slice = data.subarray(start, end + 1);
    onBytes?.(slice.length);
    res.writeHead(206, {
      'Content-Type': contentType,
      'Content-Length': String(slice.length),
      'Content-Range': `bytes ${start}-${end}/${size}`,
      'Accept-Ranges': 'bytes',
      'Cache-Control': 'public, max-age=3600',
    });
    res.end(slice);
    return;
  }
  onBytes?.(data.length);
  res.writeHead(200, {
    'Content-Type': contentType,
    'Content-Length': String(data.length),
    'Accept-Ranges': 'bytes',
    'Cache-Control': 'public, max-age=3600',
  });
  res.end(data);
}

async function startStaticProxyServer({ gatewayAddr, fpakDir, manifestPath, indexPath, probeOut }) {
  const stats = { fpakRequests: 0, fpakBytes: 0, staticRequests: 0, staticBytes: 0 };
  const resetStats = () => { stats.fpakRequests = 0; stats.fpakBytes = 0; stats.staticRequests = 0; stats.staticBytes = 0; };
  let clientIpOctet = 1;
  const nextClientIp = () => {
    const ip = `127.0.1.${clientIpOctet}`;
    clientIpOctet = (clientIpOctet % 250) + 1;
    return ip;
  };
  let currentClientIp = nextClientIp();
  const rotateClientIp = () => { currentClientIp = nextClientIp(); };

  const [gwHost, gwPortStr] = gatewayAddr.split(':');
  const gwPort = Number(gwPortStr);

  const server = createHttpServer((req, res) => {
    const urlPath = (req.url ?? '/').split('?')[0];

    if (urlPath.startsWith('/v1/')) {
      const proxy = httpRequest({
        hostname: gwHost,
        port: gwPort,
        localAddress: currentClientIp,
        path: req.url,
        method: req.method,
        headers: { ...req.headers, host: gatewayAddr },
      }, (upstream) => {
        res.writeHead(upstream.statusCode ?? 502, upstream.headers);
        upstream.pipe(res);
      });
      proxy.on('error', (err) => {
        res.writeHead(502);
        res.end(String(err));
      });
      req.pipe(proxy);
      return;
    }

    if (urlPath === '/__carentan/manifest.json') {
      serveFileWithRange(req, res, manifestPath);
      return;
    }
    if (urlPath === '/__carentan/index.json') {
      serveFileWithRange(req, res, indexPath);
      return;
    }
    if (urlPath.startsWith('/__probe/')) {
      const rel = urlPath.slice('/__probe/'.length);
      serveFileWithRange(req, res, join(probeOut, rel));
      return;
    }
    if (urlPath.startsWith('/a/')) {
      const rel = urlPath.slice(1);
      stats.fpakRequests++;
      serveFileWithRange(req, res, join(fpakDir, rel), (n) => { stats.fpakBytes += n; });
      return;
    }

    const rel = urlPath === '/' ? 'index.html' : urlPath.replace(/^\/+/, '');
    const target = join(DIST_DIR, rel);
    stats.staticRequests++;
    serveFileWithRange(req, res, target, (n) => { stats.staticBytes += n; });
  });

  await new Promise((res) => server.listen(0, '127.0.0.1', res));
  const addr = server.address();
  return {
    port: addr.port,
    stats,
    resetStats,
    rotateClientIp,
    close: () => new Promise((res) => server.close(() => res())),
  };
}

function summarize(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const n = sorted.length;
  const pick = (p) => sorted[Math.min(n - 1, Math.max(0, Math.ceil(n * p) - 1))];
  const mean = sorted.reduce((s, v) => s + v, 0) / n;
  return {
    n,
    min: Number(sorted[0].toFixed(1)),
    p50: Number(pick(0.5).toFixed(1)),
    p95: Number(pick(0.95).toFixed(1)),
    max: Number(sorted[n - 1].toFixed(1)),
    mean: Number(mean.toFixed(1)),
  };
}

async function runSingleAttempt({
  context,
  baseUrl,
  profile,
  clearOpfs,
  opfsDir,
  spawnPos,
  roleIdx,
  loadoutByTeam,
  httpStats,
  resetHttpStats,
  rotateClientIp,
  disableHttpCache,
}) {
  rotateClientIp();
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));

  // Force WebGL2 software path cleanly (consistent with client/e2e/ui-lobby.spec.ts).
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'gpu', { value: undefined });
  });

  const cdp = await context.newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.setCacheDisabled', { cacheDisabled: disableHttpCache });
  await cdp.send('Network.emulateNetworkConditions', {
    offline: false,
    latency: profile.latencyMs,
    downloadThroughput: Math.floor((profile.downMbps * 1024 * 1024) / 8),
    uploadThroughput: Math.floor((profile.upMbps * 1024 * 1024) / 8),
  });

  resetHttpStats();
  const tNavStart = performance.now();
  await page.goto(`${baseUrl}/`, { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'Click to continue' }).waitFor({ state: 'visible', timeout: 45_000 });
  const tBootReady = performance.now();

  await page.getByRole('button', { name: 'Click to continue' }).click();
  const quickJoin = page.getByRole('button', { name: 'Quick join' });
  await quickJoin.waitFor({ state: 'visible', timeout: 30_000 });
  // Wait for /v1/servers to enable Quick join.
  await page.waitForFunction(() => {
    const btns = [...document.querySelectorAll('button')];
    const b = btns.find((x) => x.textContent?.trim() === 'Quick join');
    return Boolean(b && !b.disabled);
  }, { timeout: 30_000 });
  const tMenuReady = performance.now();

  await quickJoin.click();
  const joinBtn = page.getByRole('button', { name: 'Join with automatic assignment' });
  await joinBtn.waitFor({ state: 'visible', timeout: 15_000 });

  // Measure click-to-playable starting from clicking Join!
  const tJoinClick = performance.now();
  await joinBtn.click();
  await page.getByLabel('Combat HUD').waitFor({ state: 'visible', timeout: 45_000 });
  await page.waitForFunction(() => Boolean((window).__netGame), { timeout: 15_000 });
  const tHudConnected = performance.now();

  // Inject the local measurement probe module (unthrottled CDP script import from /__probe/probe.js).
  // Pause throttling briefly ONLY to load the tiny measurement harness script itself, then restore throttling
  // before starting C0 DeployRequest + R_near OPFS prefetch so all real traffic is throttled.
  await cdp.send('Network.emulateNetworkConditions', {
    offline: false,
    latency: 0,
    downloadThroughput: -1,
    uploadThroughput: -1,
  });
  await page.evaluate(async () => {
    await import('/__probe/probe.js');
  });
  await cdp.send('Network.emulateNetworkConditions', {
    offline: false,
    latency: profile.latencyMs,
    downloadThroughput: Math.floor((profile.downMbps * 1024 * 1024) / 8),
    uploadThroughput: Math.floor((profile.upMbps * 1024 * 1024) / 8),
  });

  const fpakReqBefore = httpStats.fpakRequests;
  const fpakBytesBefore = httpStats.fpakBytes;

  const probe = await page.evaluate(
    (opts) => (window).__deployProbe.runDeployAndPrefetch(opts),
    {
      manifestUrl: '/__carentan/manifest.json',
      indexUrl: '/__carentan/index.json',
      spawnPos,
      ring: 4,
      roleIdx,
      loadoutByTeam,
      clearOpfs,
      opfsDir,
    },
  );
  const tPlayable = performance.now();

  const fpakRequests = httpStats.fpakRequests - fpakReqBefore;
  const fpakBytes = httpStats.fpakBytes - fpakBytesBefore;

  // Cleanly close WS & page so server releases the slot before next run.
  await page.evaluate(() => {
    (window).__netGame?.dispose();
  }).catch(() => {});
  await page.close();
  await new Promise((r) => setTimeout(r, 150));

  if (errors.length > 0) {
    throw new Error(`Page errors during run: ${errors.join('; ')}`);
  }

  const navToBootMs = tBootReady - tNavStart;
  const bootToMenuMs = tMenuReady - tBootReady;
  const joinToHudMs = tHudConnected - tJoinClick;
  // Click-to-playable excludes the tiny unthrottled probe injection by summing joinToHudMs + deployToPlayableMs.
  const clickToPlayableMs = joinToHudMs + probe.deployToPlayableMs;
  const navToPlayableMs = navToBootMs + bootToMenuMs + clickToPlayableMs;

  return {
    navToBootMs: Number(navToBootMs.toFixed(1)),
    bootToMenuMs: Number(bootToMenuMs.toFixed(1)),
    joinToHudMs: Number(joinToHudMs.toFixed(1)),
    c0AckMs: Number(probe.c0AckMs.toFixed(1)),
    spawnAliveMs: Number(probe.spawnAliveMs.toFixed(1)),
    spawnWaitTicks: probe.spawnWaitTicks,
    prefetchMs: Number(probe.prefetchMs.toFixed(1)),
    deployToPlayableMs: Number(probe.deployToPlayableMs.toFixed(1)),
    clickToPlayableMs: Number(clickToPlayableMs.toFixed(1)),
    navToPlayableMs: Number(navToPlayableMs.toFixed(1)),
    wallTotalMs: Number((tPlayable - tNavStart).toFixed(1)),
    prefetchReady: probe.prefetchReady,
    prefetchTotal: probe.prefetchTotal,
    prefetchFailed: probe.prefetchFailed,
    prefetchRate: Number((probe.prefetchRate * 100).toFixed(2)),
    visualResident: probe.visualResident,
    collisionInstalled: probe.collisionInstalled,
    cacheMode: probe.cacheMode,
    opfsSync: probe.opfsSync,
    fpakRequests,
    fpakBytes,
    team: probe.team,
  };
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  for (const [label, p] of [
    ['frontline-gateway binary', opts.gatewayBin],
    ['frontline-server binary', opts.serverBin],
    ['production client/dist', join(DIST_DIR, 'index.html')],
  ]) {
    if (!existsSync(p)) {
      throw new Error(`Missing ${label} at ${p}. Build gateway/server/sim-wasm and client/dist first.`);
    }
  }

  const requireFromClient = createRequire(join(CLIENT_DIR, 'package.json'));
  const { chromium } = requireFromClient('@playwright/test');

  const workDir = mkdtempSync(join(tmpdir(), 'frontline-deploy-timing-'));
  let gateway;
  let server;
  let httpSrv;
  let browser;

  try {
    const assets = prepareCarentanAssets(workDir);
    const probeOut = await buildProbeBundle(workDir);

    const spawns = JSON.parse(readFileSync(join(REPO_ROOT, 'data/maps/carentan/m1/spawns.json'), 'utf8'));
    // US Garrison candidate (650, 1470) is interior to Carentan M1 slice with 81/81 (100%) baked R_NEAR=4 chunks.
    const [sx, sz] = spawns.garrison_candidates.US.pos;
    const spawnPos = { x: sx, z: sz };
    const roleIdx = dataIndex('roles', 'rifleman');
    const loadoutByTeam = [dataIndex('loadouts', 'us_rifleman_1'), dataIndex('loadouts', 'ger_rifleman_1')];

    const internalToken = 'measure-deploy-local-token';
    gateway = launchProc(opts.gatewayBin, [], workDir, {
      GATEWAY_BIND: '127.0.0.1:0',
      INTERNAL_API_TOKEN: internalToken,
      TOKEN_SECRET: 'measure-deploy-disposable-secret',
      DATABASE_URL: 'sqlite::memory:',
      RUST_LOG: 'info',
      NO_COLOR: '1',
    });
    const gatewayAddr = await gateway.ready(() => /addr=(127\.0\.0\.1:\d+)/.exec(gateway.output())?.[1]);
    console.log(`[measure-deploy] gateway listening on ${gatewayAddr} (pid=${gateway.pid})`);

    server = launchProc(opts.serverBin, [
      '--data-dir', assets.dataDir,
      '--map', 'carentan/m1',
      '--mode', 'assault',
      '--ws-port', '0',
      '--wt-port', '0',
      '--metrics-port', '0',
      '--gateway-url', `http://${gatewayAddr}`,
      '--match-id', '00000000-0000-4000-8000-000000000010',
    ], workDir, { INTERNAL_API_TOKEN: internalToken });
    await server.ready(() => serverEvent(server, 'gateway_registered'));
    const wsPort = serverEvent(server, 'ws_listening')?.port;
    console.log(`[measure-deploy] server registered (ws_port=${wsPort}, pid=${server.pid})`);

    httpSrv = await startStaticProxyServer({
      gatewayAddr,
      fpakDir: assets.fpakDir,
      manifestPath: assets.manifestPath,
      indexPath: join(assets.chunksDir, 'index.json'),
      probeOut,
    });
    const baseUrl = `http://127.0.0.1:${httpSrv.port}`;
    console.log(`[measure-deploy] static/range/proxy server on ${baseUrl}`);

    browser = await chromium.launch({
      headless: true,
      args: [
        '--enable-unsafe-swiftshader',
        '--use-angle=swiftshader',
        '--ignore-gpu-blocklist',
      ],
    });
    const browserVersion = browser.version();
    console.log(`[measure-deploy] Chromium ${browserVersion} (SwiftShader WebGL2 software rendering)`);

    const results = [];
    for (const profile of opts.profiles) {
      for (const cacheState of ['cold', 'warm']) {
        const opfsDir = `frontline-deploy-${profile.name}`;
        const samples = [];
        if (cacheState === 'cold') {
          for (let i = 1; i <= opts.runs; i++) {
            const context = await browser.newContext({ viewport: { width: 960, height: 540 } });
            try {
              const s = await runSingleAttempt({
                context,
                baseUrl,
                profile,
                clearOpfs: true,
                opfsDir,
                spawnPos,
                roleIdx,
                loadoutByTeam,
                httpStats: httpSrv.stats,
                resetHttpStats: httpSrv.resetStats,
                rotateClientIp: httpSrv.rotateClientIp,
                disableHttpCache: true,
              });
              samples.push(s);
              console.log(
                `[${profile.name}][cold #${i}/${opts.runs}] click->playable=${s.clickToPlayableMs}ms ` +
                `(join->hud=${s.joinToHudMs}ms, c0=${s.c0AckMs}ms, alive=${s.spawnAliveMs}ms, prefetch=${s.prefetchMs}ms) ` +
                `R_near=${s.prefetchReady}/${s.prefetchTotal} (${s.prefetchRate}%) fpakReq=${s.fpakRequests} (${s.fpakBytes} B) cache=${s.cacheMode}`,
              );
            } finally {
              await context.close();
            }
          }
        } else {
          // Warm OPFS: use one BrowserContext, run 1 warmup pass to populate OPFS + HTTP cache,
          // then run N fresh pages in the same BrowserContext so JS RAM LRU is empty while OPFS is warm.
          const context = await browser.newContext({ viewport: { width: 960, height: 540 } });
          try {
            const warmSeed = await runSingleAttempt({
              context,
              baseUrl,
              profile,
              clearOpfs: true,
              opfsDir,
              spawnPos,
              roleIdx,
              loadoutByTeam,
              httpStats: httpSrv.stats,
              resetHttpStats: httpSrv.resetStats,
              rotateClientIp: httpSrv.rotateClientIp,
              disableHttpCache: false,
            });
            console.log(
              `[${profile.name}][warm seed] R_near=${warmSeed.prefetchReady}/${warmSeed.prefetchTotal} ` +
              `fpakReq=${warmSeed.fpakRequests} (${warmSeed.fpakBytes} B) cache=${warmSeed.cacheMode}`,
            );
            for (let i = 1; i <= opts.runs; i++) {
              const s = await runSingleAttempt({
                context,
                baseUrl,
                profile,
                clearOpfs: false,
                opfsDir,
                spawnPos,
                roleIdx,
                loadoutByTeam,
                httpStats: httpSrv.stats,
                resetHttpStats: httpSrv.resetStats,
                rotateClientIp: httpSrv.rotateClientIp,
                disableHttpCache: false,
              });
              samples.push(s);
              console.log(
                `[${profile.name}][warm #${i}/${opts.runs}] click->playable=${s.clickToPlayableMs}ms ` +
                `(join->hud=${s.joinToHudMs}ms, c0=${s.c0AckMs}ms, alive=${s.spawnAliveMs}ms, prefetch=${s.prefetchMs}ms) ` +
                `R_near=${s.prefetchReady}/${s.prefetchTotal} (${s.prefetchRate}%) fpakReq=${s.fpakRequests} (${s.fpakBytes} B) cache=${s.cacheMode}`,
              );
            }
          } finally {
            await context.close();
          }
        }

        const summary = {
          profile: profile.name,
          label: profile.label,
          downMbps: profile.downMbps,
          upMbps: profile.upMbps,
          latencyMs: profile.latencyMs,
          cacheState,
          runs: samples.length,
          clickToPlayableMs: summarize(samples.map((s) => s.clickToPlayableMs)),
          navToPlayableMs: summarize(samples.map((s) => s.navToPlayableMs)),
          joinToHudMs: summarize(samples.map((s) => s.joinToHudMs)),
          c0AckMs: summarize(samples.map((s) => s.c0AckMs)),
          spawnAliveMs: summarize(samples.map((s) => s.spawnAliveMs)),
          prefetchMs: summarize(samples.map((s) => s.prefetchMs)),
          prefetchRatePct: summarize(samples.map((s) => s.prefetchRate)),
          fpakRequests: summarize(samples.map((s) => s.fpakRequests)),
          fpakBytes: summarize(samples.map((s) => s.fpakBytes)),
          samples,
        };
        results.push(summary);
      }
    }

    const report = {
      schema: 1,
      measuredAt: new Date().toISOString(),
      nonReferenceWorkstation: true,
      note: 'PI-109 workstation measurement (headless Chromium SwiftShader WebGL2, local gateway+server, Carentan M1 terrain .fpak).',
      browserVersion,
      spawnPrefetchTarget: { ...spawnPos, ring: 4, expectedChunks: 81 },
      carentanFpak: {
        chunks: assets.bakeReport.chunks,
        packs: assets.bakeReport.fpak.packs,
        totalBytes: assets.bakeReport.fpak.total_fpak_bytes,
      },
      results,
    };

    if (opts.jsonOut) {
      mkdirSync(dirname(opts.jsonOut), { recursive: true });
      writeFileSync(opts.jsonOut, JSON.stringify(report, null, 2) + '\n');
      console.log(`[measure-deploy] wrote JSON report to ${opts.jsonOut}`);
    } else {
      console.log(JSON.stringify(report, null, 2));
    }
  } finally {
    await browser?.close().catch(() => {});
    await httpSrv?.close().catch(() => {});
    await server?.stop().catch(() => {});
    await gateway?.stop().catch(() => {});
    rmSync(workDir, { recursive: true, force: true });
  }
}

main().catch((err) => {
  console.error('[measure-deploy] FATAL:', err);
  process.exit(1);
});
